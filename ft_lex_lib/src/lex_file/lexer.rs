use crate::error::{LexError, LexErrorKind};

/// Sections of a .l file, separated by %%
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    Definitions,
    Rules,
    UserCode,
}

/// Raw representation of a parsed .l file before regex compilation.
#[derive(Debug, Clone)]
pub struct RawLexFile {
    /// C code from %{ ... %} blocks in the definitions section.
    pub header_code: Vec<String>,
    /// Named definitions: (name, pattern_string).
    pub definitions: Vec<(String, String)>,
    /// %option directives: list of option strings.
    pub options: Vec<String>,
    /// Rules: (pattern_string, action_string, line_number).
    pub rules: Vec<RawRule>,
    /// Verbatim user code section (after second %%).
    pub user_code: String,
}

#[derive(Debug, Clone)]
pub struct RawRule {
    pub pattern: String,
    pub action: String,
    pub line: usize,
}

impl RawLexFile {
    pub fn new() -> Self {
        Self {
            header_code: Vec::new(),
            definitions: Vec::new(),
            options: Vec::new(),
            rules: Vec::new(),
            user_code: String::new(),
        }
    }
}

/// Tokenize and parse a .l file into its three sections.
pub fn parse_l_file(input: &str, filename: &str) -> Result<RawLexFile, LexError> {
    let mut result = RawLexFile::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    let mut section = Section::Definitions;

    while i < lines.len() {
        let line = lines[i];

        match section {
            Section::Definitions => {
                if line.starts_with("%%") {
                    section = Section::Rules;
                    i += 1;
                    continue;
                }

                // %{ ... %} code block
                if line.trim_start().starts_with("%{") {
                    i += 1;
                    let mut code = String::new();
                    while i < lines.len() {
                        if lines[i].trim_start().starts_with("%}") {
                            break;
                        }
                        code.push_str(lines[i]);
                        code.push('\n');
                        i += 1;
                    }
                    if i >= lines.len() {
                        return Err(LexError::new(
                            filename,
                            i,
                            0,
                            LexErrorKind::UnterminatedCodeBlock,
                        ));
                    }
                    result.header_code.push(code);
                    i += 1;
                    continue;
                }

                // %option directive
                if line.trim_start().starts_with("%option") {
                    let opt = line.trim_start().strip_prefix("%option").unwrap_or("").trim();
                    result.options.push(opt.to_string());
                    i += 1;
                    continue;
                }

                // Empty / comment line
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    i += 1;
                    continue;
                }

                // C-style multi-line comments
                if trimmed.starts_with("/*") {
                    if !trimmed.contains("*/") {
                        // Multi-line comment — skip until */
                        i += 1;
                        while i < lines.len() {
                            if lines[i].contains("*/") {
                                break;
                            }
                            i += 1;
                        }
                    }
                    i += 1;
                    continue;
                }

                // C++ style comments 
                if trimmed.starts_with("//") {
                    i += 1;
                    continue;
                }

                // POSIX: Lines starting with whitespace in definitions section
                // are copied verbatim to the output (indented code).
                if line.starts_with(' ') || line.starts_with('\t') {
                    result.header_code.push(format!("{}\n", line));
                    i += 1;
                    continue;
                }

                // Named definition: NAME  PATTERN
                // Name starts at column 0, followed by whitespace, then pattern
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    if let Some(ws_pos) = line.find(|c: char| c == ' ' || c == '\t') {
                        let name = line[..ws_pos].to_string();
                        let pattern = line[ws_pos..].trim().to_string();
                        if !name.is_empty() && !pattern.is_empty() {
                            result.definitions.push((name, pattern));
                            i += 1;
                            continue;
                        }
                    }
                }

                i += 1;
            }

            Section::Rules => {
                if line.starts_with("%%") {
                    section = Section::UserCode;
                    i += 1;
                    continue;
                }

                let trimmed = line.trim();

                // Skip empty lines and C-style comments
                if trimmed.is_empty() {
                    i += 1;
                    continue;
                }

                // A rule line: pattern<whitespace>action
                let rule_line = i + 1; // 1-based line number
                let (pattern, action_start) = extract_pattern(line, filename, rule_line)?;

                if pattern.is_empty() {
                    i += 1;
                    continue;
                }

                let rest = &line[action_start..];
                let action = if rest.trim() == "|" {
                    // | means "same action as next rule" — store as-is
                    "|".to_string()
                } else if rest.trim_start().starts_with('{') && !rest.trim_start().contains('}') {
                    // Multi-line action block
                    let mut action = rest.trim_start().to_string();
                    action.push('\n');
                    let mut brace_depth = 0i32;
                    for c in rest.trim_start().chars() {
                        match c {
                            '{' => brace_depth += 1,
                            '}' => brace_depth -= 1,
                            _ => {}
                        }
                    }
                    i += 1;
                    while i < lines.len() && brace_depth > 0 {
                        action.push_str(lines[i]);
                        action.push('\n');
                        for c in lines[i].chars() {
                            match c {
                                '{' => brace_depth += 1,
                                '}' => brace_depth -= 1,
                                _ => {}
                            }
                        }
                        i += 1;
                    }
                    if brace_depth > 0 {
                        return Err(LexError::new(
                            filename,
                            rule_line,
                            0,
                            LexErrorKind::UnterminatedAction,
                        ));
                    }
                    action
                } else {
                    rest.trim().to_string()
                };

                result.rules.push(RawRule {
                    pattern,
                    action,
                    line: rule_line,
                });
                if !matches!(section, Section::UserCode) {
                    i += 1;
                }
            }

            Section::UserCode => {
                // Everything after the second %% is verbatim user code
                let remaining: Vec<&str> = lines[i..].to_vec();
                result.user_code = remaining.join("\n");
                if !result.user_code.is_empty() {
                    result.user_code.push('\n');
                }
                break;
            }
        }
    }

    // Resolve '|' actions — a rule with action "|" inherits the next rule's action
    resolve_pipe_actions(&mut result.rules)?;

    Ok(result)
}

