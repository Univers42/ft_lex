use std::fs;
use std::process;

use ft_lex_lib::automata::dfa::subset_construction;
use ft_lex_lib::automata::minimize::minimize_dfa;
use ft_lex_lib::automata::nfa::NfaBuilder;
use ft_lex_lib::emit::c_emitter::CEmitter;
use ft_lex_lib::emit::rust_emitter::RustEmitter;
use ft_lex_lib::emit::traits::CodeEmitter;
use ft_lex_lib::lex_file::parser::LexFile;

#[derive(Clone, Copy, PartialEq)]
enum TargetLang {
    C,
    Rust,
}

struct Options {
    to_stdout: bool,
    suppress_default: bool,
    verbose: bool,
    compress: bool,
    lang: TargetLang,
    input_file: String,
}

fn print_usage() {
    eprintln!("Usage: ft_lex [-t] [-n] [-v] [-e] [--compress] [--lang c|rust] [file.l]");
    eprintln!("  -t           Write output to stdout instead of lex.yy.c / lex.yy.rs");
    eprintln!("  -n           Suppress the default rule");
    eprintln!("  -v           Write statistics to stderr");
    eprintln!("  -e           Use extended (8-bit) mode");
    eprintln!("  --compress   Compress DFA tables using equivalence classes");
    eprintln!("  --lang LANG  Target language: c (default) or rust");
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        to_stdout: false,
        suppress_default: false,
        verbose: false,
        compress: false,
        lang: TargetLang::C,
        input_file: String::new(),
    };

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--compress" {
            opts.compress = true;
        } else if arg == "--lang" {
            i += 1;
            if i >= args.len() {
                return Err("--lang requires an argument (c or rust)".to_string());
            }
            match args[i].as_str() {
                "c" | "C" => opts.lang = TargetLang::C,
                "rust" | "rs" | "Rust" => opts.lang = TargetLang::Rust,
                other => return Err(format!("unknown language: {}", other)),
            }
        } else if arg.starts_with("--") {
            return Err(format!("unknown option: {}", arg));
        } else if arg.starts_with('-') && arg.len() > 1 {
            for ch in arg[1..].chars() {
                match ch {
                    't' => opts.to_stdout = true,
                    'n' => opts.suppress_default = true,
                    'v' => opts.verbose = true,
                    'e' => {} // 8-bit mode is always on (we process bytes)
                    _ => return Err(format!("unknown option: -{}", ch)),
                }
            }
        } else if opts.input_file.is_empty() {
            opts.input_file = arg.clone();
        } else {
            return Err("too many input files".to_string());
        }
        i += 1;
    }

    if opts.input_file.is_empty() {
        // POSIX: read from stdin if no operand is given
        opts.input_file = "-".to_string();
    }

    Ok(opts)
}

fn run(opts: &Options) -> Result<(), String> {
    // 1. Read the .l file (from file or stdin)
    let source = if opts.input_file == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("<stdin>: {}", e))?;
        buf
    } else {
        fs::read_to_string(&opts.input_file)
            .map_err(|e| format!("{}: {}", opts.input_file, e))?
    };

    let filename = if opts.input_file == "-" {
        "<stdin>"
    } else {
        &opts.input_file
    };

    // 2. Parse the .l file
    let lex_file = LexFile::parse(&source, filename)
        .map_err(|e| e.to_string())?;

    if lex_file.rules.is_empty() {
        return Err(format!("{}: no rules defined", filename));
    }

    // 3. Build NFA
    let mut nfa_builder = NfaBuilder::new();
    let rules: Vec<_> = lex_file
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.regex.clone(), i))
        .collect();
    let nfa = nfa_builder.build_combined(&rules);

    // 4. Convert to DFA
    let dfa = subset_construction(&nfa);

    // 5. Minimize DFA
    let min_dfa = minimize_dfa(&dfa);

    // 6. Print statistics if -v
    if opts.verbose {
        eprintln!("ft_lex: {} rules, {} NFA states, {} DFA states, {} minimized states",
            lex_file.rules.len(),
            nfa.state_count(),
            dfa.state_count(),
            min_dfa.state_count(),
        );
        if opts.compress {
            eprintln!("ft_lex: compression enabled (equivalence classes)");
        }
        eprintln!("ft_lex: target language: {}",
            match opts.lang { TargetLang::C => "C", TargetLang::Rust => "Rust" });
    }

    // 7. Emit code
    let emitter: Box<dyn CodeEmitter> = match (opts.lang, opts.compress) {
        (TargetLang::C, false) => Box::new(CEmitter::new(opts.suppress_default)),
        (TargetLang::C, true) => Box::new(CEmitter::with_compression(opts.suppress_default)),
        (TargetLang::Rust, false) => Box::new(RustEmitter::new(opts.suppress_default)),
        (TargetLang::Rust, true) => Box::new(RustEmitter::with_compression(opts.suppress_default)),
    };
    let output = emitter.emit(&lex_file, &min_dfa)
        .map_err(|e| e.to_string())?;

    // 8. Write output
    let out_file = match opts.lang {
        TargetLang::C => "lex.yy.c",
        TargetLang::Rust => "lex.yy.rs",
    };
    if opts.to_stdout {
        print!("{}", output);
    } else {
        fs::write(out_file, &output)
            .map_err(|e| format!("{}: {}", out_file, e))?;
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // POSIX: if no args, read from stdin (parse_args handles empty input_file)
    let opts = match parse_args(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("ft_lex: {}", e);
            print_usage();
            process::exit(1);
        }
    };

    if let Err(e) = run(&opts) {
        eprintln!("ft_lex: {}", e);
        process::exit(1);
    }
}
