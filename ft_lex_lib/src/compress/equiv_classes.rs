use crate::automata::arena::NodeId;
use crate::automata::dfa::Dfa;

/// Equivalence class mapping: maps each byte 0..=255 to a class index.
/// Two bytes are in the same class if they have identical transitions
/// in ALL DFA states.
#[derive(Debug, Clone)]
pub struct EquivClasses {
    /// For each byte 0..=255, its equivalence class index.
    pub class_of: [u8; 256],
    /// Number of distinct equivalence classes.
    pub num_classes: usize,
}

/// Compressed DFA transition table: indexed by [state][equiv_class] instead of [state][256].
#[derive(Debug, Clone)]
pub struct CompressedDfa {
    /// Equivalence class mapping.
    pub ec: EquivClasses,
    /// Transition table: transitions[state_index * num_classes + class] = target_state (-1 = none).
    pub transitions: Vec<i32>,
    /// Accept table: accept[state_index] = rule (0 = non-accepting, >0 = 1-based rule).
    pub accept: Vec<i32>,
    /// Number of states.
    pub num_states: usize,
    /// Start state index.
    pub start_state: usize,
}

/// Compute equivalence classes for a DFA.
///
/// Two bytes are equivalent if for every DFA state, both bytes lead to the same
/// target state (or both have no transition). This groups bytes that behave
/// identically across the entire automaton.
pub fn compute_equiv_classes(dfa: &Dfa) -> EquivClasses {
    let n = dfa.state_count();
    if n == 0 {
        return EquivClasses {
            class_of: [0; 256],
            num_classes: 1,
        };
    }

    // Build a signature for each byte: the vector of transitions across all states.
    // Two bytes with the same signature are equivalent.
    let mut signatures: Vec<(u8, Vec<i32>)> = Vec::with_capacity(256);
    for byte in 0u8..=255u8 {
        let mut sig = Vec::with_capacity(n);
        for state_idx in 0..n {
            let state = dfa.arena.get(NodeId(state_idx));
            let target = match state.transitions[byte as usize] {
                Some(t) => t.0 as i32,
                None => -1,
            };
            sig.push(target);
        }
        signatures.push((byte, sig));
    }

    // Assign classes by grouping identical signatures.
    let mut class_of = [0u8; 256];
    let mut class_map: Vec<Vec<i32>> = Vec::new(); // signature -> class index

    for (byte, sig) in &signatures {
        let mut found = false;
        for (ci, existing_sig) in class_map.iter().enumerate() {
            if existing_sig == sig {
                class_of[*byte as usize] = ci as u8;
                found = true;
                break;
            }
        }
        if !found {
            let ci = class_map.len();
            class_of[*byte as usize] = ci as u8;
            class_map.push(sig.clone());
        }
    }

    EquivClasses {
        class_of,
        num_classes: class_map.len(),
    }
}

/// Build a compressed DFA from the original DFA using equivalence classes.
pub fn compress_dfa(dfa: &Dfa) -> CompressedDfa {
    let ec = compute_equiv_classes(dfa);
    let n = dfa.state_count();
    let nc = ec.num_classes;

    // Build compressed transition table.
    let mut transitions = vec![-1i32; n * nc];
    for state_idx in 0..n {
        let state = dfa.arena.get(NodeId(state_idx));
        // For each equivalence class, pick the transition from any representative byte.
        for byte in 0u8..=255u8 {
            let ci = ec.class_of[byte as usize] as usize;
            if let Some(target) = state.transitions[byte as usize] {
                transitions[state_idx * nc + ci] = target.0 as i32;
            }
        }
    }

    // Build accept table.
    let mut accept = vec![0i32; n];
    for state_idx in 0..n {
        let state = dfa.arena.get(NodeId(state_idx));
        accept[state_idx] = match state.accepting {
            Some(rule) => (rule + 1) as i32,
            None => 0,
        };
    }

    CompressedDfa {
        ec,
        transitions,
        accept,
        num_states: n,
        start_state: dfa.start.0,
    }
}

