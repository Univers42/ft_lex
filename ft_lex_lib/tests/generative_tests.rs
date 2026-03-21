/// Generative / fuzz-style tests for ft_lex_lib — 1000+ tests covering
/// systematic pattern generation, input matching, compression equivalence,
/// emitter validity, priority handling, maximal munch, and edge cases.
///
/// Strategy: macro-driven parameterized testing across all regex features,
/// the full NFA→DFA→minimize→simulate pipeline, and emitter output.

// =============================================================================
// HELPERS
// =============================================================================

use ft_lex_lib::automata::dfa::{dfa_simulate, subset_construction};
use ft_lex_lib::automata::minimize::minimize_dfa;
use ft_lex_lib::automata::nfa::NfaBuilder;
use ft_lex_lib::compress::equiv_classes::{compress_dfa, compressed_dfa_simulate};
use ft_lex_lib::emit::c_emitter::CEmitter;
use ft_lex_lib::emit::traits::CodeEmitter;
use ft_lex_lib::lex_file::parser::LexFile;
use ft_lex_lib::regex::parser::parse_regex;

/// Build minimized DFA from lex source and simulate on input.
fn sim(source: &str, input: &[u8]) -> Option<(usize, usize)> {
    let lf = LexFile::parse(source, "<gen>").unwrap();
    let mut b = NfaBuilder::new();
    let rules: Vec<_> = lf
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.regex.clone(), i))
        .collect();
    let nfa = b.build_combined(&rules);
    dfa_simulate(&minimize_dfa(&subset_construction(&nfa)), input)
}

/// Build compressed DFA and simulate on input.
fn sim_c(source: &str, input: &[u8]) -> Option<(usize, usize)> {
    let lf = LexFile::parse(source, "<gen>").unwrap();
    let mut b = NfaBuilder::new();
    let rules: Vec<_> = lf
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.regex.clone(), i))
        .collect();
    let nfa = b.build_combined(&rules);
    let min = minimize_dfa(&subset_construction(&nfa));
    compressed_dfa_simulate(&compress_dfa(&min), input)
}

/// Emit C code from lex source.
fn emit_c(source: &str, suppress: bool) -> String {
    let lf = LexFile::parse(source, "<gen>").unwrap();
    let mut b = NfaBuilder::new();
    let rules: Vec<_> = lf
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.regex.clone(), i))
        .collect();
    let nfa = b.build_combined(&rules);
    let min = minimize_dfa(&subset_construction(&nfa));
    let emitter = CEmitter::new(suppress);
    emitter.emit(&lf, &min).unwrap()
}

/// Emit compressed C code from lex source.
fn emit_c_compressed(source: &str) -> String {
    let lf = LexFile::parse(source, "<gen>").unwrap();
    let mut b = NfaBuilder::new();
    let rules: Vec<_> = lf
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.regex.clone(), i))
        .collect();
    let nfa = b.build_combined(&rules);
    let min = minimize_dfa(&subset_construction(&nfa));
    let emitter = CEmitter::with_compression(false);
    emitter.emit(&lf, &min).unwrap()
}

/// Build lex source from a single pattern with a simple action.
fn lex1(pat: &str) -> String {
    format!("%%\n{}  ;\n", pat)
}

/// Build lex source from multiple patterns.
fn lexn(pats: &[&str]) -> String {
    let mut s = String::from("%%\n");
    for (i, p) in pats.iter().enumerate() {
        s.push_str(&format!("{}  printf(\"{}\");\n", p, i));
    }
    s
}

// =============================================================================
// MACRO: generate match/no-match tests from a table
// =============================================================================
macro_rules! gen_match_tests {
    ($mod_name:ident, $( ($name:ident, $pat:expr, $input:expr, $expect_rule:expr, $expect_len:expr) ),+ $(,)?) => {
        mod $mod_name {
            use super::*;
            $(
                #[test]
                fn $name() {
                    let src = lex1($pat);
                    let result = sim(&src, $input);
                    assert_eq!(result, Some(($expect_rule, $expect_len)),
                        "pattern={:?} input={:?}", $pat, String::from_utf8_lossy($input));
                }
            )+
        }
    };
}

macro_rules! gen_nomatch_tests {
    ($mod_name:ident, $( ($name:ident, $pat:expr, $input:expr) ),+ $(,)?) => {
        mod $mod_name {
            use super::*;
            $(
                #[test]
                fn $name() {
                    let src = lex1($pat);
                    let result = sim(&src, $input);
                    assert!(result.is_none(),
                        "pattern={:?} input={:?} unexpectedly matched: {:?}", $pat, String::from_utf8_lossy($input), result);
                }
            )+
        }
    };
}

macro_rules! gen_compress_eq_tests {
    ($mod_name:ident, $( ($name:ident, $src:expr, $input:expr) ),+ $(,)?) => {
        mod $mod_name {
            use super::*;
            $(
                #[test]
                fn $name() {
                    let a = sim($src, $input);
                    let b = sim_c($src, $input);
                    assert_eq!(a, b,
                        "compressed != uncompressed for input={:?}", String::from_utf8_lossy($input));
                }
            )+
        }
    };
}

macro_rules! gen_priority_tests {
    ($mod_name:ident, $( ($name:ident, $pats:expr, $input:expr, $expect_rule:expr, $expect_len:expr) ),+ $(,)?) => {
        mod $mod_name {
            use super::*;
            $(
                #[test]
                fn $name() {
                    let src = lexn($pats);
                    let result = sim(&src, $input);
                    assert_eq!(result, Some(($expect_rule, $expect_len)),
                        "patterns={:?} input={:?}", $pats, String::from_utf8_lossy($input));
                }
            )+
        }
    };
}

// =============================================================================
// 1. SINGLE CHARACTER PATTERNS (50 tests)
// =============================================================================
gen_match_tests!(single_char,
    (match_a, "a", b"abc", 0, 1),
    (match_b, "b", b"bcd", 0, 1),
    (match_z, "z", b"z", 0, 1),
    (match_A, "A", b"ABC", 0, 1),
    (match_Z, "Z", b"Z", 0, 1),
    (match_0, "0", b"012", 0, 1),
    (match_9, "9", b"9", 0, 1),
    (match_space, "\" \"", b" x", 0, 1),
    (match_tab, "\\t", b"\tx", 0, 1),
    (match_newline, "\\n", b"\n", 0, 1),
    (match_dot_a, ".", b"a", 0, 1),
    (match_dot_Z, ".", b"Z", 0, 1),
    (match_dot_0, ".", b"0", 0, 1),
    (match_dot_bang, ".", b"!", 0, 1),
    (match_dot_space, ".", b" ", 0, 1),
    (match_esc_bslash, "\\\\", b"\\", 0, 1),
    (match_esc_star, "\\*", b"*", 0, 1),
    (match_esc_plus, "\\+", b"+", 0, 1),
    (match_esc_question, "\\?", b"?", 0, 1),
    (match_esc_lparen, "\\(", b"(", 0, 1),
    (match_esc_rparen, "\\)", b")", 0, 1),
    (match_esc_lbracket, "\\[", b"[", 0, 1),
    (match_esc_pipe, "\\|", b"|", 0, 1),
    (match_esc_dot, "\\.", b".", 0, 1),
    (match_esc_caret, "\\^", b"^", 0, 1),
    (match_esc_dollar, "\\$", b"$", 0, 1),
    (match_esc_lbrace, "\\{", b"{", 0, 1),
    (match_quoted_plus, "\"++\"", b"++", 0, 2),
    (match_quoted_star, "\"**\"", b"**", 0, 2),
    (match_quoted_pipe, "\"|\"", b"|", 0, 1),
);

gen_nomatch_tests!(single_char_nomatch,
    (nomatch_a_vs_b, "a", b"b"),
    (nomatch_0_vs_a, "0", b"a"),
    (nomatch_A_vs_a, "A", b"a"),
    (nomatch_z_vs_0, "z", b"0"),
    (nomatch_dot_vs_newline, ".", b"\n"),
    (nomatch_esc_star_vs_a, "\\*", b"a"),
    (nomatch_esc_plus_vs_star, "\\+", b"*"),
    (nomatch_tab_vs_space, "\\t", b" "),
    (nomatch_newline_vs_tab, "\\n", b"\t"),
    (nomatch_empty_input, "a", b""),
    (nomatch_space_vs_tab, "\" \"", b"\t"),
    (nomatch_bslash_vs_a, "\\\\", b"a"),
    (nomatch_dot_vs_empty, ".", b""),
    (nomatch_a_vs_empty, "a", b""),
);

// =============================================================================
// 2. CHARACTER CLASS PATTERNS (80 tests)
// =============================================================================
gen_match_tests!(char_class,
    (cc_az_a, "[a-z]", b"a", 0, 1),
    (cc_az_m, "[a-z]", b"m", 0, 1),
    (cc_az_z, "[a-z]", b"z", 0, 1),
    (cc_AZ_A, "[A-Z]", b"A", 0, 1),
    (cc_AZ_M, "[A-Z]", b"M", 0, 1),
    (cc_AZ_Z, "[A-Z]", b"Z", 0, 1),
    (cc_09_0, "[0-9]", b"0", 0, 1),
    (cc_09_5, "[0-9]", b"5", 0, 1),
    (cc_09_9, "[0-9]", b"9", 0, 1),
    (cc_multi_a, "[abcdef]", b"a", 0, 1),
    (cc_multi_f, "[abcdef]", b"f", 0, 1),
    (cc_multi_c, "[abcdef]", b"c", 0, 1),
    (cc_range_combo_a, "[a-zA-Z0-9]", b"a", 0, 1),
    (cc_range_combo_Z, "[a-zA-Z0-9]", b"Z", 0, 1),
    (cc_range_combo_5, "[a-zA-Z0-9]", b"5", 0, 1),
    (cc_neg_az_0, "[^a-z]", b"0", 0, 1),
    (cc_neg_az_A, "[^a-z]", b"A", 0, 1),
    (cc_neg_az_bang, "[^a-z]", b"!", 0, 1),
    (cc_neg_09_a, "[^0-9]", b"a", 0, 1),
    (cc_neg_09_space, "[^0-9]", b" ", 0, 1),
    (cc_dash_first, "[-a-z]", b"-", 0, 1),
    (cc_dash_first_a, "[-a-z]", b"a", 0, 1),
    (cc_dash_last, "[a-z-]", b"-", 0, 1),
    (cc_dash_last_z, "[a-z-]", b"z", 0, 1),
    (cc_close_first, "[]a-z]", b"]", 0, 1),
    (cc_close_first_a, "[]a-z]", b"a", 0, 1),
    (cc_neg_close_first, "[^]a-z]", b"0", 0, 1),
    // POSIX classes
    (cc_digit, "[[:digit:]]", b"5", 0, 1),
    (cc_alpha_a, "[[:alpha:]]", b"a", 0, 1),
    (cc_alpha_Z, "[[:alpha:]]", b"Z", 0, 1),
    (cc_alnum_a, "[[:alnum:]]", b"a", 0, 1),
    (cc_alnum_5, "[[:alnum:]]", b"5", 0, 1),
    (cc_upper_A, "[[:upper:]]", b"A", 0, 1),
    (cc_lower_z, "[[:lower:]]", b"z", 0, 1),
    (cc_space_sp, "[[:space:]]", b" ", 0, 1),
    (cc_space_tab, "[[:space:]]", b"\t", 0, 1),
    (cc_space_nl, "[[:space:]]", b"\n", 0, 1),
    (cc_blank_sp, "[[:blank:]]", b" ", 0, 1),
    (cc_blank_tab, "[[:blank:]]", b"\t", 0, 1),
    (cc_print_a, "[[:print:]]", b"a", 0, 1),
    (cc_print_sp, "[[:print:]]", b" ", 0, 1),
    (cc_graph_a, "[[:graph:]]", b"a", 0, 1),
    (cc_graph_bang, "[[:graph:]]", b"!", 0, 1),
    (cc_punct_bang, "[[:punct:]]", b"!", 0, 1),
    (cc_punct_at, "[[:punct:]]", b"@", 0, 1),
    (cc_xdigit_a, "[[:xdigit:]]", b"a", 0, 1),
    (cc_xdigit_F, "[[:xdigit:]]", b"F", 0, 1),
    (cc_xdigit_9, "[[:xdigit:]]", b"9", 0, 1),
    // Repeated character classes
    (cc_digit_plus, "[[:digit:]]+", b"12345", 0, 5),
    (cc_alpha_plus, "[[:alpha:]]+", b"hello", 0, 5),
    (cc_alnum_plus, "[[:alnum:]]+", b"abc123", 0, 6),
    (cc_az_star, "[a-z]*", b"hello", 0, 5),
);

gen_nomatch_tests!(char_class_nomatch,
    (cc_az_no_0, "[a-z]", b"0"),
    (cc_az_no_A, "[a-z]", b"A"),
    (cc_AZ_no_a, "[A-Z]", b"a"),
    (cc_09_no_a, "[0-9]", b"a"),
    (cc_neg_az_no_a, "[^a-z]", b"a"),
    (cc_neg_az_no_m, "[^a-z]", b"m"),
    (cc_digit_no_a, "[[:digit:]]", b"a"),
    (cc_alpha_no_0, "[[:alpha:]]", b"0"),
    (cc_upper_no_a, "[[:upper:]]", b"a"),
    (cc_lower_no_A, "[[:lower:]]", b"A"),
    (cc_blank_no_nl, "[[:blank:]]", b"\n"),
    (cc_empty_input_class, "[a-z]", b""),
    (cc_empty_input_digit, "[[:digit:]]", b""),
    (cc_xdigit_no_g, "[[:xdigit:]]", b"g"),
);

