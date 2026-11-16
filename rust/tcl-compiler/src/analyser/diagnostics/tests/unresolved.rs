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

//! W123 unknown commands and W120 missing `package require`, across renames, aliases, deletions and `expr` functions.

use super::*;

/// A `W123` whose call the flow graph widens — it marks the call as one to
/// code the module cannot see — ends with the sentence that says so and names
/// the stub that keeps the variables: at the top level, in a `namespace eval`
/// body, in a procedure, a method and a lambda. A report whose call the graph
/// does not widen where it is written, as in an `uplevel #0` body, ends as it
/// was. The top-level `$g == 5` after `db_query x` draws no `I230`: the
/// sentence stands where the widening does.
#[test]
fn w123_says_the_call_widens_where_it_does() {
    let src = "proc p {} {\n    set g 5\n    db_query {select 1}\n    strng length abc\n    \
               namespace eval ns { db_ns_call }\n    uplevel #0 { db_up_call }\n    \
               apply {{} { db_lambda_call }}\n    puts $g\n}\n\
               oo::class create C { method m {} { db_method_call } }\n\
               set g 5\ndb_query x\nif {$g == 5} {puts five} else {puts other}\n";
    let mut a = crate::analyser::Analyser::new();
    let diagnostics = a.analyse(src, "tcl8.6").diagnostics;
    assert!(
        !diagnostics.iter().any(|d| d.code == DiagCode::I230),
        "the top-level call widens `g`: {diagnostics:?}"
    );
    let unknown: Vec<(u32, String)> = diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .map(|d| (d.span.start(), d.message.clone()))
        .collect();
    let hint = |name: &str| {
        format!(
            "The call widens the variables held at it, in a procedure's own frame its locals; a \
             `# tcl-lsp: stub {name} {{…}} -frame own` (or `-frame none`) declaration keeps them \
             when every argument is a value, name, pattern or channel and no flag but `-pure` or \
             `-unsafe` is set."
        )
    };
    let message_at = |needle: &str, nth: usize| {
        let offset = src.match_indices(needle).nth(nth).expect("the head").0;
        unknown
            .iter()
            .find(|(start, _)| *start as usize == offset)
            .map(|(_, message)| message.clone())
            .expect("a W123")
    };
    for (head, nth) in [
        ("db_query", 0),
        ("db_ns_call", 0),
        ("db_lambda_call", 0),
        ("db_method_call", 0),
        ("db_query", 1),
    ] {
        assert_eq!(
            message_at(head, nth),
            format!("Unknown command '{head}'. {}", hint(head)),
            "{head} #{nth}"
        );
    }
    assert_eq!(
        message_at("strng", 0),
        format!(
            "Unknown command 'strng'; did you mean 'string'? {}",
            hint("strng")
        )
    );
    assert_eq!(
        message_at("db_up_call", 0),
        "Unknown command 'db_up_call'",
        "an `uplevel #0` body is not lowered where it is written"
    );
}

#[test]
fn analyse_w123_suppressed_for_partial_interpolation_resolving_to_known_proc() {
    // ``set suffix _hi`` makes ``$suffix`` resolve to ``_hi``;
    // ``foo$suffix`` therefore resolves to ``foo_hi``, which
    // is a known proc.  W123 should not fire.
    let src = "\
proc foo_hi {} {}
set suffix _hi
foo$suffix
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 should be suppressed when partial interpolation resolves to a known proc; got {:?}",
        r.diagnostics,
    );
}

// Implementation contract: naming.compiler.original-slot-presence
// docs/design/analysis/name-resolution-proofs/original-slot-presence.md
// An interpolated head folding to a deleted procedure retains the positioned
// absence diagnosis. A reporting name cannot supply a live command candidate.

#[test]
fn w123_tp_issue_1010_interpolated_head_folds_to_deleted_proc_stays_flagged() {
    let src = "proc foo_hi {} {}\nrename foo_hi {}\nset suffix _hi\nfoo$suffix\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an interpolated head folding to a deleted proc must still draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_issue_1010_interpolated_head_folds_to_reestablished_proc_resolves() {
    let src =
        "proc foo_hi {} {}\nrename foo_hi {}\nproc foo_hi {} {}\nset suffix _hi\nfoo$suffix\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a proc re-established after deletion must still resolve; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn extra_commands_suppress_w123() {
    // `tclLsp.extraCommands` names are treated as known commands.
    let src = "mylibsend foo bar\n";
    // Baseline: an unknown bare command fires W123.
    let mut base = Analyser::new();
    let baseline = base.analyse(src, "tcl8.6");
    assert!(
        baseline
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123),
        "baseline W123 expected for unknown command; got {:?}",
        baseline.diagnostics,
    );
    // With the command declared extra, W123 is suppressed.
    let mut a = Analyser::new().with_extra_commands(["mylibsend".to_owned()].into_iter().collect());
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "extraCommands should suppress W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn namespace_ensemble_explicit_command_option_suppresses_w123() {
    // `namespace ensemble create -command NAME` dispatches under an
    // explicit, differently-named command — not the enclosing namespace's
    // own name (the implicit form). A call through it must not draw a
    // spurious "unknown command" (baseline: it does, before the name is
    // recorded).
    let src = "\
namespace eval ::ns {
    namespace export foo
    proc foo {a b} { return ok }
    namespace ensemble create -command ::ens -map {f foo}
}
::ens f 1 2
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an explicit -command ensemble name must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_kept_when_partial_interpolation_resolves_to_unknown() {
    // ``set suffix _missing`` makes ``foo$suffix`` resolve
    // to ``foo_missing`` — not a known command — so W123
    // should still fire.
    let src = "\
set suffix _missing
foo$suffix
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 expected when partial interpolation resolves to an unknown command",
    );
}

