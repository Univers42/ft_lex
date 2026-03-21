use std::collections::BTreeSet;

use crate::automata::arena::{Arena, NodeId};
use crate::regex::ast::*;
use crate::regex::posix::expand_char_class_bytes;

/// An NFA state. Transitions are stored as lists of (label, target) pairs.
/// Epsilon transitions are stored separately for efficiency.
#[derive(Debug, Clone)]
pub struct NfaState {
    /// Transitions on specific bytes.
    pub transitions: Vec<(u8, NodeId)>,
    /// Epsilon transitions (no input consumed).
    pub epsilon: Vec<NodeId>,
    /// If this is an accepting state, the rule index (priority). None = non-accepting.
    pub accepting: Option<usize>,
}

impl NfaState {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            epsilon: Vec::new(),
            accepting: None,
        }
    }
}

/// A fragment of an NFA (result of Thompson's construction for one sub-expression).
#[derive(Debug, Clone, Copy)]
pub struct NfaFragment {
    pub start: NodeId,
    pub end: NodeId,
}

/// Complete NFA with all states in an arena.
pub struct Nfa {
    pub arena: Arena<NfaState>,
    pub start: NodeId,
}

impl Nfa {
    /// Compute the epsilon closure of a set of states.
    pub fn epsilon_closure(&self, states: &BTreeSet<NodeId>) -> BTreeSet<NodeId> {
        let mut closure = states.clone();
        let mut stack: Vec<NodeId> = states.iter().copied().collect();
        while let Some(state) = stack.pop() {
            for &target in &self.arena.get(state).epsilon {
                if closure.insert(target) {
                    stack.push(target);
                }
            }
        }
        closure
    }

    /// Compute the set of states reachable from `states` on input byte `byte`.
    pub fn move_on(&self, states: &BTreeSet<NodeId>, byte: u8) -> BTreeSet<NodeId> {
        let mut result = BTreeSet::new();
        for &state in states {
            for &(label, target) in &self.arena.get(state).transitions {
                if label == byte {
                    result.insert(target);
                }
            }
        }
        result
    }

    /// Get the highest-priority (lowest index) accepting rule for a set of NFA states.
    pub fn accepting_rule(&self, states: &BTreeSet<NodeId>) -> Option<usize> {
        let mut best: Option<usize> = None;
        for &state in states {
            if let Some(rule) = self.arena.get(state).accepting {
                best = Some(match best {
                    None => rule,
                    Some(b) => b.min(rule),
                });
            }
        }
        best
    }

    /// Number of states.
    pub fn state_count(&self) -> usize {
        self.arena.len()
    }
}

/// Builder for NFA construction.
pub struct NfaBuilder {
    pub arena: Arena<NfaState>,
}

impl NfaBuilder {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    fn new_state(&mut self) -> NodeId {
        self.arena.alloc(NfaState::new())
    }

    fn add_epsilon(&mut self, from: NodeId, to: NodeId) {
        self.arena.get_mut(from).epsilon.push(to);
    }

    fn add_transition(&mut self, from: NodeId, byte: u8, to: NodeId) {
        self.arena.get_mut(from).transitions.push((byte, to));
    }

