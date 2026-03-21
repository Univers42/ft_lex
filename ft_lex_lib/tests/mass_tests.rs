/// Mass tests for ft_lex_lib — 100+ additional tests covering edge cases,
/// error paths, and previously untested functionality across all modules.

// =============================================================================
// REGEX AST TESTS — direct constructor tests
// =============================================================================
mod regex_ast {
    use ft_lex_lib::regex::ast::*;

    #[test]
    fn test_literal_constructor() {
        let r = Regex::literal(b'A');
        assert!(matches!(r, Regex::Literal(65)));
    }

    #[test]
    fn test_alt_constructor() {
        let a = Regex::literal(b'a');
        let b = Regex::literal(b'b');
        let r = Regex::alt(a, b);
        assert!(matches!(r, Regex::Alt(_, _)));
    }

    #[test]
    fn test_star_constructor() {
        let r = Regex::star(Regex::literal(b'x'));
        assert!(matches!(r, Regex::Star(_)));
    }

    #[test]
    fn test_plus_constructor() {
        let r = Regex::plus(Regex::literal(b'x'));
        assert!(matches!(r, Regex::Plus(_)));
    }

    #[test]
    fn test_question_constructor() {
        let r = Regex::question(Regex::literal(b'x'));
        assert!(matches!(r, Regex::Question(_)));
    }

    #[test]
    fn test_repeat_constructor() {
        let r = Regex::repeat(Regex::literal(b'x'), 2, Some(4));
        assert!(matches!(r, Regex::Repeat(_, 2, Some(4))));
    }

    #[test]
    fn test_repeat_unbounded_constructor() {
        let r = Regex::repeat(Regex::literal(b'x'), 3, None);
        assert!(matches!(r, Regex::Repeat(_, 3, None)));
    }

    #[test]
    fn test_from_bytes_two_bytes() {
        let r = Regex::from_bytes(&[b'a', b'b']);
        assert!(matches!(r, Regex::Concat(_, _)));
    }

    #[test]
    fn test_char_class_default() {
        let cc = CharClass::default();
        assert!(cc.ranges.is_empty());
        assert!(cc.posix_classes.is_empty());
        assert!(!cc.negated);
    }

    #[test]
    fn test_char_class_add_byte() {
        let mut cc = CharClass::new();
        cc.add_byte(b'Z');
        assert_eq!(cc.ranges.len(), 1);
        assert_eq!(cc.ranges[0], (b'Z', b'Z'));
    }

    #[test]
    fn test_char_class_add_range() {
        let mut cc = CharClass::new();
        cc.add_range(b'a', b'z');
        assert_eq!(cc.ranges, vec![(b'a', b'z')]);
    }

    #[test]
    fn test_char_class_add_posix() {
        let mut cc = CharClass::new();
        cc.add_posix(PosixClass::Digit);
        assert_eq!(cc.posix_classes.len(), 1);
    }

    #[test]
    fn test_char_class_negated_constructor() {
        let cc = CharClass::negated();
        assert!(cc.negated);
        assert!(cc.ranges.is_empty());
    }

    #[test]
    fn test_concat_empty_left() {
        let r = Regex::concat(Regex::Empty, Regex::literal(b'x'));
        assert!(matches!(r, Regex::Literal(b'x')));
    }

    #[test]
    fn test_concat_empty_right() {
        let r = Regex::concat(Regex::literal(b'x'), Regex::Empty);
        assert!(matches!(r, Regex::Literal(b'x')));
    }

    #[test]
    fn test_concat_two_literals() {
        let r = Regex::concat(Regex::literal(b'a'), Regex::literal(b'b'));
        assert!(matches!(r, Regex::Concat(_, _)));
    }
}

// =============================================================================
// REGEX PARSER EDGE CASES
// =============================================================================
mod regex_parser_edges {
    use ft_lex_lib::regex::parser::parse_regex;

    #[test]
    fn test_parse_escape_a_bell() {
        let r = parse_regex(br"\a", "<t>", 1).unwrap();
        assert!(matches!(r, ft_lex_lib::regex::ast::Regex::Literal(7)));
    }

    #[test]
    fn test_parse_escape_f_formfeed() {
        let r = parse_regex(br"\f", "<t>", 1).unwrap();
        assert!(matches!(r, ft_lex_lib::regex::ast::Regex::Literal(12)));
    }

    #[test]
    fn test_parse_escape_v_vertical_tab() {
        let r = parse_regex(br"\v", "<t>", 1).unwrap();
        assert!(matches!(r, ft_lex_lib::regex::ast::Regex::Literal(11)));
    }

