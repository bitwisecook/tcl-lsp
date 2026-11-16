// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! O100: constants substituted into call words, `$var` references and interpolated strings under the release's grammar.

use super::*;

#[test]
fn propagation_literal_words_retain_selected_grammar_and_exact_unicode_value() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        for value in [
            "",
            "42",
            "has space",
            "$literal [data]",
            "brace}tail",
            "λ",
            "line\nnext",
            "trailing\\",
        ] {
            let word = render_propagation_word(value, config).unwrap();
            assert!(tcl_syntax::word_rules::lexical_unicode_source_word_matches(
                &word, value, config
            ));
        }
    }
}

// internal helpers

/// The `${…}` closer comes from the shared owner, so the constant this
/// rewrite inlines belongs to the name the lexer spanned.
///
/// The rewritten text is written back into the document as an O100 fix, so
/// resolving the wrong name substitutes some other variable's value into
/// the user's string.
#[test]
fn substitute_dollar_refs_follows_the_release_close_rule() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    let braced = |braced_var| tcl_lexer::LexerConfig {
        braced_var,
        ..tcl_lexer::LexerConfig::default()
    };
    let mut c = std::collections::HashMap::new();
    c.insert("a{b}c".to_owned(), "NESTED".to_owned());
    c.insert("a{b".to_owned(), "FIRST".to_owned());

    // 9.x names `a{b}c`; the whole reference is consumed.
    assert_eq!(
        substitute_dollar_refs("v=${a{b}c}", &c, braced(Tcl9Nesting)).as_deref(),
        Some("v=NESTED")
    );
    // 8.x names `a{b`, and `c}` stays as literal word text.
    assert_eq!(
        substitute_dollar_refs("v=${a{b}c}", &c, braced(FirstClose)).as_deref(),
        Some("v=FIRSTc}")
    );
    // An unterminated reference declines the whole rewrite rather than
    // inventing a name that runs to end-of-input.
    assert!(substitute_dollar_refs("v=${a{b", &c, braced(Tcl9Nesting)).is_none());
}

#[test]
fn interpolation_uses_retained_close_policy_over_catalogue_profile() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Lexical rewrite software control, not variable/frame or Native evidence.
    let source = "\"v=${a{b}c}\"";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut context =
        PassContext::with_dialect(source, InterproceduralAnalysis::default(), Some(profile));
    let original_config = context.lexer_config();
    context.source_lexer_config = Some(tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    });
    let registry_context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = registry_context.commands();
    let module_for = |config| {
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&registry_context),
            config,
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input);
        lowerer.lower(source);
        lowerer.finish_module(source)
    };
    let selected_module = module_for(context.lexer_config());
    context.registry = Some(registry);
    context.ir_module = Some(&selected_module);
    let constants = std::collections::HashMap::from([
        ("a{b}c".into(), "NESTED".into()),
        ("a{b".into(), "FIRST".into()),
    ]);
    let config = context.lexer_config();
    visit_string_interpolation(
        &mut context,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        source,
        &constants,
        config,
    );
    assert_eq!(context.optimisations.len(), 1);
    assert_eq!(context.optimisations[0].replacement, "\"v=NESTED\"");
    context.source_lexer_config = None;
    let original_module = module_for(original_config);
    context.ir_module = Some(&original_module);
    context.optimisations.clear();
    let config = context.lexer_config();
    visit_string_interpolation(
        &mut context,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        source,
        &constants,
        config,
    );
    assert_eq!(context.optimisations.len(), 1);
    assert_eq!(context.optimisations[0].replacement, "\"v=FIRSTc}\"");
}

#[test]
fn interpolation_element_indices_keep_literal_braced_names_and_dynamic_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let constants = std::collections::HashMap::from([
        ("a($idx)".into(), "ELEMENT".into()),
        ("a($idx".into(), "SCALAR".into()),
    ]);
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    assert_eq!(
        substitute_dollar_refs("v=$a($idx)", &constants, config).as_deref(),
        Some("v=$a($idx)")
    );
    assert_eq!(
        substitute_dollar_refs("v=${a($idx)}", &constants, config).as_deref(),
        Some("v=ELEMENT")
    );
    assert_eq!(
        substitute_dollar_refs("v=${a($idx}", &constants, config).as_deref(),
        Some("v=SCALAR")
    );
}

#[test]
fn simple_var_ref_parses_bare_and_braced() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    assert_eq!(
        whole_scalar_variable_reference_name("$foo", config),
        Some("foo")
    );
    assert_eq!(
        whole_scalar_variable_reference_name("${bar}", config),
        Some("bar")
    );
    assert!(whole_scalar_variable_reference_name("$foo(idx)", config).is_none());
    assert!(whole_scalar_variable_reference_name("plain", config).is_none());
}

