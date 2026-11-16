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

use super::*;
use crate::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

fn reg() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

#[test]
fn switch_exact() {
    // Both forms lower to a structured Switch; the multi-arg form
    // and the single-braced form must agree on shape. The subject is a
    // known non-option word: an unknown one may be a `-` switch in C.
    for src in [
        "set x a\nswitch $x { a {puts a} b {puts b} }",
        "set x a\nswitch $x a {puts a} b {puts b}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            matches!(
                &m.top_level.statements[1],
                Statement::Switch { mode, .. } if *mode == SwitchMode::Exact
            ),
            "expected exact Switch for {src:?}",
        );
    }
}

#[test]
fn switch_single_braced_body_is_lowered() {
    // The single-braced arm form `switch $x { a {body} … }` must lower
    // each arm body into real IR statements, not an empty Script.
    let m = lower_to_ir("set x a\nswitch $x { a {puts hi} b {set y 1} }", &reg());
    let Statement::Switch { arms, .. } = &m.top_level.statements[1] else {
        panic!("expected Switch");
    };
    assert_eq!(arms.len(), 2);
    for arm in arms {
        let body = arm.body.as_ref().expect("arm body");
        assert!(
            !body.statements.is_empty(),
            "single-braced arm body should lower to non-empty IR, got {body:?}",
        );
    }
}

#[test]
fn switch_substituted_arm_body_barriers() {
    // A multi-arg arm body that is a substitution
    // (`$handler`, `[cmd]`, a quoted word) must NOT be lowered from its
    // unsubstituted spelling — the switch defers to the runtime command so
    // no phantom Call to `${handler}` is fabricated.
    for src in [
        "switch $x a $handler",
        "switch $x a [get_handler]",
        "switch $x a \"puts $x\"",
        "switch $x a {puts a} b $other",
    ] {
        let m = lower_to_ir(src, &reg());
        super::super::tests::assert_runtime_opaque_call(
            &m.top_level.statements[0],
            &reg(),
            "switch",
        );
    }
}

#[test]
fn switch_static_arm_body_still_lowers() {
    // FP-guard for: genuinely literal arm bodies (braced,
    // and the `-` fallthrough marker) still lower to a structured Switch.
    for src in [
        "set x a\nswitch $x a {puts a} b {puts b}",
        "set x a\nswitch $x a - b {puts b}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            matches!(&m.top_level.statements[1], Statement::Switch { .. }),
            "expected structured Switch for {src:?}, got {:?}",
            m.top_level.statements[1],
        );
    }
}

/// `case` lowers through the switch hook on its own descriptor:
/// no word is an option, the `in` word is skipped, and every clause is
/// a glob comparison — an opaque glob `Statement::Switch` naming `case`
/// in each form, its final `default` the default body.
#[test]
fn case_lowers_as_an_opaque_glob_selection() {
    for src in [
        "case abc in a* {puts A} default {puts D}",
        "case abc a* {puts A} default {puts D}",
        "case abc in {a* {puts A} default {puts D}}",
        "case abc {a* {puts A} default {puts D}}",
    ] {
        let m = lower_to_ir(src, &reg());
        let Statement::Switch {
            subject,
            arms,
            default_body,
            mode,
            nocase,
            command,
            ..
        } = &m.top_level.statements[0]
        else {
            panic!(
                "expected a Switch for {src:?}, got {:?}",
                m.top_level.statements[0]
            );
        };
        assert_eq!(subject, "abc", "{src:?}");
        assert_eq!(*mode, SwitchMode::Glob, "{src:?}");
        assert!(!nocase, "{src:?}");
        assert_eq!(command, "case", "{src:?}");
        let patterns: Vec<&str> = arms.iter().map(|arm| arm.pattern.as_str()).collect();
        assert_eq!(patterns, ["a*"], "{src:?}");
        assert!(default_body.is_some(), "{src:?}");
    }
}

