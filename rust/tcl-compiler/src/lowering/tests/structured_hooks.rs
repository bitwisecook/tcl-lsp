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

//! Registry-driven structured hooks: dispatch by hook ID, arity and shape preconditions, and the barrier on argument expansion.

use super::*;

#[test]
fn catch_options_specialisation_requires_the_selected_completion_grammar() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = format!("tcl{}", version.version_string());
        let registry = tcl_registry::model::ingress::static_context_for(&dialect).commands();
        let module = lower_to_ir("catch {set x executed} result options", registry);
        if version == tcl_dialect::TclVersion::V8_4 {
            assert!(matches!(
                module.top_level.statements[0],
                Statement::Barrier { .. }
            ));
        } else {
            assert!(matches!(
                module.top_level.statements[0],
                Statement::Catch { .. }
            ));
        }
    }
}

#[test]
fn static_bool_accepts_unique_prefix_booleans() {
    // A condition folds boolean words by unique prefix, like Tcl
    // (`if {tr}` runs, `if {of}` does not — verified against tclsh).
    assert_eq!(static_bool("tr"), Some(true));
    assert_eq!(static_bool(" ye "), Some(true));
    assert_eq!(static_bool("on"), Some(true));
    assert_eq!(static_bool("of"), Some(false));
    assert_eq!(static_bool("n"), Some(false));
    assert_eq!(static_bool("0"), Some(false));
    assert_eq!(static_bool("1"), Some(true));
    // Ambiguous `o` (on/off) is not a static boolean.
    assert_eq!(static_bool("o"), None);
    assert_eq!(static_bool("$x"), None);
}

#[test]
fn lowering_is_dialect_aware_via_expand_syntax() {
    // Lowering re-segments each
    // body under the document dialect.  For `if {*}$cond { puts hi }`,
    // on 8.5+ the `{*}` expands the condition, so the structured `if`
    // trips the expand-barrier (an opaque `Statement::Barrier` — the
    // structure can't be reasoned about); on 8.4 `{*}` is a literal
    // word, so the `if` lowers normally (no barrier).
    let reg = reg();
    let src = "set cond [gets stdin]\nif {*}$cond { puts hi }";
    let m90 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
    let m84 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::for_dialect("tcl8.4"));
    let is_barrier = |m: &Module| {
        matches!(
            m.top_level.statements.last(),
            Some(Statement::Barrier { .. })
        )
    };
    assert!(
        is_barrier(&m90),
        "9.0 expands `{{*}}` → structured `if` becomes a barrier: {:?}",
        m90.top_level.statements,
    );
    assert!(
        !is_barrier(&m84),
        "8.4 treats `{{*}}` as literal → `if` lowers without a barrier: {:?}",
        m84.top_level.statements,
    );
}

/// Every structured form `try_dispatch_structured_hook` covers must
/// barrier on `{*}` expansion.  A form that instead commits to its
/// *un-expanded* argv fabricates nonsense IR — `lmap i {*}$spec {…}`
/// becomes an `IRForeach` with one iterator over the literal word
/// `${spec}`, making W210 claim a loop variable is read before it is set.
#[test]
fn every_argv_shaped_hook_barriers_on_expansion() {
    let reg = reg();
    for src in [
        // Forms whose argv shape is fixed by the hook.
        "foreach i {*}$spec {puts $i}",
        "if {*}$cond {puts hi}",
        "switch {*}$spec {a {puts hi}}",
        "while {*}$spec {puts hi}",
        "for {*}$spec {puts hi}",
        "proc {*}$spec {puts hi}",
        "namespace eval {*}$spec {puts hi}",
        // Forms it missed.
        "lmap i {*}$spec {puts $i}",
        "dict for {k v} {*}$rest {puts $k}",
        "array for {k v} {*}$rest {puts $k}",
        "foreachLine ln {*}$rest {puts $ln}",
        "catch {puts hi} {*}$rest",
        "try {puts hi} {*}$rest",
        "eval {*}$rest",
        "uplevel {*}$rest",
        "apply {*}$rest",
    ] {
        // The expanded words hold run-time data: an unset variable would
        // raise before the command ran.
        let src = format!("set spec [gets stdin]\nset cond $spec\nset rest $spec\n{src}");
        let m = lower_to_ir(&src, &reg);
        assert!(
            matches!(
                m.top_level.statements.last(),
                Some(Statement::Barrier { .. })
            ),
            "`{src}` must barrier on `{{*}}` expansion: {:?}",
            m.top_level.statements,
        );
    }
}