#[test]
fn safe_bare_word_accepts_int_and_safe_words() {
    assert!(is_value_safe_bare_word("42"));
    assert!(is_value_safe_bare_word("-7"));
    assert!(is_value_safe_bare_word("foo_bar"));
    assert!(!is_value_safe_bare_word(""));
    assert!(!is_value_safe_bare_word("has space"));
    assert!(!is_value_safe_bare_word("$dollar"));
}

// end-to-end tests

#[test]
fn constant_int_propagates_into_call_arg() {
    let opts = run_pass("set x 42\nputs $x");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "42"),
        "expected O100 propagating 42, got {opts:?}",
    );
}

#[test]
fn unsafe_string_constant_is_not_propagated() {
    // Tcl metacharacters in the value → must not inline as
    // a bare word.
    let opts = run_pass("set x \"$other\"\nputs $x");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O100),
        "unsafe string should not be propagated, got {opts:?}",
    );
}

#[test]
fn o100_multi_word_constant_renders_via_quoter() {
    // A constant containing whitespace or
    // metacharacters is re-rendered as a single self-contained word
    // via the canonical quoter instead of bailing.
    // `return` value position → `return {Hello World}`.
    let ret = run_pass("proc ::f {} { set msg {Hello World}\nreturn $msg }");
    assert!(
        ret.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "return {Hello World}"),
        "expected O100 `return {{Hello World}}`, got {ret:?}",
    );
    // Command-argument position → `puts {Hello World}`.
    let arg = run_pass("proc ::f {} { set msg {Hello World}\nputs $msg }");
    assert!(
        arg.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "{Hello World}"),
        "expected O100 `{{Hello World}}` arg, got {arg:?}",
    );
    // A `$`/`[`-bearing literal constant is brace-quoted (the value
    // is literal — braces suppress the would-be substitution).
    let meta = run_pass("proc ::f {} { set m {a$b}\nputs $m }");
    assert!(
        meta.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "{a$b}"),
        "expected O100 `{{a$b}}` arg, got {meta:?}",
    );
}

#[test]
fn non_const_lattice_skipped() {
    // Two different writes to x → ConstSet or Overdefined,
    // not a single Const.
    let opts = run_pass("set x 1\nif {$cond} { set x 2 }\nputs $x");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O100),
        "non-const lattice should be skipped, got {opts:?}",
    );
}

#[test]
fn braced_var_reference_also_propagated() {
    let opts = run_pass("set x 7\nputs ${x}");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "7"),
        "expected O100 for braced var ref, got {opts:?}",
    );
}

#[test]
fn string_interpolation_const_is_inlined() {
    // `set count 42` → Const(42) in SCCP → interpolation into
    // `"count is $count"` substitutes to `"count is 42"`.
    let opts = run_pass("set count 42\nputs \"count is $count\"");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement.contains("42")),
        "expected O100 inlining count into interpolation, got {opts:?}",
    );
}

#[test]
fn string_interpolation_unknown_var_skipped() {
    // `$name` is not in the constants map → must not fire.
    let opts = run_pass("puts \"hello $name\"");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O100),
        "unknown var should not be inlined, got {opts:?}",
    );
}

#[test]
fn substitute_dollar_refs_handles_braced_and_missing_vars() {
    let mut c = std::collections::HashMap::new();
    c.insert("x".into(), "42".into());
    assert_eq!(
        substitute_dollar_refs("a${x}b", &c, tcl_lexer::LexerConfig::default()).as_deref(),
        Some("a42b"),
    );
    // Missing var → None (cannot partially substitute).
    assert!(substitute_dollar_refs("$unknown", &c, tcl_lexer::LexerConfig::default()).is_none());
}