// W123 must not fire on a built-in `expr` math function (`sin(...)`,
// `max(...)`, ...): `record_expr_function_invocations` resolves a
// math-function call to `::tcl::mathfunc::<name>`, and the W123 pass
// recognises that qualified name from the built-in function table as well
// as from a *user-defined* `proc ::tcl::mathfunc::<name>`
// (`proc_tail_names`).

#[test]
fn w123_tp_unknown_function_name_inside_expr_still_fires() {
    // FP fix must not swallow a genuinely unknown function — `frobnicate` is
    // not a real `::tcl::mathfunc` name in any Tcl release.
    let src = "set x [expr {frobnicate(1)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an unknown expr function must still draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_math_function_before_its_release_dual_fires_with_w002() {
    // `min`/`max` are TIP 232 (8.5+); under 8.4 the name is not a real
    // `::tcl::mathfunc` command in *this* dialect — the same "known
    // elsewhere, disabled here" split the registry-builtin path draws for
    // e.g. `dict` under `tcl8.4` (both W002 *and* W123 fire — see
    // `build_w123_known_names`'s profile-filter comment; verified live
    // against `exec` under `f5-irules`).
    let src = "set x [expr {min(1, 2)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W002),
        "min() under tcl8.4 must draw W002 (disabled in dialect); got {:?}",
        r.diagnostics,
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "min() under tcl8.4 must also draw W123 (unresolved in this dialect); got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_expr_function_after_rename_away_is_unresolved() {
    // `rename ::tcl::mathfunc::sin {}` really does break `expr {sin(...)}}`
    // in C Tcl (`invalid command name "tcl::mathfunc::sin"` — see the WASM
    // runtime's `expr_routes_through_the_command_table` test) — a call after
    // the deletion must not silently resolve.
    let src = "rename ::tcl::mathfunc::sin {}\nset x [expr {sin(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a call through a renamed-away mathfunc must be flagged unresolved; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_bareword_call_to_a_mathfunc_shaped_name_is_still_unresolved() {
    // `sin` is only ever callable via `::tcl::mathfunc::sin` or the `expr`
    // function-call grammar — a bare top-level command call to `sin` (no
    // user `proc sin`, not inside `expr`) is `invalid command name` in real
    // Tcl, and the fix must not leak resolution into ordinary bareword
    // command dispatch.
    let src = "sin 1.0\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a bareword call to a mathfunc-shaped name must still be unresolved; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_original_issue_968_repro_is_silent() {
    // Built-in `expr` math functions must resolve with no diagnostic at all.
    let src = "set x [expr {sin(1.0) + max(1, 2, 3)}]\nputs $x\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "issue #968: built-in expr math functions must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_nested_function_calls_both_resolve() {
    // `sqrt(abs($x))` records both the outer and the inner function as
    // separate mathfunc invocations (`expr_nested_function_calls_are_both_recorded`
    // in commands.rs) — both must resolve.
    let src = "set x [expr {sqrt(abs($y))}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "nested built-in expr math functions must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_version_gated_function_resolves_at_its_introducing_release() {
    // `isnan` is TIP 521 (9.0+) — available (and silent) exactly at tcl9.0,
    // unlike the tcl8.4 case above.
    let src = "set x [expr {isnan(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W002 | DiagCode::W123)),
        "isnan() under tcl9.0 must draw neither W002 nor W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_expr_function_resolves_inside_if_condition() {
    // The `expr`-function resolution path is driven by the registry's
    // `ArgRole::Expr` role, so it fires for *any* EXPR-role argument —
    // `if`'s condition, not only a literal `expr` command.
    let src = "if {sin($y) > 0} {\n    puts hi\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a math function inside an `if` condition must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_expr_function_resolves_inside_namespace_and_proc_body() {
    // Interprocedural context (a namespace-scoped proc body) must not
    // change resolution — the function dispatches through the global
    // `::tcl::mathfunc` namespace regardless of the caller's own scope.
    let src = "\
namespace eval ::acme {
    proc calc {y} {
        return [expr {sin($y) + cos($y)}]
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a math function inside a namespaced proc body must not draw W123; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tn_user_defined_mathfunc_override_still_resolves() {
    // Pre-existing behaviour (`expr_function_call_records_a_mathfunc_invocation`
    // in commands.rs), unaffected by the built-in-name fix: a user override
    // resolves via `proc_tail_names` exactly as before.
    let src = "proc ::tcl::mathfunc::myfunc {x} { return $x }\nset r [expr {myfunc(1)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a user-defined mathfunc override must resolve; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tn_unrelated_proc_sharing_a_mathfunc_name_is_unaffected() {
    // `proc abs {x} {...}` is an ordinary, unrelated command — mathfunc
    // resolution is scoped to `expr` function-call invocations only (via
    // the settled `::tcl::mathfunc::<name>` qualified name), so a same-named
    // regular proc and its ordinary bareword call sites resolve through the
    // normal proc-tail path.
    let src = "proc abs {x} {\n    if {$x < 0} { return [expr {-$x}] }\n    return $x\n}\nputs [abs -5]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an unrelated proc sharing a mathfunc name must resolve normally; got {:?}",
        r.diagnostics,
    );
}

// The `expr`-function shortcut above is keyed on `is_mathfunc_call`, not
// merely on the settled qualified name matching `::tcl::mathfunc::<name>` —
// an *ordinary* call can resolve to that same shape by being made from
// inside the real `::tcl::mathfunc` namespace, and TIP 232's command
// wrappers (the mechanism that makes such a bareword call valid at all)
// only exist from Tcl 8.5 onward, independent of any individual function's
// own, earlier expr-grammar availability.

#[test]
fn w123_tp_ordinary_call_shaped_like_mathfunc_fires_under_84() {
    // `sin` is a valid *expr* function since 8.4, but the `::tcl::mathfunc`
    // *command* namespace TIP 232 introduced does not exist until 8.5 — an
    // ordinary bareword call to `sin` from inside that namespace is not a
    // real command under an 8.4-based dialect.
    let src = "namespace eval ::tcl::mathfunc {\n    sin 1\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "an ordinary call shaped like a mathfunc dispatch must still draw \
         W123 under an 8.4-based dialect; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_ordinary_call_shaped_like_mathfunc_resolves_under_86() {
    // The same call is genuinely valid Tcl from 8.5 onward: `::tcl::mathfunc::sin`
    // really is a callable command there, so the bareword call must resolve.
    let src = "namespace eval ::tcl::mathfunc {\n    sin 1\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the command wrapper genuinely exists under tcl8.6; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_custom_mathfunc_body_bareword_call_fires_under_84() {
    // The more realistic trigger: a custom math function's own body calling
    // a built-in bareword (not through `expr`'s function-call syntax) —
    // still invalid under an 8.4-based dialect for the same reason. 8.4 has
    // no `::tcl::mathfunc` namespace until the script makes one (tclsh8.4:
    // `can't create procedure ...: unknown namespace`).
    let src = "namespace eval ::tcl::mathfunc {}\n\
               proc ::tcl::mathfunc::myfunc {x} { return [sin $x] }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a bareword call to a builtin from inside a custom mathfunc body \
         must still draw W123 under an 8.4-based dialect; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_fp_expr_function_call_unaffected_by_wrapper_availability_under_84() {
    // The wrapper-availability gate must apply only to ordinary calls, never
    // to a genuine `expr` function-call site, under an 8.4-based dialect
    // as much as any other.
    let src = "set x [expr {sin(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.4");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "expr {{sin(1.0)}} is valid under tcl8.4 regardless of TIP 232's \
         command-wrapper availability; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_static_rename_does_not_set_has_dynamic_providers() {
    // A fully static `rename OLD NEW` is now recorded precisely
    // (`renamed_commands`) instead of blanket-poisoning
    // `has_dynamic_providers` for the whole document.
    let mut a = Analyser::new();
    let r = a.analyse("rename set myset\n", "tcl");
    assert!(
        !r.has_dynamic_providers,
        "a fully static rename must not set has_dynamic_providers"
    );
    assert_eq!(
        r.renamed_commands.get("::myset").map(String::as_str),
        Some("::set")
    );
}

#[test]
fn analyse_dynamic_but_resolvable_rename_does_not_set_has_dynamic_providers() {
    // `$x` is dynamic-*looking* but `x` is a known constant (`set x
    // set`) — constant folding resolves this exactly like the fully-static
    // `analyse_static_rename_does_not_set_has_dynamic_providers` case,
    // instead of falling back to the conservative
    // `has_dynamic_providers` flag.
    let mut a = Analyser::new();
    let r = a.analyse("set x set\nrename $x myset\n", "tcl");
    assert!(
        !r.has_dynamic_providers,
        "x's value is a known constant, so this rename must resolve, not widen"
    );
    assert_eq!(
        r.renamed_commands.get("::myset").map(String::as_str),
        Some("::set")
    );
}

#[test]
fn analyse_genuinely_dynamic_rename_still_sets_has_dynamic_providers() {
    // Unlike the sibling test above, `x`'s value here is never a
    // tracked compile-time constant (piped through `gets`) — the
    // document still falls back to the conservative
    // `has_dynamic_providers` flag, matching `command_binding.rs`'s
    // wildcard-collapse convention for the identical shape.
    let mut a = Analyser::new();
    let r = a.analyse("set x [gets stdin]\nrename $x myset\n", "tcl");
    assert!(
        r.has_dynamic_providers,
        "a genuinely dynamic rename must still set has_dynamic_providers"
    );
    assert_eq!(r.renamed_commands.len(), 0);
}

#[test]
fn analyse_w123_emits_did_you_mean_suggestion() {
    // ``puta`` is one edit away from ``puts`` — the
    // emitter should attach a suggestion and a CodeFix.
    let mut a = Analyser::new();
    let r = a.analyse("puta hi", "tcl8.6");
    let w123 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W123)
        .expect("W123 emitted");
    assert!(
        w123.message.contains("did you mean 'puts'"),
        "expected suggestion in message, got: {}",
        w123.message,
    );
    assert!(!w123.fixes.is_empty(), "expected CodeFix payload");
    let fix = &w123.fixes[0];
    assert_eq!(fix.new_text, "puts");
    assert!(fix.description.contains("puts"));
}

#[test]
fn analyse_w123_suppressed_for_inline_stub_declared_command() {
    // ``my_cmd`` is declared via inline stub — W123 must
    // not fire even though it isn't in the registry.
    //
    // Its declared ``body`` word *is* analysed as a script, exactly as a
    // registry body command's is (``while 0 foo`` reports the same thing), so
    // the bare ``foo`` inside it draws its own W123. That is the declaration
    // working, not leaking: the assertion is about the stubbed head.
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body}
# tcl-lsp: stubs-end
my_cmd $x foo
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("my_cmd")),
        "W123 must not fire for stub-declared commands; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_dispatch_target_from_unknown_proc_suppresses() {
    // ``foo`` is one of the switch arms inside a
    // user-defined ``unknown`` proc — the empty-stub gate
    // doesn't fire (body is non-empty), so W123 is
    // already suppressed.  Add a fixture that verifies
    // the dispatch_targets are also in the suggestion
    // candidate set when an empty-stub unknown is in play.
    let src = "\
proc unknown {cmd args} {}
foo
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    // tclsh 8.6: the script's own `unknown` receives `foo` and returns
    // empty, so the call is no unresolved command.
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_no_suggestion_when_far_from_any_known_command() {
    let mut a = Analyser::new();
    let r = a.analyse("xyzzy_unknown_cmd", "tcl");
    let w123 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W123)
        .expect("W123 emitted");
    assert!(
        !w123.message.contains("did you mean"),
        "no suggestion expected for far-away command name; got: {}",
        w123.message,
    );
    assert_eq!(w123.fixes, [] as [crate::irules_checks::CodeFix; 0]);
}

