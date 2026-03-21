use crate::error::{LexError, LexErrorKind};
use crate::regex::ast::*;
use crate::regex::posix::parse_posix_class_name;

/// Recursive descent parser for POSIX extended regular expressions
/// with lex-specific extensions.
///
/// Grammar:
///   regex     -> alt
///   alt       -> concat ('|' concat)*
///   concat    -> quantified+
///   quantified -> atom ('*' | '+' | '?' | '{n,m}')?
///   atom      -> '(' regex ')' | '[' charclass ']' | '"' literal '"'
///              | '{' name '}' | '.' | '^' | '$' | escape | char
pub struct RegexParser<'a> {
    input: &'a [u8],
    pos: usize,
    file: String,
    line: usize,
}

impl<'a> RegexParser<'a> {
    pub fn new(input: &'a [u8], file: &str, line: usize) -> Self {
        Self {
            input,
            pos: 0,
            file: file.to_string(),
            line,
        }
    }

    /// Parse the entire input as a regex.
    pub fn parse(&mut self) -> Result<Regex, LexError> {
        if self.input.is_empty() {
            return Ok(Regex::Empty);
        }
        let result = self.parse_alt()?;
        if self.pos < self.input.len() {
            return Err(self.error(LexErrorKind::UnexpectedChar(self.input[self.pos] as char)));
        }
        Ok(result)
    }

    // ── Helpers ───────────────────────────────────────────────────────

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn expect(&mut self, expected: u8) -> Result<(), LexError> {
        match self.advance() {
            Some(b) if b == expected => Ok(()),
            Some(b) => Err(self.error(LexErrorKind::UnexpectedChar(b as char))),
            None => Err(self.error(LexErrorKind::UnexpectedChar('\0'))),
        }
    }