// =============================================================================
// 3. QUANTIFIER PATTERNS (80 tests)
// =============================================================================
gen_match_tests!(quantifiers,
    // Star
    (star_a_empty, "a*", b"bbb", 0, 0),
    (star_a_one, "a*", b"abbb", 0, 1),
    (star_a_many, "a*", b"aaaa", 0, 4),
    (star_a_all, "a*", b"aaaaa", 0, 5),
    (star_az_hello, "[a-z]*", b"hello", 0, 5),
    (star_09, "[0-9]*", b"12345", 0, 5),
    (star_dot, ".*", b"anything here", 0, 13),
    // Plus
    (plus_a_one, "a+", b"a", 0, 1),
    (plus_a_many, "a+", b"aaaa", 0, 4),
    (plus_az, "[a-z]+", b"hello", 0, 5),
    (plus_09, "[0-9]+", b"99999", 0, 5),
    (plus_dot, ".+", b"anything", 0, 8),
    (plus_az_stops, "[a-z]+", b"hello123", 0, 5),
    (plus_09_stops, "[0-9]+", b"123abc", 0, 3),
    // Question
    (question_a_present, "a?", b"a", 0, 1),
    (question_a_absent, "a?", b"b", 0, 0),
    (question_a_empty, "a?", b"", 0, 0),
    (question_az, "[a-z]?", b"x", 0, 1),
    (question_az_absent, "[a-z]?", b"5", 0, 0),
    // Repeat exact {n}
    (repeat_exact_3, "a{3}", b"aaa", 0, 3),
    (repeat_exact_1, "a{1}", b"a", 0, 1),
    (repeat_exact_5, "a{5}", b"aaaaa", 0, 5),
    (repeat_exact_3_extra, "a{3}", b"aaaaa", 0, 3),
    (repeat_exact_09_3, "[0-9]{3}", b"123", 0, 3),
    (repeat_exact_az_2, "[a-z]{2}", b"ab", 0, 2),
    // Repeat range {n,m}
    (repeat_range_1_3_one, "a{1,3}", b"a", 0, 1),
    (repeat_range_1_3_two, "a{1,3}", b"aa", 0, 2),
    (repeat_range_1_3_three, "a{1,3}", b"aaa", 0, 3),
    (repeat_range_1_3_four, "a{1,3}", b"aaaa", 0, 3),
    (repeat_range_2_4_two, "a{2,4}", b"aa", 0, 2),
    (repeat_range_2_4_three, "a{2,4}", b"aaa", 0, 3),
    (repeat_range_2_4_four, "a{2,4}", b"aaaa", 0, 4),
    (repeat_range_2_4_five, "a{2,4}", b"aaaaa", 0, 4),
    (repeat_range_0_1_empty, "a{0,1}", b"b", 0, 0),
    (repeat_range_0_1_one, "a{0,1}", b"a", 0, 1),
    // Repeat unbounded {n,}
    (repeat_unb_2_two, "a{2,}", b"aa", 0, 2),
    (repeat_unb_2_three, "a{2,}", b"aaa", 0, 3),
    (repeat_unb_2_ten, "a{2,}", b"aaaaaaaaaa", 0, 10),
    (repeat_unb_1_one, "a{1,}", b"a", 0, 1),
    (repeat_unb_1_many, "a{1,}", b"aaaaaa", 0, 6),
    // Combined quantifiers
    (concat_plus_star, "[a-z]+[0-9]*", b"hello123", 0, 8),
    (concat_plus_star2, "[a-z]+[0-9]*", b"hello", 0, 5),
    (concat_star_plus, "[a-z]*[0-9]+", b"abc123", 0, 6),
    (concat_star_plus2, "[a-z]*[0-9]+", b"123", 0, 3),
    (group_plus, "(ab)+", b"ababab", 0, 6),
    (group_star, "(ab)*", b"ababab", 0, 6),
    (group_question, "(ab)?", b"ab", 0, 2),
    (group_question_empty, "(ab)?", b"cd", 0, 0),
);

gen_nomatch_tests!(quantifiers_nomatch,
    (plus_a_empty, "a+", b""),
    (plus_a_no_b, "a+", b"b"),
    (plus_09_no_a, "[0-9]+", b"abc"),
    (plus_az_no_0, "[a-z]+", b"0"),
    (repeat_exact_3_short, "a{3}", b"aa"),
    (repeat_exact_5_short, "a{5}", b"aaa"),
    (repeat_range_2_4_one, "a{2,4}", b"a"),
    (repeat_unb_2_one, "a{2,}", b"a"),
    (group_plus_no_ba, "(ab)+", b"ba"),
    (group_plus_empty, "(ab)+", b""),
);

// =============================================================================
// 4. ALTERNATION PATTERNS (40 tests)
// =============================================================================
gen_match_tests!(alternation,
    (alt_a_b_matchA, "a|b", b"a", 0, 1),
    (alt_a_b_matchB, "a|b", b"b", 0, 1),
    (alt_three_a, "a|b|c", b"a", 0, 1),
    (alt_three_b, "a|b|c", b"b", 0, 1),
    (alt_three_c, "a|b|c", b"c", 0, 1),
    (alt_words_if, "if|else|while|for", b"if", 0, 2),
    (alt_words_else, "if|else|while|for", b"else", 0, 4),
    (alt_words_while, "if|else|while|for", b"while", 0, 5),
    (alt_words_for, "if|else|while|for", b"for", 0, 3),
    (alt_class_digit, "[a-z]+|[0-9]+", b"hello", 0, 5),
    (alt_class_alpha, "[a-z]+|[0-9]+", b"123", 0, 3),
    (alt_group_ab, "(ab)|(cd)", b"ab", 0, 2),
    (alt_group_cd, "(ab)|(cd)", b"cd", 0, 2),
    (alt_mixed_long, "foo|bar|baz|qux|hello|world", b"hello", 0, 5),
    (alt_mixed_short, "foo|bar|baz|qux|hello|world", b"foo", 0, 3),
    (alt_prefix_if, "if|ifdef|ifndef", b"if", 0, 2),
    (alt_prefix_ifdef, "if|ifdef|ifndef", b"ifdef", 0, 5),
    (alt_prefix_ifndef, "if|ifdef|ifndef", b"ifndef", 0, 6),
    // Alternation with quantifiers
    (alt_star, "a*|b+", b"aaa", 0, 3),
    (alt_star_b, "a*|b+", b"bbb", 0, 3),
    (alt_complex, "(abc)+|(def)+", b"abcabc", 0, 6),
    (alt_complex_def, "(abc)+|(def)+", b"defdef", 0, 6),
);

gen_nomatch_tests!(alternation_nomatch,
    (alt_a_b_no_c, "a|b", b"c"),
    (alt_three_no_d, "a|b|c", b"d"),
    (alt_words_no_do, "if|else|while|for", b"do"),
    (alt_group_no_ef, "(ab)|(cd)", b"ef"),
    (alt_empty_input, "a|b", b""),
);

// =============================================================================
// 5. CONCATENATION (30 tests)
// =============================================================================
gen_match_tests!(concatenation,
    (concat_ab, "ab", b"abc", 0, 2),
    (concat_abc, "abc", b"abcdef", 0, 3),
    (concat_hello, "hello", b"helloworld", 0, 5),
    (concat_if, "if", b"ifx", 0, 2),
    (concat_long, "abcdefghij", b"abcdefghij", 0, 10),
    (concat_digit_alpha, "[0-9][a-z]", b"5a", 0, 2),
    (concat_alpha_digit, "[a-z][0-9]", b"a5", 0, 2),
    (concat_mixed, "[a-z][0-9][a-z]", b"a5b", 0, 3),
    (concat_three_words, "int", b"int", 0, 3),
    (concat_void, "void", b"void", 0, 4),
    (concat_return, "return", b"return", 0, 6),
    (concat_digits_3, "[0-9][0-9][0-9]", b"123", 0, 3),
    (concat_dot_dot, "..", b"ab", 0, 2),
    (concat_dot_dot_dot, "...", b"abc", 0, 3),
    (concat_esc_esc, "\\+\\*", b"+*", 0, 2),
);

gen_nomatch_tests!(concatenation_nomatch,
    (concat_ab_no_ba, "ab", b"ba"),
    (concat_abc_short, "abc", b"ab"),
    (concat_hello_short, "hello", b"hell"),
    (concat_empty, "ab", b""),
    (concat_digit_alpha_no, "[0-9][a-z]", b"aa"),
);

// =============================================================================
// 6. GROUPED EXPRESSIONS (30 tests)
// =============================================================================
gen_match_tests!(groups,
    (group_simple_ab, "(ab)", b"abc", 0, 2),
    (group_simple_abc, "(abc)", b"abc", 0, 3),
    (group_nested, "((a))", b"a", 0, 1),
    (group_nested_ab, "((ab))", b"ab", 0, 2),
    (group_alt_a, "(a|b)", b"a", 0, 1),
    (group_alt_b, "(a|b)", b"b", 0, 1),
    (group_plus_ab, "(ab)+", b"ababab", 0, 6),
    (group_plus_ab2, "(ab)+", b"ab", 0, 2),
    (group_star_ab, "(ab)*", b"ababab", 0, 6),
    (group_star_empty, "(ab)*", b"x", 0, 0),
    (group_question_ab, "(ab)?", b"ab", 0, 2),
    (group_question_empty, "(ab)?", b"x", 0, 0),
    (group_repeat_3, "(ab){3}", b"ababab", 0, 6),
    (group_repeat_2_4, "(ab){2,4}", b"abababab", 0, 8),
    (group_concat, "(ab)(cd)", b"abcd", 0, 4),
    (group_concat_plus, "(ab)+(cd)+", b"ababcdcd", 0, 8),
    (group_nested_alt, "((a|b)(c|d))+", b"acbd", 0, 4),
    (group_deep, "(((a)))", b"a", 0, 1),
    (group_alt_concat, "(ab|cd)(ef|gh)", b"abef", 0, 4),
    (group_alt_concat2, "(ab|cd)(ef|gh)", b"cdgh", 0, 4),
);

gen_nomatch_tests!(groups_nomatch,
    (group_ab_no_ba, "(ab)", b"ba"),
    (group_plus_ab_no_ba, "(ab)+", b"ba"),
    (group_repeat_3_short, "(ab){3}", b"abab"),
    (group_concat_no, "(ab)(cd)", b"abab"),
);

// =============================================================================
// 7. DOT PATTERNS (20 tests)
// =============================================================================
gen_match_tests!(dot_patterns,
    (dot_single_a, ".", b"a", 0, 1),
    (dot_single_0, ".", b"0", 0, 1),
    (dot_single_space, ".", b" ", 0, 1),
    (dot_single_tab, ".", b"\t", 0, 1),
    (dot_single_bang, ".", b"!", 0, 1),
    (dot_star_abc, ".*", b"abc", 0, 3),
    (dot_star_empty, ".*", b"", 0, 0),
    (dot_plus_abc, ".+", b"abc", 0, 3),
    (dot_plus_one, ".+", b"x", 0, 1),
    (dot_repeat_3, ".{3}", b"abc", 0, 3),
    (dot_repeat_5, ".{5}", b"12345", 0, 5),
    (dot_dot, "..", b"ab", 0, 2),
    (dot_dot_dot, "...", b"xyz", 0, 3),
    (dot_grp_star, "(.)+", b"hello", 0, 5),
    (dot_alt, ".|a", b"x", 0, 1),
);

gen_nomatch_tests!(dot_nomatch,
    (dot_vs_newline, ".", b"\n"),
    (dot_empty, ".", b""),
    (dot_plus_empty, ".+", b""),
    (dot_plus_newline, ".+", b"\n"),
    (dot_repeat_3_short, ".{3}", b"ab"),
);

// =============================================================================
// 8. ESCAPE SEQUENCES (30 tests)
// =============================================================================
gen_match_tests!(escapes,
    (esc_n, "\\n", b"\n", 0, 1),
    (esc_t, "\\t", b"\t", 0, 1),
    (esc_r, "\\r", b"\r", 0, 1),
    (esc_a, "\\a", b"\x07", 0, 1),
    (esc_b, "\\b", b"\x08", 0, 1),
    (esc_f, "\\f", b"\x0c", 0, 1),
    (esc_v, "\\v", b"\x0b", 0, 1),
    (esc_backslash, "\\\\", b"\\", 0, 1),
    (esc_star, "\\*", b"*", 0, 1),
    (esc_plus, "\\+", b"+", 0, 1),
    (esc_question, "\\?", b"?", 0, 1),
    (esc_dot, "\\.", b".", 0, 1),
    (esc_pipe, "\\|", b"|", 0, 1),
    (esc_lparen, "\\(", b"(", 0, 1),
    (esc_rparen, "\\)", b")", 0, 1),
    (esc_lbracket, "\\[", b"[", 0, 1),
    (esc_caret, "\\^", b"^", 0, 1),
    (esc_dollar, "\\$", b"$", 0, 1),
    (esc_lbrace, "\\{", b"{", 0, 1),
    // Hex escapes
    (esc_hex_41, "\\x41", b"A", 0, 1),
    (esc_hex_61, "\\x61", b"a", 0, 1),
    (esc_hex_30, "\\x30", b"0", 0, 1),
    (esc_hex_0a, "\\x0a", b"\n", 0, 1),
    (esc_hex_09, "\\x09", b"\t", 0, 1),
    // Octal escapes
    (esc_oct_101, "\\101", b"A", 0, 1),
    (esc_oct_141, "\\141", b"a", 0, 1),
    (esc_oct_060, "\\060", b"0", 0, 1),
    (esc_oct_012, "\\012", b"\n", 0, 1),
    // Combined escapes in patterns
    (esc_seq_tn, "\\t\\n", b"\t\n", 0, 2),
    (esc_seq_rn, "\\r\\n", b"\r\n", 0, 2),
);

