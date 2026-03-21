use std::fs;
use std::process;

use ft_lex_lib::automata::dfa::subset_construction;
use ft_lex_lib::automata::minimize::minimize_dfa;
use ft_lex_lib::automata::nfa::NfaBuilder;
use ft_lex_lib::emit::c_emitter::CEmitter;
use ft_lex_lib::emit::traits::CodeEmitter;
use ft_lex_lib::lex_file::parser::LexFile;

struct Options {
    to_stdout: bool,
    suppress_default: bool,
    verbose: bool,
    input_file: String,
}

fn print_usage() {
    eprintln!("Usage: ft_lex [-t] [-n] [-v] [-e] [file.l]");
    eprintln!("  -t    Write output to stdout instead of lex.yy.c");
    eprintln!("  -n    Suppress the default rule");
    eprintln!("  -v    Write statistics to stderr");
    eprintln!("  -e    Use extended (8-bit) mode");
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        to_stdout: false,
        suppress_default: false,
        verbose: false,
        input_file: String::new(),
    };

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
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
    }

    if opts.input_file.is_empty() {
        return Err("no input file specified".to_string());
    }

    Ok(opts)
}

fn run(opts: &Options) -> Result<(), String> {
    // 1. Read the .l file
    let source = fs::read_to_string(&opts.input_file)
        .map_err(|e| format!("{}: {}", opts.input_file, e))?;

    // 2. Parse the .l file
    let lex_file = LexFile::parse(&source, &opts.input_file)
        .map_err(|e| e.to_string())?;

    if lex_file.rules.is_empty() {
        return Err(format!("{}: no rules defined", opts.input_file));
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
    }

    // 7. Emit C code
    let emitter = CEmitter::new(opts.suppress_default);
    let output = emitter.emit(&lex_file, &min_dfa)
        .map_err(|e| e.to_string())?;

    // 8. Write output
    if opts.to_stdout {
        print!("{}", output);
    } else {
        fs::write("lex.yy.c", &output)
            .map_err(|e| format!("lex.yy.c: {}", e))?;
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        process::exit(1);
    }

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
