use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::automata::arena::{Arena, NodeId};
use crate::automata::dfa::{Dfa, DfaState};

/// Minimize a DFA using Hopcroft's algorithm.
///
/// Partitions states into equivalence classes where states in the same class
/// have identical behavior. Then rebuilds the DFA with one state per class.
pub fn minimize_dfa(dfa: &Dfa) -> Dfa {
    let n = dfa.state_count();
    if n == 0 {
        return Dfa {
            arena: Arena::new(),
            start: NodeId(0),
            dead_state: None,
        };
    }

    // Step 1: Initial partition — group by accepting rule.
    let mut partition_map: BTreeMap<Option<usize>, BTreeSet<NodeId>> = BTreeMap::new();
    for (id, state) in dfa.arena.iter() {
        partition_map
            .entry(state.accepting)
            .or_insert_with(BTreeSet::new)
            .insert(id);
    }

    let mut partitions: Vec<BTreeSet<NodeId>> = partition_map.into_values().collect();

    // Step 2: Refine using Hopcroft's worklist algorithm.
    let mut worklist: VecDeque<usize> = (0..partitions.len()).collect();

    while let Some(splitter_idx) = worklist.pop_front() {
        for byte in 0u8..=255u8 {
            let splitter = &partitions[splitter_idx];
            let mut predecessors: BTreeSet<NodeId> = BTreeSet::new();
            for (id, state) in dfa.arena.iter() {
                if let Some(target) = state.transitions[byte as usize] {
                    if splitter.contains(&target) {
                        predecessors.insert(id);
                    }
                }
            }

            if predecessors.is_empty() {
                continue;
            }

            let mut new_partitions_to_add: Vec<(usize, BTreeSet<NodeId>, BTreeSet<NodeId>)> =
                Vec::new();

            for (pi, partition) in partitions.iter().enumerate() {
                let intersection: BTreeSet<NodeId> =
                    partition.intersection(&predecessors).copied().collect();
                let difference: BTreeSet<NodeId> =
                    partition.difference(&predecessors).copied().collect();

                if !intersection.is_empty() && !difference.is_empty() {
                    new_partitions_to_add.push((pi, intersection, difference));
                }
            }

            for (pi, inter, diff) in new_partitions_to_add.into_iter().rev() {
                let new_idx = partitions.len();
                if inter.len() <= diff.len() {
                    partitions[pi] = diff;
                    partitions.push(inter);
                } else {
                    partitions[pi] = inter;
                    partitions.push(diff);
                }
                worklist.push_back(new_idx);
                if !worklist.contains(&pi) {
                    worklist.push_back(pi);
                }
            }
        }
    }

    // Step 3: Build the minimized DFA.
    let mut state_to_partition: BTreeMap<NodeId, usize> = BTreeMap::new();
    for (pi, partition) in partitions.iter().enumerate() {
        for &state in partition {
            state_to_partition.insert(state, pi);
        }
    }

    let mut new_arena: Arena<DfaState> = Arena::new();
    let mut partition_to_new_id: Vec<NodeId> = Vec::with_capacity(partitions.len());
    for _ in 0..partitions.len() {
        partition_to_new_id.push(new_arena.alloc(DfaState::new()));
    }

    for (pi, partition) in partitions.iter().enumerate() {
        let representative = *partition.iter().next().unwrap();
        let old_state = dfa.arena.get(representative);
        let new_id = partition_to_new_id[pi];

        new_arena.get_mut(new_id).accepting = old_state.accepting;

        for byte in 0usize..256 {
            if let Some(target) = old_state.transitions[byte] {
                let target_partition = state_to_partition[&target];
                new_arena.get_mut(new_id).transitions[byte] =
                    Some(partition_to_new_id[target_partition]);
            }
        }
    }

    let start_partition = state_to_partition[&dfa.start];
    let new_start = partition_to_new_id[start_partition];

    let mut dead_state = None;
    for (id, state) in new_arena.iter() {
        if state.accepting.is_none() {
            let all_self_or_none = state.transitions.iter().all(|t| match t {
                Some(target) => *target == id,
                None => true,
            });
            let has_any = state.transitions.iter().any(|t| t.is_some());
            if all_self_or_none && has_any {
                dead_state = Some(id);
                break;
            }
        }
    }

    Dfa {
        arena: new_arena,
        start: new_start,
        dead_state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::dfa::{dfa_simulate, subset_construction};
    use crate::automata::nfa::NfaBuilder;
    use crate::regex::parser::parse_regex;

    fn build_minimized_dfa(patterns: &[&str]) -> Dfa {
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
        minimize_dfa(&dfa)
    }

    #[test]
    fn test_minimize_reduces_states() {
        let mut builder = NfaBuilder::new();
        let regex = parse_regex(b"a*", "<test>", 1).unwrap();
        let nfa = builder.build_combined(&[(regex, 0)]);
        let dfa = subset_construction(&nfa);
        let min_dfa = minimize_dfa(&dfa);
        assert!(min_dfa.state_count() <= dfa.state_count());
    }

    #[test]
    fn test_minimize_preserves_matching() {
        let min_dfa = build_minimized_dfa(&["if", "[a-z]+"]);
        assert_eq!(dfa_simulate(&min_dfa, b"if"), Some((0, 2)));
        assert_eq!(dfa_simulate(&min_dfa, b"ifx"), Some((1, 3)));
        assert_eq!(dfa_simulate(&min_dfa, b"hello"), Some((1, 5)));
    }

    #[test]
    fn test_minimize_preserves_priorities() {
        let min_dfa = build_minimized_dfa(&["[0-9]+", "[0-9]+\\.[0-9]*"]);
        assert_eq!(dfa_simulate(&min_dfa, b"123"), Some((0, 3)));
        assert_eq!(dfa_simulate(&min_dfa, b"123.45"), Some((1, 6)));
    }

    #[test]
    fn test_minimize_single_literal() {
        let min_dfa = build_minimized_dfa(&["a"]);
        assert_eq!(dfa_simulate(&min_dfa, b"a"), Some((0, 1)));
        assert_eq!(dfa_simulate(&min_dfa, b"b"), None);
    }

    #[test]
    fn test_minimize_alternation() {
        let min_dfa = build_minimized_dfa(&["a|b"]);
        assert_eq!(dfa_simulate(&min_dfa, b"a"), Some((0, 1)));
        assert_eq!(dfa_simulate(&min_dfa, b"b"), Some((0, 1)));
        assert_eq!(dfa_simulate(&min_dfa, b"c"), None);
    }

    #[test]
    fn test_minimize_whitespace_identifier_number() {
        let min_dfa = build_minimized_dfa(&["[ \\t\\n]+", "[a-zA-Z_][a-zA-Z0-9_]*", "[0-9]+"]);
        assert_eq!(dfa_simulate(&min_dfa, b"   "), Some((0, 3)));
        assert_eq!(dfa_simulate(&min_dfa, b"foo42"), Some((1, 5)));
        assert_eq!(dfa_simulate(&min_dfa, b"123"), Some((2, 3)));
    }

    #[test]
    fn test_minimize_is_smaller_or_equal() {
        let mut builder = NfaBuilder::new();
        let patterns = &["[ \\t\\n]+", "[a-zA-Z_][a-zA-Z0-9_]*", "[0-9]+", ".", "\\+", "-"];
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
        assert!(
            min_dfa.state_count() <= dfa.state_count(),
            "minimized DFA ({}) should not be larger than original ({})",
            min_dfa.state_count(),
            dfa.state_count()
        );
    }
}