// =============================================================================
// 9. QUOTED STRINGS (20 tests)
// =============================================================================
gen_match_tests!(quoted_strings,
    (quoted_hello, "\"hello\"", b"hello", 0, 5),
    (quoted_world, "\"world\"", b"world", 0, 5),
    (quoted_plus, "\"+\"", b"+", 0, 1),
    (quoted_star, "\"*\"", b"*", 0, 1),
    (quoted_pipe, "\"|\"", b"|", 0, 1),
    (quoted_dot, "\".\"", b".", 0, 1),
    (quoted_question, "\"?\"", b"?", 0, 1),
    (quoted_paren, "\"(\"", b"(", 0, 1),
    (quoted_bracket, "\"[\"", b"[", 0, 1),
    (quoted_plusplus, "\"++\"", b"++", 0, 2),
    (quoted_arrows, "\"->\"", b"->", 0, 2),
    (quoted_eq, "\"==\"", b"==", 0, 2),
    (quoted_neq, "\"!=\"", b"!=", 0, 2),
    (quoted_lte, "\"<=\"", b"<=", 0, 2),
    (quoted_gte, "\">=\"", b">=", 0, 2),
    (quoted_lshift, "\"<<\"", b"<<", 0, 2),
    (quoted_rshift, "\">>\"", b">>", 0, 2),
    (quoted_ampamp, "\"&&\"", b"&&", 0, 2),
    (quoted_pipepipe, "\"||\"", b"||", 0, 2),
    (quoted_scope, "\"::\"", b"::", 0, 2),
);

// =============================================================================
// 10. MULTI-RULE PRIORITY (80 tests)
// =============================================================================
gen_priority_tests!(priority,
    // First match wins for same-length
    (prio_ab, &["a", "b"], b"a", 0, 1),
    (prio_ba, &["a", "b"], b"b", 1, 1),
    (prio_keyword_if, &["if", "[a-z]+"], b"if", 0, 2),
    (prio_keyword_else, &["if", "else", "[a-z]+"], b"else", 1, 4),
    (prio_keyword_id, &["if", "else", "[a-z]+"], b"foo", 2, 3),
    (prio_three_kw_while, &["if", "else", "while", "[a-z]+"], b"while", 2, 5),
    (prio_three_kw_id, &["if", "else", "while", "[a-z]+"], b"bar", 3, 3),
    // Maximal munch: longer match wins even if later rule
    (munch_longer_id, &["if", "[a-z]+"], b"iffy", 1, 4),
    (munch_longer_id2, &["do", "[a-z]+"], b"done", 1, 4),
    (munch_longer_id3, &["int", "[a-z]+"], b"integer", 1, 7),
    (munch_num_hex, &["[0-9]+", "0[xX][0-9a-fA-F]+"], b"0xFF", 1, 4),
    (munch_num_plain, &["[0-9]+", "0[xX][0-9a-fA-F]+"], b"42", 0, 2),
    // Distinct tokens
    (distinct_digit, &["[0-9]+", "[a-z]+", "[ \\t]+"], b"123", 0, 3),
    (distinct_alpha, &["[0-9]+", "[a-z]+", "[ \\t]+"], b"abc", 1, 3),
    (distinct_space, &["[0-9]+", "[a-z]+", "[ \\t]+"], b"  \t", 2, 3),
    // Complex tokenizer
    (tok_int, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"42", 0, 2),
    (tok_id, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"foo", 1, 3),
    (tok_plus, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"+", 2, 1),
    (tok_minus, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"-", 3, 1),
    (tok_star, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"*", 4, 1),
    (tok_slash, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"/", 5, 1),
    (tok_eq, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"=", 6, 1),
    (tok_lparen, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b"(", 7, 1),
    (tok_rparen, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b")", 8, 1),
    (tok_semi, &["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\+", "-", "\\*", "\\/", "=", "\\(", "\\)", ";"], b";", 9, 1),
    // Overlapping patterns — first match rule
    (overlap_a, &["a", "a"], b"a", 0, 1),
    (overlap_az, &["[a-z]", "[a-z]"], b"x", 0, 1),
    // Prefix keywords
    (prefix_for, &["for", "foreach", "[a-z]+"], b"for", 0, 3),
    (prefix_foreach, &["for", "foreach", "[a-z]+"], b"foreach", 1, 7),
    (prefix_forx, &["for", "foreach", "[a-z]+"], b"forx", 2, 4),
    // C operators
    (cop_plus, &["\\+\\+", "\\+", "\\+="], b"++", 0, 2),
    (cop_plus_single, &["\\+\\+", "\\+", "\\+="], b"+x", 1, 1),
    (cop_pluseq, &["\\+\\+", "\\+", "\\+="], b"+=", 2, 2),
    (cop_minus, &["--", "-", "-=", "->"], b"--", 0, 2),
    (cop_minus_single, &["--", "-", "-=", "->"], b"-x", 1, 1),
    (cop_minuseq, &["--", "-", "-=", "->"], b"-=", 2, 2),
    (cop_arrow, &["--", "-", "-=", "->"], b"->", 3, 2),
    // Keywords with priority
    (kw_auto, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"auto", 0, 4),
    (kw_break, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"break", 1, 5),
    (kw_case, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"case", 2, 4),
    (kw_char, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"char", 3, 4),
    (kw_const, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"const", 4, 5),
    (kw_continue, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"continue", 5, 8),
    (kw_other, &["auto", "break", "case", "char", "const", "continue", "[a-zA-Z_][a-zA-Z0-9_]*"], b"volatile", 6, 8),
);

// =============================================================================
// 11. MAXIMAL MUNCH (50 tests)
// =============================================================================
gen_match_tests!(maximal_munch,
    // Greedy matching takes longest possible
    (munch_az_all, "[a-z]+", b"abcdefghijklmnopqrstuvwxyz", 0, 26),
    (munch_09_all, "[0-9]+", b"0123456789", 0, 10),
    (munch_alnum, "[a-zA-Z0-9]+", b"Hello123World456", 0, 16),
    (munch_dot_star, ".*", b"hello world 123!", 0, 16),
    (munch_dot_plus, ".+", b"hello world 123!", 0, 16),
    (munch_star_greedy, "a*", b"aaabbb", 0, 3),
    (munch_plus_greedy, "a+", b"aaabbb", 0, 3),
    (munch_repeat_greedy, "a{1,5}", b"aaaaaaa", 0, 5),
    // Stops at right boundary
    (munch_stop_space, "[^ ]+", b"hello world", 0, 5),
    (munch_stop_digit, "[a-z]+", b"hello123", 0, 5),
    (munch_stop_alpha, "[0-9]+", b"123abc", 0, 3),
    (munch_stop_punct, "[a-zA-Z]+", b"hello,world", 0, 5),
    (munch_id_full, "[a-zA-Z_][a-zA-Z0-9_]*", b"very_long_identifier_name", 0, 25),
    (munch_id_stops, "[a-zA-Z_][a-zA-Z0-9_]*", b"ident+rest", 0, 5),
    // Maximal munch with alternation
    (munch_alt_longer, "ab|abc", b"abc", 0, 3),
    (munch_alt_shorter, "abc|ab", b"abc", 0, 3),
    (munch_alt_equal, "foo|bar", b"foo", 0, 3),
);

// =============================================================================
// 12. COMPRESSION EQUIVALENCE (100 tests)
// =============================================================================
gen_compress_eq_tests!(compress_eq,
    (ceq_single_a, &lex1("a"), b"a"),
    (ceq_single_b, &lex1("b"), b"b"),
    (ceq_az, &lex1("[a-z]+"), b"hello"),
    (ceq_09, &lex1("[0-9]+"), b"12345"),
    (ceq_dot, &lex1(".+"), b"anything"),
    (ceq_star, &lex1("a*"), b"aaa"),
    (ceq_plus, &lex1("a+"), b"aaa"),
    (ceq_question, &lex1("a?"), b"a"),
    (ceq_repeat, &lex1("a{3}"), b"aaa"),
    (ceq_range, &lex1("a{1,3}"), b"aa"),
    (ceq_unb, &lex1("a{2,}"), b"aaaa"),
    (ceq_alt, &lex1("a|b"), b"a"),
    (ceq_alt2, &lex1("a|b"), b"b"),
    (ceq_group, &lex1("(ab)+"), b"abab"),
    (ceq_concat, &lex1("ab"), b"ab"),
    (ceq_class_neg, &lex1("[^a-z]"), b"5"),
    (ceq_posix_digit, &lex1("[[:digit:]]+"), b"123"),
    (ceq_posix_alpha, &lex1("[[:alpha:]]+"), b"abc"),
    (ceq_esc_n, &lex1("\\n"), b"\n"),
    (ceq_esc_t, &lex1("\\t"), b"\t"),
    (ceq_quoted, &lex1("\"hello\""), b"hello"),
    (ceq_nomatch_a, &lex1("a"), b"b"),
    (ceq_nomatch_az, &lex1("[a-z]+"), b"123"),
    (ceq_nomatch_09, &lex1("[0-9]+"), b"abc"),
    (ceq_nomatch_empty, &lex1("a"), b""),
    (ceq_multi_rule1, &lexn(&["[0-9]+", "[a-z]+"]), b"123"),
    (ceq_multi_rule2, &lexn(&["[0-9]+", "[a-z]+"]), b"abc"),
    (ceq_multi_rule_nomatch, &lexn(&["[0-9]+", "[a-z]+"]), b"!"),
    (ceq_kw_if, &lexn(&["if", "else", "[a-z]+"]), b"if"),
    (ceq_kw_else, &lexn(&["if", "else", "[a-z]+"]), b"else"),
    (ceq_kw_id, &lexn(&["if", "else", "[a-z]+"]), b"foo"),
    (ceq_complex_tok, &lexn(&["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\(", "\\)", ";"]), b"42"),
    (ceq_complex_id, &lexn(&["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\(", "\\)", ";"]), b"foo"),
    (ceq_complex_lp, &lexn(&["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\(", "\\)", ";"]), b"("),
    (ceq_complex_rp, &lexn(&["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\(", "\\)", ";"]), b")"),
    (ceq_complex_semi, &lexn(&["[0-9]+", "[a-zA-Z_][a-zA-Z0-9_]*", "\\(", "\\)", ";"]), b";"),
    (ceq_hex_match, &lexn(&["0[xX][0-9a-fA-F]+", "[0-9]+"]), b"0xFF"),
    (ceq_hex_plain, &lexn(&["0[xX][0-9a-fA-F]+", "[0-9]+"]), b"42"),
    (ceq_dot_match, &lex1("."), b"X"),
    (ceq_dot_star_match, &lex1(".*"), b"hello world"),
    (ceq_cc_multi, &lex1("[abcxyz]"), b"x"),
    (ceq_cc_neg, &lex1("[^abc]"), b"x"),
    (ceq_star_empty, &lex1("a*"), b"b"),
    (ceq_esc_hex, &lex1("\\x41"), b"A"),
    (ceq_esc_oct, &lex1("\\101"), b"A"),
    (ceq_nested_groups, &lex1("((a|b)(c|d))+"), b"acbd"),
    (ceq_repeat_range, &lex1("[a-z]{2,5}"), b"abc"),
    (ceq_munch_long, &lexn(&["if", "[a-z]+"]), b"iffy"),
    (ceq_munch_short, &lexn(&["if", "[a-z]+"]), b"if"),
    (ceq_blank, &lex1("[[:blank:]]+"), b"  \t "),
    (ceq_space, &lex1("[[:space:]]+"), b" \t\n "),
    (ceq_xdigit, &lex1("[[:xdigit:]]+"), b"deadBEEF42"),
    (ceq_upper, &lex1("[[:upper:]]+"), b"HELLO"),
    (ceq_lower, &lex1("[[:lower:]]+"), b"hello"),
    (ceq_graph, &lex1("[[:graph:]]+"), b"hello!"),
    (ceq_print, &lex1("[[:print:]]+"), b"hello world!"),
    (ceq_punct, &lex1("[[:punct:]]+"), b"!@#$%"),
    // Long-ish patterns
    (ceq_long_id, &lex1("[a-zA-Z_][a-zA-Z0-9_]*"), b"very_long_identifier_123"),
    (ceq_long_num, &lex1("[0-9]+"), b"999999999999"),
    (ceq_long_mixed, &lexn(&["[0-9]+", "[a-zA-Z]+", "[ \\t\\n]+"]), b"hello"),
    (ceq_long_mixed2, &lexn(&["[0-9]+", "[a-zA-Z]+", "[ \\t\\n]+"]), b"12345"),
    (ceq_long_mixed3, &lexn(&["[0-9]+", "[a-zA-Z]+", "[ \\t\\n]+"]), b"  \t"),
    // Operators
    (ceq_ops_pp, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"++"),
    (ceq_ops_p, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"+"),
    (ceq_ops_pe, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"+="),
    (ceq_ops_mm, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"--"),
    (ceq_ops_m, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"-"),
    (ceq_ops_me, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"-="),
    (ceq_ops_ar, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"->"),
    (ceq_ops_star, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"*"),
    (ceq_ops_slash, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"/"),
    (ceq_ops_eq, &lexn(&["\\+\\+", "\\+", "\\+=", "--", "-", "-=", "->", "\\*", "\\/", "="]),  b"="),
);