/// A `case` clause the IR arm cannot hold lowers to a barrier, so the
/// runtime command runs it: a pattern `case` splits as a list, a
/// substituted pattern that may be one, a `default` before the last
/// clause (a fallback wherever it stands), and a word after the subject
/// whose value may be the `in` separator.
#[test]
fn case_clauses_the_arm_cannot_hold_are_barriers() {
    for src in [
        "case abc in {x a*} {puts L} default {puts D}",
        "case abc in {a\\*} {puts E}",
        "case abc {{x a*} {puts L}}",
        "case abc in $p {puts P}",
        "case abc in default {puts D} a* {puts A}",
        "case abc $w a* {puts A}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            matches!(&m.top_level.statements[0], Statement::Barrier { .. }),
            "expected a barrier for {src:?}, got {:?}",
            m.top_level.statements[0],
        );
    }
}

/// From 9.0 there is no `case` command, so nothing lowers it.
#[test]
fn case_does_not_lower_under_9() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let m = crate::lowering::lower_to_ir_with_dialect(
        "case abc in a* {puts A}",
        registry,
        tcl_lexer::LexerConfig::default(),
        Some(profile),
    );
    assert!(
        !matches!(&m.top_level.statements[0], Statement::Switch { .. }),
        "{:?}",
        m.top_level.statements[0]
    );
}

/// Each word's delimiters reach the statement: per word, whether it was
/// braced (`raw_arg_braced`) or double-quoted (`raw_arg_quoted`), so a
/// bare `-` body is told apart from a quoted or braced one.
#[test]
fn a_switch_records_how_each_word_was_delimited() {
    let m = lower_to_ir(
        "set x a\nswitch -glob -- $x a \"-\" b - c {-} d {puts d}",
        &reg(),
    );
    let Statement::Switch {
        raw_args,
        raw_arg_braced,
        raw_arg_quoted,
        ..
    } = &m.top_level.statements[1]
    else {
        panic!("expected a Switch, got {:?}", m.top_level.statements[1]);
    };
    assert_eq!(raw_args.len(), 11, "{raw_args:?}");
    let flagged = |flags: &[bool]| -> Vec<usize> {
        flags
            .iter()
            .enumerate()
            .filter_map(|(index, &flag)| flag.then_some(index))
            .collect()
    };
    assert_eq!(flagged(raw_arg_quoted), [4], "{raw_args:?}");
    assert_eq!(flagged(raw_arg_braced), [8, 10], "{raw_args:?}");
}

// `patterns_braced` distinguishes a literal-pattern
// braced block from the substituting separate-words form.

#[test]
fn switch_braced_block_sets_patterns_braced_true() {
    let m = lower_to_ir("set x a\nswitch $x { a {puts hi} b {set y 1} }", &reg());
    let Statement::Switch {
        patterns_braced, ..
    } = &m.top_level.statements[1]
    else {
        panic!("expected Switch");
    };
    assert!(
        *patterns_braced,
        "single braced `{{pat body …}}` block → patterns_braced = true",
    );
}

#[test]
fn switch_separate_words_sets_patterns_braced_false() {
    // `switch $s a {body1} b {body2}` — patterns are separate words
    // that substitute at runtime, so patterns_braced must be false.
    let m = lower_to_ir("set x a\nswitch $x a {puts hi} b {set y 1}", &reg());
    let Statement::Switch {
        patterns_braced, ..
    } = &m.top_level.statements[1]
    else {
        panic!("expected Switch");
    };
    assert!(
        !*patterns_braced,
        "separate-words pattern form → patterns_braced = false",
    );
}

#[test]
fn try_with_finally() {
    let m = lower_to_ir("try {error oops} finally {cleanup}", &reg());
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Try {
            finally_body: Some(_),
            ..
        }
    ));
}

#[test]
fn try_with_handler() {
    let m = lower_to_ir("try {error oops} on error {e opts} {puts $e}", &reg());
    if let Statement::Try { handlers, .. } = &m.top_level.statements[0] {
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].kind, HandlerMatch::CompletionCode);
    } else {
        panic!("expected Try");
    }
}

