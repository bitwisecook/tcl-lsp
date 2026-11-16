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

//! Static `eval` and `uplevel`: level parsing, inlined bodies and the dynamic barriers that stop inlining.

use super::*;

#[test]
fn parse_uplevel_level_decimal() {
    let relative = |shift| {
        Some(UplevelLevel {
            shift,
            absolute: false,
        })
    };
    assert_eq!(parse_uplevel_level("1"), relative(1));
    assert_eq!(parse_uplevel_level("3"), relative(3));
    assert_eq!(parse_uplevel_level("0"), relative(0));
}

#[test]
fn parse_uplevel_level_hash_form() {
    let absolute = |shift| {
        Some(UplevelLevel {
            shift,
            absolute: true,
        })
    };
    assert_eq!(parse_uplevel_level("#0"), absolute(0));
    assert_eq!(parse_uplevel_level("#3"), absolute(3));
}

#[test]
fn parse_uplevel_level_zero_keeps_absolute_and_relative_apart() {
    // `uplevel #0` runs the body in the global frame; `uplevel 0` runs it
    // in the current frame. Both carry the magnitude 0, so only the
    // `absolute` flag tells them apart — conflating them miscompiled the
    // command-word namespace of every `uplevel 0` body.
    let hash_zero = parse_uplevel_level("#0").expect("#0 parses");
    let bare_zero = parse_uplevel_level("0").expect("0 parses");
    assert_eq!(hash_zero.shift, bare_zero.shift);
    assert!(hash_zero.absolute);
    assert!(!bare_zero.absolute);
    assert_ne!(hash_zero, bare_zero);
}

#[test]
fn parse_uplevel_level_dynamic_returns_none() {
    assert_eq!(parse_uplevel_level("$lvl"), None);
    assert_eq!(parse_uplevel_level("[expr {1+1}]"), None);
    assert_eq!(parse_uplevel_level("foo"), None);
}

#[test]
fn uplevel_static_body_no_level() {
    // A bare `uplevel {body}` means level 1, which raises `bad level "1"`
    // at the global level in every C release, so the form is lowered
    // where it runs: inside a procedure.
    let m = lower_to_ir("proc f {} {uplevel {set x 1}}", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert_eq!(proc.body.statements.len(), 1);
    match &proc.body.statements[0] {
        Statement::UpFrame {
            frame_shift, body, ..
        } => {
            assert_eq!(*frame_shift, 1);
            assert_eq!(body.statements.len(), 1);
        }
        other => panic!("expected UpFrame, got {other:?}"),
    }
}

#[test]
fn uplevel_static_body_with_level_one() {
    // `uplevel 1` raises `bad level "1"` at the global level.
    let m = lower_to_ir("proc f {} {uplevel 1 {set x 1}}", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    match &proc.body.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            ..
        } => {
            assert_eq!(*frame_shift, 1);
            assert!(!*absolute, "`uplevel 1` is the relative form");
        }
        other => panic!("expected UpFrame, got {other:?}"),
    }
}

#[test]
fn uplevel_static_body_with_hash_zero() {
    let m = lower_to_ir("uplevel #0 {set x 1}", &reg());
    match &m.top_level.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            ..
        } => {
            assert_eq!(*frame_shift, 0);
            assert!(*absolute, "`uplevel #0` is the absolute global form");
        }
        other => panic!("expected UpFrame for #0, got {other:?}"),
    }
}

#[test]
fn uplevel_static_body_with_bare_zero_is_relative() {
    // `uplevel 0 {body}` evaluates the body in the *current* frame, not
    // the global one. It shares the magnitude 0 with `#0` and is told
    // apart only by `absolute`.
    let m = lower_to_ir("uplevel 0 {set x 1}", &reg());
    match &m.top_level.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            ..
        } => {
            assert_eq!(*frame_shift, 0);
            assert!(!*absolute, "`uplevel 0` is the relative current-frame form");
        }
        other => panic!("expected UpFrame for bare 0, got {other:?}"),
    }
}

