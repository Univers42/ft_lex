# ft_lex

A POSIX-compliant `lex` implementation in Rust.

## Architecture

This is a Cargo workspace with two crates:

- **`ft_lex`** — thin CLI binary wrapping the library
- **`ft_lex_lib`** — all reusable logic (regex engine, automata, code emission)

### Pipeline

```
.l file → parse sections → parse regex patterns → Thompson's NFA
       → subset construction DFA → Hopcroft minimize → emit lex.yy.c
```

### Module layout (ft_lex_lib)

| Module | Purpose |
|---|---|
| `error` | Unified `LexError` type with file/line/col tracking |
| `regex/` | Regex AST, recursive descent parser, POSIX character classes |
| `automata/` | Arena allocator, NFA (Thompson), DFA (subset construction), minimization (Hopcroft) |
| `lex_file/` | `.l` file tokenizer and parser |
| `emit/` | `CodeEmitter` trait, C emitter, libl emitter |
| `compress/` | Bonus: DFA table compression via equivalence classes |

## Building

```sh
make        # builds ft_lex binary and libl.a
make clean  # remove build artifacts
make re     # clean + rebuild
```

## Usage

```sh
./ft_lex [-t] [-n] [-v] [-e] file.l
```

## Roadmap

This is step 1 of the **ft_lex → ft_yacc → cc1** compiler toolchain.
