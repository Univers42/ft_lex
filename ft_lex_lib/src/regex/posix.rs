use crate::regex::ast::{CharClass, PosixClass};

/// Convert a POSIX character class name to byte ranges (POSIX/C locale).
pub fn posix_class_to_ranges(class: PosixClass) -> Vec<(u8, u8)> {
    match class {
        PosixClass::Digit => vec![(b'0', b'9')],
        PosixClass::Upper => vec![(b'A', b'Z')],
        PosixClass::Lower => vec![(b'a', b'z')],
        PosixClass::Alpha => vec![(b'A', b'Z'), (b'a', b'z')],
        PosixClass::Alnum => vec![(b'0', b'9'), (b'A', b'Z'), (b'a', b'z')],
        PosixClass::Space => vec![(0x09, 0x0D), (b' ', b' ')], // \t \n \v \f \r and space
        PosixClass::Blank => vec![(b'\t', b'\t'), (b' ', b' ')],
        PosixClass::XDigit => vec![(b'0', b'9'), (b'A', b'F'), (b'a', b'f')],
        PosixClass::Print => vec![(0x20, 0x7E)], // space through ~
        PosixClass::Graph => vec![(0x21, 0x7E)], // ! through ~ (no space)
        PosixClass::Punct => {
            // Printable non-alphanumeric non-space
            vec![
                (0x21, 0x2F), // ! " # $ % & ' ( ) * + , - . /
                (0x3A, 0x40), // : ; < = > ? @
                (0x5B, 0x60), // [ \ ] ^ _ `
                (0x7B, 0x7E), // { | } ~
            ]
        }
        PosixClass::Cntrl => vec![(0x00, 0x1F), (0x7F, 0x7F)],
    }
}

/// Parse a POSIX class name string (e.g. "alpha") to enum variant.
pub fn parse_posix_class_name(name: &str) -> Option<PosixClass> {
    match name {
        "alpha" => Some(PosixClass::Alpha),
        "digit" => Some(PosixClass::Digit),
        "alnum" => Some(PosixClass::Alnum),
        "space" => Some(PosixClass::Space),
        "upper" => Some(PosixClass::Upper),
        "lower" => Some(PosixClass::Lower),
        "blank" => Some(PosixClass::Blank),
        "print" => Some(PosixClass::Print),
        "punct" => Some(PosixClass::Punct),
        "cntrl" => Some(PosixClass::Cntrl),
        "graph" => Some(PosixClass::Graph),
        "xdigit" => Some(PosixClass::XDigit),
        _ => None,
    }
}

/// Expand a CharClass into a sorted, deduplicated, non-overlapping list of byte ranges.
/// Includes both explicit ranges and POSIX class ranges.
/// If negated, the result is the complement over the full byte range 0x00..=0xFF.
pub fn expand_char_class(cc: &CharClass) -> Vec<(u8, u8)> {
    let mut ranges: Vec<(u8, u8)> = cc.ranges.clone();
    for &pc in &cc.posix_classes {
        ranges.extend(posix_class_to_ranges(pc));
    }
    let merged = merge_ranges(&mut ranges);
    if cc.negated {
        complement_ranges(&merged)
    } else {
        merged
    }
}

/// Expand a CharClass into a flat set of matching bytes.
pub fn expand_char_class_bytes(cc: &CharClass) -> Vec<u8> {
    let ranges = expand_char_class(cc);
    let mut bytes = Vec::new();
    for (lo, hi) in ranges {
        for b in lo..=hi {
            bytes.push(b);
        }
    }
    bytes
}

/// Sort ranges and merge overlapping/adjacent ones.
fn merge_ranges(ranges: &mut Vec<(u8, u8)>) -> Vec<(u8, u8)> {
    if ranges.is_empty() {
        return Vec::new();
    }
    ranges.sort();
    let mut merged = vec![ranges[0]];
    for &(lo, hi) in &ranges[1..] {
        let last = merged.last_mut().unwrap();
        if lo <= last.1.saturating_add(1) {
            last.1 = last.1.max(hi);
        } else {
            merged.push((lo, hi));
        }
    }
    merged
}