#[test]
fn analyse_w123_unknown_command() {
    // ``no_such_cmd hello`` — bare name that's not a
    // built-in / proc / class / alias.  W123 fires.
    let mut a = Analyser::new();
    let r = a.analyse("no_such_cmd hello", "tcl");
    let w123s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .collect();
    assert!(
        !w123s.is_empty(),
        "W123 expected for unknown command; got {:?}",
        r.diagnostics,
    );
    assert!(w123s[0].message.contains("'no_such_cmd'"));
    assert_eq!(w123s[0].severity, Severity::Hint);
}

#[test]
fn analyse_no_w123_for_builtin_command() {
    // ``puts hello`` — ``puts`` is a built-in; no W123.
    let mut a = Analyser::new();
    let r = a.analyse("puts hello", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 must not fire on built-in command; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_no_w123_for_user_proc() {
    // User-defined proc, then call it.  Both go through
    // the analyser walk; the call site must NOT trip W123.
    let mut a = Analyser::new();
    let r = a.analyse("proc greet {} { puts hi }\ngreet", "tcl");
    let w123s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .collect();
    assert_eq!(
        w123s.len(),
        0,
        "W123 must not fire on user-defined proc call; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_no_w123_for_qualified_command_name() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ns {proc cmd {arg} {return $arg}}\nns::cmd hello",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the installed qualified command must resolve; got {:?}",
        r.diagnostics,
    );
    let unknown_source = "namespace eval ns {}\nns::missing hello";
    let unknown = a.analyse(unknown_source, "tcl8.6");
    let offset = u32::try_from(unknown_source.find("ns::missing").unwrap()).unwrap();
    let binding = a
        .head_identities
        .invocation_at_source("ns::missing", offset);
    assert!(
        unknown.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "qualification alone does not establish an installed command: {:?}; point={:?}, namespaces={:?}, slot={:?}, advice={:?}, target_unknown={}, invocations={:?}, regions={:?}, unresolved_emitted={}",
        unknown.diagnostics,
        a.head_identities.diagnostic_slot_presence_at(offset),
        binding.existing_namespace_cells,
        binding.selected_slot_presence(),
        binding.selected_slot_diagnostic_presence(),
        binding.unknown,
        unknown
            .command_invocations
            .iter()
            .map(|invocation| (
                &invocation.name,
                invocation.range,
                invocation.lookup,
                invocation.existence_probe
            ))
            .collect::<Vec<_>>(),
        unknown.scoped_command_regions,
        a.unresolved_commands_emitted,
    );
    let external = a.analyse("external::operation hello", "tcl8.6");
    assert!(
        !external
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123),
        "an unknown external namespace supplies no closed absence proof: {:?}",
        external.diagnostics,
    );
}

#[test]
fn analyse_w123_package_require_gate_suppresses_when_recorded() {
    // The ``package_requires`` gate suppresses W123 entirely
    // when any package require has been recorded.  The
    // analyser walk doesn't yet record ``package require``
    // (the handler is not implemented), so we exercise the
    // gate by pre-populating ``result.package_requires``
    // and re-running the post-pass directly.
    use crate::signature_scan::types::SignaturePackageRequire;
    use tcl_lexer::Span;
    let mut a = Analyser::new();
    a.result.package_requires.push(SignaturePackageRequire {
        original_name: None,
        original_requirements: vec![None],
        name: "Tcl".to_string(),
        version: Some("8.6".to_string()),
        requirements: vec!["8.6".to_string()],
        exact: false,
        range: Span::new(0, 24),
        conditional: false,
        control_flow: false,
        origin: crate::signature_scan::types::PackageRequireOrigin::Source,
    });
    // Seed an invocation that would otherwise trip W123.
    a.result
        .command_invocations
        .push(crate::signature_scan::types::SignatureCommandInvocation {
            original_callback_signature_lookup: None,
            original_callback_prefix: None,
            original_lookup: None,
            original_name_input: None,
            lookup: crate::signature_scan::types::SignatureCommandLookup::InvocationHead,
            name: "random_cmd".to_string(),
            range: Span::new(25, 35),
            resolved_qualified_name: None,
            resolved_user_definition: false,
            resolved_definition: None,
            resolved_command_reference: None,
            resolution_candidates: Vec::new(),
            argc: Some(0),
            callback_arity: None,
            callback_baked_args: 0,
            indirect: false,
            rename_safe: true,
            existence_probe: false,
            is_mathfunc_call: false,
            ensemble_dispatch: None,
        });
    let registry = tcl_registry::CommandRegistry::build_default();
    a.emit_unresolved_command_diagnostics(&registry);
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123),
        "W123 must be fully suppressed when package_requires is non-empty; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn analyse_w123_filtered_by_disabled_diagnostics() {
    // ``# tcl-lsp: disable=W123`` at top of file silences
    // the diagnostic via the existing disable filter.
    let mut a = Analyser::new();
    let r = a.analyse("# tcl-lsp: disable=W123\nno_such_cmd hello", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "W123 must be silenced by file-suppression directive; got {:?}",
        r.diagnostics,
    );
}

