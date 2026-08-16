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

//! Variable-lifecycle diagnostics (W210 / W211 / W220) across the *tricky* Tcl
//! surfaces of issue #941: `upvar` pass-by-reference, cross-scope globals,
//! namespaces, `variable`, `eval`, `trace`, `TclOO`, `interp`, the `::tcl` /
//! `::mathop` namespaces, and `args` parameters.
//!
//! ## Why these live together
//!
//! The unused-variable / read-before-set family is exactly where a
//! "tricky feature" leaks: a variable can look dead to single-unit dataflow
//! while a *different* Tcl scope reads or writes it — through an `upvar` alias,
//! the shared global namespace, a trace, or an inlined `eval` body. Each case
//! here is one of:
//!
//!   * **FP** — must stay **silent**: the variable *is* used through a tricky
//!     surface, so a diagnostic would be a false positive.
//!   * **TP** — must still **fire**: a genuine unused / read-before-set that the
//!     FP fixes must not mask.
//!   * **TN** — a tricky surface the analyser already models correctly and must
//!     keep modelling (regression guard).
//!
//! ## C-Tcl ground truth
//!
//! W210 ("read before set") and W213 ("unset nothing") mirror a real
//! `can't read` / `can't unset` runtime error, so their reproducers are pinned
//! to `tclsh` behaviour. W211 / W220 are pure liveness lints with no direct
//! runtime analogue, but the *binding* facts they depend on (an `upvar` alias
//! writing the caller cell, a `global` read seeing a top-level `set`, a trace
//! observing a write) were confirmed against real `tclsh8.6` (and hold
//! identically on `tclsh9.0` — the C Tcl 9 truth oracle — for every construct
//! here, which uses only 8.4-era scoping semantics). Citations use `// tclsh:`.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::ir::Statement;
use tcl_registry::{CommandRegistry, registry_for_dialect};

/// Default dialect: the C Tcl 9 truth oracle (issue #941). The scoping
/// constructs exercised here behave identically on 8.4–9.0.
const D: &str = "tcl9.0";

/// Every diagnostic code the full user-facing pipeline surfaces for `src`
/// under `dialect`: the analyser pass plus the `run_all_checks` compiler-checks
/// pass, with optimisation codes excluded — exactly what `tcl diag` reports.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    for d in run_all_checks(
        &cu,
        registry,
        Some(tcl_dialect::DialectProfile::by_name(dialect)),
    ) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True when `code` appears anywhere in the merged diagnostic set for `src`.
fn fires(src: &str, code: &str) -> bool {
    codes(src, D).iter().any(|c| c == code)
}

/// True when *none* of the variable-lifecycle codes fire for `src` — the
/// assertion an FP / TN reproducer wants.
fn lifecycle_silent(src: &str) -> bool {
    !codes(src, D)
        .iter()
        .any(|c| matches!(c.as_str(), "W210" | "W211" | "W213" | "W220"))
}

// ===========================================================================
// upvar — dynamic-target pass-by-reference (the #941 headline FP)
//
// `upvar ?level? otherVar myVar` binds the *local* `myVar` to the caller-frame
// variable named by `otherVar`. `otherVar` may be a literal *or* a runtime
// value (`$name`); the two are semantically identical — each errors only when
// the caller variable is missing. The analyser used to flag the dynamic form
// alone, firing on the single most common Tcl pass-by-reference idiom.
// ===========================================================================
mod upvar_dynamic_target {
    use super::*;

    #[test]
    fn fp_dynamic_upvar_pure_read_is_silent() {
        // tclsh: `proc f {n} {upvar 1 $n a; return $a}; set x 9; f x` → 9 (no
        // error). Reading the alias is not read-before-set.
        assert!(
            lifecycle_silent("proc f {n} { upvar 1 $n a\n return $a }\n"),
            "dynamic upvar alias read fired: {:?}",
            codes("proc f {n} { upvar 1 $n a\n return $a }\n", D)
        );
    }

