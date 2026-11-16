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

//! `foreach`, `for`, `switch`, `catch`, `try` and `uplevel`.

use super::*;

// handle_foreach_command

#[test]
fn handle_foreach_defines_single_loop_var() {
    let result = Analyser::new().analyse("foreach i {1 2 3} {puts $i}", "tcl");
    assert!(result.global_scope.variables.contains_key("i"));
}

#[test]
fn handle_foreach_defines_multiple_loop_vars() {
    let source = "foreach {k v} {a 1 b 2} {}";
    let result = Analyser::new().analyse(source, "tcl");
    let k = &result.global_scope.variables["k"];
    let v = &result.global_scope.variables["v"];
    assert_eq!(k.definition_span, span(9, 10));
    assert_eq!(v.definition_span, span(11, 12));
    assert!(k.definition_span.start() < v.definition_span.start());
}

#[test]
fn handle_foreach_varlist_uses_tcl_list_names_and_source_ranges() {
    let source = r#"foreach {{one name} "two name" three\ name plain} {values} {}"#;
    let result = Analyser::new().analyse(source, "tcl");
    let variables = &result.global_scope.variables;
    for (name, spelling) in [
        ("one name", "one name"),
        ("two name", "two name"),
        ("three name", r"three\ name"),
        ("plain", "plain"),
    ] {
        let start = u32::try_from(source.find(spelling).expect("original element")).unwrap();
        let end = start + u32::try_from(spelling.len()).unwrap();
        assert_eq!(variables[name].definition_span, span(start, end));
    }
}

#[test]
fn malformed_foreach_varlist_defines_no_variables() {
    let result = Analyser::new().analyse(r#"foreach "{unterminated" values {}"#, "tcl");
    assert!(result.global_scope.variables.is_empty());
}

#[test]
fn literal_foreach_binding_parses_single_variable_as_tcl_list() {
    assert_eq!(
        Analyser::literal_foreach_binding(
            r"{one name}",
            "alpha beta",
            true,
            tcl_syntax::word_rules::WordValueRules::TCL
        ),
        Some((
            "one name".to_string(),
            vec!["alpha".to_string(), "beta".to_string()]
        ))
    );
    assert!(
        Analyser::literal_foreach_binding(
            "{unterminated",
            "alpha",
            true,
            tcl_syntax::word_rules::WordValueRules::TCL
        )
        .is_none()
    );
    assert!(
        Analyser::literal_foreach_binding(
            "one two",
            "alpha",
            true,
            tcl_syntax::word_rules::WordValueRules::TCL
        )
        .is_none()
    );
}

#[test]
fn handle_foreach_defines_every_varlist_in_the_multi_list_lock_step_form() {
    let result = Analyser::new().analyse(
        "foreach dirName {src src {src core}} name {alpha beta gamma} {puts $dirName-$name}",
        "tcl",
    );
    assert!(result.global_scope.variables.contains_key("dirName"));
    assert!(result.global_scope.variables.contains_key("name"));
}

#[test]
fn handle_foreach_multi_list_form_binds_a_third_pair_too() {
    let result = Analyser::new().analyse("foreach a {1 2} b {3 4} c {5 6} {puts $a$b$c}", "tcl");
    for name in ["a", "b", "c"] {
        assert!(result.global_scope.variables.contains_key(name));
    }
}

#[test]
fn handle_foreach_too_few_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_foreach_command(
        "foreach",
        &["i".to_string(), "{1 2}".to_string()],
        &[esc_tok(span(0, 1)), str_tok(span(2, 7))],
        &[],
    );
    assert!(!handled);
}