#[test]
fn try_trap_pattern_projection_preserves_tcl_word_literalness() {
    let source = r#"try {} \
            trap {A \$B} {m o} {} \
            trap {A {$B}} {m o} {} \
            trap {A \[B\]} {m o} {} \
            trap {A {[B]}} {m o} {} \
            trap {A C:\\tmp} {m o} {} \
            trap {A {C:\tmp}} {m o} {} \
            trap "A \{" {m o} {} \
            trap $pattern {m o} {} \
            trap "A $pattern" {m o} {} \
            trap [list A B] {m o} {}"#;
    let module = lower_to_ir(source, &reg());
    let Statement::Try { handlers, .. } = &module.top_level.statements[0] else {
        panic!("expected structured try");
    };
    let patterns: Vec<_> = handlers
        .iter()
        .map(|handler| handler.trap_pattern.clone())
        .collect();
    assert_eq!(
        patterns,
        vec![
            Some(vec!["A".into(), "$B".into()]),
            Some(vec!["A".into(), "$B".into()]),
            Some(vec!["A".into(), "[B]".into()]),
            Some(vec!["A".into(), "[B]".into()]),
            Some(vec!["A".into(), r"C:\tmp".into()]),
            Some(vec!["A".into(), r"C:\tmp".into()]),
            None,
            None,
            None,
            None,
        ]
    );
}

#[test]
fn try_trap_pattern_projection_uses_the_release_escape_grammar() {
    // Tcl 8.6 consumes the same wide-unicode escape extent as Tcl 9,
    // but its UTF-16-internal build replaces a non-BMP scalar with
    // U+FFFD. Tcl 9 retains the scalar. This is observable in trap's
    // error-code prefix and therefore must follow the lowerer's profile.
    let source = r#"try {} trap "A \U0001F600" {m o} {}"#;
    let pattern = |dialect| {
        let module = crate::lowering::lower_to_ir_with_config(
            source,
            &reg(),
            tcl_lexer::LexerConfig::for_dialect(dialect),
        );
        let Statement::Try { handlers, .. } = &module.top_level.statements[0] else {
            panic!("expected structured try");
        };
        handlers[0].trap_pattern.clone()
    };

    assert_eq!(pattern("tcl8.6"), Some(vec!["A".into(), "\u{fffd}".into()]));
    assert_eq!(pattern("tcl9.0"), Some(vec!["A".into(), "😀".into()]));
}

#[test]
fn try_dash_handler_body_is_fallthrough() {
    // A `-` handler body is a fallthrough marker (shares the
    // next non-`-` handler's body, like `switch`), not a zero-arg `-`
    // command. It must lower to a fallthrough handler with an empty body.
    let m = lower_to_ir(
        "try {set x 1} on ok result - trap NONE result {return 0} on error msg {return 1}",
        &reg(),
    );
    let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
        panic!("expected Try");
    };
    let shape: Vec<(HandlerMatch, &str, bool)> = handlers
        .iter()
        .map(|h| (h.kind, h.match_arg.as_str(), h.fallthrough))
        .collect();
    assert_eq!(
        shape,
        vec![
            (HandlerMatch::CompletionCode, "ok", true),
            (HandlerMatch::ErrorCodePrefix, "NONE", false),
            (HandlerMatch::CompletionCode, "error", false)
        ],
    );
    // The fallthrough handler carries no statements of its own.
    assert_eq!(handlers[0].body.statements, [] as [crate::ir::Statement; 0]);
    assert_ne!(handlers[1].body.statements, [] as [crate::ir::Statement; 0]);
}

#[test]
fn try_braced_dash_handler_body_is_fallthrough() {
    // Per Tcl's `TclNRTryObjCmd`, a body of `{-}` evaluates to the string
    // `-` and is equally a fallthrough marker.
    let m = lower_to_ir("try {set x 1} on ok a {-} trap NONE b {return $b}", &reg());
    let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
        panic!("expected Try");
    };
    assert!(handlers[0].fallthrough);
    assert_eq!(handlers[0].body.statements, [] as [crate::ir::Statement; 0]);
}