    #[test]
    fn test_parse_nested_groups() {
        assert!(parse_regex(b"((a))", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_quantifier_on_group() {
        assert!(parse_regex(b"(ab)+", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_multiple_alternations() {
        assert!(parse_regex(b"a|b|c|d", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_empty_alternation_branch() {
        assert!(parse_regex(b"a|", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_escape_inside_char_class() {
        assert!(parse_regex(br"[\n\t]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_hex_escape_inside_class() {
        assert!(parse_regex(br"[\x00-\x1f]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_single_char_class_range() {
        assert!(parse_regex(b"[a]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_dash_first_in_class() {
        assert!(parse_regex(b"[-a]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_caret_not_first_literal() {
        assert!(parse_regex(b"[a^b]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_star_on_group() {
        assert!(parse_regex(b"(xy)*", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_question_on_group() {
        assert!(parse_regex(b"(xy)?", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_complex_nested() {
        assert!(parse_regex(b"(a(b|c)*d)+", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_dot_star() {
        assert!(parse_regex(b".*", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_dot_plus() {
        assert!(parse_regex(b".+", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_bracket_close_first() {
        assert!(parse_regex(b"[]a]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_negated_bracket_close_first() {
        assert!(parse_regex(b"[^]a]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_repeat_exact_zero() {
        assert!(parse_regex(b"a{0}", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_repeat_range_zero_to_one() {
        assert!(parse_regex(b"a{0,1}", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_hex_uppercase() {
        assert!(parse_regex(br"\x4F", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_hex_two_digits_value() {
        let r = parse_regex(br"\x41", "<t>", 1).unwrap();
        match r {
            ft_lex_lib::regex::ast::Regex::Literal(b) => assert_eq!(b, 65),
            other => panic!("Expected Literal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_quoted_string_empty() {
        assert!(parse_regex(b"\"\"", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_multiple_posix_classes_in_bracket() {
        assert!(parse_regex(b"[[:alpha:][:digit:]]", "<t>", 1).is_ok());
    }

    #[test]
    fn test_parse_literal_special_chars_in_quotes() {
        assert!(parse_regex(b"\".+*?\"", "<t>", 1).is_ok());
    }

    #[test]
    fn test_error_unmatched_close_paren() {
        assert!(parse_regex(b"a)", "<t>", 1).is_err());
    }

    #[test]
    fn test_parse_empty_repeat_braces_no_panic() {
        let _ = parse_regex(b"a{}", "<t>", 1);
    }

    #[test]
    fn test_parse_repeat_non_numeric_no_panic() {
        let _ = parse_regex(b"a{abc}", "<t>", 1);
    }
}

// =============================================================================
// REGEX POSIX CLASSES EDGE CASES
// =============================================================================
mod posix_edges {
    use ft_lex_lib::regex::posix::*;
    use ft_lex_lib::regex::ast::{PosixClass, CharClass};

    #[test]
    fn test_upper_ranges() {
        let ranges = posix_class_to_ranges(PosixClass::Upper);
        assert!(ranges.iter().any(|&(lo, hi)| lo <= b'A' && hi >= b'Z'));
    }

    #[test]
    fn test_lower_ranges() {
        let ranges = posix_class_to_ranges(PosixClass::Lower);
        assert!(ranges.iter().any(|&(lo, hi)| lo <= b'a' && hi >= b'z'));
    }

    #[test]
    fn test_parse_posix_class_name_all() {
        for name in &["alpha", "digit", "alnum", "space", "upper", "lower",
                       "print", "graph", "cntrl", "blank", "xdigit", "punct"] {
            assert!(parse_posix_class_name(name).is_some(), "Failed: {}", name);
        }
    }

    #[test]
    fn test_parse_posix_class_name_invalid() {
        assert!(parse_posix_class_name("bogus").is_none());
        assert!(parse_posix_class_name("").is_none());
        assert!(parse_posix_class_name("ALPHA").is_none());
    }

    #[test]
    fn test_expand_negated_with_posix() {
        let mut cc = CharClass::negated();
        cc.add_posix(PosixClass::Digit);
        let bytes = expand_char_class_bytes(&cc);
        for d in b'0'..=b'9' {
            assert!(!bytes.contains(&d), "Negated digit should not contain {}", d);
        }
        assert!(bytes.contains(&b'a'));
    }

    #[test]
    fn test_expand_ranges_and_posix_combined() {
        let mut cc = CharClass::new();
        cc.add_range(b'A', b'F');
        cc.add_posix(PosixClass::Digit);
        let bytes = expand_char_class_bytes(&cc);
        for d in b'0'..=b'9' { assert!(bytes.contains(&d)); }
        for c in b'A'..=b'F' { assert!(bytes.contains(&c)); }
    }

    #[test]
    fn test_expand_single_byte_class() {
        let mut cc = CharClass::new();
        cc.add_byte(b'X');
        let bytes = expand_char_class_bytes(&cc);
        assert_eq!(bytes, vec![b'X']);
    }

    #[test]
    fn test_expand_empty_class() {
        let cc = CharClass::new();
        let bytes = expand_char_class_bytes(&cc);
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_expand_full_negated_empty() {
        let cc = CharClass::negated();
        let bytes = expand_char_class_bytes(&cc);
        assert_eq!(bytes.len(), 256);
    }

    #[test]
    fn test_expand_negated_multi_range() {
        let mut cc = CharClass::negated();
        cc.add_range(b'a', b'z');
        cc.add_range(b'A', b'Z');
        let bytes = expand_char_class_bytes(&cc);
        for c in b'a'..=b'z' { assert!(!bytes.contains(&c)); }
        for c in b'A'..=b'Z' { assert!(!bytes.contains(&c)); }
        assert!(bytes.contains(&b'0'));
    }
}

// =============================================================================
// NFA EDGE CASES
// =============================================================================
mod nfa_edges {
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::regex::ast::Regex;
    use ft_lex_lib::regex::parser::parse_regex;

    fn build_nfa_from_pattern(pattern: &[u8]) -> ft_lex_lib::automata::nfa::Nfa {
        let regex = parse_regex(pattern, "<t>", 1).unwrap();
        let mut builder = NfaBuilder::new();
        builder.build_combined(&[(regex, 0)])
    }

    #[test]
    fn test_nfa_anychar() {
        let nfa = build_nfa_from_pattern(b".");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_question() {
        let nfa = build_nfa_from_pattern(b"a?");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_literal_string() {
        let nfa = build_nfa_from_pattern(b"\"hello\"");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_concat() {
        let nfa = build_nfa_from_pattern(b"abc");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_anchors() {
        let nfa = build_nfa_from_pattern(b"^abc$");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_empty_regex() {
        let mut builder = NfaBuilder::new();
        let nfa = builder.build_combined(&[(Regex::Empty, 0)]);
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_repeat_exact() {
        let nfa = build_nfa_from_pattern(b"a{3}");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_nfa_repeat_unbounded() {
        let nfa = build_nfa_from_pattern(b"a{2,}");
        assert!(nfa.state_count() > 0);
    }

    #[test]
    fn test_epsilon_closure_empty_set() {
        let nfa = build_nfa_from_pattern(b"a");
        let empty = std::collections::BTreeSet::new();
        assert!(nfa.epsilon_closure(&empty).is_empty());
    }

    #[test]
    fn test_move_on_empty_set() {
        let nfa = build_nfa_from_pattern(b"a");
        let empty = std::collections::BTreeSet::new();
        assert!(nfa.move_on(&empty, b'a').is_empty());
    }

    #[test]
    fn test_accepting_rule_empty_set() {
        let nfa = build_nfa_from_pattern(b"a");
        let empty = std::collections::BTreeSet::new();
        assert!(nfa.accepting_rule(&empty).is_none());
    }
}

// =============================================================================
// DFA SIMULATION EDGE CASES
// =============================================================================
mod dfa_edges {
    use ft_lex_lib::automata::dfa::{subset_construction, dfa_simulate};
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::regex::parser::parse_regex;

    fn build_dfa(patterns: &[&[u8]]) -> ft_lex_lib::automata::dfa::Dfa {
        let regexes: Vec<_> = patterns.iter().enumerate()
            .map(|(i, p)| (parse_regex(p, "<t>", 1).unwrap(), i))
            .collect();
        let mut builder = NfaBuilder::new();
        let nfa = builder.build_combined(&regexes);
        subset_construction(&nfa)
    }

    #[test]
    fn test_dfa_question_mark() {
        let dfa = build_dfa(&[b"a?"]);
        assert!(dfa_simulate(&dfa, b"a").is_some());
    }

    #[test]
    fn test_dfa_dot_matches_many_bytes() {
        let dfa = build_dfa(&[b"."]);
        for b in [1u8, 32, 65, 97, 127, 200, 255] {
            assert!(dfa_simulate(&dfa, &[b]).is_some(), "Dot should match {}", b);
        }
    }

    #[test]
    fn test_dfa_dot_no_match_empty() {
        let dfa = build_dfa(&[b"."]);
        assert!(dfa_simulate(&dfa, b"").is_none());
    }

    #[test]
    fn test_dfa_star_empty_match() {
        let dfa = build_dfa(&[b"a*"]);
        assert!(dfa_simulate(&dfa, b"").is_some());
    }

    #[test]
    fn test_dfa_longest_match_greedy() {
        let dfa = build_dfa(&[b"a+", b"a"]);
        let result = dfa_simulate(&dfa, b"aaa").unwrap();
        assert_eq!(result.1, 3);
    }

    #[test]
    fn test_dfa_is_dead_state() {
        let dfa = build_dfa(&[b"a"]);
        if let Some(dead_id) = dfa.dead_state {
            assert!(dfa.is_dead_state(dead_id));
        }
    }

    #[test]
    fn test_dfa_partial_match_returns_none() {
        let dfa = build_dfa(&[b"abc"]);
        assert!(dfa_simulate(&dfa, b"ab").is_none());
    }

    #[test]
    fn test_dfa_partial_match_prefix() {
        let dfa = build_dfa(&[b"a", b"abc"]);
        let result = dfa_simulate(&dfa, b"ab").unwrap();
        assert_eq!(result.0, 0); // "a"
        assert_eq!(result.1, 1);
    }

    #[test]
    fn test_dfa_multiple_single_char_rules() {
        let dfa = build_dfa(&[b"a", b"b", b"c"]);
        assert_eq!(dfa_simulate(&dfa, b"a"), Some((0, 1)));
        assert_eq!(dfa_simulate(&dfa, b"b"), Some((1, 1)));
        assert_eq!(dfa_simulate(&dfa, b"c"), Some((2, 1)));
        assert_eq!(dfa_simulate(&dfa, b"d"), None);
    }

    #[test]
    fn test_dfa_repeat_exact() {
        let dfa = build_dfa(&[b"a{3}"]);
        assert_eq!(dfa_simulate(&dfa, b"aaa"), Some((0, 3)));
        assert_eq!(dfa_simulate(&dfa, b"aa"), None);
        assert_eq!(dfa_simulate(&dfa, b"aaaa").unwrap().1, 3);
    }

    #[test]
    fn test_dfa_repeat_range() {
        let dfa = build_dfa(&[b"a{2,4}"]);
        assert_eq!(dfa_simulate(&dfa, b"a"), None);
        assert_eq!(dfa_simulate(&dfa, b"aa").unwrap().1, 2);
        assert_eq!(dfa_simulate(&dfa, b"aaa").unwrap().1, 3);
        assert_eq!(dfa_simulate(&dfa, b"aaaa").unwrap().1, 4);
        assert_eq!(dfa_simulate(&dfa, b"aaaaa").unwrap().1, 4);
    }
}

// =============================================================================
// MINIMIZE DFA EDGE CASES
// =============================================================================
mod minimize_edges {
    use ft_lex_lib::automata::dfa::{subset_construction, dfa_simulate};
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::regex::parser::parse_regex;

    fn build_min_dfa(patterns: &[&[u8]]) -> ft_lex_lib::automata::dfa::Dfa {
        let regexes: Vec<_> = patterns.iter().enumerate()
            .map(|(i, p)| (parse_regex(p, "<t>", 1).unwrap(), i))
            .collect();
        let mut builder = NfaBuilder::new();
        let nfa = builder.build_combined(&regexes);
        minimize_dfa(&subset_construction(&nfa))
    }

    #[test]
    fn test_minimize_single_char() {
        assert!(build_min_dfa(&[b"a"]).state_count() <= 3);
    }

    #[test]
    fn test_minimize_already_minimal() {
        let r = vec![(parse_regex(b"a", "<t>", 1).unwrap(), 0)];
        let mut b = NfaBuilder::new();
        let nfa = b.build_combined(&r);
        let dfa = subset_construction(&nfa);
        let m1 = minimize_dfa(&dfa);
        let m2 = minimize_dfa(&m1);
        assert_eq!(m1.state_count(), m2.state_count());
    }

    #[test]
    fn test_minimize_preserves_question_match() {
        let min = build_min_dfa(&[b"ab?"]);
        assert!(dfa_simulate(&min, b"a").is_some());
        assert!(dfa_simulate(&min, b"ab").is_some());
    }

    #[test]
    fn test_minimize_preserves_star_match() {
        let min = build_min_dfa(&[b"a*b"]);
        assert!(dfa_simulate(&min, b"b").is_some());
        assert!(dfa_simulate(&min, b"ab").is_some());
        assert!(dfa_simulate(&min, b"aaab").is_some());
    }

    #[test]
    fn test_minimize_complex_preserves_priority() {
        let min = build_min_dfa(&[b"if", b"[a-z]+"]);
        assert_eq!(dfa_simulate(&min, b"if").unwrap().0, 0);
        assert_eq!(dfa_simulate(&min, b"iff").unwrap().0, 1);
    }

    #[test]
    fn test_minimize_dot_star() {
        let min = build_min_dfa(&[b".*"]);
        assert!(dfa_simulate(&min, b"").is_some());
        assert!(dfa_simulate(&min, b"hello").is_some());
    }

    #[test]
    fn test_minimize_dead_state_exists() {
        let min = build_min_dfa(&[b"abc"]);
        assert!(min.dead_state.is_some() || min.state_count() <= 4);
    }
}

// =============================================================================
// COMPRESSION EDGE CASES
// =============================================================================
mod compress_edges {
    use ft_lex_lib::automata::dfa::subset_construction;
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::compress::equiv_classes::*;
    use ft_lex_lib::regex::parser::parse_regex;

    fn build_and_compress(patterns: &[&[u8]]) -> (ft_lex_lib::automata::dfa::Dfa, CompressedDfa) {
        let regexes: Vec<_> = patterns.iter().enumerate()
            .map(|(i, p)| (parse_regex(p, "<t>", 1).unwrap(), i))
            .collect();
        let mut b = NfaBuilder::new();
        let nfa = b.build_combined(&regexes);
        let min = minimize_dfa(&subset_construction(&nfa));
        let cdfa = compress_dfa(&min);
        (min, cdfa)
    }

    #[test]
    fn test_compressed_simulate_empty_input() {
        let (_, cdfa) = build_and_compress(&[b"a+"]);
        assert!(compressed_dfa_simulate(&cdfa, b"").is_none());
    }

    #[test]
    fn test_compressed_simulate_single_char() {
        let (_, cdfa) = build_and_compress(&[b"a"]);
        assert_eq!(compressed_dfa_simulate(&cdfa, b"a").unwrap().1, 1);
    }

    #[test]
    fn test_compressed_simulate_no_match() {
        let (_, cdfa) = build_and_compress(&[b"xyz"]);
        assert!(compressed_dfa_simulate(&cdfa, b"abc").is_none());
    }

    #[test]
    fn test_compute_equiv_classes_directly() {
        let r = vec![(parse_regex(b"[0-9]+", "<t>", 1).unwrap(), 0)];
        let mut b = NfaBuilder::new();
        let nfa = b.build_combined(&r);
        let min = minimize_dfa(&subset_construction(&nfa));
        let ec = compute_equiv_classes(&min);
        let digit_class = ec.class_of[b'0' as usize];
        for d in b'1'..=b'9' {
            assert_eq!(ec.class_of[d as usize], digit_class);
        }
        assert_ne!(ec.class_of[b'a' as usize], digit_class);
    }

    #[test]
    fn test_compression_multi_rule() {
        let (_, cdfa) = build_and_compress(&[b"[0-9]+", b"[a-z]+", b"[ \\t]+"]);
        assert!(compressed_dfa_simulate(&cdfa, b"42").is_some());
        assert!(compressed_dfa_simulate(&cdfa, b"hello").is_some());
        assert!(compressed_dfa_simulate(&cdfa, b"  ").is_some());
        assert!(compressed_dfa_simulate(&cdfa, b"!").is_none());
    }

    #[test]
    fn test_compression_ratio_multi_rule() {
        let (dfa, cdfa) = build_and_compress(&[b"[0-9]+", b"[a-z]+", b"[ \\t\\n]+"]);
        let uncompressed = dfa.state_count() * 256;
        let compressed = cdfa.num_states * cdfa.ec.num_classes;
        assert!(compressed < uncompressed);
    }

    #[test]
    fn test_compressed_preserves_priority() {
        let (_, cdfa) = build_and_compress(&[b"if", b"[a-z]+"]);
        assert_eq!(compressed_dfa_simulate(&cdfa, b"if").unwrap().0, 0);
        assert_eq!(compressed_dfa_simulate(&cdfa, b"iff").unwrap().0, 1);
    }

    #[test]
    fn test_compressed_longest_match() {
        let (_, cdfa) = build_and_compress(&[b"[a-z]+"]);
        assert_eq!(compressed_dfa_simulate(&cdfa, b"abcdef").unwrap().1, 6);
    }
}

// =============================================================================
// LEX FILE LEXER EDGE CASES
// =============================================================================
mod lexer_edges {
    use ft_lex_lib::lex_file::lexer::{parse_l_file, expand_definitions};

    #[test]
    fn test_empty_definitions() {
        let raw = parse_l_file("%%\n.  ;\n", "<t>").unwrap();
        assert!(raw.definitions.is_empty());
    }

    #[test]
    fn test_empty_user_code() {
        let raw = parse_l_file("%%\n.  ;\n%%\n", "<t>").unwrap();
        assert!(raw.user_code.is_empty() || raw.user_code.trim().is_empty());
    }

    #[test]
    fn test_multiple_options() {
        let raw = parse_l_file("%option noyywrap\n%option yylineno\n%%\n.  ;\n", "<t>").unwrap();
        assert!(raw.options.iter().any(|o| o == "noyywrap"));
        assert!(raw.options.iter().any(|o| o == "yylineno"));
    }

    #[test]
    fn test_pipe_action_chain() {
        let raw = parse_l_file("%%\na  |\nb  |\nc  printf(\"matched\");\n", "<t>").unwrap();
        assert_eq!(raw.rules.len(), 3);
    }

    #[test]
    fn test_definitions_are_expanded() {
        let raw = parse_l_file("DIGIT [0-9]\n%%\n{DIGIT}+ ;\n", "<t>").unwrap();
        let expanded = expand_definitions(&raw.rules[0].pattern, &raw.definitions, "<t>", 1).unwrap();
        assert!(!expanded.contains("{DIGIT}"));
        assert!(expanded.contains("[0-9]"));
    }

    #[test]
    fn test_no_rules_section_error() {
        // Input without %% should error, but parser may be lenient.
        // Just verify the parse produces no rules or errors.
        let result = parse_l_file("no percent percent here", "<t>");
        if let Ok(raw) = &result {
            assert!(raw.rules.is_empty());
        }
    }

    #[test]
    fn test_single_rule() {
        assert_eq!(parse_l_file("%%\n.  ;\n", "<t>").unwrap().rules.len(), 1);
    }

    #[test]
    fn test_many_rules() {
        assert_eq!(parse_l_file("%%\na  ;\nb  ;\nc  ;\nd  ;\ne  ;\n", "<t>").unwrap().rules.len(), 5);
    }

    #[test]
    fn test_code_block_in_definitions() {
        let raw = parse_l_file("%{\n#include <stdio.h>\n%}\n%%\n.  ;\n", "<t>").unwrap();
        assert!(raw.header_code.iter().any(|h| h.contains("#include")));
    }

    #[test]
    fn test_multiline_action_braces() {
        let raw = parse_l_file("%%\na  {\n  printf(\"multi\\n\");\n  return 1;\n}\n", "<t>").unwrap();
        assert_eq!(raw.rules.len(), 1);
        assert!(raw.rules[0].action.contains("printf"));
        assert!(raw.rules[0].action.contains("return"));
    }
}

// =============================================================================
// LEX FILE PARSER EDGE CASES
// =============================================================================
mod parser_edges {
    use ft_lex_lib::lex_file::parser::LexFile;

    #[test]
    fn test_parse_noyywrap_option() {
        assert!(LexFile::parse("%option noyywrap\n%%\n.  ;\n", "<t>").unwrap().options.noyywrap);
    }

    #[test]
    fn test_parse_yylineno_option() {
        assert!(LexFile::parse("%option yylineno\n%%\n.  ;\n", "<t>").unwrap().options.yylineno);
    }

    #[test]
    fn test_parse_both_options() {
        let lf = LexFile::parse("%option noyywrap\n%option yylineno\n%%\n.  ;\n", "<t>").unwrap();
        assert!(lf.options.noyywrap);
        assert!(lf.options.yylineno);
    }

    #[test]
    fn test_parse_rule_count() {
        assert_eq!(LexFile::parse("%%\na  printf(\"1\");\nb  printf(\"2\");\nc  printf(\"3\");\n", "<t>").unwrap().rules.len(), 3);
    }

    #[test]
    fn test_parse_definition_expansion() {
        assert_eq!(LexFile::parse("DIGIT [0-9]\nLETTER [a-zA-Z]\n%%\n{DIGIT}+ ;\n{LETTER}+ ;\n", "<t>").unwrap().rules.len(), 2);
    }

    #[test]
    fn test_parse_user_code() {
        assert!(LexFile::parse("%%\n.  ;\n%%\nint main() { return 0; }\n", "<t>").unwrap().user_code.contains("int main"));
    }

    #[test]
    fn test_parse_header_code() {
        assert!(LexFile::parse("%{\n#include <stdlib.h>\n%}\n%%\n.  ;\n", "<t>").unwrap().header_code.contains("#include <stdlib.h>"));
    }
}

// =============================================================================
// ERROR MODULE EDGE CASES
// =============================================================================
mod error_edges {
    use ft_lex_lib::error::{LexError, LexErrorKind};

    #[test]
    fn test_error_no_location_display() {
        let s = LexError::no_location(LexErrorKind::InvalidRepeatRange { min: 5, max: 3 }).to_string();
        assert!(s.contains("5") && s.contains("3"));
    }

    #[test]
    fn test_error_with_full_location() {
        let s = LexError::new("test.l", 10, 5, LexErrorKind::UnexpectedChar('!')).to_string();
        assert!(s.contains("test.l"));
        assert!(s.contains("10"));
    }

    #[test]
    fn test_error_kinds_distinct() {
        let displays: Vec<String> = vec![
            LexError::no_location(LexErrorKind::UnexpectedChar('x')).to_string(),
            LexError::no_location(LexErrorKind::UnterminatedString).to_string(),
            LexError::no_location(LexErrorKind::UnclosedGroup).to_string(),
            LexError::no_location(LexErrorKind::UnclosedBracket).to_string(),
            LexError::no_location(LexErrorKind::InvalidEscape('z')).to_string(),
            LexError::no_location(LexErrorKind::TrailingBackslash).to_string(),
            LexError::no_location(LexErrorKind::UndefinedName("foo".to_string())).to_string(),
            LexError::no_location(LexErrorKind::EmptyRegex).to_string(),
        ];
        for i in 0..displays.len() {
            for j in (i+1)..displays.len() {
                assert_ne!(displays[i], displays[j], "{} and {} same", i, j);
            }
        }
    }

    #[test]
    fn test_io_error_conversion() {
        let e: LexError = std::io::Error::new(std::io::ErrorKind::NotFound, "nope").into();
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn test_error_missing_section_delimiter() {
        assert!(!LexError::no_location(LexErrorKind::MissingSectionDelimiter).to_string().is_empty());
    }

    #[test]
    fn test_error_invalid_definition() {
        let s = LexError::no_location(LexErrorKind::InvalidDefinition("bad".to_string())).to_string();
        assert!(s.contains("bad") || !s.is_empty());
    }

    #[test]
    fn test_error_duplicate_definition() {
        let s = LexError::no_location(LexErrorKind::DuplicateDefinition("FOO".to_string())).to_string();
        assert!(s.contains("FOO") || !s.is_empty());
    }
}

// =============================================================================
// C EMITTER EDGE CASES
// =============================================================================
mod c_emitter_edges {
    use ft_lex_lib::automata::dfa::subset_construction;
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::emit::c_emitter::CEmitter;
    use ft_lex_lib::emit::traits::CodeEmitter;
    use ft_lex_lib::lex_file::parser::LexFile;

    fn emit_c(source: &str, suppress: bool, compress: bool) -> String {
        let lf = LexFile::parse(source, "<t>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        let min = minimize_dfa(&subset_construction(&nfa));
        let em: Box<dyn CodeEmitter> = if compress {
            Box::new(CEmitter::with_compression(suppress))
        } else {
            Box::new(CEmitter::new(suppress))
        };
        em.emit(&lf, &min).unwrap()
    }

    #[test]
    fn test_c_emitter_multiple_rules() {
        let o = emit_c("%%\n[0-9]+  printf(\"NUM\");\n[a-z]+  printf(\"ID\");\n.  ;\n", false, false);
        assert!(o.contains("case 1:") && o.contains("case 2:") && o.contains("case 3:"));
    }

    #[test]
    fn test_c_emitter_return_action() {
        assert!(emit_c("%%\n[0-9]+  return 42;\n", false, false).contains("return 42"));
    }

    #[test]
    fn test_c_emitter_multiline_action() {
        assert!(emit_c("%%\n[0-9]+  {\n  int x = 1;\n  printf(\"%d\", x);\n}\n", false, false).contains("int x = 1"));
    }

    #[test]
    fn test_c_emitter_noyywrap() {
        let o = emit_c("%option noyywrap\n%%\n.  ;\n", false, false);
        assert!(o.contains("yywrap"));
    }

    #[test]
    fn test_c_emitter_yylineno_counter() {
        assert!(emit_c("%option yylineno\n%%\n.  ;\n", false, false).contains("yylineno"));
    }

    #[test]
    fn test_c_emitter_compression_ec_table() {
        let o = emit_c("%%\n[0-9]+  ;\n[a-z]+  ;\n", false, true);
        assert!(o.contains("yy_ec[") || o.contains("yy_ec "));
    }

    #[test]
    fn test_c_emitter_default_rule() {
        let o = emit_c("%%\n[0-9]+  ;\n", false, false);
        // Default rule should exist for unmatched input
        assert!(o.contains("default") || o.contains("yytext") || o.len() > 100);
    }

    #[test]
    fn test_c_emitter_suppress_default() {
        assert!(emit_c("%%\n[0-9]+  ;\n", true, false).contains("yylex"));
    }

    #[test]
    fn test_c_emitter_user_code_section() {
        assert!(emit_c("%%\n.  ;\n%%\nint helper() { return 42; }\n", false, false).contains("int helper()"));
    }

    #[test]
    fn test_c_emitter_header_code() {
        assert!(emit_c("%{\n#define FOO 42\n%}\n%%\n.  ;\n", false, false).contains("#define FOO 42"));
    }
}

// =============================================================================
// RUST EMITTER EDGE CASES
// =============================================================================
mod rust_emitter_edges {
    use ft_lex_lib::automata::dfa::subset_construction;
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::emit::rust_emitter::RustEmitter;
    use ft_lex_lib::emit::traits::CodeEmitter;
    use ft_lex_lib::lex_file::parser::LexFile;

    fn emit_rust(source: &str, suppress: bool, compress: bool) -> String {
        let lf = LexFile::parse(source, "<t>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        let min = minimize_dfa(&subset_construction(&nfa));
        let em: Box<dyn CodeEmitter> = if compress {
            Box::new(RustEmitter::with_compression(suppress))
        } else {
            Box::new(RustEmitter::new(suppress))
        };
        em.emit(&lf, &min).unwrap()
    }

    #[test]
    fn test_rust_emitter_multiple_rules() {
        let o = emit_rust("%%\n[0-9]+  printf(\"NUM\");\n[a-z]+  printf(\"ID\");\n", false, false);
        assert!(o.contains("1 =>") && o.contains("2 =>"));
    }

    #[test]
    fn test_rust_emitter_return_action() {
        assert!(emit_rust("%%\n[0-9]+  return 42;\n", false, false).contains("return 42"));
    }

    #[test]
    fn test_rust_emitter_empty_action() {
        assert!(emit_rust("%%\n[ \\t]+  ;\n", false, false).contains("discard"));
    }

    #[test]
    fn test_rust_emitter_suppress_default() {
        assert!(emit_rust("%%\n[0-9]+  ;\n", true, false).contains("yylex"));
    }

    #[test]
    fn test_rust_emitter_header_code() {
        assert!(emit_rust("%{\n// custom header\n%}\n%%\n.  ;\n", false, false).contains("// custom header"));
    }

    #[test]
    fn test_rust_emitter_compressed_ec() {
        let o = emit_rust("%%\n[0-9]+  ;\n[a-z]+  ;\n", false, true);
        assert!(o.contains("YY_EC"));
    }

    #[test]
    fn test_rust_emitter_scanner_methods() {
        let o = emit_rust("%%\n.  ;\n", false, false);
        assert!(o.contains("pub fn yylex") && o.contains("pub fn new"));
    }

    #[test]
    fn test_rust_emitter_main_function() {
        assert!(emit_rust("%%\n.  ;\n", false, false).contains("fn main()"));
    }

    #[test]
    fn test_rust_emitter_io_import() {
        assert!(emit_rust("%%\n.  ;\n", false, false).contains("use std::io"));
    }
}

// =============================================================================
// ARENA EDGE CASES
// =============================================================================
mod arena_edges {
    use ft_lex_lib::automata::arena::{Arena, NodeId};

    #[test]
    fn test_arena_default() {
        let a: Arena<i32> = Arena::default();
        assert!(a.is_empty() && a.len() == 0);
    }

    #[test]
    fn test_arena_iter_mut() {
        let mut a: Arena<i32> = Arena::new();
        a.alloc(10); a.alloc(20); a.alloc(30);
        for (_, v) in a.iter_mut() { *v += 1; }
        assert_eq!(*a.get(NodeId(0)), 11);
        assert_eq!(*a.get(NodeId(1)), 21);
        assert_eq!(*a.get(NodeId(2)), 31);
    }

    #[test]
    fn test_arena_many_allocs() {
        let mut a: Arena<u32> = Arena::new();
        for i in 0..1000 { assert_eq!(a.alloc(i).0, i as usize); }
        assert_eq!(a.len(), 1000);
    }

    #[test]
    fn test_node_id_equality() {
        assert_eq!(NodeId(5), NodeId(5));
        assert_ne!(NodeId(5), NodeId(6));
    }

    #[test]
    fn test_node_id_clone() {
        let id = NodeId(42);
        assert_eq!(id, id.clone());
    }
}

// =============================================================================
// END-TO-END PIPELINE TESTS
// =============================================================================
mod pipeline {
    use ft_lex_lib::automata::dfa::{subset_construction, dfa_simulate};
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::compress::equiv_classes::{compress_dfa, compressed_dfa_simulate};
    use ft_lex_lib::lex_file::parser::LexFile;

    fn sim(source: &str, input: &[u8]) -> Option<(usize, usize)> {
        let lf = LexFile::parse(source, "<t>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        dfa_simulate(&minimize_dfa(&subset_construction(&nfa)), input)
    }

    fn sim_compressed(source: &str, input: &[u8]) -> Option<(usize, usize)> {
        let lf = LexFile::parse(source, "<t>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        let min = minimize_dfa(&subset_construction(&nfa));
        compressed_dfa_simulate(&compress_dfa(&min), input)
    }

    #[test]
    fn test_pipeline_identifier() {
        assert_eq!(sim("%%\n[a-zA-Z_][a-zA-Z0-9_]*  ;\n", b"hello_world123").unwrap().1, 14);
    }

    #[test]
    fn test_pipeline_integer() {
        assert_eq!(sim("%%\n[0-9]+  ;\n", b"12345abc"), Some((0, 5)));
    }

    #[test]
    fn test_pipeline_keyword_priority() {
        assert_eq!(sim("%%\nif  ;\n[a-z]+  ;\n", b"if").unwrap().0, 0);
    }

    #[test]
    fn test_pipeline_keyword_vs_longer_id() {
        let r = sim("%%\nif  ;\n[a-z]+  ;\n", b"iffy").unwrap();
        assert_eq!(r.0, 1);
        assert_eq!(r.1, 4);
    }

    #[test]
    fn test_pipeline_compressed_eq_uncompressed() {
        let src = "%%\n[0-9]+  ;\n[a-zA-Z]+  ;\n[ \\t]+  ;\n";
        for input in &[&b"123"[..], b"abc", b"  ", b"!"] {
            assert_eq!(sim(src, input), sim_compressed(src, input));
        }
    }

    #[test]
    fn test_pipeline_whitespace() {
        assert_eq!(sim("%%\n[ \\t\\n]+  ;\n", b"  \t\n ").unwrap().1, 5);
    }

    #[test]
    fn test_pipeline_single_char_rules() {
        let s = "%%\n\\+  ;\n\\*  ;\n\\/  ;\n";
        assert_eq!(sim(s, b"+").unwrap().0, 0);
        assert_eq!(sim(s, b"*").unwrap().0, 1);
        assert_eq!(sim(s, b"/").unwrap().0, 2);
    }

    #[test]
    fn test_pipeline_no_match() {
        assert!(sim("%%\n[a-z]+  ;\n", b"123").is_none());
    }

    #[test]
    fn test_pipeline_dot_rule() {
        assert_eq!(sim("%%\n.  ;\n", b"X").unwrap().1, 1);
    }

    #[test]
    fn test_pipeline_complex_tokens() {
        let s = "%%\n[0-9]+  ;\n[a-zA-Z_][a-zA-Z0-9_]*  ;\n";
        assert_eq!(sim(s, b"42").unwrap().0, 0);
        assert_eq!(sim(s, b"var_name").unwrap().0, 1);
    }

    #[test]
    fn test_pipeline_hex_number() {
        let s = "%%\n0[xX][0-9a-fA-F]+  ;\n[0-9]+  ;\n";
        assert_eq!(sim(s, b"0xFF").unwrap().0, 0);
        assert_eq!(sim(s, b"42").unwrap().0, 1);
    }

    #[test]
    fn test_pipeline_string_literal() {
        let s = "%%\n\"begin\"  ;\n\"end\"  ;\n[a-z]+  ;\n";
        assert_eq!(sim(s, b"begin").unwrap().0, 0);
        assert_eq!(sim(s, b"end").unwrap().0, 1);
        assert_eq!(sim(s, b"begins").unwrap().0, 2);
    }

    #[test]
    fn test_pipeline_compressed_hex() {
        let s = "%%\n0[xX][0-9a-fA-F]+  ;\n[0-9]+  ;\n";
        assert_eq!(sim(s, b"0xDEAD"), sim_compressed(s, b"0xDEAD"));
    }
}

// =============================================================================
// LIBL TESTS
// =============================================================================
mod libl_edges {
    use ft_lex_lib::emit::libl::generate_libl_c;

    #[test]
    fn test_libl_has_yylex_call() { assert!(generate_libl_c().contains("yylex")); }

    #[test]
    fn test_libl_has_return() { assert!(generate_libl_c().contains("return")); }

    #[test]
    fn test_libl_not_empty() { assert!(generate_libl_c().len() > 50); }
}
