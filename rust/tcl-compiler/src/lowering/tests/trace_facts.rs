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

//! Execution and variable trace facts recorded on the module.

use super::*;

// trace add execution module-fact population

#[test]
fn trace_add_execution_literal_recorded() {
    let m = lower_to_ir("trace add execution foo enter handler", &reg());
    assert!(
        crate::command_binding::trace_registrations::command_may_be_traced(
            &m.traced_commands,
            false,
            "foo"
        )
    );
    assert!(!m.has_dynamic_trace);
}

#[test]
fn trace_add_execution_dynamic_widens() {
    let m = lower_to_ir("trace add execution $cmd enter handler", &reg());
    assert!(m.has_dynamic_trace);
    assert_eq!(m.traced_commands.len(), 0);
}

#[test]
fn trace_add_execution_qualified_canonicalised() {
    let m = lower_to_ir("trace add execution ::ns::foo enter h", &reg());
    // Stripped of leading ``::`` so the GVN gate's
    // `command.trim_start_matches("::")` lookup hits.
    assert!(
        crate::command_binding::trace_registrations::command_may_be_traced(
            &m.traced_commands,
            false,
            "ns::foo"
        )
    );
}

#[test]
fn trace_add_variable_does_not_record_execution_trace() {
    // `trace add variable` is a separate channel — should not
    // populate `traced_commands` (those are command traces only).
    let m = lower_to_ir("trace add variable x write h", &reg());
    assert_eq!(m.traced_commands.len(), 0);
    assert!(!m.has_dynamic_trace);
}

/// `ModuleTraceRegistrations::walk`'s post-lower scan recurses once per nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame`
/// body, with no depth cap of its own. Transitively bounded to
/// `MAX_LOWER_NEST_DEPTH`
/// (256) by `lower_script`/`lower_body` (this same `lower_to_ir` call
/// builds the `Script` `ModuleTraceRegistrations::walk` then scans), so this is
/// defence-in-depth / consistency with every other full-tree walker in
/// this crate, not a currently-reproducible crash. 1000 levels of
/// source nesting is comfortably past the cap; the assertion is
/// that lowering (which runs `populate_trace_facts` ->
/// `ModuleTraceRegistrations::walk`) returns at all, not what it returns. Spawns its
/// own big-stack thread since the lexer/CST/segmenter stages upstream
/// of `lower_script`'s own cap still walk the full un-truncated source
/// nesting before that cap trims it — same rationale as
/// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_the_trace_walk() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("trace add execution foo enter handler\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = lower_to_ir(&src, &reg());
        })
        .unwrap()
        .join()
        .unwrap();
}

// trace add/remove/variable/vdelete module-fact population

#[test]
fn trace_target_names_keep_empty_and_literal_sigil_data() {
    let registry = reg();
    let empty = lower_to_ir("trace add variable {} write callback", &registry);
    assert!(empty.traced_variables.contains(""));
    assert!(!empty.traced_variables.contains("$"));
    let literal = lower_to_ir("trace add variable {$counter} write callback", &registry);
    assert!(literal.traced_variables.contains("$counter"));
    assert!(!literal.traced_variables.contains("counter"));
}

