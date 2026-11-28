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

//! Handler barriers on constant branches: unresolved, embedded and shadowed handlers, and later bindings made opaque.

use super::*;

#[test]
fn unresolved_handler_barrier_blocks_scalar_constant_branch() {
    crate::install_surfaces();
    // Tcl 9.0.4: the default unresolved handler may load and invoke a command
    // which changes caller-frame state. The SCCP consumer therefore cannot
    // retain the pre-call constant for this branch, even though the runtime
    // invocation itself remains generic.
    let cu = CompilationUnit::build_for(
        "proc p {} { set x 5; missing_command; if {$x == 5} { return stale } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "unresolved handler barrier must prevent stale scalar branch folding: {:?}",
        fu.sccp.constant_branches,
    );
}

#[test]
fn embedded_handler_barrier_blocks_host_scalar_constant_branch() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc p {} { set x 5; set result [missing_command]; if {$x == 5} { return stale } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "an embedded unresolved handler must invalidate host-following scalar facts",
    );
}

#[test]
fn condition_handler_barrier_blocks_scalar_constant_branch() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc p {} { set x 5; if {[missing_command]} {}; if {$x == 5} { return stale } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "a condition substitution reaching the unresolved handler must invalidate following scalar facts",
    );
}

#[test]
fn foreach_list_handler_barrier_blocks_scalar_constant_branch() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc p {} { set x 5; foreach item [missing_command] {}; if {$x == 5} { return stale } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "a foreach list substitution reaching the unresolved handler must invalidate following scalar facts",
    );
}

#[test]
fn braced_foreach_list_does_not_create_handler_barrier() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc p {} { set x 5; foreach item {[missing_command]} {}; if {$x == 5} { return kept } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        !fu.sccp.constant_branches.is_empty(),
        "a braced foreach list does not evaluate its bracket text",
    );
}

#[test]
fn try_header_handler_barrier_blocks_scalar_constant_branch() {
    crate::install_surfaces();
    use tcl_compiler::cfg_builder::{build_cfg_codegen_with_registry, build_cfg_with_registry};

    let registry = reg();
    let source = "proc p {} {set x 5; try {} on error [missing_command] {}; if {$x == 5} {return stale} else {return changed}}";
    let cu = CompilationUnit::build_for(source, &registry, false);
    assert_eq!(
        cu.fixture_function("::p").unwrap().sccp.constant_branches,
        [] as [tcl_compiler::sccp::ConstantBranch; 0]
    );
    for cfg in [
        build_cfg_with_registry(&cu.ir_module, false, &registry),
        build_cfg_codegen_with_registry(&cu.ir_module, false, &registry),
    ] {
        let function = cfg
            .procedures
            .get(&fixture_id(&cfg.procedures, "::p"))
            .expect("authored fixture body");
        assert!(
            function.blocks.values().any(|block| {
                block.statements.iter().any(|statement| {
                    statement.synthetic_marker()
                        == Some(tcl_compiler::ir::SyntheticMarker::UnseenCall)
                })
            }),
            "both inlined and deferred try paths must retain header effects"
        );
    }

    let safe = CompilationUnit::build_for(
        "proc p {} {set x 5; try {} on error [list e o] {}; if {$x == 5} {return kept} else {return changed}}",
        &registry,
        false,
    );
    assert_ne!(
        safe.fixture_function("::p").unwrap().sccp.constant_branches,
        [] as [tcl_compiler::sccp::ConstantBranch; 0]
    );
}

#[test]
fn known_safe_registry_handler_preserves_scalar_constant_branch() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc p {} { set x 5; string length value; if {$x == 5} { return kept } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        !fu.sccp.constant_branches.is_empty(),
        "a known safe registry handler should retain scalar precision",
    );
}

#[test]
fn embedded_shadowed_builtin_does_not_borrow_code_running_traits() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc eval args {return 0}; proc p {} {set x 5; set ignored [eval {set x 6}]; if {$x == 5} {return kept} else {return changed}}",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        !fu.sccp.constant_branches.is_empty(),
        "a user procedure named eval must not inherit eval's code-running traits",
    );
}

#[test]
fn embedded_alias_to_user_proc_does_not_borrow_code_running_traits() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc fake args {return 0}; interp alias {} eval {} fake; proc p {} {set x 5; set ignored [eval {set x 6}]; if {$x == 5} {return kept} else {return changed}}",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        !fu.sccp.constant_branches.is_empty(),
        "an alias to a user procedure must not borrow its source spelling's registry traits",
    );
}

