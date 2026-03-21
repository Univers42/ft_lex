use std::collections::{BTreeMap, BTreeSet};

use crate::automata::arena::{Arena, NodeId};
use crate::automata::nfa::Nfa;

/// A DFA state.
#[derive(Debug, Clone)]
pub struct DfaState {
    /// Transitions: byte → target DFA state.
    pub transitions: [Option<NodeId>; 256],
    /// If this is an accepting state, the rule index (priority). None = non-accepting.
    pub accepting: Option<usize>,
}

impl DfaState {
    pub fn new() -> Self {
        Self {
            transitions: [None; 256],
            accepting: None,
        }
    }
}

/// Complete DFA.
pub struct Dfa {
    pub arena: Arena<DfaState>,
    pub start: NodeId,
    /// Dead state (all transitions loop to self, non-accepting). May or may not exist.
    pub dead_state: Option<NodeId>,
}

impl Dfa {
    /// Number of states.
    pub fn state_count(&self) -> usize {
        self.arena.len()
    }

    /// Check if a DFA state is a dead state (all transitions loop to self, non-accepting).
    pub fn is_dead_state(&self, id: NodeId) -> bool {
        let state = self.arena.get(id);
        if state.accepting.is_some() {
            return false;
        }
        state.transitions.iter().all(|t| match t {
            Some(target) => *target == id,
            None => true,
        })
    }
}

/// Build a DFA from an NFA via the subset construction algorithm.
pub fn subset_construction(nfa: &Nfa) -> Dfa {
    let mut dfa_arena: Arena<DfaState> = Arena::new();

    // Map from NFA state sets to DFA state IDs.
    let mut state_map: BTreeMap<BTreeSet<NodeId>, NodeId> = BTreeMap::new();

    // Worklist of NFA state sets to process.
    let mut worklist: Vec<BTreeSet<NodeId>> = Vec::new();

    // Compute start state.
    let start_set = nfa.epsilon_closure(&BTreeSet::from([nfa.start]));
    let start_dfa = dfa_arena.alloc(DfaState::new());
    dfa_arena.get_mut(start_dfa).accepting = nfa.accepting_rule(&start_set);
    state_map.insert(start_set.clone(), start_dfa);
    worklist.push(start_set);

    while let Some(current_set) = worklist.pop() {
        let current_dfa = *state_map.get(&current_set).unwrap();

        for byte in 0u8..=255u8 {
            let moved = nfa.move_on(&current_set, byte);
            if moved.is_empty() {
                continue;
            }
            let target_set = nfa.epsilon_closure(&moved);
            if target_set.is_empty() {
                continue;
            }

            let target_dfa = if let Some(&existing) = state_map.get(&target_set) {
                existing
            } else {
                let new_id = dfa_arena.alloc(DfaState::new());
                dfa_arena.get_mut(new_id).accepting = nfa.accepting_rule(&target_set);
                state_map.insert(target_set.clone(), new_id);
                worklist.push(target_set);
                new_id
            };

            dfa_arena.get_mut(current_dfa).transitions[byte as usize] = Some(target_dfa);
        }
    }

    // Find dead state if any.
    let mut dead_state = None;
    for (id, state) in dfa_arena.iter() {
        if state.accepting.is_none() {
            let all_self = state.transitions.iter().all(|t| match t {
                Some(target) => *target == id,
                None => true,
            });
            // Also check that at least one transition exists (not all None) to distinguish
            // from a regular non-accepting state with no transitions.
            let has_any = state.transitions.iter().any(|t| t.is_some());
            if all_self && has_any {
                dead_state = Some(id);
                break;
            }
        }
    }

    Dfa {
        arena: dfa_arena,
        start: start_dfa,
        dead_state,
    }
}