/// Simulate the compressed DFA on input. Returns (rule_index, length) for longest match.
pub fn compressed_dfa_simulate(cdfa: &CompressedDfa, input: &[u8]) -> Option<(usize, usize)> {
    let nc = cdfa.ec.num_classes;
    let mut current = cdfa.start_state;
    let mut last_match: Option<(usize, usize)> = None;

    if cdfa.accept[current] > 0 {
        last_match = Some(((cdfa.accept[current] - 1) as usize, 0));
    }

    for (i, &byte) in input.iter().enumerate() {
        let ci = cdfa.ec.class_of[byte as usize] as usize;
        let next = cdfa.transitions[current * nc + ci];
        if next == -1 {
            break;
        }
        current = next as usize;
        if cdfa.accept[current] > 0 {
            last_match = Some(((cdfa.accept[current] - 1) as usize, i + 1));
        }
    }

    last_match
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::dfa::subset_construction;
    use crate::automata::minimize::minimize_dfa;
    use crate::automata::nfa::NfaBuilder;
    use crate::regex::parser::parse_regex;

    fn build_compressed(patterns: &[&str]) -> (Dfa, CompressedDfa) {
        let mut builder = NfaBuilder::new();
        let rules: Vec<_> = patterns
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let regex = parse_regex(p.as_bytes(), "<test>", 1).unwrap();
                (regex, i)
            })
            .collect();
        let nfa = builder.build_combined(&rules);
        let dfa = subset_construction(&nfa);
        let min_dfa = minimize_dfa(&dfa);
        let cdfa = compress_dfa(&min_dfa);
        (min_dfa, cdfa)
    }

    #[test]
    fn test_compression_reduces_columns() {
        let (dfa, cdfa) = build_compressed(&["[a-z]+", "[0-9]+"]);
        // 256 columns should be reduced to far fewer equivalence classes
        assert!(
            cdfa.ec.num_classes < 256,
            "expected fewer than 256 classes, got {}",
            cdfa.ec.num_classes
        );
        // Table size: states * classes vs states * 256
        let original_size = dfa.state_count() * 256;
        let compressed_size = cdfa.num_states * cdfa.ec.num_classes;
        assert!(
            compressed_size < original_size,
            "compressed {} should be < original {}",
            compressed_size,
            original_size
        );
    }

    #[test]
    fn test_compression_preserves_matching() {
        let (_dfa, cdfa) = build_compressed(&["if", "[a-z]+", "[0-9]+"]);
        assert_eq!(compressed_dfa_simulate(&cdfa, b"if"), Some((0, 2)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"ifx"), Some((1, 3)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"hello"), Some((1, 5)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"42"), Some((2, 2)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"!"), None);
    }

    #[test]
    fn test_compression_single_literal() {
        let (_, cdfa) = build_compressed(&["a"]);
        assert_eq!(compressed_dfa_simulate(&cdfa, b"a"), Some((0, 1)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"b"), None);
    }

    #[test]
    fn test_compression_digit_class_few_classes() {
        let (_, cdfa) = build_compressed(&["[0-9]+"]);
        // Only 2 useful classes: digits and non-digits
        // (plus potentially a few more for boundary behavior)
        assert!(
            cdfa.ec.num_classes <= 10,
            "expected very few classes for [0-9]+, got {}",
            cdfa.ec.num_classes
        );
    }

    #[test]
    fn test_compression_all_bytes_same() {
        // . matches all bytes except \n — many bytes are equivalent
        let (_, cdfa) = build_compressed(&["."]);
        assert!(
            cdfa.ec.num_classes <= 5,
            "expected very few classes for '.', got {}",
            cdfa.ec.num_classes
        );
    }

    #[test]
    fn test_compression_ratio_significant() {
        // A typical scanner has very few equivalence classes compared to 256
        let (dfa, cdfa) = build_compressed(&[
            "[ \\t\\n]+",
            "[a-zA-Z_][a-zA-Z0-9_]*",
            "[0-9]+",
            "\\+",
            "-",
            "\\*",
            "/",
        ]);
        let ratio = 256.0 / cdfa.ec.num_classes as f64;
        assert!(
            ratio >= 2.0,
            "compression ratio {:.1}x (classes={}) should be >= 2x for a typical scanner",
            ratio,
            cdfa.ec.num_classes
        );
        // Verify matching still works
        assert_eq!(compressed_dfa_simulate(&cdfa, b"   "), Some((0, 3)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"foo"), Some((1, 3)));
        assert_eq!(compressed_dfa_simulate(&cdfa, b"42"), Some((2, 2)));
    }

    #[test]
    fn test_equiv_classes_deterministic() {
        // Same bytes in same class across multiple calls
        let (_, cdfa1) = build_compressed(&["[a-z]+"]);
        let (_, cdfa2) = build_compressed(&["[a-z]+"]);
        assert_eq!(cdfa1.ec.class_of, cdfa2.ec.class_of);
        assert_eq!(cdfa1.ec.num_classes, cdfa2.ec.num_classes);
    }

    #[test]
    fn test_equiv_classes_same_class_for_same_behavior() {
        let (_, cdfa) = build_compressed(&["[0-9]+"]);
        // All digits should be in the same class
        let digit_class = cdfa.ec.class_of[b'0' as usize];
        for d in b'1'..=b'9' {
            assert_eq!(
                cdfa.ec.class_of[d as usize], digit_class,
                "digit {} should be in same class as '0'",
                d as char
            );
        }
    }

    #[test]
    fn test_compress_empty_dfa() {
        // Edge case: DFA with no states
        let dfa = Dfa {
            arena: crate::automata::arena::Arena::new(),
            start: NodeId(0),
            dead_state: None,
        };
        let cdfa = compress_dfa(&dfa);
        assert_eq!(cdfa.num_states, 0);
        assert_eq!(cdfa.ec.num_classes, 1);
    }
}