/// Extract the pattern portion from a rule line, handling quoted strings
/// and brackets properly. Returns (pattern, byte_offset_of_action_start).
fn extract_pattern(line: &str, _filename: &str, _line_num: usize) -> Result<(String, usize), LexError> {
    let bytes = line.as_bytes();
    let mut i = 0;

    // Skip leading whitespace
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }

    if i >= bytes.len() {
        return Ok((String::new(), i));
    }

    let start = i;
    let mut in_brackets = false;
    let mut in_quotes = false;

    while i < bytes.len() {
        let b = bytes[i];

        if in_quotes {
            if b == b'"' {
                in_quotes = false;
            } else if b == b'\\' {
                i += 1; // skip escaped char
            }
            i += 1;
            continue;
        }

        if in_brackets {
            if b == b']' {
                in_brackets = false;
            } else if b == b'\\' {
                i += 1; // skip escaped char
            }
            i += 1;
            continue;
        }

        match b {
            b'"' => {
                in_quotes = true;
                i += 1;
            }
            b'[' => {
                in_brackets = true;
                i += 1;
            }
            b'\\' => {
                i += 2;
            }
            b' ' | b'\t' => {
                // End of pattern (unquoted, unbracketed whitespace)
                break;
            }
            _ => {
                i += 1;
            }
        }
    }

    let pattern = line[start..i].to_string();
    Ok((pattern, i))
}

/// Resolve pipe actions: rules with action "|" get the next rule's action.
fn resolve_pipe_actions(rules: &mut Vec<RawRule>) -> Result<(), LexError> {
    // Walk backwards — a "|" action inherits from the next non-pipe rule
    let n = rules.len();
    let mut i = n;
    while i > 0 {
        i -= 1;
        if rules[i].action == "|" {
            if i + 1 < n {
                let next_action = rules[i + 1].action.clone();
                rules[i].action = next_action;
            }
            // If it's the last rule with "|", leave it empty (will be a no-op)
        }
    }
    Ok(())
}

