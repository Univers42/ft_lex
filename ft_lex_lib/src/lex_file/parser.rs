// Full .l file parser — logic is in lexer.rs, this module
// provides the high-level API that combines parsing + regex compilation.

use crate::error::{LexError, LexErrorKind};
use crate::lex_file::lexer::{self, RawLexFile};
use crate::regex::ast::Regex;
use crate::regex::parser::parse_regex;

/// A fully parsed and compiled .l file, ready for NFA construction.
#[derive(Debug)]
pub struct LexFile {
    /// C code from %{ ... %} blocks, concatenated.
    pub header_code: String,
    /// Parsed rules with compiled regex patterns.
    pub rules: Vec<Rule>,
    /// Verbatim user code section.
    pub user_code: String,
    /// Options from %option directives.
    pub options: LexOptions,
}

/// A single rule with a compiled regex and its action.
#[derive(Debug)]
pub struct Rule {
    /// The original pattern string.
    pub pattern_str: String,
    /// The compiled regex AST.
    pub regex: Regex,
    /// The C action code.
    pub action: String,
    /// Priority (rule index — lower = higher priority for POSIX first-match).
    pub priority: usize,
    /// Source line number in the .l file.
    pub line: usize,
}

/// Options parsed from %option directives.
#[derive(Debug, Default)]
pub struct LexOptions {
    pub noyywrap: bool,
    pub yylineno: bool,
}

impl LexFile {
    /// Parse a .l file from source text.
    pub fn parse(input: &str, filename: &str) -> Result<Self, LexError> {
        let raw = lexer::parse_l_file(input, filename)?;
        compile_lex_file(raw, filename)
    }
}

fn compile_lex_file(raw: RawLexFile, filename: &str) -> Result<LexFile, LexError> {
    let header_code = raw.header_code.join("\n");

    let mut options = LexOptions::default();
    for opt in &raw.options {
        match opt.as_str() {
            "noyywrap" => options.noyywrap = true,
            "yylineno" => options.yylineno = true,
            _ => {
                return Err(LexError::new(
                    filename,
                    0,
                    0,
                    LexErrorKind::InvalidOption(opt.clone()),
                ));
            }
        }
    }

    let mut rules = Vec::with_capacity(raw.rules.len());
    for (i, raw_rule) in raw.rules.iter().enumerate() {
        // Expand {name} references
        let expanded = lexer::expand_definitions(
            &raw_rule.pattern,
            &raw.definitions,
            filename,
            raw_rule.line,
        )?;

        // Parse the regex
        let regex = parse_regex(expanded.as_bytes(), filename, raw_rule.line)?;

        rules.push(Rule {
            pattern_str: raw_rule.pattern.clone(),
            regex,
            action: raw_rule.action.clone(),
            priority: i,
            line: raw_rule.line,
        });
    }

    Ok(LexFile {
        header_code,
        rules,
        user_code: raw.user_code,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let input = "%%\n. printf(\"matched\");\n";
        let lf = LexFile::parse(input, "test.l").unwrap();
        assert_eq!(lf.rules.len(), 1);
        assert_eq!(lf.rules[0].priority, 0);
    }

    #[test]
    fn test_parse_with_definitions() {
        let input = "DIGIT [0-9]\n%%\n{DIGIT}+ printf(\"num\");\n";
        let lf = LexFile::parse(input, "test.l").unwrap();
        assert_eq!(lf.rules.len(), 1);
        // The pattern should have the definition expanded
    }

    #[test]
    fn test_parse_options() {
        let input = "%option noyywrap\n%%\n. ;\n";
        let lf = LexFile::parse(input, "test.l").unwrap();
        assert!(lf.options.noyywrap);
    }

    #[test]
    fn test_parse_subject_scanner() {
        let input = r#"%%
[0-9]+ printf("NUMBER: %s\n", yytext);
"+" |
"-" |
"*" |
"/" printf("OPERATOR: %s\n", yytext);
"(" printf("OPEN PARENTHESIS\n");
")" printf("CLOSED PARENTHESIS\n");
"\n" printf("NEWLINE\n");
[[:blank:]] ;
. printf("Invalid character: %c\n", *yytext);
"#;
        let lf = LexFile::parse(input, "scanner.l").unwrap();
        assert_eq!(lf.rules.len(), 10);
        // Verify priorities are sequential
        for (i, rule) in lf.rules.iter().enumerate() {
            assert_eq!(rule.priority, i);
        }
    }

    #[test]
    fn test_parse_header_code() {
        let input = "%{\n#include <stdio.h>\n%}\n%%\n. ;\n";
        let lf = LexFile::parse(input, "test.l").unwrap();
        assert!(lf.header_code.contains("#include <stdio.h>"));
    }

    #[test]
    fn test_parse_user_code() {
        let input = "%%\n. ;\n%%\nint main() { return yylex(); }\n";
        let lf = LexFile::parse(input, "test.l").unwrap();
        assert!(lf.user_code.contains("int main()"));
    }
}