#[test]
fn try_consecutive_dash_handlers_are_fallthrough() {
    // Several `-` handlers in a row all share the final body.
    let m = lower_to_ir(
        "try {set x 1} on ok a - on return b - trap NONE c {return $c}",
        &reg(),
    );
    let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
        panic!("expected Try");
    };
    let flags: Vec<bool> = handlers.iter().map(|h| h.fallthrough).collect();
    assert_eq!(flags, vec![true, true, false]);
}

#[test]
fn try_empty_brace_handler_body_is_not_fallthrough() {
    // A genuinely empty `{}` body is valid and distinct from `-`.
    let m = lower_to_ir("try {set x 1} on ok a {} on error b {return $b}", &reg());
    let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
        panic!("expected Try");
    };
    assert!(!handlers[0].fallthrough);
}

#[test]
fn try_quoted_dash_handler_body_is_fallthrough() {
    // A quoted `"-"` evaluates to the string `-`, like the bare/braced forms.
    let m = lower_to_ir(
        "try {set x 1} on ok a \"-\" trap NONE b {return $b}",
        &reg(),
    );
    let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
        panic!("expected Try");
    };
    assert!(handlers[0].fallthrough);
    assert_eq!(handlers[0].body.statements, [] as [crate::ir::Statement; 0]);
}

#[test]
fn try_backslash_escaped_dash_handler_body_is_fallthrough() {
    // Tcl applies backslash substitution before `try` sees the word, so a
    // bare `\-` / `\x2d` body evaluates to `-` and is a fallthrough
    // marker.
    for src in [
        "try {set x 1} on ok a \\- trap NONE b {return $b}",
        "try {set x 1} on ok a \\x2d trap NONE b {return $b}",
        "try {set x 1} on ok a \"\\-\" trap NONE b {return $b}",
    ] {
        let m = lower_to_ir(src, &reg());
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try for {src:?}");
        };
        assert!(handlers[0].fallthrough, "expected fallthrough for {src:?}");
        assert_eq!(
            handlers[0].body.statements.len(),
            0,
            "expected empty body for {src:?}",
        );
    }
}

#[test]
fn try_braced_escaped_dash_handler_body_is_not_fallthrough() {
    // Braces suppress backslash substitution: `{\-}` is the literal
    // two-char string `\-`, which is *not* the `-` fallthrough marker.
    let m = lower_to_ir(
        "try {set x 1} on ok a {\\-} trap NONE b {return $b}",
        &reg(),
    );
    let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
        panic!("expected Try");
    };
    assert!(
        !handlers[0].fallthrough,
        "braced `{{\\-}}` is a literal string, not a fallthrough",
    );
}

#[test]
fn catch_with_vars() {
    let m = lower_to_ir("catch {expr 1/0} result opts", &reg());
    if let Statement::Catch {
        result_var,
        options_var,
        ..
    } = &m.top_level.statements[0]
    {
        assert_eq!(result_var.as_deref(), Some("result"));
        assert_eq!(options_var.as_deref(), Some("opts"));
    } else {
        panic!("expected Catch");
    }
}

#[test]
fn catch_preserves_command_tokens() {
    // ``Statement::Catch`` must carry the full ``CommandTokens``
    // snapshot so the CFG's ``Catch → Call`` lowering
    // (`emit_opaque_catch`) can preserve the brace-vs-bare
    // shape of the body word when reconstructing the script
    // for the runtime's eval-fallback.
    let m = lower_to_ir("catch {set x 1} result", &reg());
    if let Statement::Catch { tokens, .. } = &m.top_level.statements[0] {
        let tokens = tokens.as_ref().expect("tokens populated");
        // 3 words: ``catch`` + ``{set x 1}`` + ``result``.
        assert_eq!(tokens.argv.len(), 3);
    } else {
        panic!("expected Catch");
    }
}

