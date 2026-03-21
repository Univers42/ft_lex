/// Regex AST — recursive representation of POSIX extended regular expressions
/// with lex-specific extensions ({name}, "literal", anchors, start conditions).

/// The core regex AST type.
#[derive(Debug, Clone)]
pub enum Regex {
    /// Matches the empty string (ε).
    Empty,
    /// Matches a single byte.
    Literal(u8),
    /// `.` — matches any character except newline.
    AnyChar,
    /// `[...]` or `[^...]` — character class.
    CharClass(CharClass),
    /// `r1 r2` — concatenation.
    Concat(Box<Regex>, Box<Regex>),
    /// `r1 | r2` — alternation.
    Alt(Box<Regex>, Box<Regex>),
    /// `r*` — Kleene star (zero or more).
    Star(Box<Regex>),
    /// `r+` — one or more (sugar for `r r*`).
    Plus(Box<Regex>),
    /// `r?` — zero or one (sugar for `r | ε`).
    Question(Box<Regex>),
    /// `r{n,m}` — bounded repetition.
    Repeat(Box<Regex>, u32, Option<u32>),
    /// `^` or `$` anchored regex.
    Anchor(Anchor),
    /// `{name}` — a named definition reference (resolved during .l file parsing).
    NameRef(String),
    /// `"literal"` — a quoted literal string (no regex interpretation).
    LiteralString(Vec<u8>),
}

/// A character class like `[a-zA-Z]` or `[[:digit:]]` or `[^abc]`.
#[derive(Debug, Clone)]
pub struct CharClass {
    /// Whether this is a negated class `[^...]`.
    pub negated: bool,
    /// Explicit byte ranges (inclusive). E.g., `a-z` becomes `(b'a', b'z')`.
    pub ranges: Vec<(u8, u8)>,
    /// POSIX named classes like `[:alpha:]`.
    pub posix_classes: Vec<PosixClass>,
}

/// The 12 POSIX character classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixClass {
    Alpha,
    Digit,
    Alnum,
    Space,
    Upper,
    Lower,
    Blank,
    Print,
    Punct,
    Cntrl,
    Graph,
    XDigit,
}

/// Anchor type for `^` (start of line) and `$` (end of line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Start,
    End,
}

// ── Constructors ──────────────────────────────────────────────────────────

impl Regex {
    pub fn literal(b: u8) -> Self {
        Regex::Literal(b)
    }

    pub fn concat(a: Regex, b: Regex) -> Self {
        // Simplify: concat with Empty is identity
        match (&a, &b) {
            (Regex::Empty, _) => b,
            (_, Regex::Empty) => a,
            _ => Regex::Concat(Box::new(a), Box::new(b)),
        }
    }

    pub fn alt(a: Regex, b: Regex) -> Self {
        Regex::Alt(Box::new(a), Box::new(b))
    }

    pub fn star(r: Regex) -> Self {
        Regex::Star(Box::new(r))
    }

    pub fn plus(r: Regex) -> Self {
        Regex::Plus(Box::new(r))
    }

    pub fn question(r: Regex) -> Self {
        Regex::Question(Box::new(r))
    }

    pub fn repeat(r: Regex, min: u32, max: Option<u32>) -> Self {
        Regex::Repeat(Box::new(r), min, max)
    }

    /// Build a concatenation from a sequence of bytes (e.g. a literal string).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Regex::Empty;
        }
        let mut result = Regex::Literal(bytes[0]);
        for &b in &bytes[1..] {
            result = Regex::concat(result, Regex::Literal(b));
        }
        result
    }
}

impl CharClass {
    pub fn new() -> Self {
        Self {
            negated: false,
            ranges: Vec::new(),
            posix_classes: Vec::new(),
        }
    }

    pub fn negated() -> Self {
        Self {
            negated: true,
            ranges: Vec::new(),
            posix_classes: Vec::new(),
        }
    }

    /// Add an explicit byte range (inclusive).
    pub fn add_range(&mut self, lo: u8, hi: u8) {
        self.ranges.push((lo, hi));
    }

    /// Add a single byte to the class.
    pub fn add_byte(&mut self, b: u8) {
        self.ranges.push((b, b));
    }

    /// Add a POSIX named class.
    pub fn add_posix(&mut self, class: PosixClass) {
        self.posix_classes.push(class);
    }
}

impl Default for CharClass {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concat_empty_simplification() {
        let r = Regex::concat(Regex::Empty, Regex::Literal(b'a'));
        matches!(r, Regex::Literal(b'a'));

        let r = Regex::concat(Regex::Literal(b'a'), Regex::Empty);
        matches!(r, Regex::Literal(b'a'));
    }

    #[test]
    fn test_from_bytes() {
        let r = Regex::from_bytes(b"abc");
        // Should be Concat(Concat(Literal('a'), Literal('b')), Literal('c'))
        if let Regex::Concat(left, right) = r {
            assert!(matches!(*right, Regex::Literal(b'c')));
            if let Regex::Concat(ll, lr) = *left {
                assert!(matches!(*ll, Regex::Literal(b'a')));
                assert!(matches!(*lr, Regex::Literal(b'b')));
            } else {
                panic!("expected inner Concat");
            }
        } else {
            panic!("expected Concat");
        }
    }

    #[test]
    fn test_from_bytes_empty() {
        let r = Regex::from_bytes(b"");
        assert!(matches!(r, Regex::Empty));
    }

    #[test]
    fn test_from_bytes_single() {
        let r = Regex::from_bytes(b"x");
        assert!(matches!(r, Regex::Literal(b'x')));
    }

    #[test]
    fn test_char_class_builder() {
        let mut cc = CharClass::new();
        cc.add_range(b'a', b'z');
        cc.add_byte(b'_');
        cc.add_posix(PosixClass::Digit);
        assert!(!cc.negated);
        assert_eq!(cc.ranges.len(), 2);
        assert_eq!(cc.posix_classes.len(), 1);
    }

    #[test]
    fn test_char_class_negated() {
        let cc = CharClass::negated();
        assert!(cc.negated);
    }
}