#[test]
fn temporal_unknown_handler_keeps_auto_load_barrier() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "set auto_index(missing_command) { proc missing_command {} { upvar 1 x x; set x 6 } }; proc p {} { set x 5; missing_command; if {$x == 5} { return stale } else { return changed } }; set observed [p]; proc unknown {args} { return harmless }; puts $observed",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "an auto-loaded caller-mutating generation must keep the unresolved barrier",
    );
}

#[test]
fn embedded_user_call_makes_later_embedded_binding_opaque() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc mutate {} {rename safe {}; interp alias {} safe {} eval; return 0}; proc safe args {return 0}; proc p {} {set x 5; if {[mutate] + [safe {set x 6}]} {}; if {$x == 5} {return stale} {return changed}}; puts [p]",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "a later embedded call must see the earlier user call's source-order binding opacity",
    );
}

#[test]
fn embedded_rename_makes_later_embedded_binding_opaque() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc p {} {set x 5; if {[rename safe {}] + [safe {set x 6}]} {}; if {$x == 5} {return stale} {return changed}}; proc safe args {return 0}; puts [p]",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "later embedded resolution must observe a preceding rename transition",
    );
}

#[test]
fn opaque_user_call_keeps_later_builtin_resolution_barrier() {
    crate::install_surfaces();
    // tclsh: evaluates the later `puts` as `eval`, so `x` becomes 6 and the
    // final branch selects `changed`. The dynamic rename subjects prevent the
    // module summary from resolving this transition ahead of the call.
    let cu = CompilationUnit::build_for(
        "proc mutate {a b} {rename $a {}; rename $b $a}; proc p {} {set x 5; mutate puts eval; puts {set x 6}; if {$x == 5} {return stale} else {return changed}}; puts [p]",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "a source-opaque user call must invalidate later direct builtin resolution",
    );
}

#[test]
fn readable_eval_user_call_makes_later_binding_opaque() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "proc mutate {} {rename safe {}; interp alias {} safe {} eval; return 0}; proc safe args {return 0}; proc p {} {set x 5; eval {mutate}; safe {set x 6}; if {$x == 5} {return stale} {return changed}}; puts [p]",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "a readable eval body shares the source-order binding transfer",
    );
}

#[test]
fn alias_to_unresolved_handler_keeps_terminal_barrier_effect() {
    crate::install_surfaces();
    let cu = CompilationUnit::build_for(
        "interp alias {} forward {} unknown\nproc p {} { set x 5; forward missing_command; if {$x == 5} { return stale } else { return changed } }",
        &reg(),
        false,
    );
    let fu = cu.fixture_function("::p").expect("procedure");
    assert!(
        fu.sccp.constant_branches.is_empty(),
        "an alias reaching the registry unknown handler must keep the scalar barrier",
    );
}

/// A `proc llength …` shadow declared elsewhere in the module is visible only
/// to the whole-module command-mutation scan, so the per-procedure lattice
/// memo must carry that scan in its key and fold under it: a module that
/// shadows a builtin is never served a unit built as if every builtin still
/// meant what it spells (#2164). The key's analysis context carries the
/// module's command-trust snapshot, so the memo stays on and answers under
/// the module's own trust.
///
/// tclsh 8.4.20 – 9.1b0 (unanimous): with the shadow, `[llength {a b c}]` is
/// 99, so a lattice claiming `v == 3` is wrong.
#[test]
fn a_shadowing_module_is_not_served_the_builtin_answer() {
    crate::install_surfaces();
    // tclsh: with `proc llength {l} { return 99 }` defined, `g` returns 99;
    // without it, 3. The shadowing module must not be answered from a lattice
    // that proves the builtin's 3, and the control must still prove it (so
    // the shadowed case measures the trust, not a lattice that never had the
    // fact). Whether the memo stays on is a performance property, kept in the
    // analysis-performance ledger (ruling of 17 November).
    let (shadowed, _, _) = memoised_unit(
        "proc llength {l} { return 99 }\nproc g {} { set v [llength {a b c}]; return $v }\n",
    );
    assert!(
        !g_proves_v_is_three(&shadowed),
        "the shadowing module must not be answered with the builtin's 3"
    );
    let (control, _, _) = memoised_unit("proc g {} { set v [llength {a b c}]; return $v }\n");
    assert!(
        g_proves_v_is_three(&control),
        "the control's lattice does prove v == 3"
    );
}