#[test]
fn catch_dollar_var_body_falls_through_to_barrier() {
    // ``catch $cmd res`` has a single VAR token body, not a
    // brace-literal `Str` token.  The lowering must treat it
    // as a dynamic body and emit a Barrier so the runtime
    // ``eval_catch`` evaluates the substituted value.
    let m = lower_to_ir("set cmd [gets stdin]\ncatch $cmd res", &reg());
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::Barrier { reason, .. } if reason == "catch with dynamic body"
    ));
}

#[test]
fn catch_bracket_command_body_falls_through_to_barrier() {
    // ``catch [build] res`` — single-token but command-subst,
    // not brace-literal.  Must hit the Barrier path.
    let m = lower_to_ir("catch [build] res", &reg());
    super::super::tests::assert_runtime_opaque_call(&m.top_level.statements[0], &reg(), "catch");
}

#[test]
fn dict_set() {
    let m = lower_to_ir("dict set d key value", &reg());
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Call { command, defs, .. }
            if command == "dict" && defs.contains(&"d".to_string())
    ));
}

#[test]
fn dict_for() {
    let m = lower_to_ir("dict for {k v} {a 1} {puts $k}", &reg());
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Foreach {
            is_dict_iteration: true,
            ..
        }
    ));
}

// `while`/`for`/`foreach`/`try`/`dict for` must gate their body word on
// the token's kind, not single-token-ness alone. `$body` is a single VAR
// token, so a single-token gate lowers it as if its reconstructed text
// (`${body}`) were the literal script — miscompiling, or (when the
// reconstruction lengthens the word, as `${body}` does over `$body`)
// panicking in codegen on an out-of-bounds span. Each must barrier
// instead, mirroring `catch_dollar_var_body_falls_through_to_barrier`.

#[test]
fn while_dollar_var_body_falls_through_to_barrier() {
    let m = lower_to_ir("set body [gets stdin]\nwhile {1} $body", &reg());
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::Barrier { reason, .. } if reason == "while with dynamic arguments"
    ));
}

#[test]
fn for_dollar_var_body_falls_through_to_barrier() {
    let m = lower_to_ir("set body [gets stdin]\nfor {} {1} {} $body", &reg());
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::Barrier { reason, .. } if reason == "for with dynamic arguments"
    ));
}

#[test]
fn foreach_dollar_var_body_falls_through_to_barrier() {
    let m = lower_to_ir(
        "set l [gets stdin]\nset body [gets stdin]\nforeach x $l $body",
        &reg(),
    );
    assert!(matches!(
        &m.top_level.statements[2],
        Statement::Barrier { reason, .. } if reason == "foreach with dynamic body"
    ));
}

#[test]
fn try_dollar_var_body_falls_through_to_barrier() {
    let m = lower_to_ir("set body [gets stdin]\ntry $body", &reg());
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::Barrier { reason, .. } if reason == "try with dynamic body"
    ));
}

// …and the `try` bodies: a gate on the primary body alone leaves
// `finally` and every non-`-` `on`/`trap` handler walking a `$body` word
// and rebasing its reconstructed text
// off the end of the source. tclsh 8.6.16 / 9.0.4 run both spellings fine
// (`set body {puts hi}; try {} finally $body` prints `hi`), so the
// degraded path must be a barrier — the runtime `try` — not a panic.
// C's `TclCompileTryCmd` bails on the same words for the same reason.

#[test]
fn try_dollar_var_finally_body_falls_through_to_barrier() {
    let m = lower_to_ir("set body [gets stdin]\ntry {} finally $body", &reg());
    assert!(
        matches!(
            &m.top_level.statements[1],
            Statement::Barrier { reason, .. } if reason == "try with dynamic finally body"
        ),
        "got {:?}",
        m.top_level.statements[1],
    );
}