/// The barrier is an *expansion* gate, not a blanket one: the same
/// structured forms without `{*}` keep their specialised lowering, and a
/// non-structured hook (`set`) never barriers on expansion at all — its
/// own `has_expansion` check declines and the call reaches
/// `lower_default`.
#[test]
fn expansion_barrier_leaves_unexpanded_and_non_structured_forms_alone() {
    let reg = reg();
    for src in [
        "foreach i $spec {puts $i}",
        "lmap i $spec {puts $i}",
        "catch {puts hi} err",
        "set {*}$rest",
        "puts {*}$rest",
    ] {
        let m = lower_to_ir(src, &reg);
        assert!(
            !matches!(
                m.top_level.statements.first(),
                Some(Statement::Barrier { .. })
            ),
            "`{src}` must not raise the expansion barrier: {:?}",
            m.top_level.statements,
        );
    }
}

/// The barrier reason names the command, which the compiler-explorer
/// `--show ir` view quotes.
#[test]
fn expansion_barrier_reason_names_the_command() {
    let m = lower_to_ir("set spec [gets stdin]\nlmap i {*}$spec {puts $i}", &reg());
    let Some(Statement::Barrier { reason, .. }) = m.top_level.statements.last() else {
        panic!("expected a barrier: {:?}", m.top_level.statements);
    };
    assert_eq!(reason, "lmap with argument expansion");
}

#[test]
fn logical_structure_does_not_grant_native_compilation() {
    let registry = reg();
    let source = "if {1} {set x 1}";
    let logical = lower_to_ir(source, &registry);
    assert!(matches!(
        logical.top_level.statements.first(),
        Some(Statement::If { .. })
    ));
    let mut native = Lowerer::new(&registry).for_bytecode_backend();
    native.source_entry_origin = SourceEntryOrigin::Driver;
    native.native_compilation.mode =
        tcl_registry::native_compilation::NativeCompilationMode::Direct;
    let emitted = lower_to_ir_with(native, source);
    let Some(Statement::Call {
        tokens: Some(tokens),
        ..
    }) = emitted.top_level.statements.first()
    else {
        panic!(
            "direct source entry has no admitted native compiler: {:?}",
            emitted.top_level
        );
    };
    assert!(
        crate::registry_invocation::admitted_native_compiler_invocation(&registry, None, tokens)
            .is_none()
    );
}

#[test]
fn foreach_line_lowers_to_foreach_statement() {
    // `foreachLine var filename body` lowers
    // to a single-iterator Statement::Foreach so variables assigned
    // inside the body propagate to the enclosing scope.  The
    // generic stdlib-proc path would route through Statement::Barrier and
    // lose that lattice information.
    let m = lower_to_ir(
        "foreachLine line /etc/hosts { set count [expr {$count + 1}] }",
        &reg(),
    );
    let stmts = &m.top_level.statements;
    let last = stmts.last().expect("at least one statement");
    match last {
        Statement::Foreach {
            iterators, is_lmap, ..
        } => {
            assert!(!is_lmap);
            assert_eq!(iterators.len(), 1);
            assert_eq!(iterators[0].vars, vec!["line".to_owned()]);
        }
        other => panic!("expected Statement::Foreach, got {other:?}"),
    }
}

