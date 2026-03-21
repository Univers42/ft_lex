use std::fmt;

/// Unified error type for all ft_lex operations.
/// Provides file/line/col tracking for precise error reporting.
#[derive(Debug)]
pub struct LexError {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub kind: LexErrorKind,
}

/// All error categories that can occur during lexing/parsing/compilation.
#[derive(Debug)]
pub enum LexErrorKind {
    // Regex parsing errors
    UnexpectedChar(char),
    UnexpectedToken(String),
    UnterminatedString,
    InvalidRepeatRange { min: u32, max: u32 },
    UnknownPosixClass(String),
    EmptyRegex,
    UnclosedGroup,
    UnclosedBracket,
    InvalidEscape(char),
    TrailingBackslash,

    // .l file parsing errors
    MissingSectionDelimiter,
    InvalidDefinition(String),
    InvalidOption(String),
    UndefinedName(String),
    DuplicateDefinition(String),
    InvalidStartCondition(String),
    UnterminatedAction,
    UnterminatedCodeBlock,

    // Code generation errors
    EmitError(String),

    // I/O errors
    IoError(std::io::Error),
}

impl LexError {
    /// Create a new error at the given location.
    pub fn new(file: impl Into<String>, line: usize, col: usize, kind: LexErrorKind) -> Self {
        Self {
            file: file.into(),
            line,
            col,
            kind,
        }
    }

    /// Create an error with no location (e.g. for I/O errors).
    pub fn no_location(kind: LexErrorKind) -> Self {
        Self {
            file: String::new(),
            line: 0,
            col: 0,
            kind,
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.file.is_empty() {
            write!(f, "{}", self.kind)
        } else if self.col > 0 {
            write!(f, "{}:{}:{} {}", self.file, self.line, self.col, self.kind)
        } else {
            write!(f, "{}:{} {}", self.file, self.line, self.kind)
        }
    }
}

impl std::error::Error for LexError {}

impl fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexErrorKind::UnexpectedChar(c) => write!(f, "unexpected character '{}'", c),
            LexErrorKind::UnexpectedToken(t) => write!(f, "unexpected token '{}'", t),
            LexErrorKind::UnterminatedString => write!(f, "unterminated string"),
            LexErrorKind::InvalidRepeatRange { min, max } => {
                write!(f, "invalid repeat range {{{},{}}}: min > max", min, max)
            }
            LexErrorKind::UnknownPosixClass(name) => {
                write!(f, "unknown POSIX character class ':{}: '", name)
            }
            LexErrorKind::EmptyRegex => write!(f, "empty regular expression"),
            LexErrorKind::UnclosedGroup => write!(f, "unclosed '('"),
            LexErrorKind::UnclosedBracket => write!(f, "unclosed '['"),
            LexErrorKind::InvalidEscape(c) => write!(f, "invalid escape sequence '\\{}'", c),
            LexErrorKind::TrailingBackslash => write!(f, "trailing backslash"),
            LexErrorKind::MissingSectionDelimiter => {
                write!(f, "missing '%%' section delimiter")
            }
            LexErrorKind::InvalidDefinition(d) => write!(f, "invalid definition '{}'", d),
            LexErrorKind::InvalidOption(o) => write!(f, "invalid option '{}'", o),
            LexErrorKind::UndefinedName(n) => write!(f, "undefined name '{}'", n),
            LexErrorKind::DuplicateDefinition(n) => {
                write!(f, "duplicate definition '{}'", n)
            }
            LexErrorKind::InvalidStartCondition(s) => {
                write!(f, "invalid start condition '{}'", s)
            }
            LexErrorKind::UnterminatedAction => write!(f, "unterminated action block"),
            LexErrorKind::UnterminatedCodeBlock => {
                write!(f, "unterminated %{{ ... %}} code block")
            }
            LexErrorKind::EmitError(msg) => write!(f, "code generation error: {}", msg),
            LexErrorKind::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl From<std::io::Error> for LexError {
    fn from(e: std::io::Error) -> Self {
        LexError::no_location(LexErrorKind::IoError(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_with_file_line_col() {
        let err = LexError::new("lexer.l", 12, 5, LexErrorKind::UnexpectedChar(')'));
        assert_eq!(err.to_string(), "lexer.l:12:5 unexpected character ')'");
    }

    #[test]
    fn test_error_display_with_file_line_no_col() {
        let err = LexError::new("lexer.l", 12, 0, LexErrorKind::UnclosedGroup);
        assert_eq!(err.to_string(), "lexer.l:12 unclosed '('");
    }

    #[test]
    fn test_error_display_no_location() {
        let err = LexError::no_location(LexErrorKind::EmptyRegex);
        assert_eq!(err.to_string(), "empty regular expression");
    }

    #[test]
    fn test_error_display_all_kinds() {
        let kinds = vec![
            LexErrorKind::UnexpectedChar('x'),
            LexErrorKind::UnexpectedToken("foo".into()),
            LexErrorKind::UnterminatedString,
            LexErrorKind::InvalidRepeatRange { min: 5, max: 3 },
            LexErrorKind::UnknownPosixClass("foo".into()),
            LexErrorKind::EmptyRegex,
            LexErrorKind::UnclosedGroup,
            LexErrorKind::UnclosedBracket,
            LexErrorKind::InvalidEscape('z'),
            LexErrorKind::TrailingBackslash,
            LexErrorKind::MissingSectionDelimiter,
            LexErrorKind::InvalidDefinition("bad".into()),
            LexErrorKind::InvalidOption("bad".into()),
            LexErrorKind::UndefinedName("foo".into()),
            LexErrorKind::DuplicateDefinition("d".into()),
            LexErrorKind::InvalidStartCondition("S".into()),
            LexErrorKind::UnterminatedAction,
            LexErrorKind::UnterminatedCodeBlock,
            LexErrorKind::EmitError("out of memory".into()),
        ];
        for kind in kinds {
            let err = LexError::no_location(kind);
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: LexError = io_err.into();
        assert!(err.to_string().contains("file not found"));
        assert!(err.file.is_empty());
    }

    #[test]
    fn test_invalid_repeat_range_display() {
        let err = LexError::new(
            "test.l",
            3,
            10,
            LexErrorKind::InvalidRepeatRange { min: 10, max: 5 },
        );
        assert_eq!(
            err.to_string(),
            "test.l:3:10 invalid repeat range {10,5}: min > max"
        );
    }
}