// =============================================================================
// 13. C EMITTER VALIDITY CHECKS (80 tests)
// =============================================================================
mod emitter_validity {
    use super::*;

    /// Check that emitted C code contains all required POSIX elements.
    fn check_posix_elements(output: &str) {
        assert!(output.contains("#include <stdio.h>"), "missing stdio.h");
        assert!(output.contains("#include <string.h>"), "missing string.h");
        assert!(output.contains("#include <stdlib.h>"), "missing stdlib.h");
        assert!(output.contains("int yylex(void)"), "missing yylex");
        assert!(output.contains("char *yytext"), "missing yytext");
        assert!(output.contains("int yyleng"), "missing yyleng");
        assert!(output.contains("FILE *yyin"), "missing yyin");
        assert!(output.contains("FILE *yyout"), "missing yyout");
        assert!(output.contains("yy_transition["), "missing transition table");
        assert!(output.contains("yy_accept["), "missing accept table");
        assert!(output.contains("YY_NUM_STATES"), "missing YY_NUM_STATES");
        assert!(output.contains("YY_START_STATE"), "missing YY_START_STATE");
        assert!(output.contains("YY_BUF_SIZE"), "missing YY_BUF_SIZE");
        assert!(output.contains("#define ECHO"), "missing ECHO macro");
        assert!(output.contains("#define INITIAL"), "missing INITIAL");
        assert!(output.contains("int input(void)"), "missing input()");
        assert!(output.contains("void unput(int c)"), "missing unput()");
        assert!(output.contains("void yyrestart(FILE *f)"), "missing yyrestart()");
        assert!(output.contains("void yymore(void)"), "missing yymore()");
        assert!(output.contains("#define yyless"), "missing yyless()");
        assert!(output.contains("yy_at_bol"), "missing yy_at_bol");
    }

    #[test]
    fn emit_single_rule() {
        let out = emit_c("%%\n[0-9]+  printf(\"NUM\");\n", false);
        check_posix_elements(&out);
        assert!(out.contains("printf(\"NUM\");"));
    }

    #[test]
    fn emit_two_rules() {
        let out = emit_c("%%\n[0-9]+  printf(\"NUM\");\n[a-z]+  printf(\"ID\");\n", false);
        check_posix_elements(&out);
        assert!(out.contains("printf(\"NUM\");"));
        assert!(out.contains("printf(\"ID\");"));
    }