#[test]
fn trace_add_variable_literal_recorded() {
    let m = lower_to_ir("trace add variable x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    assert!(!m.has_dynamic_variable_trace);
}

#[test]
fn trace_remove_variable_literal_recorded() {
    // A `remove` only proves a trace *might* have existed — still
    // conservative to record, mirroring `trace add`.
    let m = lower_to_ir("trace remove variable x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_qualified_canonicalised() {
    let m = lower_to_ir("trace add variable ::x write h", &reg());
    // `::`-stripped so a top-level `set x 5` (chain key `x`) matches.
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_dynamic_widens() {
    let m = lower_to_ir("trace add variable $name write h", &reg());
    assert!(m.has_dynamic_variable_trace);
    assert_eq!(m.traced_variables.len(), 0);
}

#[test]
fn trace_add_variable_inside_proc_recorded() {
    let m = lower_to_ir(
        "proc setup {} { trace add variable ::x read onread }",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_inside_method_recorded() {
    // TclOO method bodies are extracted into `module.methods` (a
    // separate map from `module.procedures`) before this scan runs —
    // must not be missed just because it isn't a plain proc.
    let m = lower_to_ir(
        "oo::class create C {\n method m {} { trace add variable v write cb }\n}",
        &reg(),
    );
    assert!(m.traced_variables.contains("v"), "{:?}", m.traced_variables);
}

#[test]
fn trace_legacy_variable_form_recorded() {
    // The deprecated `trace variable name ops command` spelling must
    // populate the same fact as `trace add variable` — no
    // hardcoded-per-form gap.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let m = lower_to_ir("trace variable x r onread", registry);
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_legacy_vdelete_form_recorded() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let m = lower_to_ir("trace vdelete x r onread", registry);
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn removed_legacy_trace_forms_do_not_donate_tcl9_transitions() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for source in ["trace variable x r onread", "trace vdelete x r onread"] {
        let module = lower_to_ir(source, registry);
        assert!(module.traced_variables.is_empty());
        assert!(!module.has_dynamic_variable_trace);
    }
}

#[test]
fn trace_info_variable_does_not_record() {
    // `trace info`/`vinfo` only query trace state — no
    // `ESTABLISHES_VARIABLE_TRACE` trait, so they must not widen the
    // module fact.
    let m = lower_to_ir("trace info variable x", &reg());
    assert_eq!(m.traced_variables.len(), 0);
    assert!(!m.has_dynamic_variable_trace);
}

#[test]
fn trace_add_variable_through_interp_alias_recorded() {
    // `interp alias {} tracer {} trace` means `tracer add variable ...`
    // is really a `trace add variable ...` call — the whole-module scan
    // must key off the canonical (alias-resolved) command name, not the
    // source-surface `tracer` spelling, or this trace target is missed.
    let m = lower_to_ir(
        "interp alias {} tracer {} trace\ntracer add variable x write h",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_execution_through_interp_alias_recorded() {
    // Same alias-resolution requirement for the execution-trace channel:
    // `ModuleTraceRegistrations::walk` gates entry into both `traced_commands` and
    // the variable-trace inventory on one shared canonical-name check.
    let m = lower_to_ir(
        "interp alias {} tracer {} trace\ntracer add execution foo enter h",
        &reg(),
    );
    assert!(
        crate::command_binding::trace_registrations::command_may_be_traced(
            &m.traced_commands,
            false,
            "foo"
        ),
        "{:?}",
        m.traced_commands
    );
    assert!(!m.has_dynamic_trace);
}

#[test]
fn trace_add_execution_does_not_record_variable_trace() {
    // The command-execution channel is separate — should not
    // populate `traced_variables`.
    let m = lower_to_ir("trace add execution foo enter h", &reg());
    assert_eq!(m.traced_variables.len(), 0);
    assert!(!m.has_dynamic_variable_trace);
}

#[test]
fn trace_add_execution_inside_proc_recorded() {
    let m = lower_to_ir("proc init {} { trace add execution foo enter h }", &reg());
    assert!(
        crate::command_binding::trace_registrations::command_may_be_traced(
            &m.traced_commands,
            false,
            "foo"
        ),
        "traced_commands={:?}",
        m.traced_commands,
    );
}

#[test]
fn trace_add_variable_abbreviated_type_word_recorded() {
    // `trace add v x write h` / `trace add var x write h` install the
    // same variable trace as the full `variable` spelling — C Tcl
    // accepts any unique prefix (checked against tclsh 8.6.14).
    let m = lower_to_ir("trace add v x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    let m = lower_to_ir("trace add var x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_remove_variable_abbreviated_type_word_recorded() {
    let m = lower_to_ir("trace remove var x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_execution_abbreviated_type_word_not_variable() {
    // `e`/`exec` abbreviate `execution`, not `variable` — must not
    // widen `traced_variables`.
    let m = lower_to_ir("trace add e foo enter h", &reg());
    assert_eq!(m.traced_variables.len(), 0);
    assert!(
        crate::command_binding::trace_registrations::command_may_be_traced(
            &m.traced_commands,
            false,
            "foo"
        ),
        "{:?}",
        m.traced_commands
    );
}

#[test]
fn trace_add_variable_inside_namespace_eval_body_recorded() {
    // A trace installed inside a `namespace eval` body (a synthetic
    // `Module::body_units` entry, not a named proc) must populate the
    // same whole-module fact as one inside a `proc`.
    let m = lower_to_ir(
        "namespace eval ::n { trace add variable ::x write h }",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_inside_apply_body_recorded() {
    let m = lower_to_ir("apply {{} { trace add variable ::x write h }}", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_inside_method_body_recorded() {
    let m = lower_to_ir(
        "oo::class create C { method m {} { trace add variable ::x write h } }",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}