    fn error(&self, kind: LexErrorKind) -> LexError {
        LexError::new(&self.file, self.line, self.pos + 1, kind)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    // ── Grammar productions ──────────────────────────────────────────

    /// alt -> concat ('|' concat)*
    fn parse_alt(&mut self) -> Result<Regex, LexError> {
        let mut left = self.parse_concat()?;
        while self.peek() == Some(b'|') {
            self.advance();
            let right = self.parse_concat()?;
            left = Regex::alt(left, right);
        }
        Ok(left)
    }

    /// concat -> quantified+
    fn parse_concat(&mut self) -> Result<Regex, LexError> {
        let mut result = Regex::Empty;
        while !self.at_end() {
            match self.peek() {
                Some(b'|') | Some(b')') => break,
                _ => {
                    let q = self.parse_quantified()?;
                    result = Regex::concat(result, q);
                }
            }
        }
        Ok(result)
    }

    /// quantified -> atom ('*' | '+' | '?' | '{n,m}')?
    fn parse_quantified(&mut self) -> Result<Regex, LexError> {
        let atom = self.parse_atom()?;
        match self.peek() {
            Some(b'*') => {
                self.advance();
                Ok(Regex::star(atom))
            }
            Some(b'+') => {
                self.advance();
                Ok(Regex::plus(atom))
            }
            Some(b'?') => {
                self.advance();
                Ok(Regex::question(atom))
            }
            Some(b'{') if self.is_repeat_brace() => self.parse_repeat(atom),
            _ => Ok(atom),
        }
    }

    /// Check if the upcoming '{' is a repeat quantifier {n} or {n,m},
    /// not a bare '{' literal or '{name}' reference.
    fn is_repeat_brace(&self) -> bool {
        let mut i = self.pos + 1;
        // Must start with a digit
        if i >= self.input.len() || !self.input[i].is_ascii_digit() {
            return false;
        }
        while i < self.input.len() && self.input[i].is_ascii_digit() {
            i += 1;
        }
        if i >= self.input.len() {
            return false;
        }
        match self.input[i] {
            b'}' => true,
            b',' => {
                i += 1;
                // Optional max digits
                while i < self.input.len() && self.input[i].is_ascii_digit() {
                    i += 1;
                }
                i < self.input.len() && self.input[i] == b'}'
            }
            _ => false,
        }
    }

    /// Parse {n}, {n,}, {n,m}
    fn parse_repeat(&mut self, atom: Regex) -> Result<Regex, LexError> {
        self.expect(b'{')?;
        let min = self.parse_number()?;
        let max = match self.peek() {
            Some(b',') => {
                self.advance();
                if self.peek() == Some(b'}') {
                    None // {n,} = unbounded
                } else {
                    Some(self.parse_number()?)
                }
            }
            _ => Some(min), // {n} = exactly n
        };
        self.expect(b'}')?;
        if let Some(m) = max {
            if min > m {
                return Err(self.error(LexErrorKind::InvalidRepeatRange { min, max: m }));
            }
        }
        Ok(Regex::repeat(atom, min, max))
    }

    fn parse_number(&mut self) -> Result<u32, LexError> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.error(LexErrorKind::UnexpectedChar(
                self.peek().unwrap_or(0) as char,
            )));
        }
        let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap_or("0");
        s.parse::<u32>()
            .map_err(|_| self.error(LexErrorKind::UnexpectedToken(s.to_string())))
    }

    /// atom -> group | charclass | quoted_string | name_ref | '.' | '^' | '$' | escape | char
    fn parse_atom(&mut self) -> Result<Regex, LexError> {
        match self.peek() {
            None => Err(self.error(LexErrorKind::EmptyRegex)),
            Some(b'(') => self.parse_group(),
            Some(b'[') => self.parse_char_class(),
            Some(b'"') => self.parse_quoted_string(),
            Some(b'{') if !self.is_repeat_brace() => self.parse_name_ref(),
            Some(b'.') => {
                self.advance();
                Ok(Regex::AnyChar)
            }
            Some(b'^') => {
                self.advance();
                Ok(Regex::Anchor(Anchor::Start))
            }
            Some(b'$') => {
                self.advance();
                Ok(Regex::Anchor(Anchor::End))
            }
            Some(b'\\') => self.parse_escape(),
            // These are not atom-starters; they should be handled by caller
            Some(b'*') | Some(b'+') | Some(b'?') | Some(b')') | Some(b'|') => {
                Err(self.error(LexErrorKind::UnexpectedChar(self.input[self.pos] as char)))
            }
            Some(b) => {
                self.advance();
                Ok(Regex::Literal(b))
            }
        }
    }

    /// '(' regex ')'
    fn parse_group(&mut self) -> Result<Regex, LexError> {
        self.expect(b'(')?;
        let inner = self.parse_alt()?;
        if self.peek() != Some(b')') {
            return Err(self.error(LexErrorKind::UnclosedGroup));
        }
        self.advance();
        Ok(inner)
    }

    /// '[' '^'? (range | posix_class | char)* ']'
    fn parse_char_class(&mut self) -> Result<Regex, LexError> {
        self.expect(b'[')?;
        let negated = if self.peek() == Some(b'^') {
            self.advance();
            true
        } else {
            false
        };

        let mut cc = if negated {
            CharClass::negated()
        } else {
            CharClass::new()
        };

        // ']' as first char (or after '^') is a literal ']'
        if self.peek() == Some(b']') {
            cc.add_byte(b']');
            self.advance();
        }

        loop {
            match self.peek() {
                None => return Err(self.error(LexErrorKind::UnclosedBracket)),
                Some(b']') => {
                    self.advance();
                    return Ok(Regex::CharClass(cc));
                }
                Some(b'[') if self.lookahead_posix_class() => {
                    // [:name:]
                    self.advance(); // [
                    self.advance(); // :
                    let name = self.read_until(b':')?;
                    self.expect(b':')?;
                    self.expect(b']')?;
                    let pc = parse_posix_class_name(&name).ok_or_else(|| {
                        self.error(LexErrorKind::UnknownPosixClass(name.clone()))
                    })?;
                    cc.add_posix(pc);
                }
                Some(b'\\') => {
                    let b = self.parse_escape_byte()?;
                    self.maybe_range(&mut cc, b)?;
                }
                Some(b) => {
                    self.advance();
                    self.maybe_range(&mut cc, b)?;
                }
            }
        }
    }

    /// Check if current position has `[:` (POSIX class start) inside a char class.
    fn lookahead_posix_class(&self) -> bool {
        self.pos + 1 < self.input.len()
            && self.input[self.pos] == b'['
            && self.input[self.pos + 1] == b':'
    }

    /// After reading a byte `lo` in a char class, check if `-` follows for a range.
    fn maybe_range(&mut self, cc: &mut CharClass, lo: u8) -> Result<(), LexError> {
        if self.peek() == Some(b'-') && self.pos + 1 < self.input.len() && self.input[self.pos + 1] != b']' {
            self.advance(); // consume '-'
            let hi = match self.peek() {
                Some(b'\\') => self.parse_escape_byte()?,
                Some(b) => {
                    self.advance();
                    b
                }
                None => return Err(self.error(LexErrorKind::UnclosedBracket)),
            };
            cc.add_range(lo, hi);
        } else {
            cc.add_byte(lo);
        }
        Ok(())
    }

    /// Read bytes until delimiter, return as string. Consumes the delimiter.
    fn read_until(&mut self, delim: u8) -> Result<String, LexError> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != delim {
            self.pos += 1;
        }
        if self.pos >= self.input.len() {
            return Err(self.error(LexErrorKind::UnexpectedChar('\0')));
        }
        let s = String::from_utf8_lossy(&self.input[start..self.pos]).to_string();
        // Don't consume the delimiter here (caller expects it)
        Ok(s)
    }

    /// '"' ... '"' — literal string, no regex interpretation
    fn parse_quoted_string(&mut self) -> Result<Regex, LexError> {
        self.expect(b'"')?;
        let mut bytes = Vec::new();
        loop {
            match self.advance() {
                None => return Err(self.error(LexErrorKind::UnterminatedString)),
                Some(b'"') => break,
                Some(b'\\') => {
                    let esc = self.advance().ok_or_else(|| {
                        self.error(LexErrorKind::TrailingBackslash)
                    })?;
                    bytes.push(match esc {
                        b'n' => b'\n',
                        b't' => b'\t',
                        b'r' => b'\r',
                        b'\\' => b'\\',
                        b'"' => b'"',
                        b'a' => 0x07,
                        b'f' => 0x0C,
                        b'v' => 0x0B,
                        _ => esc, // literal
                    });
                }
                Some(b) => bytes.push(b),
            }
        }
        Ok(Regex::from_bytes(&bytes))
    }

    /// '{' name '}' — named definition reference
    fn parse_name_ref(&mut self) -> Result<Regex, LexError> {
        self.expect(b'{')?;
        let name = self.read_until(b'}')?;
        self.expect(b'}')?;
        Ok(Regex::NameRef(name))
    }

    /// '\' escape — returns a Regex::Literal or Regex::CharClass
    fn parse_escape(&mut self) -> Result<Regex, LexError> {
        let b = self.parse_escape_byte()?;
        Ok(Regex::Literal(b))
    }

    /// Parse a backslash escape sequence, returning the byte value.
    fn parse_escape_byte(&mut self) -> Result<u8, LexError> {
        self.expect(b'\\')?;
        match self.advance() {
            None => Err(self.error(LexErrorKind::TrailingBackslash)),
            Some(b'n') => Ok(b'\n'),
            Some(b't') => Ok(b'\t'),
            Some(b'r') => Ok(b'\r'),
            Some(b'a') => Ok(0x07),
            Some(b'f') => Ok(0x0C),
            Some(b'v') => Ok(0x0B),
            Some(b'0') => self.parse_octal(),
            Some(b'x') => self.parse_hex(),
            // Escaped special characters — literal
            Some(b) if b"()[]{}*+?.|^$\\/\"".contains(&b) => Ok(b),
            Some(b) => Err(self.error(LexErrorKind::InvalidEscape(b as char))),
        }
    }

    /// Parse octal digits after '\0'
    fn parse_octal(&mut self) -> Result<u8, LexError> {
        let mut val: u32 = 0;
        for _ in 0..3 {
            match self.peek() {
                Some(b) if (b'0'..=b'7').contains(&b) => {
                    self.advance();
                    val = val * 8 + (b - b'0') as u32;
                }
                _ => break,
            }
        }
        Ok(val as u8)
    }

    /// Parse hex digits after '\x'
    fn parse_hex(&mut self) -> Result<u8, LexError> {
        let mut val: u32 = 0;
        let mut count = 0;
        for _ in 0..2 {
            match self.peek() {
                Some(b) if b.is_ascii_hexdigit() => {
                    self.advance();
                    let digit = match b {
                        b'0'..=b'9' => b - b'0',
                        b'a'..=b'f' => b - b'a' + 10,
                        b'A'..=b'F' => b - b'A' + 10,
                        _ => unreachable!(),
                    };
                    val = val * 16 + digit as u32;
                    count += 1;
                }
                _ => break,
            }
        }
        if count == 0 {
            return Err(self.error(LexErrorKind::InvalidEscape('x')));
        }
        Ok(val as u8)
    }
}