#[test]
fn uplevel_dynamic_body_falls_back_to_default() {
    // ``uplevel 1 $body`` body is a $var, not a brace literal —
    // can't be statically resolved without const-propagation.
    // Falls back to ``lower_default`` (Statement::Call).
    let m = lower_to_ir("uplevel 1 $body", &reg());
    assert!(matches!(
        m.top_level.statements[0],
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn uplevel_dynamic_level_falls_back_to_default() {
    // ``uplevel $lvl {body}`` — level is dynamic, can't pick a
    // ``frame_shift``. Falls back to default lowering.
    let m = lower_to_ir("uplevel $lvl {set x 1}", &reg());
    assert!(matches!(
        m.top_level.statements[0],
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

// barrier-gate

#[test]
fn body_has_dynamic_barrier_clean() {
    // No barriers at all.
    assert!(!body_has_dynamic_barrier(
        "set x 1",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // Barrier with a fully literal body.
    assert!(!body_has_dynamic_barrier(
        "eval { set x 1 }",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // Nested literal.
    assert!(!body_has_dynamic_barrier(
        "if { 1 } { eval { set x 1 } }",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_barrier_metadata_keeps_exact_selected_rooted_descriptor() {
    let mut registry = reg();
    assert!(body_has_dynamic_barrier(
        "::eval $body",
        &registry,
        tcl_lexer::LexerConfig::default()
    ));
    let mut rooted = registry.get_exact("list").unwrap().clone();
    rooted.name = "::eval";
    registry.insert(rooted);
    assert!(!body_has_dynamic_barrier(
        "::eval $body",
        &registry,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "eval $body",
        &registry,
        tcl_lexer::LexerConfig::default()
    ));
    for head in ["::::eval", ":eval", "eval::", "eval:::member"] {
        assert!(!body_has_dynamic_barrier(
            &format!("{head} $body"),
            &registry,
            tcl_lexer::LexerConfig::default()
        ));
    }
}

#[test]
fn body_has_dynamic_barrier_dynamic_eval_body() {
    // ``eval $x`` inside the outer body — dynamic.
    assert!(body_has_dynamic_barrier(
        "eval $x",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // Same shape nested.
    assert!(body_has_dynamic_barrier(
        "if { 1 } { eval $x }",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_has_dynamic_barrier_dynamic_uplevel_body() {
    assert!(body_has_dynamic_barrier(
        "uplevel 1 $body",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "uplevel #0 $b",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_has_dynamic_barrier_requires_proved_uplevel_level() {
    // A numeric value selects a frame, but "marker" is the first script
    // word on every pinned C release. Bracing the last word cannot decide
    // which operation an unknown first value requests.
    for source in ["uplevel 0 {set x 1}", "uplevel #0 {set x 1}"] {
        assert!(!body_has_dynamic_barrier(
            source,
            &reg(),
            tcl_lexer::LexerConfig::default()
        ));
    }
    assert!(body_has_dynamic_barrier(
        "uplevel $lvl {set x 1}",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_has_dynamic_barrier_qualified_eval_uplevel() {
    // ``::eval`` and ``::uplevel`` are also caught.
    assert!(body_has_dynamic_barrier(
        "::eval $x",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "::uplevel 1 $body",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

/// The gate resolves `uplevel`'s script word through the
/// registry's `ArgRole::Body` resolver, not a local level-word sniff, so
/// every documented `uplevel ?level? arg ?arg ...?` shape agrees with
/// `uplevel_body_arg_role_skips_optional_level` in `tcl-registry`.
#[test]
fn body_has_dynamic_barrier_uplevel_level_shapes_follow_the_registry() {
    let r = reg();
    // TN — a literal script in each level spelling relaxes.
    for clean in [
        "uplevel {set x 1}",
        "uplevel 1 {set x 1}",
        "uplevel #0 {set x 1}",
        "uplevel 0 {set x 1}",
    ] {
        assert!(
            !body_has_dynamic_barrier(clean, &r, tcl_lexer::LexerConfig::default()),
            "{clean} must not poison"
        );
    }
    // TP — a substituted script word in each level spelling poisons.
    for dirty in [
        "uplevel $body",
        "uplevel 1 $body",
        "uplevel #0 $body",
        "uplevel $lvl $body",
        "uplevel $lvl {set x 1}",
        "uplevel [expr {$n - 1}] {set x 1}",
        "uplevel 1 [gen]",
    ] {
        assert!(
            body_has_dynamic_barrier(dirty, &r, tcl_lexer::LexerConfig::default()),
            "{dirty} must poison"
        );
    }
    // FN guard — the words after the script word concatenate into it
    // (`SCRIPT_CONCATENATES_ARGS`), so a dynamic tail is still dynamic
    // even though the marked body word is a braced literal.
    assert!(body_has_dynamic_barrier(
        "uplevel 1 {set x} $tail",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "eval {set x} $tail",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // …and an all-literal multi-word tail stays clean.
    assert!(!body_has_dynamic_barrier(
        "uplevel 1 {set x} {1}",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // A bodyless `uplevel 1` is a wrong-#args error the registry exposes
    // no body word for — poison rather than guess.
    assert!(body_has_dynamic_barrier(
        "uplevel 1",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // A local level sniff that strips a leading `-` before the digit test
    // would accept `-N` as a level; the registry does not, and the
    // registry is right.  On tclsh 8.6.14, with `proc -1 {args} {…}`
    // defined, `uplevel -1 {set x 1}` *calls* `-1` with `set x 1` — the
    // word is the script's first word, not a level.  tclsh9.0.4 instead
    // errors `bad level "-1"`.  Under either reading the word is an
    // unbraced script word, so the gate poisons.
    assert!(body_has_dynamic_barrier(
        "uplevel -1 {set x 1}",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

/// The eval-family *subcommand*
/// members carry `EVALUATES_CODE` / `SCRIPT_CONCATENATES_ARGS` on the
/// subcommand, not on the `namespace` / `interp` spec.  Composing them
/// (`invocation_traits`) plus registry-resolved body indices makes them
/// flow through the same path a bare `eval` does, which a parent-only
/// trait test missed entirely.
#[test]
fn body_has_dynamic_barrier_covers_compound_eval_family() {
    let r = reg();
    // TP — the script is substituted.
    assert!(body_has_dynamic_barrier(
        "namespace eval ns $body",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "namespace inscope ns $body",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "interp eval slave $body",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    // TN — a braced literal script relaxes, and its own contents are
    // still recursed.
    assert!(!body_has_dynamic_barrier(
        "namespace eval ns {set x 1}",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "namespace eval ns {eval $x}",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    // TN — a `namespace` subcommand that evaluates nothing is not a
    // barrier, however dynamic its arguments are.
    assert!(!body_has_dynamic_barrier(
        "namespace delete $ns",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_has_dynamic_barrier(
        "namespace current",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
}

/// TN for the barrier test itself: an ordinary command with a
/// substituted argument is not a script evaluator, so it never poisons —
/// the gate keys on the registry trait, not on "has a `$var` argument".
#[test]
fn body_has_dynamic_barrier_non_body_command_is_clean() {
    let r = reg();
    assert!(!body_has_dynamic_barrier(
        "set x $y",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_has_dynamic_barrier(
        "puts [format %s $x]",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_has_dynamic_barrier(
        "lappend acc $item",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    // A user command the registry does not know at all is not a barrier.
    assert!(!body_has_dynamic_barrier(
        "mycmd $script",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn try_lower_eval_static_rejects_nested_dynamic_barrier() {
    // The relaxer would normally promote ``eval { ... }`` to a
    // ``Statement::Block``; with the barrier gate, the nested
    // ``eval $x`` poisons the relaxation and we fall back to
    // ``Statement::Barrier``.
    let m = lower_to_ir(r"eval { eval $x }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert_runtime_opaque_call(stmt, &reg(), "eval");
}

#[test]
fn try_lower_eval_static_clean_body_relaxes() {
    // No nested barrier — relaxes to Block.
    let m = lower_to_ir("eval { set x 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    let Statement::Block { error_context, .. } = stmt else {
        panic!("expected Block (relaxed), got {stmt:?}");
    };
    assert_eq!(
        *error_context,
        Some(tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation),
    );
}
