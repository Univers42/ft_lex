/// Integration tests for ft_lex.
///
/// These tests run the full pipeline: .l file → ft_lex → lex.yy.c → compile → run → check output.
/// They use the test fixtures in tests/fixtures/.
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn ft_lex_binary() -> PathBuf {
    let root = workspace_root();
    // Try debug build first, then release
    let debug = root.join("target/debug/ft_lex");
    if debug.exists() {
        return debug;
    }
    root.join("target/release/ft_lex")
}

fn fixtures_dir() -> PathBuf {
    workspace_root().join("tests/fixtures")
}

fn libl_path() -> PathBuf {
    workspace_root().join("libl.a")
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ft_lex_test_{}_{}", std::process::id(), name
    ));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Run a full end-to-end test:
/// 1. Run ft_lex on the .l file to produce lex.yy.c (via -t flag to stdout)
/// 2. Compile lex.yy.c with cc
/// 3. Run the resulting binary on the input file
/// 4. Compare output to expected
fn run_e2e_test(
    l_file: &str,
    input_file: &str,
    expected_file: &str,
    extra_flags: &[&str],
    needs_libl: bool,
) {
    let fixtures = fixtures_dir();
    let l_path = fixtures.join(l_file);
    let input_path = fixtures.join(input_file);
    let expected_path = fixtures.join(expected_file);

    let tmp_dir = tempdir(l_file);
    let lex_yy_c = tmp_dir.join("lex.yy.c");
    let binary = tmp_dir.join(format!("scanner_{}", l_file.replace('.', "_")));

    // Step 1: Run ft_lex -t
    let mut cmd = Command::new(ft_lex_binary());
    cmd.arg("-t");
    for flag in extra_flags {
        cmd.arg(flag);
    }
    cmd.arg(&l_path);

    let output = cmd.output().expect("failed to run ft_lex");
    assert!(
        output.status.success(),
        "ft_lex failed on {}: {}",
        l_file,
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(&lex_yy_c, &output.stdout).expect("failed to write lex.yy.c");

    // Step 2: Compile lex.yy.c
    let mut cc_args: Vec<String> = vec![
        "-o".to_string(),
        binary.to_string_lossy().to_string(),
        lex_yy_c.to_string_lossy().to_string(),
    ];

    if needs_libl {
        let libl = libl_path();
        if libl.exists() {
            cc_args.push(libl.to_string_lossy().to_string());
        } else {
            // Build libl.a on the fly
            let root = workspace_root();
            let libl_c = root.join("libl.c");
            let libl_o = tmp_dir.join("libl.o");
            let libl_a = tmp_dir.join("libl.a");
            let status = Command::new("cc")
                .arg("-c")
                .arg(&libl_c)
                .arg("-o")
                .arg(&libl_o)
                .status()
                .expect("failed to compile libl.c");
            assert!(status.success(), "failed to compile libl.c");
            let status = Command::new("ar")
                .arg("rcs")
                .arg(&libl_a)
                .arg(&libl_o)
                .status()
                .expect("failed to create libl.a");
            assert!(status.success(), "failed to create libl.a");
            cc_args.push(libl_a.to_string_lossy().to_string());
        }
    }

    let compile = Command::new("cc")
        .args(&cc_args)
        .output()
        .expect("failed to compile lex.yy.c");
    assert!(
        compile.status.success(),
        "cc failed for {}: {}",
        l_file,
        String::from_utf8_lossy(&compile.stderr)
    );

    // Step 3: Run the scanner
    let input = fs::read(&input_path).expect("failed to read input file");
    let mut child = Command::new(&binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run scanner");

    use std::io::Write;
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let result = child.wait_with_output().expect("failed to wait for scanner");

    let actual = String::from_utf8_lossy(&result.stdout).to_string();
    let expected = fs::read_to_string(&expected_path).expect("failed to read expected file");

    assert_eq!(
        actual.trim(),
        expected.trim(),
        "Output mismatch for {}\n\nActual:\n{}\n\nExpected:\n{}",
        l_file,
        actual,
        expected
    );

    // Clean up
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_subject_scanner() {
    run_e2e_test(
        "scanner.l",
        "scanner_input.txt",
        "scanner_expected.txt",
        &[],
        true,
    );
}

#[test]
fn test_tokenizer() {
    run_e2e_test(
        "tokenizer.l",
        "tokenizer_input.txt",
        "tokenizer_expected.txt",
        &[],
        true, // needs libl for main()
    );
}

#[test]
fn test_keywords_priority() {
    run_e2e_test(
        "keywords.l",
        "keywords_input.txt",
        "keywords_expected.txt",
        &[],
        false, // defines its own main + yywrap
    );
}

#[test]
fn test_ft_lex_no_args() {
    let output = Command::new(ft_lex_binary())
        .output()
        .expect("failed to run ft_lex");
    assert!(!output.status.success());
}

#[test]
fn test_ft_lex_nonexistent_file() {
    let output = Command::new(ft_lex_binary())
        .arg("/nonexistent/file.l")
        .output()
        .expect("failed to run ft_lex");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No such file") || stderr.contains("not found") || stderr.contains("error"),
        "Expected error message, got: {}",
        stderr
    );
}

#[test]
fn test_ft_lex_verbose_flag() {
    let l_path = fixtures_dir().join("scanner.l");
    let output = Command::new(ft_lex_binary())
        .arg("-tv")
        .arg(&l_path)
        .output()
        .expect("failed to run ft_lex");
    assert!(
        output.status.success(),
        "ft_lex -tv failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("rules"), "Expected 'rules' in verbose output");
    assert!(stderr.contains("NFA states"), "Expected 'NFA states' in verbose output");
    assert!(stderr.contains("DFA states"), "Expected 'DFA states' in verbose output");
}