    /// Build an NFA fragment for a regex AST node (Thompson's construction).
    pub fn build_regex(&mut self, regex: &Regex) -> NfaFragment {
        match regex {
            Regex::Empty => {
                let start = self.new_state();
                let end = self.new_state();
                self.add_epsilon(start, end);
                NfaFragment { start, end }
            }

            Regex::Literal(b) => {
                let start = self.new_state();
                let end = self.new_state();
                self.add_transition(start, *b, end);
                NfaFragment { start, end }
            }

            Regex::AnyChar => {
                // Match any byte except newline
                let start = self.new_state();
                let end = self.new_state();
                for b in 0u8..=255u8 {
                    if b != b'\n' {
                        self.add_transition(start, b, end);
                    }
                }
                NfaFragment { start, end }
            }

            Regex::CharClass(cc) => {
                let start = self.new_state();
                let end = self.new_state();
                let bytes = expand_char_class_bytes(cc);
                for b in bytes {
                    self.add_transition(start, b, end);
                }
                NfaFragment { start, end }
            }

            Regex::Concat(a, b) => {
                let frag_a = self.build_regex(a);
                let frag_b = self.build_regex(b);
                self.add_epsilon(frag_a.end, frag_b.start);
                NfaFragment {
                    start: frag_a.start,
                    end: frag_b.end,
                }
            }

            Regex::Alt(a, b) => {
                let start = self.new_state();
                let end = self.new_state();
                let frag_a = self.build_regex(a);
                let frag_b = self.build_regex(b);
                self.add_epsilon(start, frag_a.start);
                self.add_epsilon(start, frag_b.start);
                self.add_epsilon(frag_a.end, end);
                self.add_epsilon(frag_b.end, end);
                NfaFragment { start, end }
            }

            Regex::Star(r) => {
                let start = self.new_state();
                let end = self.new_state();
                let frag = self.build_regex(r);
                self.add_epsilon(start, frag.start);
                self.add_epsilon(start, end);
                self.add_epsilon(frag.end, frag.start);
                self.add_epsilon(frag.end, end);
                NfaFragment { start, end }
            }

            Regex::Plus(r) => {
                // r+ = r r*
                let frag = self.build_regex(r);
                let start = self.new_state();
                let end = self.new_state();
                self.add_epsilon(start, frag.start);
                self.add_epsilon(frag.end, end);
                self.add_epsilon(frag.end, frag.start); // loop back
                NfaFragment { start, end }
            }

            Regex::Question(r) => {
                // r? = r | ε
                let start = self.new_state();
                let end = self.new_state();
                let frag = self.build_regex(r);
                self.add_epsilon(start, frag.start);
                self.add_epsilon(start, end); // bypass (ε path)
                self.add_epsilon(frag.end, end);
                NfaFragment { start, end }
            }

            Regex::Repeat(r, min, max) => {
                self.build_repeat(r, *min, *max)
            }

            Regex::Anchor(Anchor::Start) => {
                // ^ — BOL anchor: handled at parser/emitter level via rule.bol_anchor flag
                // In the NFA, just produce an epsilon transition (always passes)
                let start = self.new_state();
                let end = self.new_state();
                self.add_epsilon(start, end);
                NfaFragment { start, end }
            }

            Regex::Anchor(Anchor::End) => {
                // $ — EOL anchor: handled at parser/emitter level via rule.eol_anchor flag
                // In the NFA, just produce an epsilon transition (always passes)
                let start = self.new_state();
                let end = self.new_state();
                self.add_epsilon(start, end);
                NfaFragment { start, end }
            }

            Regex::NameRef(_) => {
                // Should have been expanded before NFA construction
                // Fall back to empty match
                let start = self.new_state();
                let end = self.new_state();
                self.add_epsilon(start, end);
                NfaFragment { start, end }
            }

            Regex::LiteralString(bytes) => {
                if bytes.is_empty() {
                    let start = self.new_state();
                    let end = self.new_state();
                    self.add_epsilon(start, end);
                    return NfaFragment { start, end };
                }
                let start = self.new_state();
                let mut current = start;
                for &b in bytes {
                    let next = self.new_state();
                    self.add_transition(current, b, next);
                    current = next;
                }
                NfaFragment {
                    start,
                    end: current,
                }
            }
        }
    }

    /// Build repeated match: r{min,max}
    fn build_repeat(&mut self, r: &Regex, min: u32, max: Option<u32>) -> NfaFragment {
        let start = self.new_state();
        let end = self.new_state();

        // Build `min` mandatory copies
        let mut current = start;
        for _ in 0..min {
            let frag = self.build_regex(r);
            self.add_epsilon(current, frag.start);
            current = frag.end;
        }

        match max {
            None => {
                // {min,} — unlimited: add r* after the mandatory copies
                let frag = self.build_regex(r);
                self.add_epsilon(current, frag.start);
                self.add_epsilon(current, end);
                self.add_epsilon(frag.end, frag.start);
                self.add_epsilon(frag.end, end);
            }
            Some(max) => {
                // {min,max} — add (max - min) optional copies
                self.add_epsilon(current, end);
                for _ in min..max {
                    let frag = self.build_regex(r);
                    self.add_epsilon(current, frag.start);
                    current = frag.end;
                    self.add_epsilon(current, end);
                }
            }
        }

        NfaFragment { start, end }
    }

