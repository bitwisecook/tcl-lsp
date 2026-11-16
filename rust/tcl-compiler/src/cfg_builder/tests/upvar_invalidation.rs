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

//! Call statements widened with the caller-side names an `upvar` or `uplevel` callee writes.

use super::*;

#[test]
fn rooted_regexp_value_form_does_not_widen_runtime_selected_method_frame() {
    let registry = native_registry();
    let module = crate::lowering::lower_to_ir(
        "oo::class create C {\n\
                 method value {re s} {::regexp $re $s}\n\
                 method capture {re s target} {::regexp $re $s $target}\n\
             }",
        &registry,
    );
    let context = prepare_cfg_context_with_registry(&module, &registry);
    let build = |name: &str| {
        let method = &module
            .methods
            .get(&fixture_id(&module.methods, name))
            .expect("authored fixture body");
        build_cfg_method_function_with_upvars(
            name,
            &method.body,
            method.execution_namespace.clone(),
            true,
            &registry,
            module.plain_command_dispatch,
            context.clone(),
            false,
        )
    };
    let has_opaque_variable_barrier = |function: &Function| {
        function.blocks.values().any(|block| {
            block.statements.iter().any(|statement| {
                matches!(
                    statement,
                    Statement::Barrier { reason, .. }
                        if reason.contains("writes a source-opaque variable name")
                )
            })
        })
    };

    let value = build("::C::value");
    assert!(
        !has_opaque_variable_barrier(&value),
        "an absolute two-operand regexp has no capture variable"
    );

    let capture = build("::C::capture");
    assert!(
        has_opaque_variable_barrier(&capture),
        "a dynamic capture name must still widen the variable frame"
    );
}

#[test]
fn literal_upvar_proc_call_augments_defs() {
    // Caller invokes a proc whose body has `upvar 1 caller_x x`.
    // The call should land with `caller_x` in its defs.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in defs, got {defs:?}",
    );
}

#[test]
fn alias_to_upvar_uses_its_prepended_frame_level() {
    // Tcl 9.0.4: `bind` executes `upvar 1 target local`; assigning
    // `local` in `setter` therefore assigns `target` in its caller.
    // The frame descriptor and argv are both supplied by the closed
    // command binding, not by the source spelling `bind`.
    let module = lower_module(
        "interp alias {} bind {} upvar 1\n\
             proc setter {} { bind target local; set local 99 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.contains(&"target".to_owned()),
        "the aliased upvar target must invalidate its caller: {defs:?}"
    );
}

#[test]
fn param_upvar_proc_call_resolves_call_site_arg() {
    // `proc setter {name} { upvar 1 $name x }` aliased to whatever
    // the caller passes for `name`.  Call `setter my_var` should
    // augment the call with `my_var` in defs.
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter my_var",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.contains(&"my_var".to_string()),
        "expected my_var in defs, got {defs:?}",
    );
}

#[test]
fn early_and_late_aliases_to_upvar_proc_prepend_the_caller_target() {
    for source in [
        "interp alias {} assign {} setter target\n\
             proc setter {name} { upvar 1 $name x; set x 1 }\n\
             proc outer {} { assign }",
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             proc outer {} { assign }\n\
             interp alias {} assign {} setter target",
    ] {
        let module = lower_module(source);
        let cfg = build_cfg(&module, false);
        let outer = fixture_id_if_present(&cfg.procedures, "::outer")
            .and_then(|id| cfg.procedures.get(&id))
            .expect("outer proc CFG should exist");
        let defs = find_call_defs(outer, "assign").expect("alias call should be in outer CFG");
        assert!(
            defs.contains(&"target".to_owned()),
            "alias definition order must not hide its prepended upvar target: {defs:?}",
        );
    }
}

#[test]
fn alias_chain_preserves_prefix_order_for_upvar_proc_arguments() {
    let module = lower_module(
        "proc setter {first name} { upvar 1 $name x; set x 1 }\n\
             interp alias {} inner {} setter first\n\
             interp alias {} outer {} inner second\n\
             outer",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "outer")
        .expect("outer alias call should be in top-level CFG");
    assert!(
        defs.contains(&"second".to_owned()),
        "inner then outer alias prefixes must remain in runtime argv order: {defs:?}",
    );
    assert!(
        !defs.contains(&"first".to_owned()),
        "the first fixed argument must not be projected onto the second parameter: {defs:?}",
    );
}

#[test]
fn substituted_upvar_actual_is_opaque_not_a_literal_def() {
    // The source spelling `$caller_var` is not the variable name passed
    // at runtime. No precise def may be manufactured from that spelling.
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x }\n\
             setter $caller_var",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        !defs.contains(&"caller_var".to_string()),
        "substituted source text became a literal caller def: {defs:?}",
    );
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier {
            writes: true,
            destroys: false,
            reads: false,
        }
    );
}