#[test]
fn foreach_line_var_body_routes_through_barrier() {
    // `foreachLine line file $body` — the body is a Var token,
    // which is single-token but NOT a braced literal.  Compiling
    // the literal `$body` text as a static loop body would
    // produce incorrect IR / data-flow; the lowering must fall
    // through to the runtime command via Barrier.
    let m = lower_to_ir(
        "set body [gets stdin]\nforeachLine line /etc/hosts $body",
        &reg(),
    );
    let last = m.top_level.statements.last().expect("top-level statement");
    match last {
        Statement::Barrier {
            reason, command, ..
        } => {
            assert_eq!(command, "foreachLine");
            assert!(
                reason.contains("dynamic body"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected Barrier for Var body, got {other:?}"),
    }
}

#[test]
fn foreach_line_cmd_body_routes_through_barrier() {
    // Same guard for a `[cmd]` body — single-token but a Cmd
    // substitution, not a braced literal.
    let m = lower_to_ir("foreachLine line /etc/hosts [build-body]", &reg());
    let last = m.top_level.statements.last().expect("top-level statement");
    assert_runtime_opaque_call(last, &reg(), "foreachLine");
}

#[test]
fn descriptor_binding_requirement_keeps_original_head_and_exact_metadata_without_native_receipt() {
    let mut registry = reg();
    let prototype = registry.get_exact("list").unwrap().clone();
    for label in ["::probe", ":probe", "A:::probe", "probe::", ""] {
        let mut descriptor = prototype.clone();
        descriptor.name = label;
        registry.insert(descriptor);
    }
    for label in ["::probe", ":probe", "A:::probe", "probe::", ""] {
        let source = if label.is_empty() {
            "{} value".to_owned()
        } else {
            format!("{label} value")
        };
        let mut lowerer = Lowerer::new(&registry);
        lowerer.word_space = WordSpace::new(&source, 0, lowerer.source_channel);
        lowerer.command_binding_site_stack.push(Vec::new());
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(&source, 0, lowerer.config);
        let key = registry.get_exact(label).unwrap().descriptor_key();
        lowerer.record_consumed_command_binding(
            &commands[0],
            "::",
            crate::registry_invocation::SelectedLoweringBindingIdentity::Logical(key),
        );
        let site = &lowerer.command_binding_site_stack[0][0];
        assert_eq!(site.binding.name, label);
        assert_eq!(
            site.binding.identity,
            tcl_syntax::naming::unroot_rooted_key(label).unwrap_or(label)
        );
        assert!(site.binding.namespace_context.is_none());
        assert!(
            site.source_tokens
                .as_deref()
                .unwrap()
                .source_binding
                .is_none()
        );
    }
}

// The next three tests confirm `eval` and `uplevel` dispatch
// through the registry-driven hook ID to the expected output.

#[test]
fn registry_dispatch_eval_static_body_lowers_to_block() {
    // `eval {body}` with a literal body should still relax
    // to `Statement::Block` when dispatched through the
    // registry-driven path.
    let m = lower_to_ir("eval { set y 2 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Block { .. }),
        "expected Block via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_uplevel_static_body_lowers_to_upframe() {
    // `uplevel 1 {body}` with a literal body should lower
    // to a UpFrame via the registry-driven path. It runs inside a
    // procedure: at the global level C raises `bad level "1"`.
    let m = lower_to_ir("proc f {} {uplevel 1 { set z 3 }}", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    let stmt = proc.body.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::UpFrame { .. }),
        "expected UpFrame via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_unknown_command_falls_through_to_default() {
    // `lower_command` has no per-
    // name fallback — commands that aren't in the registry
    // (or whose `lowering_hook` is `None`) route directly
    // to [`Lowerer::lower_default`], which emits a generic
    // `Statement::Call`.  Pin that contract with a clearly
    // unknown command name.
    let m = lower_to_ir("totallyMadeUpCommand arg1 arg2", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "totallyMadeUpCommand",
            "expected generic Call via lower_default, got command={command}",
        ),
        other => panic!("expected Call via lower_default, got {other:?}"),
    }
}

// These tests pin the registry-driven hook-ID dispatch of `if`,
// `switch`, `for`, `while`, `catch`, `try` to the expected shape.

#[test]
fn registry_dispatch_if_lowers_to_if() {
    let m = lower_to_ir("if {1} { set q 4 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::If { .. }),
        "expected If via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_switch_lowers_to_switch() {
    let m = lower_to_ir("switch a { a { set q 1 } b { set q 2 } }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Switch { .. }),
        "expected Switch via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_for_lowers_to_for() {
    let m = lower_to_ir("for {set i 0} {$i < 3} {incr i} { set q $i }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::For { .. }),
        "expected For via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_while_lowers_to_while() {
    let m = lower_to_ir("while {0} { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::While { .. }),
        "expected While via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_catch_lowers_to_catch() {
    let m = lower_to_ir("catch { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Catch { .. }),
        "expected Catch via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_try_lowers_to_try() {
    let m = lower_to_ir("try { set q 1 } on error e { set q 2 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Try { .. }),
        "expected Try via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_namespace_eval_lowers_to_barrier() {
    // `lower_namespace_eval` emits a `Statement::Barrier`
    // tagged with `reason: "namespace eval"`.
    let m = lower_to_ir("namespace eval ::myns { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Barrier { reason, .. } => assert_eq!(
            reason, "namespace eval",
            "expected `namespace eval` barrier via registry hook, got reason={reason}",
        ),
        other => panic!("expected Barrier via registry hook, got {other:?}"),
    }
}

#[test]
fn registry_dispatch_namespace_non_eval_subcommand_falls_through() {
    // `namespace import` has no `lowering_hook` on its
    // subcommand spec, so `resolve_call` returns hook=None.
    // The registry-driven dispatcher returns None and the
    // residual string match's `_ => lower_default(...)`
    // arm produces a generic `Call`.
    let m = lower_to_ir("namespace import ::foo::*", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Call { .. }),
        "expected Call via lower_default fallback, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_foreach_lowers_to_foreach() {
    let m = lower_to_ir("foreach v {1 2 3} { set q $v }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Foreach { .. }),
        "expected Foreach via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_lmap_lowers_to_foreach() {
    // `lmap` shares `lower_foreach(... is_lmap=true)` with
    // `foreach`; both produce `Statement::Foreach`.
    let m = lower_to_ir("lmap v {1 2 3} { expr {$v + 1} }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Foreach { .. }),
        "expected Foreach (is_lmap=true) via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_dict_lowers_via_lower_dict() {
    // `dict set d k v` exercises the `lower_dict`
    // subcommand-dispatch path.  Pin the registry-driven
    // route by checking the canonical command name on the
    // emitted Call.
    let m = lower_to_ir("dict set d k v", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "dict",
            "expected `dict` call via registry hook, got command={command}",
        ),
        other => panic!("expected Call via registry hook, got {other:?}"),
    }
}

#[test]
fn registry_dispatch_dict_empty_args_falls_through_to_default() {
    // Bare `dict` — fails the `!args.is_empty()`
    // precondition, falls through to `lower_default`.
    let m = lower_to_ir("dict", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Call { .. }),
        "expected Call via lower_default fallback, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_foreach_line_lowers_via_lower_foreach_line() {
    // `foreachLine` is a Tcl 9.0+ command (TIP 670) and
    // `build_default()` registers it via the
    // `tcl::foreachline` spec.  No dialect-load dance is
    // needed.  The body is a brace-string literal here, so
    // the dedicated lowerer relaxes to a typed `Foreach`
    // (not a barrier).
    let m = lower_to_ir("foreachLine line readme.txt { set q $line }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Foreach { .. }),
        "expected Foreach via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_foreach_line_wrong_arity_falls_through_to_default() {
    // `foreachLine` with two args — fails the
    // `args.len() == 3` precondition, falls through to
    // `lower_default`.
    let m = lower_to_ir("foreachLine onlytwo args", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "foreachLine",
            "expected generic Call via lower_default, got command={command}",
        ),
        other => panic!("expected Call via lower_default, got {other:?}"),
    }
}