    #[test]
    fn fp_dynamic_upvar_array_getter_is_silent() {
        // The canonical accessor helper: alias an array by name, read an
        // element. tclsh: works whenever the caller array exists.
        assert!(lifecycle_silent(
            "proc getField {arrayName field} { upvar 1 $arrayName arr\n return $arr($field) }\n"
        ));
    }

    #[test]
    fn fp_dynamic_upvar_hash_level_is_silent() {
        // `upvar #0 $global local` — alias a namespaced global by computed name
        // (the tcllib picoirc idiom). Must be silent like the relative form.
        assert!(lifecycle_silent(
            "proc f {ctx} { upvar #0 $ctx state\n return $state }\n"
        ));
    }

    #[test]
    fn parity_literal_and_dynamic_upvar_agree() {
        // The whole point: the literal- and dynamic-target twins must reach the
        // *same* verdict (both silent), because tclsh treats them identically.
        let dynamic = lifecycle_silent("proc f {n} { upvar 1 $n a\n return $a }\n");
        let literal = lifecycle_silent("proc f {} { upvar 1 outer a\n return $a }\n");
        assert!(dynamic && literal, "dynamic={dynamic} literal={literal}");
    }

    #[test]
    fn tp_unrelated_local_after_upvar_still_fires() {
        // TP control: `$unrelated` is not the alias, is never set, and is not a
        // scope alias — a genuine read-before-set. tclsh: `can't read
        // "unrelated"`.
        assert!(
            fires("proc f {n} { upvar 1 $n a\n return $unrelated }\n", "W210"),
            "unrelated unset local must fire W210"
        );
    }

    #[test]
    fn tn_dynamic_upvar_write_through_alias_is_not_dead_store() {
        // TN: writing through the alias reaches the caller cell, so it is not a
        // dead store even though this frame never reads it back. tclsh:
        // `proc f {n} {upvar 1 $n a; set a 5}; set x 0; f x; set x` → 5.
        assert!(lifecycle_silent("proc f {n} { upvar 1 $n a\n set a 5 }\n"));
    }

    #[test]
    fn tn_guarded_dynamic_upvar_read_is_silent() {
        // An `[info exists a]` guard makes the read safe on every path.
        assert!(lifecycle_silent(
            "proc f {n} { upvar 1 $n a\n if {[info exists a]} { return $a }\n return {} }\n"
        ));
    }
}

// ===========================================================================
// Cross-scope globals — the top-level script and every proc share the global
// namespace, so a top-level `set` is "used" when a proc reads it (and the
// existing reverse direction, a proc write feeding a top-level read).
// ===========================================================================
mod cross_scope_globals {
    use super::*;

    #[test]
    fn fp_top_level_global_read_by_proc_via_global_decl() {
        // tclsh: `proc f {} {global cfg; return $cfg}; set cfg 1; f` → 1. The
        // top-level `set cfg 1` feeds the proc, so it is not a dead store.
        assert!(lifecycle_silent(
            "proc f {} { global cfg\n return $cfg }\nset cfg 1\nputs [f]\n"
        ));
    }

    #[test]
    fn fp_top_level_global_read_by_proc_via_qualified_ref() {
        // Same, read through a fully-qualified `$::cfg` rather than a `global`
        // declaration.
        assert!(lifecycle_silent(
            "proc f {} { return $::cfg }\nset cfg 1\nputs [f]\n"
        ));
    }

    #[test]
    fn fp_top_level_namespace_var_read_by_proc() {
        // A namespaced `set ::n::cfg 1` consumed by `variable cfg; $cfg` inside
        // an `::n::` proc. The qualified top-level name is externally
        // consumable, so W211 must not fire.
        assert!(lifecycle_silent(
            "namespace eval ::n {}\nproc ::n::f {} { variable cfg\n return $cfg }\nset ::n::cfg 1\n"
        ));
    }