/// Inside a command substitution a braced word substitutes nothing, so
/// its `$x` stays; a `$x` elsewhere, including one after a mid-word
/// brace, is inlined. tclsh 8.4.20 through 9.1b0 print `b:{$x}`,
/// `b:{a $x b}`, `m:a{5}` and `w:{5} {$x} 5` for these (#2250).
#[test]
fn substitute_dollar_refs_leaves_a_braced_word_in_a_cmd_sub_as_written() {
    let mut c = std::collections::HashMap::new();
    c.insert("x".into(), "5".into());
    let sub = |text: &str| substitute_dollar_refs(text, &c, tcl_lexer::LexerConfig::default());
    for (text, want) in [
        ("b:[list {$x}]", "b:[list {$x}]"),
        ("b:[list {a $x b}]", "b:[list {a $x b}]"),
        ("b:[list [list {$x}]]", "b:[list [list {$x}]]"),
        ("s:[set y 1;list {$x}]", "s:[set y 1;list {$x}]"),
        ("c:[list {a]b} $x]", "c:[list {a]b} 5]"),
        ("b:[list {a\\}b $x}]", "b:[list {a\\}b $x}]"),
        ("m:[list a{$x}]", "m:[list a{5}]"),
        ("w:{$x} [list {$x}] $x", "w:{5} [list {$x}] 5"),
        // A `{` inside a quoted word of the substituted script is literal
        // text, and Tcl substitutes the `$x` after it (found in review).
        ("q:[list \"foo {$x}\"]", "q:[list \"foo {5}\"]"),
        ("q:[list \"a]b\" $x]", "q:[list \"a]b\" 5]"),
    ] {
        assert_eq!(sub(text).as_deref(), Some(want), "{text}");
    }
    // A braced word that never closes rewrites nothing.
    assert!(sub("b:[list {$x]").is_none_or(|r| r == "b:[list {$x]"));
}

/// Unchanged text keeps its bytes: copying a byte at a time as `char`
/// turned `é` into `Ã©` (#2251).
#[test]
fn substitute_dollar_refs_keeps_non_ascii_text() {
    let mut c = std::collections::HashMap::new();
    c.insert("x".into(), "5".into());
    for (text, want) in [
        ("café $x", "café 5"),
        ("é\\t$x€", "é\\t5€"),
        ("v:[string length é$x]", "v:[string length é5]"),
    ] {
        assert_eq!(
            substitute_dollar_refs(text, &c, tcl_lexer::LexerConfig::default()).as_deref(),
            Some(want),
            "{text}"
        );
    }
}

#[test]
fn substitute_dollar_refs_ends_the_empty_braced_name_at_its_brace() {
    let mut c = std::collections::HashMap::new();
    c.insert(String::new(), "5".into());
    for (text, want) in [
        ("a=${}b", "a=5b"),
        ("a=${}", "a=5"),
        ("${}${}", "55"),
        ("v:[list ${}]c", "v:[list 5]c"),
    ] {
        assert_eq!(
            substitute_dollar_refs(text, &c, tcl_lexer::LexerConfig::default()).as_deref(),
            Some(want),
            "{text}"
        );
    }
}

#[test]
fn substitute_dollar_refs_guards_multiword_in_cmd_sub() {
    let mut c = std::collections::HashMap::new();
    c.insert("lst".into(), "a b c".into());
    c.insert("n".into(), "5".into());
    // A multi-word value in plain string text inlines verbatim (it stays
    // part of the one string word).
    assert_eq!(
        substitute_dollar_refs("x=$lst", &c, tcl_lexer::LexerConfig::default()).as_deref(),
        Some("x=a b c"),
    );
    // The SAME value inside a `[…]` command substitution would split one
    // argument into several — bail, keeping `$lst`.
    assert!(
        substitute_dollar_refs("r: [lsearch $lst x]", &c, tcl_lexer::LexerConfig::default())
            .is_none()
    );
    // A single-word value inside a cmd-sub is still safe to inline.
    assert_eq!(
        substitute_dollar_refs("r: [expr $n + 1]", &c, tcl_lexer::LexerConfig::default())
            .as_deref(),
        Some("r: [expr 5 + 1]"),
    );
}

#[test]
fn whole_word_cmd_sub_not_wrapped_as_string_interpolation() {
    // Regression: `puts [expr {$a + $b}]` is a free-standing command
    // substitution, not a `"…"` string. The interpolation path must not
    // wrap it in quotes / mis-span it (which produced the corrupt
    // `puts "[expr {3 + 4}]"]`). No O100 string-interpolation rewrite may
    // fire on a whole-word `[…]`.
    let opts = run_pass("set a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter()
            .all(|o| o.replacement != "\"[expr {3 + 4}]\"" && !o.replacement.contains("\"]")),
        "whole-word cmd-sub must not be string-interpolated, got {opts:?}",
    );
}

#[test]
fn list_constant_not_inlined_into_string_cmd_sub() {
    // Regression: `[lsearch -exact $tokens uic]` must NOT become
    // `[lsearch -exact tran 1n 100n uic uic]` — that splits the list into
    // separate args and errors at runtime (`bad option "tran"`).
    let opts = run_pass("set tokens {tran 1n 100n uic}\nputs \"r: [lsearch -exact $tokens uic]\"");
    assert!(
        opts.iter()
            .all(|o| !o.replacement.contains("tran 1n 100n uic uic")),
        "list literal must not be word-split into the cmd-sub, got {opts:?}",
    );
}