// W120: missing package require

#[test]
fn w120_fires_for_package_gated_command_without_require() {
    // `tcl::idna` carries `required_package = "tcl::idna"`.
    // Using it without a `package require` emits W120.
    let mut a = Analyser::new();
    let r = a.analyse("tcl::idna decode example.com\n", "tcl9.0");
    let w120: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W120)
        .collect();
    assert_eq!(w120.len(), 1, "expected one W120; got {:?}", r.diagnostics);
    assert!(w120[0].message.contains("package require tcl::idna"));
    // Carries a fix that inserts the require at the top.
    assert_eq!(w120[0].fixes.len(), 1);
    assert_eq!(w120[0].fixes[0].new_text, "package require tcl::idna\n");
    assert!(
        w120[0].fixes[0]
            .description
            .contains("Add 'package require")
    );
}

#[test]
fn w120_suppressed_when_package_required() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "package require tcl::idna\ntcl::idna decode example.com\n",
        "tcl9.0",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W120),
        "W120 must not fire when the package is required; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w120_fix_inserts_after_existing_require() {
    // With an unrelated `package require` present, the fix
    // inserts on the line after it.
    let src = "package require Tcl 8.6\ntcl::idna decode x\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let w120 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W120)
        .expect("W120 expected");
    let fix = &w120.fixes[0];
    // Insertion offset is past the first line's newline
    // (byte 23 = start of line 1).
    let off = fix.span.start() as usize;
    assert_eq!(&src[..off], "package require Tcl 8.6\n");
}