    #[test]
    fn tn_reverse_direction_proc_writes_global_top_reads() {
        // The pre-existing mirror: a proc populates the global before the
        // top-level read. Must stay silent (regression guard for
        // `globals_written_by_procs`).
        assert!(lifecycle_silent(
            "proc setup {} { global cfg\n set cfg 1 }\nsetup\nputs $cfg\n"
        ));
    }

    #[test]
    fn tp_genuine_top_level_dead_store_still_fires() {
        // TP: no proc (and no top-level reader) ever reads `cfg`, so the
        // top-level `set` is genuinely unused. The FP fix must not mask this.
        assert!(
            fires("proc f {} { return 0 }\nset cfg 1\nputs [f]\n", "W211"),
            "a top-level global no proc reads must still fire W211"
        );
    }

    #[test]
    fn tp_namespaced_read_does_not_mask_unrelated_bare_global() {
        // TP (regression, code-review finding): a proc reading a *namespaced*
        // variable `$::n::cfg` must not be mistaken for a read of the
        // unrelated bare global `::cfg` merely because they share a tail
        // name — the two are different storage cells. Collapsing a
        // qualified read to its last segment (rather than keeping the
        // remainder after the leading `::`, as `globals_written_by_procs`
        // does) would wrongly suppress this genuinely-unused top-level `cfg`.
        assert!(
            fires(
                "namespace eval ::n {}\nproc reader {} { return $::n::cfg }\nset cfg 1\nputs done\n",
                "W211"
            ),
            "a namespaced read of ::n::cfg must not suppress the unrelated bare ::cfg dead store"
        );
    }

    #[test]
    fn tp_bare_local_unused_still_fires() {
        // TP: an ordinary proc-local that is set and never read.
        assert!(fires("proc f {} { set tmp 5\n return 1 }\n", "W211"));
    }
}

// ===========================================================================
// Tricky surfaces the analyser already models — regression guards (TN).
// Each `set` here is consumed through a feature that is NOT a plain `$var`
// read in the same frame; the analyser must keep recognising the use.
// ===========================================================================
mod already_modelled_surfaces {
    use super::*;

    #[test]
    fn tn_namespace_variable_used_by_qualified_proc() {
        // A namespace `variable` written in the eval body and read in a member
        // proc — a real cross-proc use.
        assert!(lifecycle_silent(
            "namespace eval ::a { variable counter 0 }\nproc ::a::bump {} { variable counter\n incr counter }\n"
        ));
    }

    #[test]
    fn tn_eval_body_uses_local_in_current_frame() {
        // `eval {…}` runs in the current frame; the inlined body's read counts.
        assert!(lifecycle_silent("proc f {} { set c 5\n eval {incr c} }\n"));
    }

    #[test]
    fn tn_trace_add_variable_only_var_is_observed() {
        // A write trace observes every write, so `set x 1` is not dead even with
        // no `$x` read. tclsh: the trace callback fires on the write.
        assert!(lifecycle_silent(
            "proc f {} { set x 0\n trace add variable x write ::onwrite\n set x 1 }\nproc onwrite {a b op} {}\n"
        ));
    }

    #[test]
    fn tn_tcloo_instance_variable_across_methods() {
        // A TclOO instance `variable` written in one method, read in another —
        // a cross-method use through the object's private namespace.
        assert!(lifecycle_silent(
            "oo::class create C {\n variable n\n method inc {} { incr n }\n method get {} { return $n }\n}\n"
        ));
    }

    #[test]
    fn tn_tcloo_my_variable_binding() {
        // `my variable state` binds the instance variable into method scope.
        assert!(lifecycle_silent(
            "oo::class create C {\n method a {} { my variable state\n set state 1 }\n method b {} { my variable state\n return $state }\n}\n"
        ));
    }

    #[test]
    fn tn_args_parameter_collects_rest() {
        // The variadic `args` parameter is consumed by `llength`.
        assert!(lifecycle_silent(
            "proc f {args} { return [llength $args] }\n"
        ));
    }

    #[test]
    fn tn_default_parameter_used_in_body() {
        // An optional parameter with a default, used in the body.
        assert!(lifecycle_silent(
            "proc f {a {b 5}} { return [expr {$a + $b}] }\n"
        ));
    }