#[test]
fn handle_foreach_rename_reinstall_idiom_resolves_every_literal_element() {
    // TP — `tk/library/accessibility.tcl` renames each classic widget
    // command away and
    // reinstalls a wrapper proc under the same name, once per element
    // of a literal `foreach` list. tclsh9.0/8.6 both prove `button`/
    // `entry` are the *new* wrapper procs afterwards; the original bodies
    // live only at `::tk::accessible::orig_button`/`orig_entry`.
    let mut a = Analyser::new();
    let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    // Both wrapper redefinitions are registered, at the *same* physical
    // source location (the one templated `proc ::$wtype ...`
    // statement) — genuinely correct: there is only one place in the
    // source that defines either of them.
    let button_def = r.all_procs.get("::button").expect("::button registered");
    let entry_def = r.all_procs.get("::entry").expect("::entry registered");
    assert_eq!(button_def.body_span, entry_def.body_span);
    // No garbled `${wtype}`-named entry left behind.
    assert!(
        !r.all_procs.keys().any(|k| k.contains('$')),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    // The old, pre-rename bodies are reachable only via their new
    // `orig_*` names.
    assert_eq!(
        r.renamed_commands.get("::tk::accessible::orig_button"),
        Some(&"::button".to_string())
    );
    assert_eq!(
        r.renamed_commands.get("::tk::accessible::orig_entry"),
        Some(&"::entry".to_string())
    );
}

#[test]
fn handle_foreach_rename_reinstall_idiom_is_not_overfit_to_two_elements() {
    // FP guard — a third element proves the per-iteration simulation
    // isn't hardcoded to exactly the corpus's own two widget names.
    let mut a = Analyser::new();
    let src = "namespace eval ::ns {\n    \
                   foreach wtype {button entry checkbutton} {\n        \
                   rename ::$wtype ::ns::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    for name in ["::button", "::entry", "::checkbutton"] {
        assert!(
            r.all_procs.contains_key(name),
            "{name} missing: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn handle_foreach_multi_pair_form_does_not_trigger_the_literal_simulation() {
    // TN — the multi-list lock-step form (`foreach v1 l1 v2 l2
    // body`) is a different arg shape; the single-pair-only literal
    // simulation must not misfire on it.
    let mut a = Analyser::new();
    let src = "foreach a {1 2} b {3 4} {\n    rename ::$a ::orig_$a\n}\n";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(r.renamed_commands.len(), 0, "{:?}", r.renamed_commands);
}

#[test]
fn handle_foreach_dynamic_list_does_not_trigger_the_literal_simulation() {
    // TN — a non-literal list (a variable, not a literal element list)
    // can't be simulated; must abstain.
    let mut a = Analyser::new();
    let src =
        "set items {button entry}\nforeach wtype $items {\n    rename ::$wtype ::orig_$wtype\n}\n";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(r.renamed_commands.len(), 0, "{:?}", r.renamed_commands);
}

#[test]
fn handle_foreach_non_idiom_body_is_unaffected() {
    // TN — an ordinary literal-list foreach with no rename/proc inside
    // must see no new side effects from the idiom-recognition code.
    let mut a = Analyser::new();
    let src = "foreach x {a b c} {\n    puts $x\n}\n";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(r.renamed_commands.len(), 0);
    assert!(!r.all_procs.keys().any(|k| k.contains('$')));
}

// handle_switch_command

fn switch_analyser() -> Analyser {
    let mut analyser = Analyser::new();
    analyser.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    analyser
}

#[test]
fn handle_switch_returns_true_for_canonical_shape() {
    let mut a = switch_analyser();
    let handled = a.handle_switch_command(
        "switch",
        &["$x".to_string(), "{a {puts a} b {puts b}}".to_string()],
        &[esc_tok(span(7, 9)), str_tok(span(10, 36))],
        &[],
    );
    assert!(handled);
}

#[test]
fn handle_switch_too_few_args_returns_false() {
    let mut a = switch_analyser();
    let handled = a.handle_switch_command("switch", &["$x".to_string()], &[], &[]);
    assert!(!handled);
}

#[test]
fn handle_switch_form1_walks_each_arm_body() {
    // Form 1: each arm body lands its `set` in the surrounding scope.
    let a = analysed("set x a\nswitch $x a {set y 1} b {set z 2}\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
    assert!(a.result.global_scope.variables.contains_key("z"));
}

#[test]
fn handle_switch_form2_braced_body_walks_each_arm() {
    // Form 2: the single braced word holds all pattern/body pairs, and
    // each body is walked.
    let a = analysed("set x a\nswitch $x { a {set y 1} b {set z 2} }\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
    assert!(a.result.global_scope.variables.contains_key("z"));
}

#[test]
fn handle_switch_form1_skips_fallthrough_marker() {
    // ``switch $x a - b {set y 1}`` — the ``-`` body for
    // pattern ``a`` is fall-through (next arm fires); only
    // ``b``'s body should be walked.
    let a = analysed("set x a\nswitch $x a - b {set y 1}\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
}

#[test]
fn handle_switch_recognises_dashdash_options_terminator() {
    // ``switch -- $x a {set y 1}`` — ``--`` ends the option
    // section; the string arg follows.  Walker still finds
    // the arm body and lands ``y``.
    let a = analysed("set x a\nswitch -- $x a {set y 1}\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
}

#[test]
fn handle_switch_dynamic_form2_body_skips_walk() {
    // Form 2 with a dynamic body (``$body`` instead of a
    // braced literal) yields no elements; the walk no-ops.
    let mut a = switch_analyser();
    let var_tok = Token::new(TokenType::Var, span(10, 15));
    a.handle_switch_command(
        "switch",
        &["$x".to_string(), "$body".to_string()],
        &[esc_tok(span(7, 9)), var_tok],
        &[],
    );
    // No body walked → no vars defined.
    assert_eq!(a.result.global_scope.variables.len(), 0);
}

// handle_catch_command

#[test]
fn handle_catch_canonical_returns_true() {
    let mut a = Analyser::new();
    let handled = a.handle_catch_command(&["body".to_string()], &[esc_tok(span(0, 4))], &[]);
    assert!(handled);
}

#[test]
fn handle_catch_with_result_var_defines_it() {
    let mut a = Analyser::new();
    // The binding positions come from the registry's VarWrite roles.
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_catch_command(
        &["body".to_string(), "res".to_string()],
        &[esc_tok(span(0, 4)), esc_tok(span(5, 8))],
        &[],
    );
    assert!(a.result.global_scope.variables.contains_key("res"));
}

#[test]
fn handle_catch_with_options_var_defines_both() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
    ));
    a.handle_catch_command(
        &["body".to_string(), "res".to_string(), "opts".to_string()],
        &[
            esc_tok(span(0, 4)),
            esc_tok(span(5, 8)),
            esc_tok(span(9, 13)),
        ],
        &[],
    );
    assert!(a.result.global_scope.variables.contains_key("res"));
    assert!(a.result.global_scope.variables.contains_key("opts"));
}

#[test]
fn handle_catch_no_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_catch_command(&[], &[], &[]);
    assert!(!handled);
}

// try (no handler of its own: it goes through the generic dispatch `for`
// uses; `try`'s own clause-timing resolution is `clause_grammar.rs`'s
// `try_grammar_agrees_with_the_retired_walk` and its neighbours)

#[test]
fn handle_try_walks_main_body() {
    // ``try {set y 1}`` — main body walks and lands ``y``.
    let mut a = Analyser::new();
    let r = a.analyse("try {set y 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("y"));
}

#[test]
fn handle_try_walks_finally_body() {
    // ``try {} finally {set z 1}`` — finally clause body walks.
    let mut a = Analyser::new();
    let r = a.analyse("try {} finally {set z 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("z"));
}

#[test]
fn handle_try_walks_on_handler_body() {
    // ``try {} on error {result options} {set q 1}`` — the handler
    // body walks and its var-list binds both names (so completion
    // offers `$result` / `$options`).
    let mut a = Analyser::new();
    let r = a.analyse("try {} on error {result options} {set q 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("q"));
    assert!(r.global_scope.variables.contains_key("result"));
    assert!(r.global_scope.variables.contains_key("options"));
}

#[test]
fn handle_try_walks_trap_handler_body() {
    // ``try {} trap NONE {result} {set q 1}`` — same shape as ``on``,
    // but the keyword is ``trap`` and the var-list has one name.
    let mut a = Analyser::new();
    let r = a.analyse("try {} trap NONE {result} {set q 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("q"));
    assert!(r.global_scope.variables.contains_key("result"));
}

// The `INSTALLS_NAMED_DEFINITION` re-dispatch contract.

#[test]
fn every_installer_spec_lands_on_a_live_redispatch_arm() {
    // Drift gate — the trait is a *promise* that
    // `simulate_remaining_foreach_iterations` re-runs the command once
    // per literal `foreach` element.  A spec that carries it but whose
    // analyser hook has no arm in that match silently promises nothing
    // (`oo::objdefine` is exactly that shape).
    // Registry-driven, so a newly-stamped spec fails here rather than
    // in a corpus months later.
    use tcl_registry::hooks::AnalyserHookId as Hook;
    let redispatched = [Hook::Proc, Hook::Rename, Hook::OoDefine, Hook::OoObjdefine];
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut checked = 0usize;
    for name in registry.command_names() {
        for spec in registry.specs(name) {
            if !spec
                .traits
                .contains(tcl_registry::Traits::INSTALLS_NAMED_DEFINITION)
            {
                continue;
            }
            checked += 1;
            let hook = spec.analyser_hook.unwrap_or_else(|| {
                panic!("{name} carries INSTALLS_NAMED_DEFINITION but has no analyser hook")
            });
            assert!(
                redispatched.contains(&hook),
                "{name} carries INSTALLS_NAMED_DEFINITION but its hook {hook:?} has no \
                     arm in the foreach re-dispatch — the trait promises a re-dispatch \
                     the match does not deliver",
            );
        }
    }
    assert!(
        checked >= 4,
        "expected proc/rename/oo::define/oo::objdefine, saw {checked}"
    );
}

/// `for`'s bodies take the generic body walk at their clause's timing
/// (D2.55), the reading the retired `handle_for_command` gave: `start`
/// runs once, at the enclosing depth; `next` and the body run per
/// iteration, at a control-flow depth. None is branch-selected.
#[test]
fn for_start_runs_at_the_enclosing_depth_and_next_and_body_per_iteration() {
    let mut a = crate::analyser::Analyser::new();
    let depths: Vec<(bool, bool)> = a
        .analyse(
            "for {package require A} {$i < 3} {package require B} {package require C}\n\
                 package require D\n",
            "tcl8.6",
        )
        .package_requires
        .iter()
        .map(|p| (p.conditional, p.control_flow))
        .collect();
    assert_eq!(
        depths,
        vec![(false, false), (false, true), (false, true), (false, false)],
        "start straight-line; next and body control flow; the depth \
             restored after the loop",
    );
}

#[test]
fn uplevel_nonzero_isolates_body_vars_from_proc() {
    // `uplevel 1 {set x 5}` runs in the caller's frame, not the enclosing
    // proc's — its `x` must not merge into the proc's locals (they are
    // different variables), and it lands in an isolated `Uplevel` child
    // scope tagged with the level word.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {} {\n    uplevel 1 {set x 5}\n    set y 1\n}\n",
        "tcl8.6",
    );
    let f_scope = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "f")
        .expect("f proc scope");
    assert!(f_scope.variables.contains_key("y"), "y is a proc local");
    assert!(
        !f_scope.variables.contains_key("x"),
        "uplevel body's `x` must not merge into the proc scope",
    );
    let up = f_scope
        .children
        .iter()
        .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
        .expect("isolated uplevel scope");
    assert_eq!(up.name, "1", "the level word tags the frame");
    assert!(up.variables.contains_key("x"));
}

#[test]
fn uplevel_zero_still_resets_to_global_frame() {
    // `uplevel #0` keeps the global-frame tag so variable resolution
    // resolves outward to the global namespace (not the caller-frame
    // abstention a non-`#0` level uses).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc g {} {\n    uplevel #0 {set z 5}\n}\n", "tcl8.6");
    let g_scope = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "g")
        .expect("g proc scope");
    let up = g_scope
        .children
        .iter()
        .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
        .expect("uplevel scope");
    assert_eq!(up.name, "#0");
    assert!(up.variables.contains_key("z"));
}