// NB: the workspace-level behaviour — suppressing W120 when a required
// package (transitively) provides the gated package — is the LSP server's
// `package_resolver`-backed post-filter, tested in `tcl-lsp-server`. The
// analyser's single-file W120 here intentionally fires whenever the gated
// package is not required/provided *in this document*.

#[test]
fn w120_emitted_once_per_command_name() {
    let src = "tcl::idna decode a\ntcl::idna encode b\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let w120: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W120)
        .collect();
    assert_eq!(w120.len(), 1, "expected one W120 per name; got {w120:?}");
}

/// **The W120 placement ruling.** `Tk` is a package with a placement, and W120's
/// existing suppression rule is "the package is ambient here" — so the
/// ruling falls out of the placement rather than being written into the
/// diagnostic: under the `tk` environment (a `wish` script, whose
/// interpreter has already loaded Tk before the first byte runs) there is
/// no `package require Tk` to write and W120 must stay silent, while under
/// every plain-Tcl environment Tk is *hosted* and the nag is right.
#[test]
fn w120_is_silent_under_the_tk_environment_and_nags_under_plain_tcl() {
    let src = "button .b -text hi\npack .b\n";
    let fires = |dialect: &str| {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W120 && d.message.contains("package require Tk"))
            .count()
    };
    // Hosted: `tclsh` needs the require, so the nag is right — once per
    // gated command name (`button`, `pack`), as W120 always emits.
    assert_eq!(fires("tcl8.6"), 2, "tcl8.6");
    assert_eq!(fires("tcl9.0"), 2, "tcl9.0");
    assert_eq!(fires("tcl"), 2, "the lenient sink hosts Tk too");
    // Ambient: `wish` ships Tk. Nothing to require, nothing to nag about.
    assert_eq!(fires("tk"), 0, "tk");
    assert_eq!(
        fires("wish"),
        0,
        "the `wish` alias resolves to the same row"
    );
    // …and the nag stops on a plain-Tcl document that does require it.
    let mut a = Analyser::new();
    let required = a.analyse(&format!("package require Tk\n{src}"), "tcl8.6");
    assert!(
        !required
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W120),
        "{:?}",
        required.diagnostics
    );
}

