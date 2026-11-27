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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

mod function_facts;
mod interprocedural_analysis;
mod memory_and_semantic_facts;
mod source_inputs;
mod unit_build;

use super::*;

fn registry() -> CommandRegistry {
    // These fixtures assert concrete C Tcl behaviour, including 9.1 grammar.
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}

/// Interprocedural call-site literal seeding (TP/FP/TN/FN suite for
/// [`collect_call_site_constants`] / [`params_constants_from_native_call_sites`]).
mod call_site_param_constants {
    use super::*;
    use crate::analyses::LatticeValue;

    /// FN regression: a call from inside a `TclOO` method body to an
    /// ordinary user proc is a real caller with a differing argument,
    /// but methods are built in a *separate* pass that runs after the
    /// call-site scan. Without
    /// `build_extra_call_site_scan_contexts` the scan sees only the one
    /// external `helper a` call and wrongly seeds `mode` as the constant
    /// `"a"`.
    #[test]
    fn call_site_inside_tcloo_method_body_is_not_missed() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                oo::class create Widget {
                    method go {} { helper b }
                }
                helper a
                [Widget new] go
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(f, "mode"),
            "helper is called with both \"a\" (external) and \"b\" (from the method body); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// Imported calls retain the original implementation allocation and
    /// contribute their differing arguments to the shared caller evidence.
    #[test]
    fn call_site_via_namespace_import_alias_is_not_missed() {
        let reg = registry();
        let src = "
                namespace eval ::lib {
                    namespace export helper
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                }
                namespace eval ::app {
                    namespace import ::lib::helper
                    proc go {} { helper b }
                }
                ::lib::helper a
                ::app::go
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::lib::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(f, "mode"),
            "helper is called with both \"a\" (direct) and \"b\" (via the ::app import); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// A real wildcard import requires the source namespace to export
    /// its procedure. Publishing that command retains an open caller set.
    const WILDCARD_IMPORT_SRC: &str = "
            namespace eval ::lib {
                namespace export helper
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
            }
            namespace eval ::app {
                namespace import ::lib::*
                proc go {} { helper prod }
            }
            ::lib::helper prod
            ::app::go
        ";

    #[test]
    fn wildcard_namespace_import_preserves_actual_identity_and_open_callers() {
        let reg = registry();
        let cu = CompilationUnit::build_for(WILDCARD_IMPORT_SRC, &reg, false);
        let bindings = crate::command_binding::SourceCommandBindings::analyse(
            WILDCARD_IMPORT_SRC,
            tcl_lexer::LexerConfig::default(),
            &reg,
        );
        let imported = bindings.invocation_at(
            "helper",
            "::app",
            u32::try_from(WILDCARD_IMPORT_SRC.find("helper prod").unwrap()).unwrap(),
        );
        let direct = bindings.invocation_at(
            "::lib::helper",
            "::",
            u32::try_from(WILDCARD_IMPORT_SRC.find("::lib::helper prod").unwrap()).unwrap(),
        );
        let imported_target = imported
            .proved_handler_target()
            .expect("reached imported proc");
        let direct_target = direct.proved_handler_target().expect("reached direct proc");
        assert_eq!(imported_target.command, "::lib::helper");
        assert_eq!(imported_target.identity, direct_target.identity);
        assert_eq!(
            imported_target.implementation_allocation,
            direct_target.implementation_allocation,
        );
        assert_eq!(imported.evaluated_written_argument_value(0), Some("prod"));
        assert_eq!(direct.evaluated_written_argument_value(0), Some("prod"));
        let function = fixture_id_if_present(&cu.procedures, "::lib::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(!knows_condition_mentioning(function, "mode"));
        assert_eq!(
            function.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
    }

    #[test]
    fn wildcard_import_without_export_does_not_manufacture_a_caller() {
        let reg = registry();
        let source = WILDCARD_IMPORT_SRC.replace("namespace export helper", "");
        let bindings = crate::command_binding::SourceCommandBindings::analyse(
            &source,
            tcl_lexer::LexerConfig::default(),
            &reg,
        );
        let imported = bindings.invocation_at(
            "helper",
            "::app",
            u32::try_from(source.find("helper prod").unwrap()).unwrap(),
        );
        assert!(imported.proved_handler_target().is_none());
        assert!(
            imported
                .targets
                .iter()
                .all(|target| target.command != "::lib::helper")
        );
    }

    /// Wildcard publication retains an open caller set even when a host
    /// supplies a closed view of the files in its current workspace.
    /// An export publishes `::lib::helper` for *any* other unit to import
    /// and call with a different literal, including one in a different
    /// checkout that no project enumeration can reach.
    #[test]
    fn exported_namespace_declines_the_seed_even_with_a_workspace_view() {
        let reg = registry();
        let exported = WILDCARD_IMPORT_SRC.replace(
            "namespace eval ::lib {",
            "namespace eval ::lib {\n                namespace export *",
        );
        for cu in vec![
            CompilationUnit::build_for(&exported, &reg, false),
            build_closed_world(&exported, &reg),
        ]
        .into_boxed_slice()
        {
            let f = fixture_id_if_present(&cu.procedures, "::lib::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(f, "mode"),
                "`namespace export *` publishes helper beyond any enumerable project: {:?}",
                f.sccp.constant_branches,
            );
        }
    }

    /// A proc reached only through a
    /// `namespace ensemble` `-map` redirection is a real caller the
    /// call-site scan cannot resolve — it has no model of ensemble
    /// dispatch. tclsh8.6/9.0-confirmed: with `namespace ensemble
    /// create -command myens -map {go helper}`, `myens go dev` prints
    /// `helper mode=dev`, so `mode` genuinely varies and must not fold.
    ///
    /// Nothing pinned that today. What makes it safe is *indirect*: the
    /// registry marks `namespace ensemble` with `Traits::EXPORTS_COMMAND`,
    /// which `params_constants_from_call_sites` treats as a boundary
    /// publishing the file's commands to callers it cannot enumerate, so
    /// the whole module declines seeding. Sound but blunt — narrowing
    /// that trait, or resolving *some* ensemble maps, would silently
    /// reopen the false-fold shape here. This test guards the mechanism,
    /// whatever replaces it: the fold must stay off unless a real
    /// ensemble-map resolution exists.
    #[test]
    fn ensemble_map_redirected_caller_does_not_fold_issue_979() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
                helper prod
                helper prod
                namespace ensemble create -command myens -map {go helper}
                myens go dev
            ";
        for cu in vec![
            CompilationUnit::build_for(src, &reg, false),
            build_closed_world(src, &reg),
        ]
        .into_boxed_slice()
        {
            let f = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(f, "mode"),
                "`myens go dev` reaches helper with \"dev\" (tclsh8.6/9.0-confirmed), \
                     so mode is not caller-invariant: {:?}",
                f.sccp.constant_branches,
            );
        }
    }

    /// FP guard: a `TclOO` method body resolves
    /// bare commands against the GLOBAL namespace, never the class's own
    /// namespace — tclsh8.6-confirmed live: `[::foo::Widget new] go`
    /// (method body `helper b`) calls `::helper`, never
    /// `::foo::Widget::helper`, even though the latter exists and is
    /// exactly what naively deriving the caller's namespace from the
    /// method's own qualified name (`::foo::Widget::go` → `::foo::Widget`)
    /// would try first. Before forcing method bodies to resolve against
    /// global, this misattributed the method's call to
    /// `::foo::Widget::helper` (folding a condition on a call that never
    /// actually happens) while simultaneously losing it as evidence for
    /// the real target `::helper` (which then wrongly folded on its one
    /// remaining, external-only literal).
    #[test]
    fn method_body_bare_call_resolves_against_global_not_class_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    oo::class create Widget {
                        method go {} { helper b }
                    }
                }
                namespace eval ::foo::Widget {
                    proc helper {mode} {
                        if {$mode eq \"WRONG\"} { set r 1 } else { set r 2 }
                    }
                }
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                helper a
                [::foo::Widget new] go
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let global_helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::helper analysed");
        let ns_helper = fixture_id_if_present(&cu.procedures, "::foo::Widget::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::Widget::helper analysed");
        assert!(
            !knows_condition_mentioning(global_helper, "mode"),
            "::helper is called with both \"a\" and \"b\" (the method body, once \
                 correctly resolved to global): {:?}",
            global_helper.sccp.constant_branches,
        );
        assert!(
            !knows_condition_mentioning(ns_helper, "mode"),
            "::foo::Widget::helper is never actually called by real Tcl semantics, \
                 so no call-site evidence should ever reach it: {:?}",
            ns_helper.sccp.constant_branches,
        );
    }