/// Complement a sorted, non-overlapping set of ranges over 0x00..=0xFF.
fn complement_ranges(ranges: &[(u8, u8)]) -> Vec<(u8, u8)> {
    let mut result = Vec::new();
    let mut pos: u16 = 0;
    for &(lo, hi) in ranges {
        if pos < lo as u16 {
            result.push((pos as u8, lo - 1));
        }
        pos = hi as u16 + 1;
    }
    if pos <= 255 {
        result.push((pos as u8, 255));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digit_ranges() {
        let r = posix_class_to_ranges(PosixClass::Digit);
        assert_eq!(r, vec![(b'0', b'9')]);
    }

    #[test]
    fn test_alpha_ranges() {
        let r = posix_class_to_ranges(PosixClass::Alpha);
        assert_eq!(r, vec![(b'A', b'Z'), (b'a', b'z')]);
    }

    #[test]
    fn test_alnum_ranges() {
        let r = posix_class_to_ranges(PosixClass::Alnum);
        assert_eq!(r, vec![(b'0', b'9'), (b'A', b'Z'), (b'a', b'z')]);
    }

    #[test]
    fn test_space_ranges() {
        let r = posix_class_to_ranges(PosixClass::Space);
        assert_eq!(r, vec![(0x09, 0x0D), (b' ', b' ')]);
    }

    #[test]
    fn test_blank_ranges() {
        let r = posix_class_to_ranges(PosixClass::Blank);
        assert_eq!(r, vec![(b'\t', b'\t'), (b' ', b' ')]);
    }

    #[test]
    fn test_xdigit_ranges() {
        let r = posix_class_to_ranges(PosixClass::XDigit);
        assert_eq!(r, vec![(b'0', b'9'), (b'A', b'F'), (b'a', b'f')]);
    }

    #[test]
    fn test_print_ranges() {
        let r = posix_class_to_ranges(PosixClass::Print);
        assert_eq!(r, vec![(0x20, 0x7E)]);
    }

    #[test]
    fn test_graph_ranges() {
        let r = posix_class_to_ranges(PosixClass::Graph);
        assert_eq!(r, vec![(0x21, 0x7E)]);
    }

    #[test]
    fn test_cntrl_ranges() {
        let r = posix_class_to_ranges(PosixClass::Cntrl);
        assert_eq!(r, vec![(0x00, 0x1F), (0x7F, 0x7F)]);
    }

    #[test]
    fn test_punct_ranges() {
        let r = posix_class_to_ranges(PosixClass::Punct);
        // Verify some known punct characters are included
        let bytes: Vec<u8> = r.iter().flat_map(|&(lo, hi)| lo..=hi).collect();
        assert!(bytes.contains(&b'!'));
        assert!(bytes.contains(&b'.'));
        assert!(bytes.contains(&b':'));
        assert!(bytes.contains(&b'['));
        assert!(bytes.contains(&b'{'));
        assert!(bytes.contains(&b'~'));
        // Verify alphanumeric are NOT included
        assert!(!bytes.contains(&b'a'));
        assert!(!bytes.contains(&b'0'));
        assert!(!bytes.contains(&b' '));
    }

    #[test]
    fn test_parse_posix_class_name() {
        assert_eq!(parse_posix_class_name("alpha"), Some(PosixClass::Alpha));
        assert_eq!(parse_posix_class_name("digit"), Some(PosixClass::Digit));
        assert_eq!(parse_posix_class_name("xdigit"), Some(PosixClass::XDigit));
        assert_eq!(parse_posix_class_name("unknown"), None);
    }

    #[test]
    fn test_expand_char_class_simple() {
        let mut cc = CharClass::new();
        cc.add_range(b'a', b'z');
        let expanded = expand_char_class(&cc);
        assert_eq!(expanded, vec![(b'a', b'z')]);
    }

    #[test]
    fn test_expand_char_class_with_posix() {
        let mut cc = CharClass::new();
        cc.add_posix(PosixClass::Digit);
        let expanded = expand_char_class(&cc);
        assert_eq!(expanded, vec![(b'0', b'9')]);
    }

    #[test]
    fn test_expand_char_class_overlapping() {
        let mut cc = CharClass::new();
        cc.add_range(b'a', b'm');
        cc.add_range(b'k', b'z');
        let expanded = expand_char_class(&cc);
        assert_eq!(expanded, vec![(b'a', b'z')]);
    }

    #[test]
    fn test_expand_char_class_negated() {
        let mut cc = CharClass::negated();
        cc.add_byte(b'a');
        let expanded = expand_char_class(&cc);
        // Should be [0x00, 0x60] + [0x62, 0xFF]
        assert_eq!(expanded, vec![(0x00, b'a' - 1), (b'a' + 1, 0xFF)]);
    }

    #[test]
    fn test_expand_char_class_bytes() {
        let mut cc = CharClass::new();
        cc.add_range(b'a', b'c');
        let bytes = expand_char_class_bytes(&cc);
        assert_eq!(bytes, vec![b'a', b'b', b'c']);
    }

    #[test]
    fn test_merge_adjacent_ranges() {
        let mut ranges = vec![(b'a', b'c'), (b'd', b'f')];
        let merged = merge_ranges(&mut ranges);
        assert_eq!(merged, vec![(b'a', b'f')]);
    }

    #[test]
    fn test_complement_empty() {
        let comp = complement_ranges(&[]);
        assert_eq!(comp, vec![(0, 255)]);
    }

    #[test]
    fn test_complement_full() {
        let comp = complement_ranges(&[(0, 255)]);
        assert!(comp.is_empty());
    }
}