/// Expand {name} references in a pattern string using the definitions.
pub fn expand_definitions(pattern: &str, definitions: &[(String, String)], filename: &str, line: usize) -> Result<String, LexError> {
    let mut result = String::new();
    let bytes = pattern.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Check if this is a {name} (not a repeat {n,m})
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j >= bytes.len() {
                result.push('{');
                i += 1;
                continue;
            }
            let name = &pattern[start..j];
            // Check if it's a repeat: all digits/commas
            if name.chars().all(|c| c.is_ascii_digit() || c == ',') {
                // It's a repeat quantifier, keep as-is
                result.push('{');
                i += 1;
                continue;
            }
            // Look up definition
            if let Some((_, def_pattern)) = definitions.iter().find(|(n, _)| n == name) {
                // Wrap in group to preserve semantics
                result.push('(');
                result.push_str(def_pattern);
                result.push(')');
                i = j + 1;
            } else {
                return Err(LexError::new(
                    filename,
                    line,
                    i + 1,
                    LexErrorKind::UndefinedName(name.to_string()),
                ));
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_l_file() {
        let input = "%%\n. printf(\"matched\\n\");\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.rules.len(), 1);
        assert_eq!(result.rules[0].pattern, ".");
        assert!(result.rules[0].action.contains("printf"));
    }

    #[test]
    fn test_definitions_section() {
        let input = "DIGIT [0-9]\nALPHA [a-zA-Z]\n%%\n{DIGIT}+ printf(\"num\");\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.definitions.len(), 2);
        assert_eq!(result.definitions[0].0, "DIGIT");
        assert_eq!(result.definitions[0].1, "[0-9]");
        assert_eq!(result.definitions[1].0, "ALPHA");
    }

    #[test]
    fn test_code_block() {
        let input = "%{\n#include <stdio.h>\nint count = 0;\n%}\n%%\n. count++;\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.header_code.len(), 1);
        assert!(result.header_code[0].contains("#include <stdio.h>"));
        assert!(result.header_code[0].contains("int count = 0;"));
    }

    #[test]
    fn test_option_directive() {
        let input = "%option noyywrap\n%%\n. ;\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.options, vec!["noyywrap"]);
    }

    #[test]
    fn test_pipe_action() {
        let input = "%%\n\"+\" |\n\"-\" |\n\"*\" printf(\"op\");\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.rules.len(), 3);
        // All three should have the same action after pipe resolution
        assert!(result.rules[0].action.contains("printf"));
        assert!(result.rules[1].action.contains("printf"));
        assert!(result.rules[2].action.contains("printf"));
    }

    #[test]
    fn test_user_code_section() {
        let input = "%%\n. ;\n%%\nint main() { return yylex(); }\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert!(result.user_code.contains("int main()"));
    }

    #[test]
    fn test_subject_scanner() {
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
        let result = parse_l_file(input, "scanner.l").unwrap();
        assert_eq!(result.rules.len(), 10);
        assert_eq!(result.rules[0].pattern, "[0-9]+");
        assert_eq!(result.rules[1].pattern, "\"+\"");
        assert_eq!(result.rules[8].pattern, "[[:blank:]]");
        assert_eq!(result.rules[9].pattern, ".");
    }

    #[test]
    fn test_unterminated_code_block() {
        let input = "%{\n#include <stdio.h>\n%%\n. ;\n";
        let result = parse_l_file(input, "test.l");
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_definitions() {
        let defs = vec![
            ("DIGIT".to_string(), "[0-9]".to_string()),
            ("ALPHA".to_string(), "[a-zA-Z]".to_string()),
        ];
        let expanded = expand_definitions("{DIGIT}+", &defs, "test.l", 1).unwrap();
        assert_eq!(expanded, "([0-9])+");
    }

    #[test]
    fn test_expand_definitions_repeat() {
        let defs = vec![];
        // {3,5} should not be treated as a name reference
        let expanded = expand_definitions("a{3,5}", &defs, "test.l", 1).unwrap();
        assert_eq!(expanded, "a{3,5}");
    }

    #[test]
    fn test_expand_definitions_undefined() {
        let defs = vec![];
        let result = expand_definitions("{UNKNOWN}+", &defs, "test.l", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiline_action() {
        let input = "%%\n[0-9]+ {\n    int val = atoi(yytext);\n    printf(\"%d\", val);\n}\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.rules.len(), 1);
        assert!(result.rules[0].action.contains("int val"));
        assert!(result.rules[0].action.contains("printf"));
    }

    #[test]
    fn test_line_tracking() {
        let input = "%%\n. printf(\"a\");\n[0-9]+ printf(\"b\");\n";
        let result = parse_l_file(input, "test.l").unwrap();
        assert_eq!(result.rules[0].line, 2);
        assert_eq!(result.rules[1].line, 3);
    }
}