#[test]
fn trailing_expansion_preserves_a_known_upvar_actual_prefix() {
    let module = lower_module(
        "proc setter {name args} { upvar 1 $name x; set x 1 }\n\
             setter my_var {*}$rest",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(defs.contains(&"my_var".to_string()), "defs={defs:?}");
}

#[test]
fn expanded_upvar_actual_is_opaque_not_a_literal_def() {
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter {*}$actual",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.is_empty(),
        "expanded source text became defs: {defs:?}"
    );
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier {
            writes: true,
            destroys: false,
            reads: false,
        }
    );
}

/// A call omitting a `Name` parameter defines the place the parameter's
/// default names, as Tcl binds the default; a default that names no
/// variable the analysis can place widens the caller's frame, as a
/// substituted argument does.
#[test]
fn an_omitted_upvar_parameter_defines_its_default() {
    let module = lower_module(
        "proc setter {{name n}} { upvar 1 $name x; set x 1 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert_eq!(defs, vec!["n".to_owned()]);
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier::default()
    );
    let module = lower_module(
        "proc setter {{name a(1)}} { upvar 1 $name x; set x 1 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier {
            writes: true,
            destroys: false,
            reads: false,
        }
    );
}

#[test]
fn computed_or_expanded_direct_head_is_not_misidentified_as_a_static_callee() {
    for call in ["$command target", "{*}$invocation"] {
        let module = lower_module(&format!(
            "proc setter {{name}} {{ upvar 1 $name x; set x 1 }}\n{call}"
        ));
        let cfg = build_cfg(&module, false);
        assert_eq!(
            cfg.top_level.caller_frame_barrier,
            crate::dynamic_names::DynamicNameBarrier::default(),
            "call={call}"
        );
    }
}

#[test]
fn non_upvar_proc_call_unchanged() {
    // No upvar in the callee — the call's defs should be empty.
    let module = lower_module("proc no_upvar {} { set x 1 }\nno_upvar");
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "no_upvar")
        .expect("no_upvar call should be in top-level CFG");
    assert!(defs.is_empty(), "expected no augmented defs, got {defs:?}");
}

#[test]
fn qualified_call_resolves_via_qualified_key() {
    // `proc ::ns::setter` is registered under both `::ns::setter`
    // and `setter`.  A qualified call site `::ns::setter` should
    // resolve via the qualified key.
    let module = lower_module(
        "namespace eval ::ns {}\n\
             proc ::ns::setter {} { upvar 1 caller_x x }\n\
             ::ns::setter",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "::ns::setter")
        .expect("::ns::setter call should be in top-level CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in defs (qualified call), got {defs:?}",
    );
}

#[test]
fn cross_proc_call_inside_proc_body_augments_defs() {
    // Outer proc calls an inner upvar-using proc.  The outer
    // proc's CFG should reflect the augmented defs.
    let module = lower_module(
        "proc inner {} { upvar 1 caller_x x }\n\
             proc outer {} { inner }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("outer proc CFG should exist");
    let defs = find_call_defs(outer, "inner").expect("inner call should be in outer CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in inner's defs (called from outer), got {defs:?}",
    );
}

#[test]
fn late_alias_call_inside_proc_uses_projected_global_write_summary() {
    let module = lower_module(
        "proc ::mutate {} { global x; set x 99 }\n\
             namespace eval ::z {}\n\
             proc ::z::mutate {} {}\n\
             proc ::outer {} { call_mutate }\n\
             interp alias {} call_mutate {} mutate",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("outer proc CFG should exist");
    let defs =
        find_call_defs(outer, "call_mutate").expect("late alias call should be in outer CFG");
    assert!(
        defs.contains(&"x".to_owned()),
        "projected alias summary must invalidate x at the inner call: {defs:?}",
    );
}

#[test]
fn upvar_call_inside_if_branch_augments_defs() {
    // Calls inside structured constructs (if branches, while
    // bodies, ...) must also be augmented — the wiring runs in
    // `lower_script`, which is invoked recursively for every
    // body.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_y y }\n\
             if {1} { setter }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "setter")
        .expect("setter call in if-branch should be in top-level CFG");
    assert!(
        defs.contains(&"caller_y".to_string()),
        "expected caller_y in defs (call inside if branch), got {defs:?}",
    );
}

#[test]
fn empty_module_no_upvar_context_no_panic() {
    // Empty module → empty upvar_procs and proc_params.  Building
    // a CFG should still succeed.
    let module = Module::default();
    let cfg = build_cfg(&module, false);
    assert_eq!(cfg.top_level.name, "::top");
    assert!(cfg.procedures.is_empty());
}

#[test]
fn build_cfg_function_does_not_apply_upvar_wiring() {
    // `build_cfg_function` is the no-context variant used by
    // tests and one-off CFG construction.  It MUST NOT augment
    // defs even when the script calls a known upvar proc —
    // the function only sees the script, not a module.
    let module = lower_module("proc setter {} { upvar 1 caller_x x }\nsetter");
    // Build only the top-level CFG via the no-context API.
    let func = build_cfg_function(
        "::top",
        &module.top_level,
        true,
        &native_registry(),
        module.plain_command_dispatch,
    );
    let defs = find_call_defs(&func, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.is_empty(),
        "build_cfg_function without context should leave defs empty, got {defs:?}",
    );
}