/// Convenience function: parse a regex pattern string.
pub fn parse_regex(pattern: &[u8], file: &str, line: usize) -> Result<Regex, LexError> {
    let mut parser = RegexParser::new(pattern, file, line);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Regex {
        parse_regex(s.as_bytes(), "<test>", 1).expect(&format!("failed to parse '{}'", s))
    }

    fn p_err(s: &str) -> LexError {
        parse_regex(s.as_bytes(), "<test>", 1).expect_err(&format!("expected error for '{}'", s))
    }

    #[test]
    fn test_parse_literal() {
        match p("a") {
            Regex::Literal(b'a') => {}
            other => panic!("expected Literal('a'), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_concat() {
        match p("ab") {
            Regex::Concat(a, b) => {
                assert!(matches!(*a, Regex::Literal(b'a')));
                assert!(matches!(*b, Regex::Literal(b'b')));
            }
            other => panic!("expected Concat, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_alt() {
        match p("a|b") {
            Regex::Alt(a, b) => {
                assert!(matches!(*a, Regex::Literal(b'a')));
                assert!(matches!(*b, Regex::Literal(b'b')));
            }
            other => panic!("expected Alt, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_star() {
        match p("a*") {
            Regex::Star(inner) => assert!(matches!(*inner, Regex::Literal(b'a'))),
            other => panic!("expected Star, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_plus() {
        match p("a+") {
            Regex::Plus(inner) => assert!(matches!(*inner, Regex::Literal(b'a'))),
            other => panic!("expected Plus, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_question() {
        match p("a?") {
            Regex::Question(inner) => assert!(matches!(*inner, Regex::Literal(b'a'))),
            other => panic!("expected Question, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_group() {
        match p("(a|b)") {
            Regex::Alt(a, b) => {
                assert!(matches!(*a, Regex::Literal(b'a')));
                assert!(matches!(*b, Regex::Literal(b'b')));
            }
            other => panic!("expected Alt, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_char_class() {
        match p("[a-z]") {
            Regex::CharClass(cc) => {
                assert!(!cc.negated);
                assert_eq!(cc.ranges, vec![(b'a', b'z')]);
            }
            other => panic!("expected CharClass, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_char_class_negated() {
        match p("[^abc]") {
            Regex::CharClass(cc) => {
                assert!(cc.negated);
                assert_eq!(cc.ranges.len(), 3);
            }
            other => panic!("expected CharClass, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_posix_class() {
        match p("[[:alpha:]]") {
            Regex::CharClass(cc) => {
                assert!(!cc.negated);
                assert_eq!(cc.posix_classes, vec![PosixClass::Alpha]);
            }
            other => panic!("expected CharClass with POSIX, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_dot() {
        match p(".") {
            Regex::AnyChar => {}
            other => panic!("expected AnyChar, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_anchors() {
        match p("^") {
            Regex::Anchor(Anchor::Start) => {}
            other => panic!("expected Anchor(Start), got {:?}", other),
        }
        match p("$") {
            Regex::Anchor(Anchor::End) => {}
            other => panic!("expected Anchor(End), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_escape_sequences() {
        assert!(matches!(p("\\n"), Regex::Literal(b'\n')));
        assert!(matches!(p("\\t"), Regex::Literal(b'\t')));
        assert!(matches!(p("\\r"), Regex::Literal(b'\r')));
        assert!(matches!(p("\\."), Regex::Literal(b'.')));
        assert!(matches!(p("\\*"), Regex::Literal(b'*')));
        assert!(matches!(p("\\+"), Regex::Literal(b'+')));
    }

    #[test]
    fn test_parse_hex_escape() {
        match p("\\x41") {
            Regex::Literal(b'A') => {}
            other => panic!("expected Literal('A'), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_octal_escape() {
        match p("\\0101") {
            Regex::Literal(b'A') => {} // 0o101 = 65 = 'A'
            other => panic!("expected Literal('A'), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_quoted_string() {
        let r = p("\"hello\"");
        // Should be Concat chain of literals for h,e,l,l,o
        fn flatten(r: &Regex) -> Vec<u8> {
            match r {
                Regex::Literal(b) => vec![*b],
                Regex::Concat(a, b) => {
                    let mut v = flatten(a);
                    v.extend(flatten(b));
                    v
                }
                _ => panic!("unexpected node {:?}", r),
            }
        }
        assert_eq!(flatten(&r), b"hello");
    }

    #[test]
    fn test_parse_name_ref() {
        match p("{digit}") {
            Regex::NameRef(name) => assert_eq!(name, "digit"),
            other => panic!("expected NameRef, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeat_exact() {
        match p("a{3}") {
            Regex::Repeat(inner, 3, Some(3)) => {
                assert!(matches!(*inner, Regex::Literal(b'a')));
            }
            other => panic!("expected Repeat(a, 3, Some(3)), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeat_range() {
        match p("a{2,5}") {
            Regex::Repeat(inner, 2, Some(5)) => {
                assert!(matches!(*inner, Regex::Literal(b'a')));
            }
            other => panic!("expected Repeat(a, 2, Some(5)), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeat_unbounded() {
        match p("a{2,}") {
            Regex::Repeat(inner, 2, None) => {
                assert!(matches!(*inner, Regex::Literal(b'a')));
            }
            other => panic!("expected Repeat(a, 2, None), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_complex_float() {
        // [0-9]+\.[0-9]*
        let _ = p("[0-9]+\\.[0-9]*");
    }

    #[test]
    fn test_parse_classic_dfa_example() {
        // (a|b)*abb
        let _ = p("(a|b)*abb");
    }

    #[test]
    fn test_parse_empty() {
        match p("") {
            Regex::Empty => {}
            other => panic!("expected Empty, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_bracket_literal_first() {
        // ']' as first char in a class is a literal
        match p("[]abc]") {
            Regex::CharClass(cc) => {
                assert!(cc.ranges.contains(&(b']', b']')));
            }
            other => panic!("expected CharClass, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_char_class_blank() {
        match p("[[:blank:]]") {
            Regex::CharClass(cc) => {
                assert_eq!(cc.posix_classes, vec![PosixClass::Blank]);
            }
            other => panic!("expected CharClass with blank, got {:?}", other),
        }
    }

    #[test]
    fn test_error_unclosed_group() {
        let err = p_err("(abc");
        assert!(matches!(err.kind, LexErrorKind::UnclosedGroup));
    }

    #[test]
    fn test_error_unclosed_bracket() {
        let err = p_err("[abc");
        assert!(matches!(err.kind, LexErrorKind::UnclosedBracket));
    }

    #[test]
    fn test_error_invalid_repeat_range() {
        let err = p_err("a{5,3}");
        assert!(matches!(
            err.kind,
            LexErrorKind::InvalidRepeatRange { min: 5, max: 3 }
        ));
    }

    #[test]
    fn test_error_trailing_backslash() {
        let err = p_err("\\");
        assert!(matches!(err.kind, LexErrorKind::TrailingBackslash));
    }

    #[test]
    fn test_error_unterminated_string() {
        let err = p_err("\"hello");
        assert!(matches!(err.kind, LexErrorKind::UnterminatedString));
    }

    #[test]
    fn test_parse_multi_posix_class() {
        match p("[[:alpha:][:digit:]]") {
            Regex::CharClass(cc) => {
                assert_eq!(cc.posix_classes.len(), 2);
            }
            other => panic!("expected CharClass, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_quoted_escape() {
        let r = p("\"\\n\"");
        match r {
            Regex::Literal(b'\n') => {}
            other => panic!("expected Literal(newline), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_dash_at_end_of_class() {
        // A '-' at end of class is a literal
        match p("[a-]") {
            Regex::CharClass(cc) => {
                assert!(cc.ranges.contains(&(b'a', b'a')));
                assert!(cc.ranges.contains(&(b'-', b'-')));
            }
            other => panic!("expected CharClass, got {:?}", other),
        }
    }
}