#[test]
fn try_dollar_var_handler_body_falls_through_to_barrier() {
    for src in [
        "try {error x} on error {} $body",
        "try {error x} trap NONE {} $body",
        "try {error x} on error {} [subst $body]",
        "try {error x} on error {} $body finally {puts bye}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            matches!(
                &m.top_level.statements[0],
                Statement::Barrier { reason, .. } if reason == "try with dynamic handler body"
            ),
            "expected a dynamic-handler-body barrier for {src:?}, got {:?}",
            m.top_level.statements[0],
        );
    }
}

#[test]
fn try_static_finally_and_handler_bodies_still_lower() {
    // FP-guard: the gate narrows the walk, it does not switch it off.
    for src in [
        "try {error x} finally {puts bye}",
        "try {error x} on error {} {puts bad}",
        "try {error x} trap NONE {m o} {puts $m} finally {puts bye}",
        // The `-` fallthrough marker is not a body and keeps its own path.
        "try {set x 1} on ok a - trap NONE b {return $b}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            matches!(&m.top_level.statements[0], Statement::Try { .. }),
            "expected a structured Try for {src:?}, got {:?}",
            m.top_level.statements[0],
        );
    }
}

#[test]
fn dict_for_dollar_var_body_falls_through_to_barrier() {
    let m = lower_to_ir("set body [gets stdin]\ndict for {k v} {a 1} $body", &reg());
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::Barrier { reason, .. } if reason == "dict for with dynamic body"
    ));
}

// A varList word is a Tcl list, not a whitespace split: a splitter that
// takes only the first word of a braced element, or cuts a
// backslash-escaped space in two, binds the wrong names for both spellings
// of a one-variable binding.

/// Loop-variable names of the single-iterator `foreach` at statement 0.
fn foreach_vars(src: &str) -> Vec<String> {
    let m = lower_to_ir(src, &reg());
    let Statement::Foreach { iterators, .. } = &m.top_level.statements[0] else {
        panic!("expected Foreach for {src:?}, got {:?}", m.top_level);
    };
    assert_eq!(iterators.len(), 1, "one iterator for {src:?}");
    iterators[0].vars.clone()
}

#[test]
fn foreach_grouped_var_element_binds_one_name() {
    // tclsh 9.0: `proc s {} { foreach {{x y}} {1} {}; info locals }`
    // reports the single local named `x y`.
    assert_eq!(foreach_vars("foreach {{x y}} {1} {puts hi}"), vec!["x y"]);
}

#[test]
fn foreach_escaped_space_var_binds_one_name() {
    // Same for the escaped spelling: `foreach {a\ b} {1} {}` binds
    // the one local `a b`, not the two names `a\` and `b`.
    assert_eq!(foreach_vars("foreach {a\\ b} {1} {puts hi}"), vec!["a b"]);
}

#[test]
fn foreach_plain_var_list_still_binds_every_name() {
    // FP-guard: the ordinary multi-variable list is unchanged.
    assert_eq!(
        foreach_vars("foreach {m n} {1 2} {puts hi}"),
        vec!["m", "n"]
    );
    assert_eq!(foreach_vars("foreach x {1 2} {puts hi}"), vec!["x"]);
}

#[test]
fn dict_for_grouped_var_element_binds_one_name() {
    assert_eq!(
        foreach_vars("dict for {{k 1} v} {a 1} {puts hi}"),
        vec!["k 1", "v"],
    );
}

#[test]
fn malformed_var_list_falls_through_to_barrier() {
    // An unbalanced varList names no binding, so each loop defers to its
    // runtime command, which raises Tcl's own list error.
    for (src, expected) in [
        (
            "foreach {a \"b} {1} {puts hi}",
            "foreach with malformed variable list",
        ),
        (
            "dict for {k \"v} {a 1} {puts hi}",
            "dict for with malformed variable list",
        ),
        (
            "try {puts hi} on error {m \"o} {puts bad}",
            "try with malformed handler variable list",
        ),
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            matches!(&m.top_level.statements[0], Statement::Barrier { reason, .. } if reason == expected),
            "expected {expected:?} barrier for {src:?}, got {:?}",
            m.top_level.statements[0],
        );
    }
}