/// Simulate the DFA on input. Returns the accepting rule of the longest match
/// and the number of bytes consumed (leftmost-longest match semantics).
pub fn dfa_simulate(dfa: &Dfa, input: &[u8]) -> Option<(usize, usize)> {
    let mut current = dfa.start;
    let mut last_match: Option<(usize, usize)> = None; // (rule, length)

    if let Some(rule) = dfa.arena.get(current).accepting {
        last_match = Some((rule, 0));
    }

    for (i, &byte) in input.iter().enumerate() {
        match dfa.arena.get(current).transitions[byte as usize] {
            Some(next) => {
                current = next;
                if let Some(rule) = dfa.arena.get(current).accepting {
                    last_match = Some((rule, i + 1));
                }
            }
            None => break,
        }
    }

    last_match
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::nfa::NfaBuilder;
    use crate::regex::parser::parse_regex;

    fn build_dfa_for(patterns: &[&str]) -> Dfa {
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
        subset_construction(&nfa)
    }

    #[test]
    fn test_dfa_single_literal() {
        let dfa = build_dfa_for(&["a"]);
        assert!(dfa.state_count() >= 2);
        let result = dfa_simulate(&dfa, b"a");
        assert_eq!(result, Some((0, 1)));
    }

    #[test]
    fn test_dfa_no_match() {
        let dfa = build_dfa_for(&["a"]);
        let result = dfa_simulate(&dfa, b"b");
        assert_eq!(result, None);
    }

    #[test]
    fn test_dfa_alternation() {
        let dfa = build_dfa_for(&["a|b"]);
        assert_eq!(dfa_simulate(&dfa, b"a"), Some((0, 1)));
        assert_eq!(dfa_simulate(&dfa, b"b"), Some((0, 1)));
        assert_eq!(dfa_simulate(&dfa, b"c"), None);
    }

    #[test]
    fn test_dfa_star() {
        let dfa = build_dfa_for(&["a*"]);
        // a* matches empty string (0 chars)
        assert_eq!(dfa_simulate(&dfa, b""), Some((0, 0)));
        assert_eq!(dfa_simulate(&dfa, b"aaa"), Some((0, 3)));
    }

    #[test]
    fn test_dfa_plus() {
        let dfa = build_dfa_for(&["a+"]);
        assert_eq!(dfa_simulate(&dfa, b""), None);
        assert_eq!(dfa_simulate(&dfa, b"a"), Some((0, 1)));
        assert_eq!(dfa_simulate(&dfa, b"aaa"), Some((0, 3)));
    }

    #[test]
    fn test_dfa_keyword_vs_identifier() {
        // Rule 0: "if" (keyword, higher priority)
        // Rule 1: [a-z]+ (identifier)
        let dfa = build_dfa_for(&["if", "[a-z]+"]);
        // "if" matches both rule 0 and rule 1, but rule 0 has higher priority
        assert_eq!(dfa_simulate(&dfa, b"if"), Some((0, 2)));
        // "ifx" — longest match is "ifx" (3 chars), rule 1
        assert_eq!(dfa_simulate(&dfa, b"ifx"), Some((1, 3)));
        // "hello" — only rule 1
        assert_eq!(dfa_simulate(&dfa, b"hello"), Some((1, 5)));
    }

    #[test]
    fn test_dfa_longest_match() {
        let dfa = build_dfa_for(&["[0-9]+", "[0-9]+\\.[0-9]*"]);
        // "123" matches rule 0 (integer)
        assert_eq!(dfa_simulate(&dfa, b"123"), Some((0, 3)));
        // "123.45" — longest match is rule 1 (float)
        assert_eq!(dfa_simulate(&dfa, b"123.45"), Some((1, 6)));
    }

    #[test]
    fn test_dfa_digit_class() {
        let dfa = build_dfa_for(&["[0-9]+"]);
        assert_eq!(dfa_simulate(&dfa, b"42"), Some((0, 2)));
        assert_eq!(dfa_simulate(&dfa, b"abc"), None);
    }

    #[test]
    fn test_dfa_mixed_rules() {
        let dfa = build_dfa_for(&["[ \\t\\n]+", "[a-zA-Z_][a-zA-Z0-9_]*", "[0-9]+"]);
        // whitespace
        assert_eq!(dfa_simulate(&dfa, b"   "), Some((0, 3)));
        // identifier
        assert_eq!(dfa_simulate(&dfa, b"foo42"), Some((1, 5)));
        // number
        assert_eq!(dfa_simulate(&dfa, b"123"), Some((2, 3)));
    }

    #[test]
    fn test_dfa_empty_input() {
        let dfa = build_dfa_for(&["a"]);
        assert_eq!(dfa_simulate(&dfa, b""), None);
    }
}