    #[test]
    fn tn_mathop_namespace_command_result_used() {
        // `::tcl::mathop::+` is a real command whose result is consumed.
        assert!(lifecycle_silent(
            "set r [::tcl::mathop::+ 1 2 3]\nputs $r\n"
        ));
    }

    #[test]
    fn tn_scan_writes_vars_by_name_then_read() {
        // `scan` writes `a` / `b` by name; both are then read.
        assert!(lifecycle_silent(
            "proc f {s} { scan $s \"%d %d\" a b\n return [expr {$a + $b}] }\n"
        ));
    }

    #[test]
    fn tn_dict_with_injects_keys_as_locals() {
        // `dict with d {…}` unpacks the dict keys as locals in the body.
        assert!(lifecycle_silent(
            "proc f {d} { dict with d { return $name } }\n"
        ));
    }
}

// ===========================================================================
// Read-before-set TPs on tricky surfaces — the FP suppressions above must not
// leak into genuine errors.
// ===========================================================================
mod read_before_set_tps {
    use super::*;

    #[test]
    fn tp_plain_read_before_set() {
        // tclsh: `proc f {} {puts $u}` → `can't read "u"`.
        assert!(fires("proc f {} { puts $u }\n", "W210"));
    }

    #[test]
    fn tp_namespace_local_never_set_read() {
        // A bare local in a namespaced proc, never set nor aliased, read
        // directly — a genuine read-before-set.
        assert!(fires(
            "namespace eval ::n {}\nproc ::n::f {} { return $missing }\n",
            "W210"
        ));
    }

    /// Issue #1403 — safe read-modify-write is a registry and dialect fact,
    /// rather than a blanket `reads_own_defs` exemption. Tcl 8.6's declared
    /// self-initialisers are silent, while `incr` under Tcl 8.4 keeps W210.
    #[test]
    fn safe_on_uninit_is_lowered_from_the_active_dialect_profile() {
        for source in [
            "proc f {} { append value x }\n",
            "proc f {} { lappend value x }\n",
            "proc f {} { dict set value key x }\n",
            "proc f {} { incr value }\n",
        ] {
            assert!(
                !codes(source, "tcl8.6").iter().any(|code| code == "W210"),
                "safe Tcl 8.6 self-initialiser fired W210: {source:?}"
            );
        }
        assert!(
            codes("proc f {} { incr value }\n", "tcl8.4")
                .iter()
                .any(|code| code == "W210"),
            "Tcl 8.4 must not inherit Tcl 8.5+'s incr self-initialisation"
        );
        for source in [
            "proc f {} { append value x }\n",
            "proc f {} { lappend value x }\n",
        ] {
            assert!(
                !codes(source, "tcl8.4").iter().any(|code| code == "W210"),
                "safe Tcl 8.4 self-initialiser fired W210: {source:?}"
            );
            assert!(
                !codes(source, "f5-irules").iter().any(|code| code == "W210"),
                "safe iRules self-initialiser fired W210: {source:?}"
            );
        }
        assert!(
            codes("proc f {} { incr value }\n", "f5-irules")
                .iter()
                .any(|code| code == "W210"),
            "iRules' Tcl 8.4 base must not inherit Tcl 8.5+'s incr self-initialisation"
        );
    }

    #[test]
    fn profileless_registry_abstains_in_the_lowered_ir() {
        // A profile-less registry is the union command table, not evidence of
        // one release's runtime behaviour. The typed IR must therefore retain
        // the conservative false fact before W210 consumes it.
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("proc f {} { incr value }\n", &registry, false);
        let f = cu.function("::f").expect("procedure analysed");
        assert!(
            f.cfg
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .any(|stmt| {
                    matches!(
                        stmt,
                        Statement::Incr {
                            safe_on_uninit: false,
                            ..
                        }
                    )
                }),
            "profile-less lowering must not claim Tcl 8.5+ incr semantics"
        );
    }
}