    /// FN regression: `uplevel #0
    /// { … }`'s body resolves bare commands against the GLOBAL
    /// namespace — tclsh8.6/9.0-confirmed live: `uplevel #0 { helper b
    /// }` inside `::foo::runIt` prints `GLOBAL helper mode=b`, never the
    /// `::foo::helper` sitting in the enclosing namespace.
    /// `Statement::UpFrame`'s body survives CFG construction as a block
    /// *statement*, which `scan_cfg_callers` (walking only
    /// `Call`/`Barrier`) skips entirely, so this real, differing call
    /// site would vanish from `::helper`'s evidence, leaving its one
    /// remaining caller's `"a"` to fold the condition.
    ///
    /// `build_extra_call_site_scan_contexts` builds a bare CFG for
    /// every *absolute* shift-`0` `UpFrame` body, resolved as `"::top"`
    /// — mirroring how a `TclOO` method body is forced global.
    ///
    /// `uplevel N` for any relative (non-`#0`) level stays out of scope:
    /// the target frame's namespace depends on the live call stack,
    /// which single-file static analysis cannot decide — a documented,
    /// permanent approximation.
    #[test]
    fn uplevel_zero_body_resolves_against_global_not_enclosing_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"WRONG\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel #0 { helper b }
                    }
                }
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let global_helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::helper analysed");
        let ns_helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(global_helper, "mode"),
            "::helper is called with both \"a\" and \"b\" (uplevel #0, once correctly \
                 resolved to global): {:?}",
            global_helper.sccp.constant_branches,
        );
        assert!(
            !knows_condition_mentioning(ns_helper, "mode"),
            "::foo::helper is never actually called by real Tcl semantics: {:?}",
            ns_helper.sccp.constant_branches,
        );
    }

    /// MISCOMPILE regression: `uplevel 0 { … }` is the *relative*
    /// current-frame form and must NOT be treated as the absolute global
    /// form. Encoding `#0` and `0` as the same `frame_shift == 0` resolves
    /// this body against `"::top"`, losing `::foo::helper`'s only varying
    /// call site — folding a branch that real Tcl reaches both ways.
    ///
    /// On tclsh8.6 and tclsh9.0: inside
    /// `::foo::runIt`, `uplevel #0 { helper b }` prints `GLOBAL helper`
    /// while `uplevel 0 { helper c }` prints `FOO helper`.
    #[test]
    fn uplevel_bare_zero_body_resolves_against_the_enclosing_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel 0 { helper b }
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "`uplevel 0` runs in the current frame, so ::foo::helper sees both \
                 \"a\" (direct) and \"b\" (the uplevel body): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Approximation pin: a *relative* `uplevel N { … }`
    /// keeps resolving against the enclosing unit's own namespace. The
    /// target frame's namespace depends on the live call stack, so this
    /// is a deliberate, permanent approximation — but it must stay an
    /// approximation that still *counts* the call site, not one that
    /// drops it. `::foo::helper` here sees both `"a"` and `"b"`.
    #[test]
    fn uplevel_relative_body_keeps_the_enclosing_units_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel 1 { helper b }
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::foo::helper sees both \"a\" (direct) and \"b\" (the relative uplevel \
                 body, approximated to the enclosing namespace): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FN guard for the `Traits::DEFINES_PROCEDURE` body-recursion skip
    /// a definition body is not re-walked from the
    /// definition site, so its call sites must still arrive through the
    /// defined procedure's *own* CFG. A conditionally-defined proc is
    /// the shape most at risk — its `proc` call is not a plain top-level
    /// statement.
    #[test]
    fn conditionally_defined_proc_body_call_sites_are_still_counted() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                if {[info exists ::env(X)]} {
                    proc runIt {} { helper b }
                }
                helper a
                runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::helper is called with both \"a\" and \"b\" (the conditionally-defined \
                 runIt's body): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FP guard: `namespace eval ::other { … }`
    /// runs its body in `::other`, never the enclosing proc's own
    /// namespace — tclsh8.6-confirmed live: a bare call inside such a
    /// block, nested arbitrarily deep inside an unrelated proc, still
    /// resolves against `::other`. Before threading the block's real
    /// target namespace into its body-unit qname
    /// (`register_body_unit`/`lower_namespace_eval`), every
    /// `namespace eval` body unit's qname reduced to the *global*
    /// namespace regardless of its actual target, so this call would
    /// have resolved (or misattributed) against global instead of
    /// `::other`.
    #[test]
    fn namespace_eval_body_nested_in_a_proc_resolves_against_its_own_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::other {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                }
                proc runIt {} {
                    namespace eval ::other { helper b }
                }
                ::other::helper a
                runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::other::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::other::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::other::helper is called with both \"a\" (direct) and \"b\" (via the \
                 namespace eval block nested inside runIt): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Namespace-resolution regression: `apply {params body
    /// ns}`'s third element names the namespace the body runs in, so a
    /// bare command word inside it resolves against *that* namespace.
    /// `lower_apply` computed the right `body_ns` and lowered the body
    /// against it, then registered the body unit under the bare `apply`
    /// marker — whose qname puts it in the global namespace. The call
    /// site was attributed to a `::helper` that does not exist and
    /// vanished from `::foo::helper`'s evidence, folding a branch real
    /// Tcl reaches both ways.
    ///
    /// On tclsh8.6 and tclsh9.0: inside `::foo::runIt`,
    /// `apply {{x} { helper $x } ::foo} b`
    /// returns 2 — the `else` arm of `::foo::helper` — while the direct
    /// `::foo::helper a` returns 1.
    #[test]
    fn apply_with_a_namespace_element_resolves_the_body_against_it() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        apply {{x} { helper $x } ::foo} b
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::foo::helper is called with both \"a\" (direct) and \"b\" (through the \
                 ::foo-pinned lambda): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// The two-element form is unchanged: with no namespace element,
    /// `apply` runs the body in the *global* namespace, not the caller's
    /// (Tcl `apply` manual). So the enclosing namespace's `helper` is
    /// genuinely never called from the lambda and keeps its fold.
    #[test]
    fn apply_without_a_namespace_element_still_resolves_globally() {
        let reg = registry();
        let src = "
                proc ::helper {x} {return $x}
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        apply {{x} { helper $x }} b
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            knows_condition_mentioning(helper, "mode"),
            "an unpinned lambda body resolves globally, so ::foo::helper only ever \
                 sees the direct \"a\": {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Test diagnostic branch knowledge without donating caller contents
    /// to an executable replacement of retained native object coercions.
    /// Build `src` under a **closed world**: an empty cross-file evidence
    /// set, which is the host asserting "I enumerated the project and no
    /// other file calls into this one" — the semantics for a file that is
    /// genuinely the whole program.
    fn build_closed_world(src: &str, reg: &CommandRegistry) -> CompilationUnit {
        let empty = crate::unit_scope::CallSiteEvidence::default();
        CompilationUnit::build_with_options(
            src,
            UnitBuildOptions {
                registry: reg,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::default(),
                dialect: None,
                external_call_sites: Some(&empty),
                declared_commands: None,
            },
        )
    }

    /// The literal every call in this file passes at `qname`'s formal
    /// `index`: advice for display, never a seed for the body. tclsh 8.4-9.1
    /// and jimsh: after sourcing a file whose only calls are `helper prod`,
    /// `list [helper prod] [helper dev]` is `1 2`.
    fn every_call_passes(cu: &CompilationUnit, qname: &str, index: usize) -> Option<String> {
        cu.caller_scope
            .call_sites
            .get(qname)?
            .uniform_literal_at(index)
            .map(ToOwned::to_owned)
    }

    fn knows_condition_mentioning(fu: &FunctionUnit, needle: &str) -> bool {
        fu.diagnostic_value_facts()
            .constant_branches()
            .iter()
            .any(|branch| branch.condition.contains(needle))
    }

    #[test]
    fn caller_contents_do_not_license_erasing_retained_parameter_coercions() {
        let reg = registry();
        let source =
            "proc helper {n} {if {$n + 0 == 7} {set result yes} else {set result no}}; helper 007";
        let unit = CompilationUnit::build_for(source, &reg, false);
        let function = fixture_id_if_present(&unit.procedures, "::helper")
            .and_then(|id| unit.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            knows_condition_mentioning(function, "n"),
            "caller_scope={:?} names={:?} preparations={} branch_bindings={} semantic_expressions={}",
            unit.caller_scope,
            function.ssa.var_names(),
            function.cfg.expression_preparations.len(),
            function.cfg.condition_binding_sites.len(),
            function
                .semantic_values()
                .map_or(0, |facts| facts.expression_iter().count())
        );
        assert_eq!(
            function.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
        let symbol = function.ssa.var_symbol("n").expect("retained formal slot");
        assert_eq!(
            function.semantic_values().unwrap().contents((symbol, 0)),
            Some(&crate::analyses::LatticeValue::Const(
                crate::analyses::ConstValue::String("007".into())
            ))
        );
    }

    /// FN: a proc declared inside a `namespace eval` block recurses into
    /// itself by its *bare* name. A resolver that only tries
    /// global-qualified spellings of the command word can never match the
    /// proc's namespaced qualified name — the recursive call (whose
    /// argument necessarily varies call to call) vanishes from the
    /// call-site scan, leaving only the one external caller's literal `0`
    /// visible. `params_constants_from_call_sites` would then seed `count`
    /// as the compile-time constant `0`, folding the always-alternating
    /// `$count & 1` parity check to a fixed `false` — exactly the
    /// reported false positive.
    #[test]
    fn namespaced_recursive_proc_parity_check_is_not_constant_folded() {
        let reg = registry();
        let src = "
                namespace eval ::graph {
                    proc dfs {count} {
                        if {$count & 1} {
                            set parity odd
                        } else {
                            set parity even
                        }
                        if {$count < 3} {
                            dfs [expr {$count + 1}]
                        }
                    }
                }
                ::graph::dfs 0
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let dfs = fixture_id_if_present(&cu.procedures, "::graph::dfs")
            .and_then(|id| cu.procedures.get(&id))
            .expect("dfs analysed");
        assert!(
            !knows_condition_mentioning(dfs, "count"),
            "recursive parity check on `count` must not fold to a constant: {:?}",
            dfs.sccp.constant_branches,
        );
    }

    /// TN control: the same shape at the *top level* (no namespace),
    /// where a bare recursive call resolves to the right un-namespaced
    /// qualified name on its own, must stay sound.
    #[test]
    fn top_level_recursive_proc_parity_check_is_not_constant_folded() {
        let reg = registry();
        let src = "
                proc count_up {n} {
                    if {$n & 1} { set p odd } else { set p even }
                    if {$n < 3} { count_up [expr {$n + 1}] }
                }
                count_up 0
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::count_up")
            .and_then(|id| cu.procedures.get(&id))
            .expect("count_up analysed");
        assert!(
            !knows_condition_mentioning(f, "n"),
            "recursive parity check on `n` must not fold to a constant: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// TP control: the interprocedural seed must still fire for a
    /// genuinely proc-call-invariant parameter — two callers (one via a
    /// same-namespace bare call, proving the namespace-aware resolver
    /// still positively resolves, not just negatively declines) passing
    /// the identical literal.
    #[test]
    fn two_same_namespace_callers_with_uniform_literal_still_folds() {
        let reg = registry();
        let src = "
                namespace eval ::a {
                    proc go {mode} {
                        if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                    }
                    proc caller1 {} { go prod }
                    proc caller2 {} { go prod }
                }
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let go = fixture_id_if_present(&cu.procedures, "::a::go")
            .and_then(|id| cu.procedures.get(&id))
            .expect("go analysed");
        assert!(
            knows_condition_mentioning(go, "mode"),
            "two same-namespace callers passing the identical literal should still fold: caller_scope={:?}, contents={:?}, preparations={}, condition_bindings={}",
            cu.caller_scope,
            go.semantic_values()
                .map(|facts| facts.contents_iter().collect::<Vec<_>>()),
            go.cfg.expression_preparations.len(),
            go.cfg.condition_binding_sites.len(),
        );
        assert!(
            go.sccp.constant_branches.is_empty(),
            "logical incoming contents do not prove erasure of native operand conversion",
        );
    }

    /// FP guard: two same-leaf-name procs in different namespaces must
    /// never be conflated by a bare same-namespace call — `::a::go`'s
    /// only caller passes `"same"`; `::b::go`'s only caller passes
    /// `"different"`. Each must fold to *its own* literal, not the
    /// other's — a namespace-blind resolver matches neither bare call to a
    /// real proc.
    #[test]
    fn sibling_namespace_procs_with_same_leaf_name_are_not_conflated() {
        let reg = registry();
        let src = "
                namespace eval ::a {
                    proc go {mode} {
                        if {$mode eq \"same\"} { set r 1 } else { set r 2 }
                    }
                    proc caller {} { go same }
                }
                namespace eval ::b {
                    proc go {mode} {
                        if {$mode eq \"same\"} { set r 1 } else { set r 2 }
                    }
                }
                ::b::go different
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let a_go = fixture_id_if_present(&cu.procedures, "::a::go")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::a::go analysed");
        let b_go = fixture_id_if_present(&cu.procedures, "::b::go")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::b::go analysed");
        assert!(
            knows_condition_mentioning(a_go, "mode"),
            "::a::go's sole caller passes a uniform literal, should fold: {:?}",
            a_go.sccp.constant_branches,
        );
        assert!(
            b_go.sccp
                .constant_branches
                .iter()
                .any(|b| b.condition.contains("mode") && !b.value),
            "::b::go's sole caller passes \"different\", condition should fold false: {:?}",
            b_go.sccp.constant_branches,
        );
    }

    /// FP guard: a `rename` that could redirect a *different* command
    /// onto the callee's own name must disqualify the seed, even though
    /// every call site this scan can see (all made before the rename,
    /// textually) passes a uniform literal — `trusts_proc_binding` is
    /// flow-insensitive/whole-module by design (matching the identical
    /// gate already trusted for the optimiser's O103 proc-call fold), so
    /// it can't assume the calls happen before the rename takes effect.
    #[test]
    fn rename_onto_callee_name_disqualifies_call_site_seed() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc other {mode} {
                    if {$mode eq \"prod\"} { set y 3 } else { set y 4 }
                }
                helper prod
                helper prod
                rename other helper
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a callee whose name is later rebound must not fold on stale call-site evidence: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// A dispatch through `$cmd` whose value the scope assigns is resolved by
    /// value and recorded as a `helper prod` call, so the evidence still
    /// says every call passes `prod`. That is advice only: the body is
    /// analysed from a generic entry.
    #[test]
    fn dynamic_dispatch_elsewhere_keeps_an_unrelated_procedures_evidence() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
                set cmd helper
                $cmd prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a body is analysed from a generic entry; this file's calls are advice only: {:?}",
            helper.sccp.constant_branches,
        );
        assert_eq!(
            every_call_passes(&cu, "::helper", 0).as_deref(),
            Some("prod"),
        );
    }

    /// FN regression: a call site embedded inside a `catch { … }` body is
    /// a real caller with a differing argument, but `catch`'s body is an
    /// `ArgRole::Body` argument of the *builtin* `catch` — never a user
    /// proc — so a flat, one-level call-site walk resolves `catch`,
    /// finds no matching proc, and moves on without ever noticing the
    /// `isEven 4` sitting inside it. Before recursing into `ArgRole::Body`
    /// arguments, this scan only ever saw the one external `isEven 3`
    /// call, so it wrongly seeded `n` as the constant `3`.
    #[test]
    fn call_site_inside_catch_body_is_not_missed() {
        let reg = registry();
        let src = "
                proc is_even {n} {
                    if {$n % 2 == 0} { return 1 } else { return 0 }
                }
                proc main {} {
                    is_even 3
                    catch { is_even 4 }
                }
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::is_even")
            .and_then(|id| cu.procedures.get(&id))
            .expect("is_even analysed");
        assert!(
            !knows_condition_mentioning(f, "n"),
            "is_even is called with both 3 and 4 (the latter inside `catch`); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// FN regression, `uplevel` flavour: a literal `uplevel {…}` body is
    /// an `ArgRole::Body` argument of the builtin `uplevel`, exactly like
    /// `catch`'s.
    #[test]
    fn call_site_inside_uplevel_body_is_not_missed() {
        let reg = registry();
        let src = "
                proc is_even {n} {
                    if {$n % 2 == 0} { return 1 } else { return 0 }
                }
                proc main {} {
                    is_even 3
                    uplevel 1 { is_even 4 }
                }
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::is_even")
            .and_then(|id| cu.procedures.get(&id))
            .expect("is_even analysed");
        assert!(
            !knows_condition_mentioning(f, "n"),
            "is_even is called with both 3 and 4 (the latter inside `uplevel`); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// An unrelated call inside a `catch` body leaves a *different*
    /// procedure's evidence in place: the recursive Body-arg scan attributes
    /// evidence to the right callee.
    #[test]
    fn unrelated_catch_body_does_not_disqualify_a_different_proc() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc noisy {} {
                    catch { error expected_failure }
                }
                helper prod
                helper prod
                noisy
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a body is analysed from a generic entry; this file's calls are advice only: {:?}",
            helper.sccp.constant_branches,
        );
        assert_eq!(
            every_call_passes(&cu, "::helper", 0).as_deref(),
            Some("prod"),
        );
    }

    #[test]
    fn caught_unresolved_command_keeps_its_callback_mutation_boundary() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc noisy {} {catch {unresolved_command}}
                helper prod
                noisy
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(!knows_condition_mentioning(helper, "mode"));
        assert_eq!(
            helper.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
    }

    /// FP guard: a `package provide` file may export procs another file
    /// calls with a different literal — this (single-file) compilation
    /// unit can never see that caller, so it must never seed from the
    /// call sites it happens to see locally.
    #[test]
    fn package_provide_file_disqualifies_every_seed() {
        let reg = registry();
        let src = "
                package provide mylib 1.0
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a package-providing file must not seed from locally-visible call sites: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// A plain script with no `package provide` still records its calls'
    /// literals as advice; the body is analysed from a generic entry.
    #[test]
    fn non_package_file_is_unaffected_by_package_provide_guard() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a body is analysed from a generic entry; this file's calls are advice only: {:?}",
            helper.sccp.constant_branches,
        );
        assert_eq!(
            every_call_passes(&cu, "::helper", 0).as_deref(),
            Some("prod"),
        );
    }

    /// `package provide` merely *mentioned* in a comment leaves the calls'
    /// evidence in place: the guard reads a real, resolved invocation, not
    /// a raw-text substring of the file.
    #[test]
    fn package_provide_mentioned_only_in_a_comment_does_not_disqualify() {
        let reg = registry();
        let src = "
                # this file does not package provide anything itself
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a body is analysed from a generic entry; this file's calls are advice only: {:?}",
            helper.sccp.constant_branches,
        );
        assert_eq!(
            every_call_passes(&cu, "::helper", 0).as_deref(),
            Some("prod"),
        );
    }

    /// FN guard: a real `package provide` invocation must still
    /// disqualify the seed even when it is spelled with unusual
    /// whitespace or fully namespace-qualified — cases a
    /// `source.contains("package provide")` substring check would miss.
    #[test]
    fn package_provide_with_unusual_spelling_still_disqualifies() {
        let reg = registry();
        let src = "
                ::package\tprovide mylib 1.0
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a real (if oddly-spelled) package provide must still disqualify: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Two-file repro: `lib.tcl` has **no** `package provide`, so the
    /// provide guard never fires; its only two visible callers both pass
    /// `"prod"`, so seeding `mode` as that constant folds the condition —
    /// even though `main.tcl` `source`s it and calls `helper dev`.
    mod cross_file {
        use super::*;
        use crate::unit_scope::CallSiteEvidence;

        /// A plain library file, no `package provide`, whose in-file
        /// callers agree.
        const LIB: &str = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
                helper prod
                helper prod
            ";

        /// Sources the library and calls its proc with a literal
        /// `lib.tcl` never sees.
        const MAIN: &str = "
                source lib.tcl
                helper dev
            ";

        /// Cross-file evidence a host builds from `other`, resolved
        /// against the procedures `LIB` declares.
        fn library_entry() -> crate::command_binding::SourceAnalysisEntry {
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
            crate::command_binding::SourceAnalysisEntry {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                trusted_source_modules: vec![
                    crate::command_binding::TrustedSourceModuleLoader::new(
                        tcl_dialect::model::Family::Tcl,
                        "lib.tcl".into(),
                        None,
                        "workspace/library/revision-one".into(),
                        &Arc::from(LIB),
                    ),
                ],
                ..crate::command_binding::SourceAnalysisEntry::default()
            }
        }

        fn evidence_from(other: &str, reg: &CommandRegistry) -> CallSiteEvidence {
            let known: HashSet<String> = ["::helper".to_owned()].into_iter().collect();
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
            let entry = library_entry();
            // The driver actually loads the library before this caller file.
            // MAIN already performs that read at its authored source site.
            let loaded;
            let source = if other == MAIN || other.starts_with("source lib.tcl") {
                other
            } else {
                loaded = format!("source lib.tcl\n{other}");
                &loaded
            };
            let context = Arc::new(
                crate::environment_ingress::context_for_profile(profile)
                    .with_command_store(reg.snapshot().shared_registry()),
            );
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            );
            crate::unit_scope::scan_source_call_sites_with_source_input(
                source,
                None,
                &known,
                &[],
                &input,
                &entry,
            )
        }

        fn build_with_evidence(
            src: &str,
            reg: &CommandRegistry,
            evidence: &CallSiteEvidence,
        ) -> CompilationUnit {
            build_with_source_evidence(src, reg, Some(evidence))
        }

        fn build_with_source_evidence(
            src: &str,
            reg: &CommandRegistry,
            evidence: Option<&CallSiteEvidence>,
        ) -> CompilationUnit {
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
            let context = Arc::new(
                crate::environment_ingress::context_for_profile(profile)
                    .with_command_store(reg.snapshot().shared_registry()),
            );
            CompilationUnit::build_with_context_registry(
                src,
                UnitBuildOptions {
                    registry: reg,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    dialect: Some(profile),
                    external_call_sites: evidence,
                    declared_commands: None,
                },
                Some(&library_entry()),
                context,
            )
        }

        /// FP guard — the reported bug. With `main.tcl`'s `helper dev`
        /// merged in, `mode` has two distinct literals and must not fold.
        #[test]
        fn sourcing_file_with_a_differing_literal_retracts_the_fold() {
            let reg = registry();
            let evidence = evidence_from(MAIN, &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "main.tcl calls helper with \"dev\"; lib.tcl must not fold on \"prod\": {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// The evidence itself, before any lattice: the cross-file scan
        /// must attribute `helper dev` to `::helper` at position 0.
        #[test]
        fn cross_file_scan_attributes_the_call_to_the_library_proc() {
            let reg = registry();
            let evidence = evidence_from(MAIN, &reg);
            let helper = evidence.get("::helper").expect("helper call recorded");
            assert_eq!(helper.uniform_literal_at(0), Some("dev"));
        }

        /// `main.tcl` passing `prod` too leaves one literal across the
        /// project's calls, which is advice only: the body is still analysed
        /// from a generic entry, since a later `source` or an interactive
        /// session can call it with any value.
        #[test]
        fn sourcing_file_agreeing_on_the_literal_is_advice_only() {
            let reg = registry();
            let evidence = evidence_from("source lib.tcl\nhelper prod\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "a body is analysed from a generic entry; this file's calls are advice only: {:?}",
                helper.sccp.constant_branches,
            );
            assert_eq!(
                every_call_passes(&cu, "::helper", 0).as_deref(),
                Some("prod"),
            );
        }

        /// A workspace file that never mentions `helper` contributes no
        /// evidence, and the body stays at its generic entry.
        #[test]
        fn unrelated_workspace_file_contributes_nothing() {
            let reg = registry();
            let known = HashSet::from(["::helper".to_owned()]);
            let evidence = crate::unit_scope::scan_source_call_sites(
                "proc other {x} { return $x }\nother 1\n",
                &reg,
                None,
                tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile"),
                &known,
                &[],
            );
            assert!(
                evidence.get("::helper").is_none(),
                "an unrelated file supplies no caller of ::helper: {evidence:?}",
            );
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "a body is analysed from a generic entry; this file's calls are advice only: {:?}",
                helper.sccp.constant_branches,
            );
            assert_eq!(
                every_call_passes(&cu, "::helper", 0).as_deref(),
                Some("prod"),
            );
        }

        /// FN guard — a cross-file caller reached through a *dynamic*
        /// argument is still a caller. `helper $mode` in `main.tcl`
        /// poisons position 0, so the fold is retracted even though no
        /// second literal was ever written.
        #[test]
        fn cross_file_dynamic_argument_poisons_the_position() {
            let reg = registry();
            let evidence = evidence_from("set m dev\nhelper $m\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "a cross-file caller with a non-literal argument must retract the fold: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard — a deferred script reads an unknown value when the
        /// callback runs. Its possible caller cannot inherit the immediate
        /// file's uniform argument. The body role and future frame come
        /// from the selected native descriptor, independently of spelling.
        #[test]
        fn cross_file_command_prefix_callback_poisons_every_position() {
            let reg = registry();
            let evidence = evidence_from("after 0 {helper $::runtime_mode}\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "a deferred cross-file callback reads an unknown argument: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard — a cross-file `rename` moves `helper`'s binding, so
        /// a call reaching it need not be one any scan attributed to it.
        /// `command_mutations` only sees the file being compiled, so
        /// without this the rebinding is invisible across files.
        #[test]
        fn cross_file_rename_poisons_the_callee() {
            let reg = registry();
            let evidence = evidence_from("rename helper legacy_helper\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "another file renamed helper; its binding has moved: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// A `source`ing file is itself a boundary: `lib.tcl` on its own
        /// (no workspace view) has no boundary at all and still folds —
        /// the documented, accepted limit of a standalone single-file
        /// build — but the *sourcing* file's own procs do not, because
        /// `source` declares `LOADS_EXTERNAL_UNIT`.
        #[test]
        fn a_file_that_sources_another_declines_the_seed_on_its_own() {
            let reg = registry();
            let src = "
                    source lib.tcl
                    proc local {mode} {
                        if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                    }
                    local prod
                    local prod
                ";
            let cu = build_with_source_evidence(src, &reg, None);
            let f = fixture_id_if_present(&cu.procedures, "::local")
                .and_then(|id| cu.procedures.get(&id))
                .expect("local analysed");
            assert!(
                !knows_condition_mentioning(f, "mode"),
                "the sourced file's script can call ::local with anything: {:?}",
                f.sccp.constant_branches,
            );
            // …and with a workspace view that says otherwise, it folds.
            let empty = CallSiteEvidence::default();
            let closed = build_with_evidence(src, &reg, &empty);
            let f = fixture_id_if_present(&closed.procedures, "::local")
                .and_then(|id| closed.procedures.get(&id))
                .expect("local analysed");
            assert!(
                knows_condition_mentioning(f, "mode"),
                "with the project enumerated, the boundary is no longer a blind spot: {:?}",
                f.sccp.constant_branches,
            );
        }
    }

    /// FN regression pinned at the `LatticeValue` level (not just
    /// "no constant branch"): the seed for a genuinely-recursive
    /// parameter must never even reach `Const` for the parameter's
    /// version-0 lattice entry, confirming the fix operates at the SCCP
    /// seed itself (the deepest layer), not merely suppressing the
    /// downstream diagnostic.
    #[test]
    fn recursive_param_version_zero_is_never_const_in_lattice() {
        let reg = registry();
        let src = "
                namespace eval ::graph {
                    proc dfs {count} {
                        if {$count & 1} { set p odd } else { set p even }
                        if {$count < 3} { dfs [expr {$count + 1}] }
                    }
                }
                ::graph::dfs 0
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let dfs = fixture_id_if_present(&cu.procedures, "::graph::dfs")
            .and_then(|id| cu.procedures.get(&id))
            .expect("dfs analysed");
        // Incoming activation alternatives do not manufacture one scalar
        // SSA symbol. If a physical template slot exists, its entry value
        // must still refuse a recursive caller's constant seed.
        let v0 = dfs
            .ssa
            .var_symbol("count")
            .and_then(|sym| dfs.sccp.values.get(&(sym, 0)));
        assert_eq!(
            dfs.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
        assert!(
            cu.caller_scope
                .call_sites
                .get("::graph::dfs")
                .is_some_and(|calls| calls
                    .slots
                    .get(&0)
                    .is_some_and(|slot| slot.unknown || slot.values.len() > 1))
        );
        assert!(
            !matches!(v0, Some(LatticeValue::Const(_))),
            "recursive param's version-0 lattice entry must not be Const, got {v0:?}",
        );
    }
}