/// The other half of the same placement: the Tk geometry/widget checks
/// (`TK100x`) activate without a `package require` under the `tk`
/// environment and only *with* one under plain Tcl. The gate is
/// `ResolvedContext::ambient_package("Tk")` rather than the environment's
/// *name* (`is_tk()`), so the two facts W120 and the TK checks read are the
/// same fact.
#[test]
fn tk_checks_activate_on_the_ambient_placement_not_the_environment_name() {
    let src = "frame .top\npack .top.a\ngrid .top.b\n";
    let tk_codes = |dialect: &str, source: &str| {
        let mut a = Analyser::new();
        a.analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string().starts_with("TK"))
            .count()
    };
    assert!(tk_codes("tk", src) > 0, "wish: ambient Tk activates");
    assert_eq!(tk_codes("tcl8.6", src), 0, "plain Tcl without a require");
    assert!(
        tk_codes("tcl8.6", &format!("package require Tk\n{src}")) > 0,
        "plain Tcl with a require"
    );
}

#[test]
fn w120_disabled_via_directive() {
    let mut a = Analyser::new();
    let r = a.analyse("# tcl-lsp: disable=W120\ntcl::idna decode x\n", "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W120),
        "{:?}",
        r.diagnostics
    );
}

// A proc / class / rename / alias target that is renamed or deleted away,
// with no later re-establishment under the same name, must not still read
// as "known" for W123: calling it fails "invalid command name" in real Tcl.
// `finalise_invocation_resolutions`'s `known` predicate (scope.rs) and
// `build_w123_known_names` (unresolved.rs — the pass that actually decides
// W123) gate deletion for user procs and classes as well as for registry
// builtins / aliases / rename targets.
//
// They ask the same "was this fact re-established after its last deletion"
// question the arity resolver's `fact_superseded_by_deletion` answers for
// E002/E003 (see
// `same_file_rename_reestablished_after_deletion_checks_new_arity` above),
// through `fact_live_for_call` in unresolved.rs — with the same call-site +
// conditional-body awareness
// `qualified_name_deleted_before` gives registry builtins: a
// deletion recorded inside a proc/class/method body is conditional (it
// executes only if that body is ever invoked) and a top-level call
// textually before a later deletion still resolves. All cases below
// confirmed against tclsh 8.6.14.
fn w123_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W123)
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn w123_tp_rename_of_a_nonexistent_command_is_flagged() {
    // TP. `rename OLD NEW` requires `OLD` to exist:
    // tclsh 9.0.4 and 8.6.16 both abort with `can't rename
    // "definitelyNotDefinedAnywhere": command doesn't exist` (exit 1). `OLD`
    // is recorded as an ordinary command reference, so W123 reports the
    // guaranteed failure at the token that causes it.
    let src = "rename definitelyNotDefinedAnywhere someAlias\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_deleting_rename_of_a_nonexistent_command_is_flagged() {
    // TP — the delete form of the same finding: tclsh 9.0.4 and 8.6.16 both
    // abort with `can't delete "totallyBogusCommand": command doesn't exist`.
    let src = "rename totallyBogusCommand {}\nputs done\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tn_rename_of_an_existing_proc_is_silent() {
    // TN — the paired guard: the source exists, so the rename is legal
    // (tclsh: `hello` then returns `hi`) and neither operand is flagged.
    let src = "proc greet {} { return hi }\nrename greet hello\nhello\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tn_rename_of_a_builtin_is_silent() {
    // TN — renaming a registry builtin away is the standard wrap idiom and
    // must not report the builtin as unknown.
    let src = "rename puts ::original_puts\nproc puts {args} { }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_proc_call_before_rename_resolves() {
    let src = "proc helper {} { return 1 }\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_proc_deleted_via_rename_no_reestablishment() {
    let src = "\
namespace eval ::a {
    proc helper {} { return 1 }
}
rename ::a::helper {}
namespace eval ::a {
    proc caller {} { helper }
}
";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_proc_reestablished_after_deletion_resolves() {
    let src = "proc helper {} { return 1 }\nrename helper {}\nproc helper {} { return 2 }\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_class_deleted_via_rename_no_reestablishment() {
    // tclsh 8.6.18, 9.0.4 and 9.1.0: after `oo::class create Helper; rename
    // Helper {}`, `Helper new` raises `invalid command name "Helper"` (and
    // the class's subclasses and instances are gone with it).
    let src = "oo::class create Helper\nrename Helper {}\nproc caller {} { Helper new }\n";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_class_call_before_rename_resolves() {
    let src = "oo::class create Helper\nproc caller {} { Helper new }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_class_reestablished_after_deletion_resolves() {
    let src = "oo::class create Helper\nrename Helper {}\noo::class create Helper\nproc caller {} { Helper new }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_interp_alias_deleted_no_reestablishment() {
    // tclsh 8.4.20 to 9.1.0: after `interp alias {} short {} target; interp
    // alias {} short {}`, `short` raises `invalid command name "short"` and
    // `target` is still a command.
    let src = "proc target {} {}\ninterp alias {} short {} target\ninterp alias {} short {}\nproc caller {} { short }\n";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_interp_alias_call_before_deletion_resolves() {
    let src = "proc target {} {}\ninterp alias {} short {} target\nproc caller {} { short }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_interp_alias_reestablished_after_deletion_resolves() {
    let src = "\
proc target {} {}
proc target2 {} {}
interp alias {} short {} target
interp alias {} short {}
interp alias {} short {} target2
proc caller {} { short }
";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_973_rename_to_new_name_call_to_old_name_stays_unknown() {
    // A rename that *moves* the name (not a deletion) leaves the old name
    // permanently gone — must stay unknown.
    let src = "proc helper {} { return 1 }\nrename helper newhelper\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), vec!["W123".to_owned()]);
}

#[test]
fn w123_fp_issue_973_top_level_call_before_later_proc_deletion_resolves() {
    // A top-level call textually before a *later* same-file deletion still
    // resolves — order matters at top level (tclsh: `proc helper {} {};
    // helper; rename helper {}` succeeds).
    let src = "proc helper {} { return 1 }\nhelper\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_973_deletion_inside_never_triggered_proc_body_resolves() {
    // A `rename` recorded inside a proc body that's never called is
    // conditional — it never executes, so the name stays live (tclsh:
    // calling `caller` without ever calling `maybeDelete` succeeds).
    let src = "proc helper {} { return 1 }\nproc maybeDelete {} { rename helper {} }\nproc caller {} { helper }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

// The W123 alias / rename-target checks are call-site- and
// conditional-body-aware (`fact_live_for_call`), exactly as the proc/class
// checks are: resolution consumes `alias_defs_by_tail` /
// `rename_defs_by_tail` (mirroring `proc_defs_by_tail` /
// `class_defs_by_tail`) checked per call site, not file-end-only tail
// `HashSet`s of names. All cases confirmed against tclsh 8.6.14.

#[test]
fn w123_fp_issue_1006_alias_call_before_later_deletion_resolves() {
    let src = "proc target {} { return 1 }\ninterp alias {} short {} target\nshort\ninterp alias {} short {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1006_alias_deletion_inside_never_triggered_body_resolves() {
    let src = "proc target {} { return 1 }\ninterp alias {} short {} target\nproc maybeDelete {} { interp alias {} short {} }\nproc caller {} { short }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1006_rename_target_call_before_later_deletion_resolves() {
    let src = "proc helper {} { return 1 }\nrename helper ha2\nha2\nrename ha2 {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1006_rename_target_deletion_inside_never_triggered_body_resolves() {
    let src = "proc helper {} { return 1 }\nrename helper ha2\nproc maybeDelete {} { rename ha2 {} }\nproc caller {} { ha2 }\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

// `fact_live_for_call`'s body-call escape hatch: a call *inside* a
// proc/class body carries no execution-order meaning from its own textual
// position.  Treating it as automatically after every top-level deletion
// draws a spurious W123 on a body call whose enclosing definition
// demonstrably ran before that deletion. Confirmed against tclsh 8.6.14
// throughout.

#[test]
fn w123_fp_issue_1009_codex_review_body_call_before_later_deletion_resolves() {
    // FP guard: `caller`'s own top-level
    // invocation runs before `rename helper {}`, so the `helper` call
    // inside its body must still resolve — confirmed against tclsh 8.6.14
    // (the script prints "ok" and exits 0).
    let src = "proc helper {} {}\nproc caller {} { helper }\ncaller\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_1009_codex_review_deleted_before_definition_still_flags() {
    // TP regression: `helper` is deleted *before* `caller` is even
    // defined, with no re-establishment — the body call must still draw
    // W123 (confirmed against tclsh 8.6.14: `invalid command name
    // "helper"`).
    let src = "proc helper {} {}\nrename helper {}\nproc caller {} { helper }\ncaller\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_issue_1009_codex_review_deleted_between_definition_and_call_still_flags() {
    // TP regression: `helper` is deleted *after* `caller` is defined but
    // *before* `caller` is ever invoked — the body call must still draw
    // W123 (confirmed against tclsh 8.6.14: `invalid command name
    // "helper"`, even though `caller`'s own definition predates the
    // deletion).
    let src = "proc helper {} {}\nproc caller {} { helper }\nrename helper {}\ncaller\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_fp_issue_1009_codex_review_method_body_call_before_later_deletion_resolves() {
    // FP guard, TclOO variant: a method body call before a later
    // unconditional deletion of the command it calls must also resolve —
    // confirmed against tclsh 8.6.14 (the script prints "ok" and exits 0).
    let src = "proc helper {} { return hi }\noo::class create Widget {\n    method run {} { helper }\n}\nWidget create w1\nw1 run\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_1009_codex_review_escape_hatch_requires_specific_enclosing_call() {
    // FN guard: an unrelated proc's top-level invocation must not "lend"
    // liveness to a different, never-invoked enclosing definition — only
    // `unrelated` is called at the top level before the deletion, `caller`
    // itself never is, so the `helper` call inside `caller`'s body must
    // still draw W123 (the same base shape that holds when there is no
    // competing top-level call at all).
    let src = "proc helper {} {}\nproc caller {} { helper }\nproc unrelated {} { return 1 }\nunrelated\nrename helper {}\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_fp_issue_1015_two_level_call_chain_before_later_deletion_resolves() {
    // FP guard: the escape hatch must follow a *chain* of
    // enclosing definitions, not one level. `inner` is never invoked at
    // the top level — only `outer` is — but `outer` calls `inner`, which
    // calls `helper`, all before the rename. tclsh8.6/9.0 both run this
    // clean (exit 0, no error).
    let src = "proc helper {} { return hi }\nproc inner {} { helper }\nproc outer {} { inner }\nouter\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1015_three_level_call_chain_before_later_deletion_resolves() {
    // FP guard, one level deeper still — the reachability query is a
    // fixpoint, so depth is not bounded by the number of hops.
    let src = "proc helper {} { return hi }\nproc l1 {} { helper }\nproc l2 {} { l1 }\nproc l3 {} { l2 }\nl3\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_tp_issue_1015_mutual_recursion_cycle_never_entered_still_flags() {
    // TP guard: `pingCaller`/`pongCaller` call each other and
    // nothing calls either at the top level, so neither is ever reached —
    // the cycle must terminate as "unreachable" rather than looping, and
    // the `helper` call inside must still draw W123. tclsh8.6 loads the
    // file fine precisely *because* the cycle is never entered; calling
    // into it after the rename fails `invalid command name "helper"`.
    let src = "proc helper {} {}\nproc pingCaller {} { helper\n pongCaller }\nproc pongCaller {} { pingCaller }\nrename helper {}\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_a_dead_body_edge_does_not_lower_a_later_top_level_offset() {
    // TP guard (adversarial soundness case):
    // `a` runs before the rename, but its only call to `b` sits inside `if
    // {0} { … }` and never executes. `b`'s real first invocation is the
    // top-level one *after* the rename, so `b`'s `helper` call fails.
    //
    // An unrestricted fixpoint lets the dead `a` -> `b` edge lower `b`'s
    // offset to `a`'s, which reads as "reached before the deletion" and
    // withdraws the warning. A body edge may not undercut a callee's own
    // top-level offset.
    //
    // Oracle (tclsh8.6): exits 1 with
    // `invalid command name "helper"` from `b`, invoked at line 6.
    let src = "proc helper {} { return hi }\nproc b {} { helper }\nproc a {} { if {0} { b } }\na\nrename helper {}\nb\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_tp_an_empty_enclosing_body_leaves_the_later_top_level_offset() {
    // The paired FP guard for the test above: same shape with `a`'s body
    // empty, so there is no `a` -> `b` edge to drop in the first place.
    // Both must warn, or the edge restriction would be indistinguishable from
    // "the edge never mattered".
    //
    // Oracle (tclsh8.6): exits 1, same error.
    let src = "proc helper {} { return hi }\nproc b {} { helper }\nproc a {} { }\na\nrename helper {}\nb\n";
    assert_eq!(w123_codes(src), vec!["W123".to_string()]);
}

#[test]
fn w123_fp_a_live_body_edge_still_reaches_a_callee_with_no_top_level_call() {
    // FP guard for the restriction: `b` has no top-level call site of its
    // own, so the `a` -> `b` edge is the only evidence there is and must
    // still resolve.  The restriction above is written to leave this chained
    // shape alone — otherwise every such chain becomes a false positive.
    let src = "proc helper {} { return hi }\nproc b {} { helper }\nproc a {} { b }\na\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}

#[test]
fn w123_fp_issue_1015_mutual_recursion_cycle_entered_at_top_level_resolves() {
    // FP guard: the same cycle, but entered by a real top-level call
    // before the deletion — every member is then reachable, so the nested
    // `helper` call resolves. Confirms the cycle handling short-circuits
    // without also poisoning the reachable case.
    let src = "proc helper {} { return hi }\nproc pingCaller {} { helper }\nproc pongCaller {} { pingCaller }\nproc entry {} { pongCaller }\nentry\nrename helper {}\n";
    assert_eq!(w123_codes(src), Vec::<String>::new());
}
