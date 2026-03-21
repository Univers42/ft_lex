use std::process;

fn print_usage() {
    eprintln!("Usage: ft_lex [-t] [-n] [-v] [-e] [file.l]");
    eprintln!("  -t    Write output to stdout instead of lex.yy.c");
    eprintln!("  -n    Suppress the default rule");
    eprintln!("  -v    Write statistics to stderr");
    eprintln!("  -e    Use extended (8-bit) mode");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        process::exit(1);
    }

    // Stub: will be wired to ft_lex_lib pipeline
    eprintln!("ft_lex: not yet implemented");
    process::exit(1);
}