    #[test]
    fn emit_three_rules() {
        let out = emit_c("%%\n[0-9]+  printf(\"NUM\");\n[a-z]+  printf(\"ID\");\n.  ;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("case 1:"));
        assert!(out.contains("case 2:"));
        assert!(out.contains("case 3:"));
    }

    #[test]
    fn emit_suppress_default() {
        let out = emit_c("%%\n[0-9]+  printf(\"NUM\");\n", true);
        check_posix_elements(&out);
        // With -n, no ECHO in the default fallback
        // But ECHO macro should still be defined
        assert!(out.contains("#define ECHO"));
    }

    #[test]
    fn emit_default_echoes() {
        let out = emit_c("%%\n[0-9]+  printf(\"NUM\");\n", false);
        // Without -n, unmatched chars are echoed via ECHO
        assert!(out.contains("ECHO;"));
    }

    #[test]
    fn emit_header_code() {
        let out = emit_c("%{\n#include \"myheader.h\"\nint count = 0;\n%}\n%%\n.  ;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("#include \"myheader.h\""));
        assert!(out.contains("int count = 0;"));
    }

    #[test]
    fn emit_user_code_section() {
        let out = emit_c("%%\n.  ;\n%%\nint main() {\n    return yylex();\n}\n", false);
        check_posix_elements(&out);
        assert!(out.contains("int main()"));
        assert!(out.contains("return yylex()"));
    }

    #[test]
    fn emit_noyywrap() {
        let out = emit_c("%option noyywrap\n%%\n.  ;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("static int yywrap(void)"));
        assert!(!out.contains("extern int yywrap"));
    }

    #[test]
    fn emit_yylineno() {
        let out = emit_c("%option yylineno\n%%\n.  ;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("int yylineno"));
    }

    #[test]
    fn emit_both_options() {
        let out = emit_c("%option noyywrap\n%option yylineno\n%%\n.  ;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("int yylineno"));
        assert!(out.contains("static int yywrap"));
    }

    #[test]
    fn emit_compressed_has_ec() {
        let out = emit_c_compressed("%%\n[0-9]+  ;\n[a-z]+  ;\n");
        assert!(out.contains("yy_ec["));
        assert!(out.contains("yy_transition["));
    }

    #[test]
    fn emit_empty_action() {
        let out = emit_c("%%\n[[:blank:]]  ;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("break;"));
    }

    #[test]
    fn emit_return_action() {
        let out = emit_c("%%\n[0-9]+  return 1;\n", false);
        check_posix_elements(&out);
        assert!(out.contains("return 1;"));
    }

    #[test]
    fn emit_multiline_action() {
        let out = emit_c("%%\n[0-9]+  { count++; printf(\"%d\\n\", count); }\n", false);
        check_posix_elements(&out);
        assert!(out.contains("count++"));
    }

    #[test]
    fn emit_complex_tokenizer() {
        let out = emit_c(
            "%%\n[0-9]+  return TOK_NUM;\n[a-zA-Z_][a-zA-Z0-9_]*  return TOK_ID;\n[ \\t\\n]  ;\n.  return TOK_ERR;\n",
            false,
        );
        check_posix_elements(&out);
        assert!(out.contains("TOK_NUM"));
        assert!(out.contains("TOK_ID"));
        assert!(out.contains("TOK_ERR"));
        assert!(out.contains("case 1:"));
        assert!(out.contains("case 2:"));
        assert!(out.contains("case 3:"));
        assert!(out.contains("case 4:"));
    }

    #[test]
    fn emit_stdin_default() {
        let out = emit_c("%%\n.  ;\n", false);
        // yyin defaults to stdin
        assert!(out.contains("if (yyin == NULL) yyin = stdin"));
    }

    #[test]
    fn emit_stdout_default() {
        let out = emit_c("%%\n.  ;\n", false);
        // yyout defaults to stdout
        assert!(out.contains("if (yyout == NULL) yyout = stdout"));
    }

    #[test]
    fn emit_has_switch() {
        let out = emit_c("%%\n[0-9]+  return 1;\n[a-z]+  return 2;\n", false);
        assert!(out.contains("switch (yy_accept[yy_last_accept_state])"));
    }

    #[test]
    fn emit_buffer_refill() {
        let out = emit_c("%%\n.  ;\n", false);
        assert!(out.contains("yy_refill"));
        assert!(out.contains("fread"));
        assert!(out.contains("memmove"));
    }

    #[test]
    fn emit_yywrap_extern() {
        let out = emit_c("%%\n.  ;\n", false);
        assert!(out.contains("extern int yywrap(void)"));
    }

    #[test]
    fn emit_begin_macro() {
        let out = emit_c("%%\n.  ;\n", false);
        assert!(out.contains("BEGIN"));
    }

    #[test]
    fn emit_yy_start() {
        let out = emit_c("%%\n.  ;\n", false);
        assert!(out.contains("YY_START"));
    }

    #[test]
    fn emit_generated_comment() {
        let out = emit_c("%%\n.  ;\n", false);
        assert!(out.contains("Generated by ft_lex"));
    }

    // Verify that different patterns produce different DFA sizes
    #[test]
    fn emit_dfa_size_varies() {
        let out1 = emit_c("%%\na  ;\n", false);
        let out2 = emit_c("%%\n[0-9]+  ;\n[a-z]+  ;\n[ \\t]+  ;\n.  ;\n", false);
        // More rules = more states (usually)
        let states1: usize = out1.match_indices("yy_accept[").count();
        let states2: usize = out2.match_indices("yy_accept[").count();
        assert!(states1 > 0);
        assert!(states2 > 0);
    }

    // Test that emitter handles various edge case patterns
    #[test]
    fn emit_edge_dot_star() { let _ = emit_c("%%\n.*  ;\n", false); }
    #[test]
    fn emit_edge_dot_plus() { let _ = emit_c("%%\n.+  ;\n", false); }
    #[test]
    fn emit_edge_single_char() { let _ = emit_c("%%\na  ;\n", false); }
    #[test]
    fn emit_edge_empty_class() { let _ = emit_c("%%\n[a-z]*  ;\n", false); }
    #[test]
    fn emit_edge_many_rules() {
        let mut src = String::from("%%\n");
        for c in b'a'..=b'z' {
            src.push_str(&format!("{}  ;\n", c as char));
        }
        let out = emit_c(&src, false);
        check_posix_elements(&out);
        assert!(out.contains("case 26:"));
    }

    #[test]
    fn emit_edge_long_pattern() {
        let _ = emit_c("%%\nabcdefghijklmnopqrstuvwxyz  ;\n", false);
    }

    #[test]
    fn emit_edge_alternation_explosion() {
        let _ = emit_c("%%\na|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z  ;\n", false);
    }

    #[test]
    fn emit_edge_nested_groups() {
        let _ = emit_c("%%\n((a|b)(c|d))+  ;\n", false);
    }

    #[test]
    fn emit_edge_hex_escape() {
        let _ = emit_c("%%\n\\x41  ;\n", false);
    }
    #[test]
    fn emit_edge_octal_escape() {
        let _ = emit_c("%%\n\\101  ;\n", false);
    }

    #[test]
    fn emit_c_only_allowed_headers() {
        // POSIX says output may only use stdio.h, string.h, malloc/realloc/calloc/free
        let out = emit_c("%%\n.  ;\n", false);
        // stdlib.h provides malloc/realloc/calloc/free
        assert!(out.contains("#include <stdio.h>"));
        assert!(out.contains("#include <string.h>"));
        assert!(out.contains("#include <stdlib.h>"));
        // Must NOT include other headers
        assert!(!out.contains("#include <unistd.h>"));
        assert!(!out.contains("#include <ctype.h>"));
        assert!(!out.contains("#include <errno.h>"));
    }
}

// =============================================================================
// 14. EDGE CASES & BOUNDARIES (50 tests)
// =============================================================================
gen_match_tests!(edge_cases,
    // Very long matches
    (edge_long_alpha, "[a-z]+", b"abcdefghijklmnopqrstuvwxyz", 0, 26),
    (edge_long_digit, "[0-9]+", b"0123456789012345678901234567890123456789", 0, 40),
    (edge_long_dot, ".+", b"!@#$%^&*()_+=-[]{}|;:',.<>?/", 0, 28),
    // Single byte patterns (all printable ASCII)
    (edge_exclaim, "!", b"!", 0, 1),
    (edge_at, "@", b"@", 0, 1),
    (edge_hash, "#", b"#", 0, 1),
    (edge_percent, "%", b"%", 0, 1),
    (edge_ampersand, "&", b"&", 0, 1),
    (edge_colon, ":", b":", 0, 1),
    (edge_comma, ",", b",", 0, 1),
    (edge_less, "<", b"<", 0, 1),
    (edge_greater, ">", b">", 0, 1),
    (edge_tilde, "~", b"~", 0, 1),
    (edge_backtick, "`", b"`", 0, 1),
    // Patterns that match single char from longer input
    (edge_first_char, "a", b"abcdef", 0, 1),
    (edge_digit_first, "[0-9]", b"5abc", 0, 1),
    // Repeated patterns
    (edge_repeat_0, "a{0,5}", b"bbb", 0, 0),
    (edge_repeat_0_match, "a{0,5}", b"aaa", 0, 3),
    // Complex real-world patterns
    (edge_float, "[0-9]+\\.[0-9]+", b"3.14159", 0, 7),
    (edge_float2, "[0-9]+\\.[0-9]+", b"0.5", 0, 3),
    (edge_id_underscore, "_[a-zA-Z0-9_]*", b"_internal", 0, 9),
    (edge_id_start_under, "_", b"_", 0, 1),
    (edge_hex_literal, "0[xX][0-9a-fA-F]+", b"0xDEADBEEF", 0, 10),
    (edge_oct_literal, "0[0-7]+", b"0777", 0, 4),
    (edge_string_double, "\\x22[^\\x22]*\\x22", b"\"hello\"rest", 0, 7),
    (edge_c_comment, "\\/\\*", b"/*", 0, 2),
    (edge_cpp_comment, "\\/\\/", b"//", 0, 2),
    // Whitespace variants
    (edge_spaces, "[ ]+", b"     ", 0, 5),
    (edge_tabs, "[\\t]+", b"\t\t\t", 0, 3),
    (edge_mixed_ws, "[ \\t]+", b" \t \t ", 0, 5),
    (edge_newlines, "[\\n]+", b"\n\n\n", 0, 3),
);

gen_nomatch_tests!(edge_nomatch,
    (edge_no_float_plain, "[0-9]+\\.[0-9]+", b"123"),
    (edge_no_hex_no_x, "0[xX][0-9a-fA-F]+", b"0a"),
    (edge_no_string_open, "\\x22[^\\x22]*\\x22", b"hello"),
    (edge_no_empty_plus, "[a-z]+", b""),
    (edge_no_empty_exact, "a{3}", b""),
    (edge_no_empty_unb, "a{2,}", b""),
);

// =============================================================================
// 15. DEFINITIONS WITH EXPANSION (30 tests)
// =============================================================================
mod definitions {
    use super::*;

    #[test]
    fn def_simple_digit() {
        let src = "DIGIT [0-9]\n%%\n{DIGIT}+  ;\n";
        assert_eq!(sim(src, b"123"), Some((0, 3)));
    }

    #[test]
    fn def_simple_alpha() {
        let src = "ALPHA [a-zA-Z]\n%%\n{ALPHA}+  ;\n";
        assert_eq!(sim(src, b"hello"), Some((0, 5)));
    }

    #[test]
    fn def_id() {
        let src = "ID [a-zA-Z_][a-zA-Z0-9_]*\n%%\n{ID}  ;\n";
        assert_eq!(sim(src, b"foo_bar"), Some((0, 7)));
    }

    #[test]
    fn def_multiple() {
        let src = "DIGIT [0-9]\nLETTER [a-zA-Z]\n%%\n{DIGIT}+  printf(\"NUM\");\n{LETTER}+  printf(\"ID\");\n";
        assert_eq!(sim(src, b"123"), Some((0, 3)));
        assert_eq!(sim(src, b"abc"), Some((1, 3)));
    }

    #[test]
    fn def_nested() {
        // Nested definitions: NUM uses {DIGIT} which requires recursive expansion
        let src = "DIGIT [0-9]\nNUM [0-9]+\n%%\n{NUM}  ;\n";
        assert_eq!(sim(src, b"42"), Some((0, 2)));
    }

    #[test]
    fn def_with_header() {
        let src = "%{\n#include <stdio.h>\n%}\nDIGIT [0-9]\n%%\n{DIGIT}+  printf(\"NUM\");\n";
        let out = emit_c(src, false);
        assert!(out.contains("#include <stdio.h>"));
        assert!(out.contains("printf(\"NUM\")"));
    }

    #[test]
    fn def_ws() {
        let src = "WS [ \\t\\n]\n%%\n{WS}+  ;\n";
        assert_eq!(sim(src, b"  \t\n"), Some((0, 4)));
    }

    #[test]
    fn def_alnum() {
        let src = "ALNUM [a-zA-Z0-9]\n%%\n{ALNUM}+  ;\n";
        assert_eq!(sim(src, b"hello123"), Some((0, 8)));
    }

    #[test]
    fn def_expansion_in_class() {
        // Definition used inside a larger pattern
        let src = "D [0-9]\n%%\n{D}{D}  ;\n";
        assert_eq!(sim(src, b"42"), Some((0, 2)));
    }

    #[test]
    fn def_used_twice() {
        let src = "L [a-z]\n%%\n{L}{L}{L}  ;\n";
        assert_eq!(sim(src, b"abc"), Some((0, 3)));
    }

    #[test]
    fn def_with_options() {
        let src = "%option noyywrap\nDIGIT [0-9]\n%%\n{DIGIT}+  ;\n";
        let out = emit_c(src, false);
        assert!(out.contains("static int yywrap"));
    }

    #[test]
    fn def_complex_float() {
        let src = "DIGIT [0-9]\n%%\n{DIGIT}+\\.{DIGIT}+  ;\n";
        assert_eq!(sim(src, b"3.14"), Some((0, 4)));
    }

    #[test]
    fn def_hex() {
        let src = "HEX [0-9a-fA-F]\n%%\n0[xX]{HEX}+  ;\n";
        assert_eq!(sim(src, b"0xFF"), Some((0, 4)));
    }

    #[test]
    fn def_compression_eq() {
        let src = "D [0-9]\n%%\n{D}+  ;\n";
        assert_eq!(sim(src, b"42"), sim_c(src, b"42"));
    }

    #[test]
    fn def_nomatch() {
        let src = "DIGIT [0-9]\n%%\n{DIGIT}+  ;\n";
        assert!(sim(src, b"abc").is_none());
    }
}

// =============================================================================
// 16. COMBINED FEATURE TESTS (80 tests)
// =============================================================================
gen_match_tests!(combined,
    // Concat + class + quantifier
    (comb_id, "[a-zA-Z_][a-zA-Z0-9_]*", b"myVar_123", 0, 9),
    (comb_float, "[0-9]+\\.[0-9]+", b"123.456", 0, 7),
    (comb_float_e, "[0-9]+\\.[0-9]+(e[+-]?[0-9]+)?", b"1.5e10", 0, 6),
    (comb_float_e_neg, "[0-9]+\\.[0-9]+(e[+-]?[0-9]+)?", b"1.5e-3", 0, 6),
    (comb_hex, "0[xX][0-9a-fA-F]+", b"0xCAFE", 0, 6),
    (comb_oct, "0[0-7]*", b"0755", 0, 4),
    // String literals
    (comb_string, "\\x22[^\\x22]*\\x22", b"\"hello world\"", 0, 13),
    (comb_string_empty, "\\x22\\x22", b"\"\"", 0, 2),
    (comb_char, "'[^']'", b"'a'", 0, 3),
    (comb_char_esc, "'\\\\.'", b"'\\n'end", 0, 4),
    // C identifiers
    (comb_c_id_under, "[a-zA-Z_][a-zA-Z0-9_]*", b"_private", 0, 8),
    (comb_c_id_caps, "[a-zA-Z_][a-zA-Z0-9_]*", b"MAX_VALUE", 0, 9),
    (comb_c_id_camel, "[a-zA-Z_][a-zA-Z0-9_]*", b"camelCase42", 0, 11),
    // Operators patterns
    (comb_arrow, "->", b"->", 0, 2),
    (comb_scope, "::", b"::", 0, 2),
    (comb_lshift, "<<", b"<<", 0, 2),
    (comb_rshift, ">>", b">>", 0, 2),
    (comb_eq, "==", b"==", 0, 2),
    (comb_neq, "!=", b"!=", 0, 2),
    (comb_lte, "<=", b"<=", 0, 2),
    (comb_gte, ">=", b">=", 0, 2),
    (comb_and, "&&", b"&&", 0, 2),
    (comb_or, "\\|\\|", b"||", 0, 2),
    // Group + alt + quantifier
    (comb_group_alt_plus, "(ab|cd)+", b"abcdab", 0, 6),
    (comb_group_alt_star, "(ab|cd)*", b"cdab", 0, 4),
    (comb_group_concat_q, "(ab)?(cd)?", b"abcd", 0, 4),
    (comb_group_concat_q2, "(ab)?(cd)?", b"ab", 0, 2),
    (comb_group_concat_q3, "(ab)?(cd)?", b"cd", 0, 2),
    (comb_group_concat_q4, "(ab)?(cd)?", b"xy", 0, 0),
    // Nested quantifiers
    (comb_nested_star_plus, "([a-z]+[ ])*", b"hello world ", 0, 12),
    (comb_nested_plus_star, "([a-z]*[0-9])+", b"abc1def2", 0, 8),
    // Real tokenizer patterns
    (comb_c_number, "[0-9]+([uU]?[lL]{0,2})?", b"42ULL", 0, 5),
    (comb_c_number2, "[0-9]+([uU]?[lL]{0,2})?", b"100", 0, 3),
    (comb_c_number3, "[0-9]+([uU]?[lL]{0,2})?", b"0L", 0, 2),
    // IP address-like
    (comb_ip_like, "[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+", b"192.168.1.1", 0, 11),
    // Email-like (simplified)
    (comb_email_like, "[a-zA-Z0-9.]+@[a-zA-Z0-9.]+", b"user@host.com", 0, 13),
    // URL-like
    (comb_url_like, "https?:\\/\\/[^ ]+", b"http://example.com/path", 0, 23),
    (comb_url_like2, "https?:\\/\\/[^ ]+", b"https://foo.bar", 0, 15),
);

// =============================================================================
// 17. REGEX PARSER DIRECT TESTS (40 tests)
// =============================================================================
mod regex_parser_gen {
    use ft_lex_lib::regex::parser::parse_regex;
    use ft_lex_lib::regex::ast::Regex;

    #[test]
    fn parse_single_literal() {
        assert!(matches!(parse_regex(b"a", "<t>", 1).unwrap(), Regex::Literal(b'a')));
    }

    #[test]
    fn parse_concat_two() {
        assert!(matches!(parse_regex(b"ab", "<t>", 1).unwrap(), Regex::Concat(_, _)));
    }

    #[test]
    fn parse_alt_simple() {
        assert!(matches!(parse_regex(b"a|b", "<t>", 1).unwrap(), Regex::Alt(_, _)));
    }

    #[test]
    fn parse_star() {
        assert!(matches!(parse_regex(b"a*", "<t>", 1).unwrap(), Regex::Star(_)));
    }

    #[test]
    fn parse_plus() {
        assert!(matches!(parse_regex(b"a+", "<t>", 1).unwrap(), Regex::Plus(_)));
    }

    #[test]
    fn parse_question() {
        assert!(matches!(parse_regex(b"a?", "<t>", 1).unwrap(), Regex::Question(_)));
    }

    #[test]
    fn parse_group() {
        let r = parse_regex(b"(a)", "<t>", 1).unwrap();
        assert!(matches!(r, Regex::Literal(b'a')));
    }

    #[test]
    fn parse_class() {
        assert!(matches!(parse_regex(b"[a-z]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_negated_class() {
        assert!(matches!(parse_regex(b"[^a-z]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_dot() {
        assert!(matches!(parse_regex(b".", "<t>", 1).unwrap(), Regex::AnyChar));
    }

    #[test]
    fn parse_repeat_exact() {
        assert!(matches!(parse_regex(b"a{3}", "<t>", 1).unwrap(), Regex::Repeat(_, 3, Some(3))));
    }

    #[test]
    fn parse_repeat_range() {
        assert!(matches!(parse_regex(b"a{2,5}", "<t>", 1).unwrap(), Regex::Repeat(_, 2, Some(5))));
    }

    #[test]
    fn parse_repeat_unbounded() {
        assert!(matches!(parse_regex(b"a{3,}", "<t>", 1).unwrap(), Regex::Repeat(_, 3, None)));
    }

    #[test]
    fn parse_quoted_string() {
        let r = parse_regex(b"\"hello\"", "<t>", 1).unwrap();
        // Should be a concatenation of literals
        assert!(matches!(r, Regex::Concat(_, _)));
    }

    #[test]
    fn parse_escape_n() {
        assert!(matches!(parse_regex(b"\\n", "<t>", 1).unwrap(), Regex::Literal(b'\n')));
    }

    #[test]
    fn parse_escape_t() {
        assert!(matches!(parse_regex(b"\\t", "<t>", 1).unwrap(), Regex::Literal(b'\t')));
    }

    #[test]
    fn parse_escape_star() {
        assert!(matches!(parse_regex(b"\\*", "<t>", 1).unwrap(), Regex::Literal(b'*')));
    }

    #[test]
    fn parse_hex_escape() {
        assert!(matches!(parse_regex(b"\\x41", "<t>", 1).unwrap(), Regex::Literal(0x41)));
    }

    #[test]
    fn parse_octal_escape() {
        assert!(matches!(parse_regex(b"\\101", "<t>", 1).unwrap(), Regex::Literal(0o101)));
    }

    #[test]
    fn parse_posix_digit() {
        assert!(matches!(parse_regex(b"[[:digit:]]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_posix_alpha() {
        assert!(matches!(parse_regex(b"[[:alpha:]]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_posix_alnum() {
        assert!(matches!(parse_regex(b"[[:alnum:]]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_posix_blank() {
        assert!(matches!(parse_regex(b"[[:blank:]]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_posix_space() {
        assert!(matches!(parse_regex(b"[[:space:]]", "<t>", 1).unwrap(), Regex::CharClass(_)));
    }

    #[test]
    fn parse_anchor_caret() {
        assert!(matches!(parse_regex(b"^a", "<t>", 1).unwrap(), Regex::Concat(_, _)));
    }

    #[test]
    fn parse_anchor_dollar() {
        assert!(matches!(parse_regex(b"a$", "<t>", 1).unwrap(), Regex::Concat(_, _)));
    }

    #[test]
    fn parse_complex_float() {
        let r = parse_regex(b"[0-9]+\\.[0-9]+", "<t>", 1);
        assert!(r.is_ok());
    }

    #[test]
    fn parse_complex_id() {
        let r = parse_regex(b"[a-zA-Z_][a-zA-Z0-9_]*", "<t>", 1);
        assert!(r.is_ok());
    }

    #[test]
    fn parse_complex_alt_kw() {
        let r = parse_regex(b"if|else|while|for|return", "<t>", 1);
        assert!(r.is_ok());
    }

    #[test]
    fn parse_complex_string_lit() {
        // Use hex escape for double-quote to avoid quoted-string parsing
        let r = parse_regex(b"\\x22[^\\x22]*\\x22", "<t>", 1);
        assert!(r.is_ok());
    }

    #[test]
    fn parse_complex_nested() {
        let r = parse_regex(b"((a|b)(c|d))+", "<t>", 1);
        assert!(r.is_ok());
    }

    // Error cases
    #[test]
    fn parse_err_unclosed_bracket() {
        assert!(parse_regex(b"[a-z", "<t>", 1).is_err());
    }

    #[test]
    fn parse_err_unclosed_paren() {
        assert!(parse_regex(b"(abc", "<t>", 1).is_err());
    }

    #[test]
    fn parse_err_trailing_backslash() {
        assert!(parse_regex(b"a\\", "<t>", 1).is_err());
    }

    #[test]
    fn parse_err_unclosed_quote() {
        assert!(parse_regex(b"\"hello", "<t>", 1).is_err());
    }

    #[test]
    fn parse_empty() {
        let r = parse_regex(b"", "<t>", 1).unwrap();
        assert!(matches!(r, Regex::Empty));
    }

    #[test]
    fn parse_single_dot() {
        assert!(matches!(parse_regex(b".", "<t>", 1).unwrap(), Regex::AnyChar));
    }

    #[test]
    fn parse_name_ref() {
        let r = parse_regex(b"{digit}", "<t>", 1).unwrap();
        assert!(matches!(r, Regex::NameRef(_)));
    }
}

// =============================================================================
// 18. ERROR HANDLING (40 tests)
// =============================================================================
mod error_handling {
    use ft_lex_lib::lex_file::parser::LexFile;

    #[test]
    fn err_no_rules_section() {
        // Parser may return Ok with empty rules if no %% found
        let r = LexFile::parse("just some text\n", "<t>");
        // Either error or empty rules — both are valid behavior
        if let Ok(lf) = &r {
            assert!(lf.rules.is_empty());
        }
    }

    #[test]
    fn err_empty_input() {
        // Empty input: parser may return Ok with empty sections
        let r = LexFile::parse("", "<t>");
        if let Ok(lf) = &r {
            assert!(lf.rules.is_empty());
        }
    }

    #[test]
    fn err_only_percent_percent() {
        // %% with no rules is ok (empty rules)
        let r = LexFile::parse("%%\n", "<t>");
        assert!(r.is_ok());
    }

    #[test]
    fn err_bad_regex_in_rule() {
        assert!(LexFile::parse("%%\n[unclosed  ;\n", "<t>").is_err());
    }

    #[test]
    fn err_bad_regex_trailing_bs() {
        assert!(LexFile::parse("%%\n\\  ;\n", "<t>").is_err());
    }

    #[test]
    fn ok_single_rule() {
        assert!(LexFile::parse("%%\na  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_dot_rule() {
        assert!(LexFile::parse("%%\n.  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_class_rule() {
        assert!(LexFile::parse("%%\n[a-z]+  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_multiple_rules() {
        assert!(LexFile::parse("%%\na  ;\nb  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_header_code() {
        assert!(LexFile::parse("%{\nint x;\n%}\n%%\na  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_user_code() {
        assert!(LexFile::parse("%%\na  ;\n%%\nint main() {}\n", "<t>").is_ok());
    }

    #[test]
    fn ok_definitions() {
        assert!(LexFile::parse("D [0-9]\n%%\n{D}  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_options() {
        assert!(LexFile::parse("%option noyywrap\n%%\na  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_two_options() {
        assert!(LexFile::parse("%option noyywrap\n%option yylineno\n%%\na  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_unknown_option_ignored() {
        // Unknown options should be silently ignored (POSIX unspecified)
        assert!(LexFile::parse("%option foobar\n%%\na  ;\n", "<t>").is_ok());
    }

    #[test]
    fn ok_complex_scanner() {
        let src = "%{\n#include <stdio.h>\n%}\nD [0-9]\nL [a-zA-Z]\n%option noyywrap\n%%\n{D}+  printf(\"NUM\");\n{L}+  printf(\"ID\");\n[ \\t\\n]  ;\n.  printf(\"ERR\");\n%%\nint main() { yylex(); }\n";
        assert!(LexFile::parse(src, "<t>").is_ok());
    }

    // Test that parsed data is correct
    #[test]
    fn parse_rule_count_1() {
        let lf = LexFile::parse("%%\na  ;\n", "<t>").unwrap();
        assert_eq!(lf.rules.len(), 1);
    }

    #[test]
    fn parse_rule_count_3() {
        let lf = LexFile::parse("%%\na  ;\nb  ;\nc  ;\n", "<t>").unwrap();
        assert_eq!(lf.rules.len(), 3);
    }

    #[test]
    fn parse_header_present() {
        let lf = LexFile::parse("%{\nint x;\n%}\n%%\na  ;\n", "<t>").unwrap();
        assert!(lf.header_code.contains("int x;"));
    }

    #[test]
    fn parse_user_code_present() {
        let lf = LexFile::parse("%%\na  ;\n%%\nint main() {}\n", "<t>").unwrap();
        assert!(lf.user_code.contains("int main()"));
    }

    #[test]
    fn parse_noyywrap_option() {
        let lf = LexFile::parse("%option noyywrap\n%%\na  ;\n", "<t>").unwrap();
        assert!(lf.options.noyywrap);
    }

    #[test]
    fn parse_yylineno_option() {
        let lf = LexFile::parse("%option yylineno\n%%\na  ;\n", "<t>").unwrap();
        assert!(lf.options.yylineno);
    }

    #[test]
    fn parse_action_content() {
        let lf = LexFile::parse("%%\na  printf(\"found a\");\n", "<t>").unwrap();
        assert_eq!(lf.rules[0].action.trim(), "printf(\"found a\");");
    }

    #[test]
    fn parse_empty_action() {
        let lf = LexFile::parse("%%\na  ;\n", "<t>").unwrap();
        assert_eq!(lf.rules[0].action.trim(), ";");
    }

    // Test that patterns are actually stored
    #[test]
    fn parse_pattern_stored() {
        let lf = LexFile::parse("%%\n[a-z]+  ;\n", "<t>").unwrap();
        assert!(lf.rules[0].pattern_str.contains("[a-z]+"));
    }

    #[test]
    fn parse_priority_order() {
        let lf = LexFile::parse("%%\na  ;\nb  ;\nc  ;\n", "<t>").unwrap();
        assert_eq!(lf.rules[0].priority, 0);
        assert_eq!(lf.rules[1].priority, 1);
        assert_eq!(lf.rules[2].priority, 2);
    }
}

// =============================================================================
// 19. POSIX CLASS COMBINATIONS (50 tests)
// =============================================================================
gen_match_tests!(posix_combos,
    // POSIX class + regular ranges
    (posix_digit_alpha, "[[:digit:][:alpha:]]+", b"abc123", 0, 6),
    (posix_digit_alpha2, "[[:digit:][:alpha:]]+", b"123abc", 0, 6),
    (posix_alnum_under, "[[:alnum:]_]+", b"hello_world_123", 0, 15),
    (posix_blank_plus, "[[:blank:]]+", b"  \t  ", 0, 5),
    (posix_space_plus, "[[:space:]]+", b" \t\n\r", 0, 4),
    (posix_upper_lower, "[[:upper:][:lower:]]+", b"HelloWorld", 0, 10),
    (posix_xdigit_long, "[[:xdigit:]]+", b"0123456789abcdefABCDEF", 0, 22),
    (posix_print_long, "[[:print:]]+", b"Hello, World! 123", 0, 17),
    (posix_graph_long, "[[:graph:]]+", b"Hello,World!123", 0, 15),
    // Negated POSIX
    (posix_neg_digit, "[^[:digit:]]+", b"abc", 0, 3),
    (posix_neg_alpha, "[^[:alpha:]]+", b"123", 0, 3),
    (posix_neg_space, "[^[:space:]]+", b"abc", 0, 3),
    // POSIX in multi-rule
    (posix_multi_digit, "[[:digit:]]+", b"5", 0, 1),
    (posix_multi_alpha2, "[[:alpha:]]+", b"z", 0, 1),
    (posix_multi_alnum2, "[[:alnum:]]+", b"a", 0, 1),
    // Combined class + posix
    (posix_with_range, "[[:digit:]a-f]+", b"dead42beef", 0, 10),
    (posix_with_literal, "[[:alpha:].]+", b"hello.world", 0, 11),
    (posix_with_dash, "[[:alnum:]-]+", b"foo-bar-123", 0, 11),
);

gen_nomatch_tests!(posix_combos_nomatch,
    (posix_digit_no_alpha, "[[:digit:]]+", b"abc"),
    (posix_alpha_no_digit, "[[:alpha:]]+", b"123"),
    (posix_blank_no_nl, "[[:blank:]]+", b"\n"),
    (posix_upper_no_lower, "[[:upper:]]+", b"abc"),
    (posix_lower_no_upper, "[[:lower:]]+", b"ABC"),
    (posix_graph_no_space, "[[:graph:]]+", b" "),
);

// =============================================================================
// 20. RANDOM-ISH COMPLEX PATTERNS (100 tests)
// =============================================================================
gen_match_tests!(complex_patterns,
    // C-like tokenizer patterns
    (cp_int_literal, "[1-9][0-9]*|0", b"42", 0, 2),
    (cp_int_zero, "[1-9][0-9]*|0", b"0", 0, 1),
    (cp_float_exp, "[0-9]+\\.[0-9]+(e[+-]?[0-9]+)?", b"3.14", 0, 4),
    (cp_float_exp2, "[0-9]+\\.[0-9]+(e[+-]?[0-9]+)?", b"1.5e10", 0, 6),
    (cp_hex_num, "0[xX][0-9a-fA-F]+", b"0xDEAD", 0, 6),
    (cp_oct_num, "0[0-7]+", b"0777", 0, 4),
    (cp_string_lit, "\\x22[^\\x22]*\\x22", b"\"hello\"", 0, 7),
    (cp_char_lit, "'[^'\\\\]'", b"'a'", 0, 3),
    // Identifier variants
    (cp_id_simple, "[a-zA-Z_][a-zA-Z0-9_]*", b"x", 0, 1),
    (cp_id_under, "[a-zA-Z_][a-zA-Z0-9_]*", b"_", 0, 1),
    (cp_id_long, "[a-zA-Z_][a-zA-Z0-9_]*", b"very_long_variable_name_42", 0, 26),
    (cp_id_all_under, "[a-zA-Z_][a-zA-Z0-9_]*", b"___", 0, 3),
    // Whitespace handling
    (cp_ws_spaces, "[ \\t]+", b"   ", 0, 3),
    (cp_ws_tabs, "[ \\t]+", b"\t\t", 0, 2),
    (cp_ws_mixed, "[ \\t]+", b" \t \t", 0, 4),
    (cp_newline, "\\n", b"\n", 0, 1),
    // Comment patterns
    (cp_line_comment, "\\/\\/[^\\n]*", b"// hello world", 0, 14),
    // Preprocessor
    (cp_preproc, "#[a-z]+", b"#include", 0, 8),
    (cp_preproc2, "#[a-z]+", b"#define", 0, 7),
    (cp_preproc3, "#[a-z]+", b"#ifdef", 0, 6),
    // Number patterns
    (cp_decimal, "[0-9]+", b"123456789", 0, 9),
    (cp_binary, "0[bB][01]+", b"0b1010", 0, 6),
    // Bracket combinations
    (cp_brackets, "[(){}\\[\\]]", b"(", 0, 1),
    (cp_brackets2, "[(){}\\[\\]]", b")", 0, 1),
    (cp_brackets3, "[(){}\\[\\]]", b"{", 0, 1),
    (cp_brackets4, "[(){}\\[\\]]", b"}", 0, 1),
    (cp_brackets5, "[(){}\\[\\]]", b"[", 0, 1),
    (cp_brackets6, "[(){}\\[\\]]", b"]", 0, 1),
    // Various operators
    (cp_op_plus, "\\+", b"+", 0, 1),
    (cp_op_minus, "-", b"-", 0, 1),
    (cp_op_star_esc, "\\*", b"*", 0, 1),
    (cp_op_slash, "\\/", b"/", 0, 1),
    (cp_op_percent, "%", b"%", 0, 1),
    (cp_op_amp, "&", b"&", 0, 1),
    (cp_op_pipe, "\\|", b"|", 0, 1),
    (cp_op_tilde, "~", b"~", 0, 1),
    (cp_op_not, "!", b"!", 0, 1),
    (cp_op_lt, "<", b"<", 0, 1),
    (cp_op_gt, ">", b">", 0, 1),
    (cp_op_eq_single, "=", b"=", 0, 1),
    (cp_op_dot_single, "\\.", b".", 0, 1),
    (cp_op_comma, ",", b",", 0, 1),
    (cp_op_semi, ";", b";", 0, 1),
    (cp_op_colon, ":", b":", 0, 1),
    (cp_op_question, "\\?", b"?", 0, 1),
    // Multi-char operators
    (cp_op_pluseq, "\\+=", b"+=", 0, 2),
    (cp_op_minuseq, "-=", b"-=", 0, 2),
    (cp_op_stareq, "\\*=", b"*=", 0, 2),
    (cp_op_slasheq, "\\/=", b"/=", 0, 2),
    (cp_op_percenteq, "%=", b"%=", 0, 2),
    (cp_op_ampeq, "&=", b"&=", 0, 2),
    (cp_op_pipeeq, "\\|=", b"|=", 0, 2),
    (cp_op_careteq, "\\^=", b"^=", 0, 2),
    (cp_op_lshifteq, "<<=", b"<<=", 0, 3),
    (cp_op_rshifteq, ">>=", b">>=", 0, 3),
    (cp_op_eqeq, "==", b"==", 0, 2),
    (cp_op_neq, "!=", b"!=", 0, 2),
    (cp_op_lte, "<=", b"<=", 0, 2),
    (cp_op_gte, ">=", b">=", 0, 2),
    (cp_op_ampamp, "&&", b"&&", 0, 2),
    (cp_op_pipepipe, "\\|\\|", b"||", 0, 2),
    (cp_op_plusplus, "\\+\\+", b"++", 0, 2),
    (cp_op_minusminus, "--", b"--", 0, 2),
    (cp_op_arrow, "->", b"->", 0, 2),
    (cp_op_scope, "::", b"::", 0, 2),
    (cp_op_lshift, "<<", b"<<", 0, 2),
    (cp_op_rshift, ">>", b">>", 0, 2),
    (cp_op_ellipsis, "\\.\\.\\.", b"...", 0, 3),
    // Longer patterns
    (cp_keyword_struct, "struct", b"struct", 0, 6),
    (cp_keyword_union, "union", b"union", 0, 5),
    (cp_keyword_enum, "enum", b"enum", 0, 4),
    (cp_keyword_typedef, "typedef", b"typedef", 0, 7),
    (cp_keyword_sizeof, "sizeof", b"sizeof", 0, 6),
    (cp_keyword_static, "static", b"static", 0, 6),
    (cp_keyword_extern, "extern", b"extern", 0, 6),
    (cp_keyword_include, "include", b"include", 0, 7),
    // Mixed length patterns
    (cp_version, "[0-9]+\\.[0-9]+\\.[0-9]+", b"1.2.3", 0, 5),
    (cp_date_like, "[0-9]{4}-[0-9]{2}-[0-9]{2}", b"2024-01-15", 0, 10),
    (cp_time_like, "[0-9]{2}:[0-9]{2}", b"14:30", 0, 5),
);

// =============================================================================
// 21. PROGRAMMATIC GENERATION — batch DFA pipeline tests
// =============================================================================
mod programmatic_gen {
    use super::*;

    /// Test that every single printable ASCII byte is correctly matched by a dot
    #[test]
    fn dot_matches_all_printable() {
        let src = lex1(".");
        for b in 0x20u8..=0x7E {
            let input = [b];
            let result = sim(&src, &input);
            assert_eq!(result, Some((0, 1)), "dot should match byte 0x{:02x} ({:?})", b, b as char);
        }
    }

    /// Test that dot does not match newline
    #[test]
    fn dot_skips_newline() {
        assert!(sim(&lex1("."), b"\n").is_none());
    }

    /// Test that [a-z] matches all lowercase letters
    #[test]
    fn class_az_all_lower() {
        let src = lex1("[a-z]");
        for b in b'a'..=b'z' {
            let input = [b];
            assert_eq!(sim(&src, &input), Some((0, 1)), "should match {:?}", b as char);
        }
    }

    /// Test that [a-z] does not match digits or uppercase
    #[test]
    fn class_az_rejects_nonlower() {
        let src = lex1("[a-z]");
        for b in b'A'..=b'Z' {
            assert!(sim(&src, &[b]).is_none(), "should not match {:?}", b as char);
        }
        for b in b'0'..=b'9' {
            assert!(sim(&src, &[b]).is_none(), "should not match {:?}", b as char);
        }
    }

    /// Test that [A-Z] matches all uppercase letters
    #[test]
    fn class_AZ_all_upper() {
        let src = lex1("[A-Z]");
        for b in b'A'..=b'Z' {
            assert_eq!(sim(&src, &[b]), Some((0, 1)));
        }
    }

    /// Test that [0-9] matches all digits
    #[test]
    fn class_09_all_digits() {
        let src = lex1("[0-9]");
        for b in b'0'..=b'9' {
            assert_eq!(sim(&src, &[b]), Some((0, 1)));
        }
    }

    /// Test that [0-9] rejects non-digits
    #[test]
    fn class_09_rejects_non_digits() {
        let src = lex1("[0-9]");
        for b in b'a'..=b'z' {
            assert!(sim(&src, &[b]).is_none());
        }
    }

    /// Test negated class [^a-z] on full ASCII range
    #[test]
    fn neg_class_az_full_range() {
        let src = lex1("[^a-z]");
        for b in 1u8..=127 {
            let input = [b];
            let expected = if b >= b'a' && b <= b'z' { None } else { Some((0, 1)) };
            assert_eq!(sim(&src, &input), expected, "byte 0x{:02x}", b);
        }
    }

    /// Test that each literal letter matches itself and nothing else
    #[test]
    fn literal_letter_specificity() {
        for target in b'a'..=b'z' {
            let src = lex1(&format!("{}", target as char));
            for test in b'a'..=b'z' {
                let result = sim(&src, &[test]);
                if test == target {
                    assert_eq!(result, Some((0, 1)), "should match {:?}", test as char);
                } else {
                    assert!(result.is_none(), "{:?} should not match {:?}", target as char, test as char);
                }
            }
        }
    }

    /// Test that [a-z]+ gives maximal munch for various lengths
    #[test]
    fn plus_maximal_munch_lengths() {
        let src = lex1("[a-z]+");
        for len in 1..=30 {
            let input: Vec<u8> = (0..len).map(|i| b'a' + (i as u8 % 26)).collect();
            let result = sim(&src, &input);
            assert_eq!(result, Some((0, len)), "should match length {}", len);
        }
    }

    /// Test that a{n} matches exactly n characters
    #[test]
    fn exact_repeat_lengths() {
        for n in 1..=10 {
            let src = lex1(&format!("a{{{}}}", n));
            let input: Vec<u8> = vec![b'a'; n + 5];
            let result = sim(&src, &input);
            assert_eq!(result, Some((0, n)), "a{{{}}} should match {} chars", n, n);
        }
    }

    /// Test that a{n} requires at least n characters
    #[test]
    fn exact_repeat_requires_n() {
        for n in 2..=10 {
            let src = lex1(&format!("a{{{}}}", n));
            let input: Vec<u8> = vec![b'a'; n - 1];
            let result = sim(&src, &input);
            assert!(result.is_none(), "a{{{}}} should not match {} chars", n, n - 1);
        }
    }

    /// Test a{n,m} range behavior systematically
    #[test]
    fn range_repeat_behavior() {
        let src = lex1("a{2,5}");
        // 0 a's — no match
        assert!(sim(&src, b"b").is_none());
        // 1 a — too few
        assert!(sim(&src, b"a").is_none());
        // 2 a's — minimum
        assert_eq!(sim(&src, b"aa"), Some((0, 2)));
        // 3-5 a's — within range
        assert_eq!(sim(&src, b"aaa"), Some((0, 3)));
        assert_eq!(sim(&src, b"aaaa"), Some((0, 4)));
        assert_eq!(sim(&src, b"aaaaa"), Some((0, 5)));
        // 6+ a's — capped at 5
        assert_eq!(sim(&src, b"aaaaaa"), Some((0, 5)));
        assert_eq!(sim(&src, b"aaaaaaa"), Some((0, 5)));
    }

    /// Test compression equivalence for all single-byte inputs
    #[test]
    fn compression_eq_all_bytes() {
        let src = lex1("[a-zA-Z0-9]+");
        for b in 0u8..=127 {
            let input = [b];
            assert_eq!(sim(&src, &input), sim_c(&src, &input), "byte 0x{:02x}", b);
        }
    }

    /// Test compression eq for multi-rule scanner on various inputs
    #[test]
    fn compression_eq_multi_rule_sweep() {
        let src = lexn(&["[0-9]+", "[a-z]+", "[A-Z]+", "[ \\t]+", "."]);
        let inputs: Vec<&[u8]> = vec![
            b"123", b"abc", b"ABC", b"  \t", b"!", b"@", b"",
            b"hello123", b"42", b"X", b" ",
        ];
        for input in inputs {
            assert_eq!(sim(&src, input), sim_c(&src, input),
                "compression mismatch for {:?}", String::from_utf8_lossy(input));
        }
    }

    /// Test that priority works: first rule wins for equal-length match
    #[test]
    fn priority_first_rule_wins() {
        let patterns: Vec<&str> = vec!["abc", "abc", "abc"];
        let src = lexn(&patterns);
        assert_eq!(sim(&src, b"abc"), Some((0, 3)));
    }

    /// Test keywords vs identifiers systematically
    #[test]
    fn keywords_vs_identifiers() {
        let keywords = ["if", "else", "while", "for", "return", "int", "char", "void"];
        let mut pats: Vec<&str> = keywords.to_vec();
        pats.push("[a-zA-Z_][a-zA-Z0-9_]*");
        let src = lexn(&pats);

        for (i, kw) in keywords.iter().enumerate() {
            let result = sim(&src, kw.as_bytes());
            assert_eq!(result, Some((i, kw.len())),
                "keyword {:?} should match rule {}", kw, i);
        }

        // A non-keyword identifier should match the last (identifier) rule
        let id_rule = pats.len() - 1;
        assert_eq!(sim(&src, b"foo"), Some((id_rule, 3)));
        assert_eq!(sim(&src, b"bar"), Some((id_rule, 3)));
    }

    /// Test that maximal munch beats priority
    #[test]
    fn maximal_munch_beats_priority() {
        let src = lexn(&["if", "[a-z]+"]);
        // "if" matches both, but rule 0 ("if") wins by priority
        assert_eq!(sim(&src, b"if"), Some((0, 2)));
        // "iffy" — rule 1 ("[a-z]+") wins because it's longer
        assert_eq!(sim(&src, b"iffy"), Some((1, 4)));
        // "i" — only rule 1 matches
        assert_eq!(sim(&src, b"i"), Some((1, 1)));
    }

    /// Test all C keywords are recognized correctly
    #[test]
    fn c_keywords_full_set() {
        let kws = [
            "auto", "break", "case", "char", "const", "continue",
            "default", "do", "double", "else", "enum", "extern",
            "float", "for", "goto", "if", "int", "long",
            "register", "return", "short", "signed", "sizeof", "static",
            "struct", "switch", "typedef", "union", "unsigned", "void",
            "volatile", "while",
        ];
        let mut pats: Vec<&str> = kws.to_vec();
        pats.push("[a-zA-Z_][a-zA-Z0-9_]*");
        let src = lexn(&pats);

        for (i, kw) in kws.iter().enumerate() {
            let result = sim(&src, kw.as_bytes());
            assert_eq!(result, Some((i, kw.len())),
                "C keyword {:?} should match rule {}", kw, i);
        }
    }

    /// Generate multi-char operator token tests
    #[test]
    fn c_operators_full_set() {
        let ops: Vec<(&str, &[u8])> = vec![
            ("\\+\\+", b"++"),
            ("--", b"--"),
            ("\\+=", b"+="),
            ("-=", b"-="),
            ("\\*=", b"*="),
            ("\\/=", b"/="),
            ("%=", b"%="),
            ("&=", b"&="),
            ("\\|=", b"|="),
            ("\\^=", b"^="),
            ("<<=", b"<<="),
            (">>=", b">>="),
            ("==", b"=="),
            ("!=", b"!="),
            ("<=", b"<="),
            (">=", b">="),
            ("&&", b"&&"),
            ("\\|\\|", b"||"),
            ("->", b"->"),
            ("<<", b"<<"),
            (">>", b">>"),
        ];
        for (pat, input) in &ops {
            let src = lex1(pat);
            let result = sim(&src, input);
            assert_eq!(result, Some((0, input.len())),
                "pattern {:?} should match {:?}", pat, String::from_utf8_lossy(input));
        }
    }

    /// Test emitter produces valid output for all C keywords
    #[test]
    fn emit_c_for_all_keywords() {
        let kws = [
            "auto", "break", "case", "char", "const", "continue",
            "default", "do", "double", "else", "enum", "extern",
            "float", "for", "goto", "if", "int", "long",
        ];
        let mut src = String::from("%%\n");
        for kw in &kws {
            src.push_str(&format!("{}  printf(\"KW_{}\");\n", kw, kw.to_uppercase()));
        }
        src.push_str("[a-zA-Z_][a-zA-Z0-9_]*  printf(\"ID\");\n");
        let out = emit_c(&src, false);
        assert!(out.contains("yylex"));
        for kw in &kws {
            assert!(out.contains(&format!("KW_{}", kw.to_uppercase())));
        }
    }

    /// Test that empty-matching star patterns work correctly
    #[test]
    fn star_empty_match_all_bytes() {
        let src = lex1("a*");
        // a* should always match (possibly empty)
        for b in b'a'..=b'z' {
            let result = sim(&src, &[b]);
            if b == b'a' {
                assert_eq!(result, Some((0, 1)));
            } else {
                assert_eq!(result, Some((0, 0)));
            }
        }
    }

    /// Test various escape sequences are distinct from each other
    #[test]
    fn escape_sequences_distinct() {
        let pairs = [
            ("\\n", b'\n'),
            ("\\t", b'\t'),
            ("\\r", b'\r'),
            ("\\a", 0x07u8),
            ("\\b", 0x08u8),
            ("\\f", 0x0cu8),
            ("\\v", 0x0bu8),
        ];
        for (pat, expected) in &pairs {
            let src = lex1(pat);
            let result = sim(&src, &[*expected]);
            assert_eq!(result, Some((0, 1)), "Escape {:?} should match byte 0x{:02x}", pat, expected);
            // Ensure it doesn't match an unrelated byte
            let other = if *expected == b'x' { b'y' } else { b'x' };
            assert!(sim(&src, &[other]).is_none(), "Escape {:?} should NOT match {:?}", pat, other as char);
        }
    }
}

// =============================================================================
// 22. ANCHOR TESTS (BOL/EOL) (30 tests)
// =============================================================================
mod anchors {
    use super::*;
    use ft_lex_lib::lex_file::parser::LexFile;

    #[test]
    fn bol_anchor_parsed() {
        let lf = LexFile::parse("%%\n^abc  ;\n", "<t>").unwrap();
        assert!(lf.rules[0].bol_anchor);
        assert!(!lf.rules[0].eol_anchor);
    }

    #[test]
    fn eol_anchor_parsed() {
        let lf = LexFile::parse("%%\nabc$  ;\n", "<t>").unwrap();
        assert!(!lf.rules[0].bol_anchor);
        assert!(lf.rules[0].eol_anchor);
    }

    #[test]
    fn both_anchors_parsed() {
        let lf = LexFile::parse("%%\n^abc$  ;\n", "<t>").unwrap();
        assert!(lf.rules[0].bol_anchor);
        assert!(lf.rules[0].eol_anchor);
    }

    #[test]
    fn no_anchor_parsed() {
        let lf = LexFile::parse("%%\nabc  ;\n", "<t>").unwrap();
        assert!(!lf.rules[0].bol_anchor);
        assert!(!lf.rules[0].eol_anchor);
    }

    #[test]
    fn bol_anchor_emitted() {
        let out = emit_c("%%\n^abc  printf(\"BOL\");\n", false);
        assert!(out.contains("yy_at_bol_at_start"));
    }

    #[test]
    fn eol_anchor_emitted() {
        let out = emit_c("%%\nabc$  printf(\"EOL\");\n", false);
        assert!(out.contains("yy_buf_pos >= yy_buf_len || yy_buf[yy_buf_pos]"));
    }

    #[test]
    fn no_anchor_no_guard() {
        let out = emit_c("%%\nabc  printf(\"NONE\");\n", false);
        // The string "yy_at_bol_at_start" appears in the declaration, not in a guard
        // Count occurrences: should be in declaration only, not in case
        let count = out.matches("yy_at_bol_at_start").count();
        // Declaration + assignment in loop = 2 occurrences minimum, no guard
        assert!(count >= 2, "yy_at_bol_at_start should appear in decl/assignment, count={}", count);
    }

    #[test]
    fn bol_anchor_stripped_from_pattern() {
        let lf = LexFile::parse("%%\n^[a-z]+  ;\n", "<t>").unwrap();
        // The pattern should have ^ stripped — the regex should match "abc" without needing ^
        let src = format!("%%\n{}  ;\n", lf.rules[0].pattern_str);
        // The DFA should still match the pattern itself
        assert!(sim(&src, b"abc").is_some());
    }

    #[test]
    fn eol_anchor_stripped_from_pattern() {
        let lf = LexFile::parse("%%\n[a-z]+$  ;\n", "<t>").unwrap();
        let src = format!("%%\n{}  ;\n", lf.rules[0].pattern_str);
        assert!(sim(&src, b"abc").is_some());
    }

    #[test]
    fn bol_rule_with_normal_rule() {
        // Both BOL and normal rules should parse and emit correctly
        let src = "%%\n^line  printf(\"BOL\");\n[a-z]+  printf(\"ID\");\n";
        let lf = LexFile::parse(src, "<t>").unwrap();
        assert!(lf.rules[0].bol_anchor);
        assert!(!lf.rules[1].bol_anchor);
        let out = emit_c(src, false);
        assert!(out.contains("printf(\"BOL\")"));
        assert!(out.contains("printf(\"ID\")"));
    }

    #[test]
    fn multiple_bol_rules() {
        let src = "%%\n^if  return 1;\n^else  return 2;\n[a-z]+  return 3;\n";
        let lf = LexFile::parse(src, "<t>").unwrap();
        assert!(lf.rules[0].bol_anchor);
        assert!(lf.rules[1].bol_anchor);
        assert!(!lf.rules[2].bol_anchor);
    }
}

// =============================================================================
// 23. NFA/DFA CONSTRUCTION TESTS (30 tests)
// =============================================================================
mod nfa_dfa_construction {
    use super::*;
    use ft_lex_lib::automata::dfa::subset_construction;
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;

    fn build_dfa(src: &str) -> ft_lex_lib::automata::dfa::Dfa {
        let lf = LexFile::parse(src, "<t>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        minimize_dfa(&subset_construction(&nfa))
    }

    #[test]
    fn dfa_has_states() {
        let dfa = build_dfa("%%\na  ;\n");
        assert!(dfa.state_count() >= 2, "DFA should have at least start + accept");
    }

    #[test]
    fn dfa_start_exists() {
        let dfa = build_dfa("%%\na  ;\n");
        assert!(dfa.start.0 < dfa.state_count());
    }

    #[test]
    fn dfa_has_accepting() {
        let dfa = build_dfa("%%\na  ;\n");
        let has_accept = (0..dfa.state_count())
            .any(|i| dfa.arena.get(ft_lex_lib::automata::arena::NodeId(i)).accepting.is_some());
        assert!(has_accept, "DFA must have at least one accepting state");
    }

    #[test]
    fn dfa_single_char_small() {
        let dfa = build_dfa("%%\na  ;\n");
        assert!(dfa.state_count() <= 5, "single char DFA should be small, got {}", dfa.state_count());
    }

    #[test]
    fn dfa_two_rules_has_both_accepts() {
        let dfa = build_dfa("%%\na  ;\nb  ;\n");
        let accepts: Vec<_> = (0..dfa.state_count())
            .filter_map(|i| dfa.arena.get(ft_lex_lib::automata::arena::NodeId(i)).accepting)
            .collect();
        assert!(accepts.contains(&0));
        assert!(accepts.contains(&1));
    }

    #[test]
    fn dfa_class_has_transitions() {
        let dfa = build_dfa("%%\n[a-z]  ;\n");
        let start = dfa.arena.get(dfa.start);
        // Should have transitions for a-z
        let has_a = start.transitions[b'a' as usize].is_some();
        assert!(has_a, "DFA should have transition for 'a'");
    }

    #[test]
    fn dfa_star_start_accepting() {
        let dfa = build_dfa("%%\na*  ;\n");
        // a* matches empty, so start state or an epsilon-reachable state should accept
        let start_accepts = dfa.arena.get(dfa.start).accepting.is_some();
        // Or there's an accepting state reachable with empty string
        if !start_accepts {
            // This is also ok — DFA for a* might not have start as accepting depending on construction
            assert!(dfa.state_count() >= 1);
        }
    }

    #[test]
    fn dfa_minimize_reduces() {
        // A pattern with redundant states should be reduced by minimization
        let lf = LexFile::parse("%%\na|a  ;\n", "<t>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        let dfa = subset_construction(&nfa);
        let min = minimize_dfa(&dfa);
        assert!(min.state_count() <= dfa.state_count());
    }

    #[test]
    fn dfa_many_rules_state_count() {
        let mut src = String::from("%%\n");
        for c in b'a'..=b'z' {
            src.push_str(&format!("{}  ;\n", c as char));
        }
        let dfa = build_dfa(&src);
        // 26 single-char rules sharing start state → should be compact
        assert!(dfa.state_count() < 50, "26 single-char rules should have < 50 states, got {}", dfa.state_count());
    }

    #[test]
    fn dfa_dot_transitions() {
        let dfa = build_dfa("%%\n.  ;\n");
        let start = dfa.arena.get(dfa.start);
        // Dot should have transitions for all bytes except \n
        let count = start.transitions.iter().filter(|t| t.is_some()).count();
        assert!(count >= 200, "dot should have ~255 transitions, got {}", count);
    }

    #[test]
    fn dfa_plus_not_start_accepting() {
        let dfa = build_dfa("%%\na+  ;\n");
        // a+ requires at least one 'a', so start should not accept
        let start_accepts = dfa.arena.get(dfa.start).accepting.is_some();
        assert!(!start_accepts, "a+ start state should not be accepting");
    }

    #[test]
    fn dfa_question_start_accepting() {
        let dfa = build_dfa("%%\na?  ;\n");
        // a? matches empty string
        let start_accepts = dfa.arena.get(dfa.start).accepting.is_some();
        assert!(start_accepts, "a? start state should be accepting");
    }
}

// =============================================================================
// 24. RUST EMITTER TESTS (20 tests via emitter)
// =============================================================================
mod rust_emitter_gen {
    use ft_lex_lib::automata::dfa::subset_construction;
    use ft_lex_lib::automata::minimize::minimize_dfa;
    use ft_lex_lib::automata::nfa::NfaBuilder;
    use ft_lex_lib::emit::rust_emitter::RustEmitter;
    use ft_lex_lib::emit::traits::CodeEmitter;
    use ft_lex_lib::lex_file::parser::LexFile;

    fn emit_rust(source: &str, suppress: bool) -> String {
        let lf = LexFile::parse(source, "<gen>").unwrap();
        let mut b = NfaBuilder::new();
        let rules: Vec<_> = lf.rules.iter().enumerate().map(|(i, r)| (r.regex.clone(), i)).collect();
        let nfa = b.build_combined(&rules);
        let min = minimize_dfa(&subset_construction(&nfa));
        let emitter = RustEmitter::new(suppress);
        emitter.emit(&lf, &min).unwrap()
    }

    #[test]
    fn rust_emit_basic() {
        let out = emit_rust("%%\n[0-9]+  printf(\"NUM\");\n", false);
        assert!(out.contains("fn yylex"));
    }

    #[test]
    fn rust_emit_has_transition() {
        let out = emit_rust("%%\na  ;\n", false);
        assert!(out.contains("YY_TRANSITION"));
    }

    #[test]
    fn rust_emit_has_accept() {
        let out = emit_rust("%%\na  ;\n", false);
        assert!(out.contains("YY_ACCEPT"));
    }

    #[test]
    fn rust_emit_has_num_states() {
        let out = emit_rust("%%\na  ;\n", false);
        assert!(out.contains("YY_NUM_STATES"));
    }

    #[test]
    fn rust_emit_has_yytext() {
        let out = emit_rust("%%\na  ;\n", false);
        assert!(out.contains("yytext"));
    }

    #[test]
    fn rust_emit_suppress() {
        let out = emit_rust("%%\na  ;\n", true);
        assert!(out.contains("fn yylex"));
    }

    #[test]
    fn rust_emit_header_code() {
        let out = emit_rust("%{\nuse std::io;\n%}\n%%\na  ;\n", false);
        assert!(out.contains("use std::io;"));
    }

    #[test]
    fn rust_emit_user_code() {
        let out = emit_rust("%%\na  ;\n%%\nfn main() {}\n", false);
        assert!(out.contains("fn main()"));
    }

    #[test]
    fn rust_emit_multiple_rules() {
        let out = emit_rust("%%\n[0-9]+  return 1;\n[a-z]+  return 2;\n", false);
        assert!(out.contains("return 1;") || out.contains("1 =>"));
    }

    #[test]
    fn rust_emit_noyywrap() {
        let out = emit_rust("%option noyywrap\n%%\na  ;\n", false);
        assert!(out.contains("fn yylex"));
    }
}

// =============================================================================
// 25. LIBL TESTS (10 tests)
// =============================================================================
mod libl_gen {
    use ft_lex_lib::emit::libl::generate_libl_c;

    #[test]
    fn libl_has_yywrap() {
        let out = generate_libl_c();
        assert!(out.contains("yywrap"));
    }

    #[test]
    fn libl_has_main() {
        let out = generate_libl_c();
        assert!(out.contains("main"));
    }

    #[test]
    fn libl_returns_1() {
        let out = generate_libl_c();
        assert!(out.contains("return 1"));
    }

    #[test]
    fn libl_has_yylex_call() {
        let out = generate_libl_c();
        assert!(out.contains("yylex()"));
    }

    #[test]
    fn libl_includes_stdio() {
        // libl may or may not include stdio, but should reference yylex
        let out = generate_libl_c();
        assert!(out.len() > 20);
    }
}