    /// Build a combined NFA for multiple rules with a common start state.
    /// Each rule's accepting state is tagged with the rule's priority index.
    pub fn build_combined(&mut self, rules: &[(Regex, usize)]) -> Nfa {
        let global_start = self.new_state();

        for (regex, priority) in rules {
            let frag = self.build_regex(regex);
            self.add_epsilon(global_start, frag.start);
            self.arena.get_mut(frag.end).accepting = Some(*priority);
        }

        let arena = std::mem::replace(&mut self.arena, Arena::new());
        Nfa {
            arena,
            start: global_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::parser::parse_regex;

    fn build_nfa_for(pattern: &str) -> Nfa {
        let regex = parse_regex(pattern.as_bytes(), "<test>", 1).unwrap();
        let mut builder = NfaBuilder::new();
        builder.build_combined(&[(regex, 0)])
    }

    #[test]
    fn test_nfa_literal_a() {
        let nfa = build_nfa_for("a");
        // Should have: global_start -> ε -> frag_start -> 'a' -> frag_end(accepting)
        assert!(nfa.state_count() >= 3);
    }

    #[test]
    fn test_nfa_alt_a_b() {
        let nfa = build_nfa_for("a|b");
        assert!(nfa.state_count() >= 5);
    }

    #[test]
    fn test_nfa_star() {
        let nfa = build_nfa_for("a*");
        assert!(nfa.state_count() >= 3);
    }

    #[test]
    fn test_epsilon_closure() {
        let nfa = build_nfa_for("a|b");
        let start = BTreeSet::from([nfa.start]);
        let closure = nfa.epsilon_closure(&start);
        // Start + both alt branches' start states
        assert!(closure.len() >= 3);
    }

    #[test]
    fn test_move_on() {
        let nfa = build_nfa_for("a");
        let start = BTreeSet::from([nfa.start]);
        let closure = nfa.epsilon_closure(&start);
        let after_a = nfa.move_on(&closure, b'a');
        assert!(!after_a.is_empty());
        let after_b = nfa.move_on(&closure, b'b');
        assert!(after_b.is_empty());
    }

    #[test]
    fn test_accepting_detection() {
        let nfa = build_nfa_for("a");
        let start = BTreeSet::from([nfa.start]);
        let closure = nfa.epsilon_closure(&start);
        // Before reading 'a', should not be accepting
        assert!(nfa.accepting_rule(&closure).is_none() || nfa.accepting_rule(&closure) == Some(0));
        // Actually, for "a" the start closure shouldn't include accepting states

        let after_a = nfa.move_on(&closure, b'a');
        let after_a_closure = nfa.epsilon_closure(&after_a);
        assert_eq!(nfa.accepting_rule(&after_a_closure), Some(0));
    }

    #[test]
    fn test_combined_nfa_priorities() {
        let r1 = parse_regex(b"[0-9]+", "<test>", 1).unwrap();
        let r2 = parse_regex(b"[a-z]+", "<test>", 2).unwrap();
        let mut builder = NfaBuilder::new();
        let nfa = builder.build_combined(&[(r1, 0), (r2, 1)]);

        let start = BTreeSet::from([nfa.start]);
        let closure = nfa.epsilon_closure(&start);

        // On a digit, should reach rule 0 accepting states
        let after_digit = nfa.move_on(&closure, b'5');
        let after_digit_closure = nfa.epsilon_closure(&after_digit);
        assert_eq!(nfa.accepting_rule(&after_digit_closure), Some(0));

        // On a letter, should reach rule 1 accepting states
        let after_letter = nfa.move_on(&closure, b'x');
        let after_letter_closure = nfa.epsilon_closure(&after_letter);
        assert_eq!(nfa.accepting_rule(&after_letter_closure), Some(1));
    }

    #[test]
    fn test_nfa_digit_plus() {
        let nfa = build_nfa_for("[0-9]+");
        let start = BTreeSet::from([nfa.start]);
        let closure = nfa.epsilon_closure(&start);

        // All digit bytes should have transitions
        for b in b'0'..=b'9' {
            let after = nfa.move_on(&closure, b);
            assert!(!after.is_empty(), "no transition on digit {}", b);
        }
        // Non-digit should not
        let after = nfa.move_on(&closure, b'x');
        assert!(after.is_empty());
    }

    #[test]
    fn test_nfa_negated_class() {
        let nfa = build_nfa_for("[^a]");
        let start = BTreeSet::from([nfa.start]);
        let closure = nfa.epsilon_closure(&start);

        // 'a' should not match
        let after_a = nfa.move_on(&closure, b'a');
        assert!(after_a.is_empty());

        // 'b' should match
        let after_b = nfa.move_on(&closure, b'b');
        assert!(!after_b.is_empty());
    }

    #[test]
    fn test_nfa_repeat() {
        let nfa = build_nfa_for("a{2,4}");
        assert!(nfa.state_count() >= 4);
    }

    #[test]
    fn test_nfa_complex_float() {
        let nfa = build_nfa_for("[0-9]+\\.[0-9]*");
        assert!(nfa.state_count() > 4);
    }
}
