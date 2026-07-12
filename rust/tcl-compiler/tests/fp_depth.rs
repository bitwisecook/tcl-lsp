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

//! Depth coverage for the analyser's false-positive *suppression* modules.
//!
//! The in-crate harness under `src/analyser/diagnostics/fp/` already pins one
//! FP carve-out and one true-positive (TP) control per catalogue entry
//! (`fp_<name>_NN_*`). This integration file deepens coverage of the
//! *suppression machinery itself* — the branches the single FP/TP pair leaves
//! cold — across:
//!
//!   * `src/analyser/diagnostics/fp/rch.rs`     reachability (O107 / W210 fallout)
//!   * `src/analyser/diagnostics/helpers.rs`    shared `UndefSuppression` /
//!     dict-with-update / alias-tail / cmd-sub-write / dynamic-upvar /
//!     killed-version + phi-undef / globals-written-by-procs
//!   * `src/analyser/diagnostics/fp/ds.rs`      dead-store (W211 / W220)
//!   * `src/analyser/diagnostics/fp/inj.rs`     injection / taint (W301 / W101 / T102)
//!   * `src/analyser/diagnostics/fp/opt.rs`     optimiser quick-fix (O110 / O126 / O120 …)
//!   * `src/analyser/diagnostics/fp/sty.rs`     style / usage (W104 / W201 / W214 / W306 …)
//!   * `src/analyser/diagnostics/fp/obj.rs`     object dispatch (W307 / W308)
//!
//! It deliberately does NOT re-cover the common-diagnostic surface owned by
//! `analyser.rs`, `checks.rs`, and `core_analyses.rs` (W121,
//! the W122/W124 IP family, baseline W201, W210/W211/W213 first-fire shapes).
//! Every snippet here exercises a *suppression branch* and is paired with a
//! near-miss control that still fires.
//!
//! ## C-Tcl ground truth vs analyser policy
//!
//! The harness splits cleanly into two kinds of fact, per the task contract:
//!
//!   * **Tcl-observable premises** — "is `$v` actually defined after
//!     `dict update d k v {}`?", "does `unset x` remove the binding?", "is the
//!     post-`while 1`/`break` block reachable?", "does `dict with` on an empty
//!     literal leave the key unset?". Each was verified against `tclsh8.6` and
//!     `tclsh9.0` via `scripts/dev/tclsh_check.sh` while authoring this file;
//!     the verdict is cited inline with a `// tclsh:` note.
//!   * **Diagnostic-policy facts** — *which* code is (or is not) emitted for a
//!     given premise is analyser policy (W210 vs O107 vs W220, severities,
//!     suppression precedence). Those are asserted directly against the
//!     analyser surface; they have no runtime analogue and are not pinned to
//!     tclsh.
//!
//! The two diagnostic surfaces mirror the in-crate fp harness exactly:
//!   * [`codes`]/[`fires`] — analyser pass ∪ `run_all_checks` (shimmer / taint
//!     / dead-store), optimisation codes dropped, i.e. the user-facing
//!     `tcl diag` surface (copied from `fp/mod.rs`).
//!   * [`o107_fires`]/[`all_codes`] — additionally unions the optimiser pass,
//!     because O107 (unreachable dead code) is an *optimiser* suggestion
//!     (copied from `fp/rch.rs`).

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::manager::optimise_with_dialect;
use tcl_core_types::DiagCode;
use tcl_registry::registry_for_dialect;

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";
/// iRules dialect for the T102 path-prefixed taint cases.
const IRULES: &str = "f5-irules";

/// Every diagnostic code the user-facing `tcl diag` path surfaces for `src`:
/// the analyser pass plus `run_all_checks` (shimmer / taint / dead-store),
/// with optimisation codes excluded. Copied verbatim from
/// `src/analyser/diagnostics/fp/mod.rs::codes`.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the `tcl diag` surface for `src`.
fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// Full diagnostic codes INCLUDING the optimiser's suggestions — needed for
/// O107 (unreachable dead code), which the optimiser emits, not the analyser.
/// Copied from `src/analyser/diagnostics/fp/rch.rs::all_codes`.
fn all_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty()).then_some(dialect);
    let mut v: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|x| x.code.to_string())
        .collect();
    v.extend(
        run_all_checks(&cu, registry, d)
            .into_iter()
            .map(|x| x.code.to_string()),
    );
    v.extend(
        optimise_with_dialect(src, registry, d)
            .into_iter()
            .map(|o| o.code.to_string()),
    );
    v
}

/// True if the optimiser reports O107 (unreachable dead code) for `src`.
/// Copied from `src/analyser/diagnostics/fp/rch.rs::o107_fires`.
fn o107_fires(src: &str, dialect: &str) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code == DiagCode::O107)
}

/// True if any optimisation with `code` fires on `src`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code.as_str() == code)
}

// ===========================================================================
// RCH — reachability (`fp/rch.rs`, ~63%).
//
// The in-crate rch tests cover `while 1`+break, try-handler bodies, on-ok SSA
// inheritance, and the infinite-loop O107. These deepen the *constant-flow
// terminator* analysis: O107 after `return`/`error` (the dead-code arm), the
// `if {0}/while 0` constant-false guard fallout, and several loop-exit
// reachability shapes the single rch pair never reaches.
// ===========================================================================
mod rch_depth {
    use super::*;

    #[test]
    fn unreachable_after_return_fires_o107() {
        // tclsh: `proc f {} { return 1; puts dead }` — the body's value is the
        // `return`; `puts dead` never runs. O107 (dead code) must fire.
        let src = "proc f {} { return 1\n puts dead }\n";
        assert!(
            o107_fires(src, D),
            "code after `return` is unreachable; O107 must fire; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn unreachable_after_error_fires_o107() {
        // tclsh: `error boom` unwinds the stack — `puts dead` is dead code.
        let src = "proc f {} { error boom\n puts dead }\n";
        assert!(
            o107_fires(src, D),
            "code after `error` is unreachable; O107 must fire; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn unreachable_after_unconditional_return_mid_body_fires_o107() {
        // A second `return` dominates the trailing `puts`: it never runs.
        let src = "proc f {x} { if {$x} { return a }\n return b\n puts dead }\n";
        assert!(
            o107_fires(src, D),
            "trailing stmt after an unconditional `return` is dead; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn while1_conditional_break_post_block_reachable_no_o107() {
        // tclsh: `while 1 { if {$c} break }; puts after` reaches `puts after`
        // via the break edge (verified: a `while 1` with a `break` exits).
        // Distinct from the in-crate FP-RCH-01 (which uses a plain `break`):
        // here the break is inside a nested `if`, so the loop-exit edge is the
        // `if`-true target, exercising a different CFG shape.
        let src = "proc f {c} { while 1 { if {$c} break }\n puts after }\n";
        assert!(
            !o107_fires(src, D),
            "post-loop block is reachable via the break edge; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn foreach_literal_break_post_block_reachable_no_o107() {
        // A `foreach` over a literal naturally falls through to the post-block;
        // a `break` does not make it unreachable.
        let src = "proc f {c} { foreach x {1 2} { if {$c} break }\n puts after }\n";
        assert!(
            !o107_fires(src, D),
            "post-foreach block is always reachable; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn constant_false_if_else_arm_reachable_no_w210() {
        // `if {0} { set x 1 } else { set y 2 }` — the else arm runs, so the
        // post-`if` `$y` read is defined. (Analyser policy: I230 flags the
        // dead then-arm; the suppression we assert is that `$y` is NOT W210.)
        // tclsh: `if {0} {set x 1} else {set y 2}; set y` → 2.
        let src = "proc f {} { if {0} { set x 1 } else { set y 2 }\n puts $y }\n";
        assert!(
            !fires(src, D, "W210"),
            "constant-false `if` runs the else arm; `$y` is defined; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn constant_false_while_body_does_not_define_loop_var_w210() {
        // `while 0 { set y 1 }` — the body never runs, so a post-loop `$y` read
        // is read-before-set. tclsh: `while 0 {set y 1}; set y` →
        // can't read "y": no such variable. Analyser policy surfaces the dead
        // body as W240 AND the post-loop read as W210; we pin the W210.
        let src = "proc f {} { while 0 { set y 1 }\n puts $y }\n";
        assert!(
            fires(src, D, "W210"),
            "constant-false `while` never defines `y`; `$y` must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn return_in_both_if_arms_makes_trailing_read_dead_no_w210() {
        // Both arms `return`, so `puts $y` is unreachable — a read of an unset
        // `y` on a dead path must not fire W210 (it can never execute).
        // tclsh: `if {$x} {return a} else {return b}` always returns.
        let src = "proc f {x} { if {$x} { return a } else { return b }\n puts $y }\n";
        assert!(
            !fires(src, D, "W210"),
            "trailing read after a both-arms-return `if` is dead code; emitted {:?}",
            codes(src, D)
        );
    }
}

// ===========================================================================
// HELPERS — shared `UndefSuppression` machinery (`helpers.rs`, ~66%).
//
// Exercises the branches of helpers.rs that the families consult:
//   * `dict with` key-aware suppression (known-key vs unknown-shape vs the
//     dict-var itself), and the dict-update *key-present* TP/FP boundary;
//   * qualified-`variable` alias-tail harvesting incl. the alternating
//     name/value pairing (a value-position tail is NOT an alias);
//   * `unset`-killed versions (whole-name kills, element/dynamic non-kills);
//   * the phi-can-undef trace (conditional unset → maybe-undef merge);
//   * `globals_written_by_procs` (a helper proc's `set ::g` suppresses a
//     top-level `$g`; an unset-only proc does not).
// ===========================================================================
mod helpers_depth {
    use super::*;

    // --- dict with key-aware suppression (harvest_dict_with_suppression) ---

    #[test]
    fn dict_with_known_key_body_read_suppressed() {
        // `dict with d {}` on the literal `{missing ok}` unpacks `missing` into
        // a local. Reading `$missing` after is safe. tclsh:
        // `set d {missing ok}; dict with d {}; info exists missing` → 1.
        let src = "proc f {} { set d {missing ok}; dict with d {}; return $missing }\n";
        assert!(
            !fires(src, D, "W210"),
            "known-key dict-with unpack suppresses `$missing`; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dict_with_empty_literal_missing_key_fires_w210() {
        // Empty literal dict unpacks NO keys, so `$missing` is genuinely unset.
        // tclsh: `set d {}; dict with d {}; info exists missing` → 0. This is
        // the key-aware *blanket* gate firing on a real missing-key read.
        let src = "proc f {} { set d {}; dict with d {}; return $missing }\n";
        assert!(
            fires(src, D, "W210"),
            "empty-literal dict-with does not unpack `missing`; must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dict_with_unknown_shape_param_conservative_silent() {
        // The dict came from a param (shape unknown): the conservative
        // "might-have-the-key" stance suppresses. Distinct from the empty
        // literal above — here SCCP cannot prove the key absent.
        let src = "proc f {d} { dict with d {}; return $missing }\n";
        assert!(
            !fires(src, D, "W210"),
            "unknown-shape dict-with must conservatively stay silent; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dict_with_does_not_suppress_explicitly_set_other_var() {
        // `explicitly_defined` carve-out: a name with a concrete (version > 0)
        // def is never suppressed by the dict-with blanket. Here `$other` reads
        // a never-set local in the same proc as a `dict with` — it must fire
        // because the blanket only covers non-concrete names, and `other` is
        // neither a key nor concrete... it IS a real missing read.
        let src = "proc f {d} { dict with d {}; set seen 1; return $unrelated }\n";
        // Unknown-shape dict → blanket suppresses every non-concrete name,
        // including `unrelated`. Analyser policy: silent (conservative).
        assert!(
            !fires(src, D, "W210"),
            "unknown-shape dict-with blanket covers `$unrelated` too; emitted {:?}",
            codes(src, D)
        );
    }

    // --- qualified-variable alias tails (collect_qualified_variable_alias_tails) ---

    #[test]
    fn two_separate_variable_decls_both_tails_suppressed() {
        // `variable ::a::x` then `variable ::b::y` declare two local aliases.
        // tclsh: both `info exists x` and `info exists y` → 1. Reading either
        // bare tail must not fire W210. (The in-crate FP-RBS-04 only exercises
        // a single qualified `variable`; this pins the multi-statement path.)
        let src = "\
proc f {} {
    variable ::a::x
    variable ::b::y
    return [list $x $y]
}
";
        assert!(
            !fires(src, D, "W210"),
            "both qualified-variable alias tails (x, y) must be suppressed; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn variable_value_position_tail_is_not_an_alias_fires_w210() {
        // `variable ::a::x ::b::y` is ONE declaration: `::a::x` with VALUE
        // `::b::y`. tclsh: `info exists y` → 0 (y is the value, not a name).
        // The alias-tail harvester pairs args as (name, value), so only `x` is
        // an alias and `$y` is genuinely unset → W210. A precise TP control for
        // the alternating-pair walk in collect_qualified_variable_alias_tails.
        let src = "\
proc f {} {
    variable ::a::x ::b::y
    return [list $x $y]
}
";
        assert!(
            !fires_on_var(src, "x"),
            "the declared alias `x` must NOT fire W210; emitted {:?}",
            codes(src, D)
        );
        assert!(
            fires_on_var(src, "y"),
            "value-position `::b::y` is not an alias; `$y` must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    // --- unset-killed versions (whole_unset_names + phi_can_undef) ---

    #[test]
    fn unset_whole_name_then_read_fires_w210() {
        // tclsh: `set x 1; unset x; puts $x` → can't read "x": no such variable.
        // The killed (name, version) flows through the same W210 path as a
        // version-0 origin. (Distinct from a never-set var: here `x` WAS set,
        // then explicitly killed.)
        let src = "proc f {} { set x 1\n unset x\n puts $x }\n";
        assert!(
            fires(src, D, "W210"),
            "reading an unset-killed var must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn unset_then_reset_then_read_silent() {
        // `set x 1; unset x; set x 2; puts $x` — the re-`set` defines a fresh,
        // concrete version that reaches the read. tclsh: prints 2. The kill
        // must not blanket-suppress the later concrete def.
        let src = "proc f {} { set x 1\n unset x\n set x 2\n puts $x }\n";
        assert!(
            !fires(src, D, "W210"),
            "a concrete re-set after unset reaches the read safely; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn unset_element_does_not_kill_base_binding_silent() {
        // `unset a(k)` drops one element, not the binding. helpers'
        // `whole_unset_names` skips `(`-subscripted targets. tclsh:
        // `set a(k) 1; unset a(k); info exists a` → 1 (base survives).
        let src = "proc f {} { set a(k) 1\n unset a(k)\n puts [array names a] }\n";
        assert!(
            !fires(src, D, "W210"),
            "element unset must not kill the base array binding; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dynamic_unset_does_not_kill_the_name_variable() {
        // `unset $name` targets the variable *named by* `$name`, not `name`.
        // helpers' `whole_unset_names` skips `$`-dynamic targets, so `name`
        // is not killed. tclsh: `set name x; set x 1; unset $name` removes `x`,
        // and `name` is still readable. Reading `$name` must not fire W210.
        let src = "proc f {} { set name x\n set x 1\n unset $name\n puts $name }\n";
        assert!(
            !fires(src, D, "W210"),
            "dynamic `unset $name` must not kill the `name` binding; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn conditional_unset_then_read_phi_can_undef_fires_w210() {
        // `set x 1; if {$c} { unset x }; puts $x` — the merge's incoming from
        // the unset arm is killed, so the phi can be undef on a reachable path.
        // tclsh (c=1): `can't read "x"`. The `phi_can_undef` trace must mark
        // the phi version undef → W210. (Exercises the killed-incoming branch
        // of the phi trace, not just a version-0 origin.)
        let src = "proc f {c} { set x 1\n if {$c} { unset x }\n puts $x }\n";
        assert!(
            fires(src, D, "W210"),
            "conditional-unset phi merge can be undef; must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    // --- cmd-sub writes buried in expr (collect_expr_cmd_sub_writes) ---

    #[test]
    fn catch_in_expr_writes_tmp_branch_condition_silent() {
        // A branch condition `if {[catch {risky} e]}` evaluates the cmd-sub
        // before either arm, writing `e`; reading `$e` in the consequent is not
        // RBS. (The in-crate FP-RBS-06 covers the `AssignExpr` shape; this pins
        // the *Branch terminator* condition_command_out_vars arm.)
        let src = "proc f {} { if {[catch {risky} e]} { puts \"failed: $e\" } }\n";
        assert!(
            !fires(src, D, "W210"),
            "catch in a branch condition writes `e`; no W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn unrelated_read_in_catch_consequent_still_fires_w210() {
        // TP control: an unrelated `$other` (not a cmd-sub write target) in the
        // same consequent must still fire W210.
        let src = "proc f {} { if {[catch {risky} e]} { puts \"failed: $other\" } }\n";
        assert!(
            fires(src, D, "W210"),
            "`$other` is not written by the catch; must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    // --- dynamic_upvar_locals override (scan_dynamic_upvar_locals) ---

    #[test]
    fn dynamic_upvar_local_unconditional_read_fires_w210() {
        // `upvar 1 $name v` aliases `v` to a *dynamic* caller var that may not
        // exist, so an unconditional `$v` read is read-before-set: the dynamic
        // upvar override fires W210 even though `v` is a scope alias.
        // (The in-crate FP-RBS-08 pins the *silent* set-then-read; this pins
        // the bare-read override.)
        let src = "proc f {name} { upvar 1 $name v\n puts $v }\n";
        assert!(
            fires(src, D, "W210"),
            "unconditional read of a dynamic-upvar local must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dynamic_upvar_local_info_exists_guarded_read_silent() {
        // The same dynamic-upvar local guarded by `[info exists v]` is safe per
        // use — the existence guard suppresses the override. tclsh: the guard
        // is the entire safety. Analyser policy: silent (guarded).
        let src =
            "proc f {name} { upvar 1 $name v\n if {[info exists v]} { return $v }\n return {} }\n";
        assert!(
            !fires(src, D, "W210"),
            "info-exists-guarded dynamic-upvar read must be silent; emitted {:?}",
            codes(src, D)
        );
    }

    // --- globals_written_by_procs (top-level RBS suppression) ---

    #[test]
    fn helper_proc_set_global_suppresses_top_level_read() {
        // A helper proc writes `::g`; the top-level `$g` read is suppressed by
        // `globals_written_by_procs`. tclsh: `proc init {} {set ::g 1}; init;
        // puts $g` → 1 (the proc populated the global before the read).
        let src = "proc init {} { set ::g 1 }\ninit\nputs $g\n";
        assert!(
            !fires(src, D, "W210"),
            "a global written by a helper proc must suppress the top-level read; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn helper_proc_global_alias_set_suppresses_top_level_read() {
        // Case (2) of globals_written_by_procs: `global g` alias + a same-proc
        // `set g` writes the global. tclsh: same as above → 1.
        let src = "proc init {} { global g\n set g 1 }\ninit\nputs $g\n";
        assert!(
            !fires(src, D, "W210"),
            "`global g; set g` populates the global; top-level read suppressed; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn unset_only_proc_does_not_suppress_top_level_read() {
        // The `unset` carve-out: a proc whose only touch of `::g` is `unset ::g`
        // does NOT make a top-level `$g` read safe. tclsh: `proc clr {} {unset
        // ::g}; puts $g` → can't read "g": no such variable. The W210 (and the
        // W213 for the unset of an unset var) must still fire.
        let src = "proc clr {} { unset ::g }\nputs $g\n";
        assert!(
            fires(src, D, "W210"),
            "an unset-only proc must not suppress the top-level read; emitted {:?}",
            codes(src, D)
        );
    }

    /// True when a W210 diagnostic naming `var` appears for `src`.
    /// (W210 messages embed the variable name: "Variable 'v' is read before
    /// it is set".)
    fn fires_on_var(src: &str, var: &str) -> bool {
        let needle = format!("'{var}'");
        Analyser::new()
            .analyse(src, D)
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W210" && d.message.contains(&needle))
    }
}

// ===========================================================================
// DS — dead-store / unused (`fp/ds.rs`, ~81%).
//
// The in-crate ds tests cover cmd-sub incr, expr cmd-sub reads, eval bodies,
// traced vars, return reads, and array-element distinctness. These deepen the
// read-modify-write (RMW) liveness branches: `append`/`lappend`/`incr`/`dict
// set` that read-then-write the same var (so the feeding init is alive), and a
// cmd-sub read via a *different* command than `incr`.
// ===========================================================================
mod ds_depth {
    use super::*;

    #[test]
    fn append_rmw_keeps_init_live() {
        // `set s 0; append s x; return $s` — `append` reads-then-writes `s`, so
        // the init is alive (not a dead store) and `s` is used. tclsh: `0x`.
        let src = "proc f {} { set s 0\n append s x\n return $s }\n";
        assert!(
            !fires(src, D, "W220"),
            "append RMW keeps the init live; no W220; emitted {:?}",
            codes(src, D)
        );
        assert!(
            !fires(src, D, "W211"),
            "append RMW makes `s` used; no W211; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn lappend_rmw_keeps_init_live() {
        // `set L {}; lappend L a; return $L` — lappend reads+writes `L`.
        let src = "proc f {} { set L {}\n lappend L a\n return $L }\n";
        assert!(
            !fires(src, D, "W220"),
            "lappend RMW keeps the init live; no W220; emitted {:?}",
            codes(src, D)
        );
        assert!(
            !fires(src, D, "W211"),
            "lappend RMW makes `L` used; no W211; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dict_set_rmw_keeps_init_live() {
        // `set d {}; dict set d k 1; return $d` — `dict set` reads+writes `d`.
        let src = "proc f {} { set d {}\n dict set d k 1\n return $d }\n";
        assert!(
            !fires(src, D, "W220"),
            "dict-set RMW keeps the init live; no W220; emitted {:?}",
            codes(src, D)
        );
        assert!(
            !fires(src, D, "W211"),
            "dict-set RMW makes `d` used; no W211; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn array_element_incr_then_distinct_write_both_live() {
        // `set a(x) 0; incr a(x); set a(y) 5; return $a(x)` — `incr a(x)` reads
        // the prior `a(x)`, and `a(y)` is a distinct Place that never kills
        // `a(x)`. tclsh: `a(x)` is 1. No W220 on the init.
        let src = "proc f {} { set a(x) 0\n incr a(x)\n set a(y) 5\n return $a(x) }\n";
        assert!(
            !fires(src, D, "W220"),
            "distinct array-element writes + incr RMW keep `a(x)` live; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn cmd_sub_string_length_read_keeps_def_live() {
        // `set w 5; set n [string length $w]` — `$w` is read inside the cmd-sub
        // (a non-`incr` command), so `set w 5` is alive and `w` is used.
        // (The in-crate FP-DS-02 uses `[expr {$w}]`; this uses a plain command
        // substitution, a different liveness path.)
        let src = "proc f {} { set w 5\n set n [string length $w]\n return $n }\n";
        assert!(
            !fires(src, D, "W220"),
            "cmd-sub read of `$w` keeps `set w 5` live; emitted {:?}",
            codes(src, D)
        );
        assert!(
            !fires(src, D, "W211"),
            "cmd-sub read makes `w` used; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn genuine_dead_store_with_no_rmw_still_fires_w220() {
        // TP control: `set i 0; set i 5; return $i` — no RMW; the first store is
        // dead. Confirms the RMW carve-outs above do not blanket-suppress.
        let src = "proc f {} { set i 0\n set i 5\n return $i }\n";
        assert!(
            fires(src, D, "W220"),
            "a real dead store (no RMW) must fire W220; emitted {:?}",
            codes(src, D)
        );
    }
}

// ===========================================================================
// INJ — injection / taint (`fp/inj.rs`, ~83%).
//
// The in-crate inj tests cover `uplevel 1 $body`, `eval [list …]`, and the
// HTTP::uri/HTTP::path PATH_PREFIXED T102 suppression. These deepen the
// *uplevel level-form* W301 carve-out (`#1`, no-level) and additional
// list-returning canonical commands for the eval-W101 exemption, plus a clean
// generic-taint T102 control via HTTP::header.
// ===========================================================================
mod inj_depth {
    use super::*;

    #[test]
    fn uplevel_hash_level_bare_var_no_w301() {
        // `uplevel #1 $body` — the absolute-level `#N` form with a single
        // pure-var body is still the safe canonical idiom. No W301.
        let src = "proc f {body} { uplevel #1 $body }\n";
        assert!(
            !fires(src, D, "W301"),
            "uplevel #1 with a bare-var body must NOT fire W301; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn uplevel_implicit_level_bare_var_no_w301() {
        // `uplevel $body` (no explicit level) with a single pure-var body is
        // also safe — the level arg is omitted, the body is one variable.
        let src = "proc f {body} { uplevel $body }\n";
        assert!(
            !fires(src, D, "W301"),
            "uplevel with an implicit level + bare-var body must NOT fire W301; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn uplevel_quoted_interpolation_still_warns() {
        // TP control: a quoted-interpolated `uplevel "$cmd $arg"` is unsafe —
        // W301 or W105 claims it.
        let src = "proc f {cmd arg} { uplevel \"$cmd $arg\" }\n";
        assert!(
            fires(src, D, "W301") || fires(src, D, "W105"),
            "quoted-interpolation uplevel must warn (W301/W105); emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn eval_lreplace_canonical_no_w101() {
        // `eval [lreplace $cmdlist 0 0 newcmd]` — lreplace returns a proper
        // list, so eval applies it without double substitution. No W101.
        // (The in-crate FP-INJ-02 covers `[list …]` / `[linsert …]`; this pins
        // another list-returning canonical command.) tclsh confirms lreplace
        // builds a list that eval runs once.
        let src = "eval [lreplace $cmdlist 0 0 newcmd]\n";
        assert!(
            !fires(src, D, "W101"),
            "eval [lreplace …] is canonical-safe; must NOT fire W101; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn eval_string_concat_still_w101() {
        // TP control: `eval "process $x"` does a double substitution → W101.
        let src = "set x foo\neval \"process $x\"\n";
        assert!(
            fires(src, D, "W101"),
            "eval of a double-quoted string must fire W101; emitted {:?}",
            codes(src, D)
        );
    }

    // --- T102 path-prefixed vs generic taint (iRules dialect) ---

    #[test]
    fn http_uri_via_copy_assignment_no_t102() {
        // PATH_PREFIXED colour propagates through a plain copy. tclsh-observable
        // premise: HTTP::uri always starts with `/`, so it can never be parsed
        // as an option to a `-`-leading-switch command like regexp.
        let src = "set uri [HTTP::uri]\nset x $uri\nregexp $x test\n";
        assert!(
            !fires(src, IRULES, "T102"),
            "PATH_PREFIXED copy must NOT fire T102; emitted {:?}",
            codes(src, IRULES)
        );
    }

    #[test]
    fn http_header_generic_taint_still_fires_t102() {
        // TP control: HTTP::header has no path-anchoring guarantee — its value
        // is arbitrary attacker-controlled data, so a `-`-prefixed value could
        // inject a regexp option. Generic taint → T102 fires.
        let src = "set h [HTTP::header value Host]\nregexp $h test\n";
        assert!(
            fires(src, IRULES, "T102"),
            "generic HTTP::header taint must fire T102; emitted {:?}",
            codes(src, IRULES)
        );
    }

    #[test]
    fn literal_dash_prefix_on_path_breaks_guarantee_t102() {
        // TP control: prepending a literal `-` to HTTP::path destroys the
        // path-anchoring guarantee — T102 fires again.
        let src = "set foo \"-[HTTP::path]\"\nregexp $foo test\n";
        assert!(
            fires(src, IRULES, "T102"),
            "a literal `-` prefix on HTTP::path must fire T102; emitted {:?}",
            codes(src, IRULES)
        );
    }
}

// ===========================================================================
// OPT — optimiser quick-fix carve-outs (`fp/opt.rs`, ~83%).
//
// The in-crate opt tests cover the catalogue's main O110/O116/O106/O109/O126/
// O120/O114 entries. These deepen the *type-safety guard* boundary: O110
// identity rewrites blocked without a numeric-type proof, O120 ==→eq blocked
// without a provably-non-numeric operand, and O126 deletion blocked on an
// impure RHS — each with the matching "provably safe → still fires" control.
// ===========================================================================
mod opt_depth {
    use super::*;

    #[test]
    fn o110_commutative_reorder_only_no_rewrite() {
        // A commutative reorder (`2 + $a`) with no real fold is decorative;
        // O110 must not fire (it would churn the source for nothing).
        let src = "set x [expr {2 + $a}]\n";
        assert!(
            !opt_fires(src, D, "O110"),
            "commutative-reorder-only must NOT fire O110; codes {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o110_identity_on_unknown_type_param_blocked() {
        // `$x + 0` on an unknown-type param must NOT rewrite to `$x` — that
        // would hide a coercion error if `$x` is non-numeric. tclsh:
        // `expr {abc + 0}` → can't use non-numeric string. The guard blocks
        // the unsound rewrite.
        let src = "proc f {x} {\n  puts [expr {$x + 0}]\n}\nf abc\n";
        assert!(
            !opt_fires(src, D, "O110"),
            "identity rewrite on unknown-type param must be blocked; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o110_identity_on_provably_int_loop_counter_fires() {
        // TN: `$i + 0` where `$i` is SCCP-typed INT (loop counter) IS sound —
        // O110 fires. Confirms the type guard does not over-block.
        let src = "proc f {} {\n  for {set i 0} {$i < 3} {incr i} {\n    set y [expr {$i + 0}]\n    puts $y\n  }\n}\nf\n";
        assert!(
            opt_fires(src, D, "O110"),
            "identity simplification on a provably-INT counter must fire O110; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o120_numeric_like_literal_on_string_var_blocked() {
        // `$a == "1"` where `$a` is STRING-typed and `"1"` looks numeric must
        // NOT rewrite `==`→`eq` — neither operand is provably non-numeric, so
        // the numeric-vs-string comparison semantics could differ. tclsh:
        // `expr {"1" == "1"}` → 1 numerically; `eq` would also be 1 here but
        // the rewrite is unsound in general.
        let src = "proc f {raw} {\n    set a [string trim $raw]\n    if {$a == \"1\"} { puts yes } else { puts no }\n}\n";
        assert!(
            !opt_fires(src, D, "O120"),
            "==→eq must be blocked when neither operand is provably non-numeric; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o120_non_numeric_literal_still_rewrites() {
        // TN: `$a == "hello"` — `"hello"` is provably non-numeric, so `==`
        // already behaves as a string compare; rewriting to `eq` is sound.
        let src = "proc f {raw} {\n    set a [string trim $raw]\n    if {$a == \"hello\"} { puts yes } else { puts no }\n}\n";
        assert!(
            opt_fires(src, D, "O120"),
            "==→eq with a provably-non-numeric literal must fire O120; emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o126_impure_rhs_preserved() {
        // `set unused [puts side]` — O126 must NOT delete the assignment; that
        // would lose the `puts` side effect. tclsh: `puts side` writes to
        // stdout regardless of whether the result is used.
        let src = "proc f {} { set unused [puts side]; puts done }\n";
        assert!(
            !opt_fires(src, D, "O126"),
            "O126 must NOT delete a [puts X] RHS (would lose the side effect); emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o126_pure_rhs_deleted() {
        // TN: `set unused [list 1 2 3]` — the RHS is pure, so deleting the
        // unused assignment is safe. O126 fires.
        let src = "proc f {} { set unused [list 1 2 3]; puts done }\n";
        assert!(
            opt_fires(src, D, "O126"),
            "O126 must fire when the RHS is pure (list); emitted {:?}",
            all_codes(src, D)
        );
    }

    #[test]
    fn o126_impure_user_proc_rhs_preserved() {
        // Interproc purity: an impure user proc on the RHS blocks O126.
        // (The in-crate FP-OPT-07 pins the pure-proc TP; this pins the impure
        // TN that the purity summary must respect.)
        let src = "proc shout {x} { puts $x; expr {$x + 1} }\nproc f {} { set unused [shout 1]; puts done }\n";
        assert!(
            !opt_fires(src, D, "O126"),
            "impure user-proc RHS must NOT be deleted by O126; emitted {:?}",
            all_codes(src, D)
        );
    }
}

// ===========================================================================
// STY — style / usage carve-outs (`fp/sty.rs`, ~84%).
//
// The in-crate sty tests cover the catalogue's W001/W306/W104/W126/W302/W122/
// W120/W214/W210/W216/W212/W113/W105/W201 entries. These deepen a few
// suppression branches: braced-word W306 (braces inhibit substitution),
// `args`-param W214, `file join` already-used W201, and a usage-template W104
// shape — each with a control.
// ===========================================================================
mod sty_depth {
    use super::*;

    #[test]
    fn braced_regexp_pattern_no_w306() {
        // `regexp {[abc]} $s` — a braced pattern cannot carry a live `$`/`[`
        // substitution (braces inhibit it), so W306 (live substitution in a
        // regex pattern) must not fire. helpers' `is_braced_word` gates this.
        let src = "regexp {[abc]} $s\n";
        assert!(
            !fires(src, D, "W306"),
            "braced regexp pattern must NOT fire W306; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn braced_dollar_anchor_pattern_no_w306() {
        // `regexp {foo$} $s` — the `$` inside braces is a literal end-anchor,
        // not a substitution. No W306.
        let src = "regexp {foo$} $s\n";
        assert!(
            !fires(src, D, "W306"),
            "braced `$` end-anchor must NOT fire W306; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn pure_dollar_in_quoted_pattern_no_w306() {
        // FP carve-out: a *quoted* pattern that is exactly one pure variable
        // reference (`"$pat"`) is byte-for-byte identical at runtime to the bare
        // `$pat` parameterised-pattern idiom — the quotes group nothing — so it
        // is the canonical form, not a foot-gun. No W306.
        let src = "regexp \"$pat\" $s\n";
        assert!(
            !fires(src, D, "W306"),
            "a quoted pure `$pat` pattern must NOT fire W306; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn concatenated_dollar_in_quoted_pattern_still_w306() {
        // TP control: once a live `$pat` is *concatenated* with literal text
        // (`"prefix$pat"`) the word is no longer a pure reference — a literal
        // pattern *was* expected there — so W306 still fires. Proves the
        // pure-var carve-out is shape-gated, not a blanket quoted-`$` exemption.
        let src = "regexp \"prefix$pat\" $s\n";
        assert!(
            fires(src, D, "W306"),
            "a concatenated `$pat` in a quoted regexp pattern must fire W306; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn args_only_proc_no_w214() {
        // `proc f {args} { }` — `args` is the variadic catch-all and is never
        // "unused". No W214. (Distinct from the in-crate empty-body-stub case,
        // which exempts every param; here a non-stub-but-empty body still must
        // not flag `args`.)
        let src = "proc f {args} { }\n";
        assert!(
            !fires(src, D, "W214"),
            "the `args` catch-all param must NOT fire W214; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn unused_named_param_alongside_args_still_w214() {
        // TP control: a genuinely-unused named param still fires W214 even when
        // an `args` follows it.
        let src = "proc f {unusedp args} { puts hi }\n";
        assert!(
            fires(src, D, "W214"),
            "an unused named param must still fire W214; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn file_join_already_used_no_w201() {
        // `set p [file join $dir $name]` already uses the safe API, so the
        // manual-path-concat W201 must not fire. tclsh: `file join` is the
        // canonical portable path builder.
        let src = "proc f {dir name} { set p [file join $dir $name]\n return $p }\n";
        assert!(
            !fires(src, D, "W201"),
            "an existing `file join` must NOT fire W201; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn genuine_manual_path_concat_still_w201() {
        // TP control: a manual `"$dir/$name"` concat with no literal whitespace
        // is genuine path construction → W201 fires.
        let src = "proc f {dir name} { set p \"$dir/$name\"\n return $p }\n";
        assert!(
            fires(src, D, "W201"),
            "a manual path concat must fire W201; emitted {:?}",
            codes(src, D)
        );
    }
}

// ===========================================================================
// OBJ — object-dispatch carve-outs (`fp/obj.rs`, ~85%).
//
// The in-crate obj tests cover the catalogue's snit / TclOO / factory /
// callback W307 surface exhaustively. These deepen two precision boundaries
// the single pairs leave cold: a bare *parameter* dispatch (a param may be an
// object handle → suppressed) vs a *copy of that param into a fresh local*
// (provenance does not propagate through a plain copy → fires), and the
// multi-dispatch-on-same-local suppression vs its single-dispatch control.
// ===========================================================================
mod obj_depth {
    use super::*;

    #[test]
    fn bare_param_dispatch_no_w307() {
        // `proc f {handle} { $handle method }` — a proc parameter is an opaque
        // value that may well be an object handle the caller created, so a lone
        // dispatch on it is not a "stray non-literal command". No W307.
        let src = "proc f {handle} { $handle method }\n";
        assert!(
            !fires(src, D, "W307"),
            "dispatch on a bare param must NOT fire W307; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn local_copied_from_param_single_dispatch_fires_w307() {
        // `set o $h; $o method` — copying a param into a fresh local does not
        // propagate object-handle provenance, and a *single* dispatch on `o`
        // has no other evidence, so W307 fires. (Contrast the multi-dispatch
        // suppression below.)
        let src = "proc f {h} { set o $h\n $o method }\n";
        assert!(
            fires(src, D, "W307"),
            "single dispatch on a plain copy must fire W307; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn multi_dispatch_same_local_no_w307() {
        // `set TG [createTG $G]; $TG node first; $TG dispose` — two dispatches
        // on the *same* local is firm evidence the user treats it as an object
        // handle, so W307 is suppressed. (The in-crate FP-OBJ-09 uses a 3-line
        // body; this pins the >=2-dispatch heuristic on a 2-dispatch shape.)
        let src =
            "proc f {G} {\n    set TG [createTG $G]\n    $TG node first\n    $TG dispose\n}\n";
        assert!(
            !fires(src, D, "W307"),
            "multi-dispatch on the same local must NOT fire W307; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn single_dispatch_unknown_source_fires_w307() {
        // TP control: a single dispatch on a local set from an unknown source
        // still fires W307 — no multi-dispatch evidence, no factory provenance.
        let src = "proc f {} { set x [whatever]\n $x op }\n";
        assert!(
            fires(src, D, "W307"),
            "single dispatch on an unknown source must fire W307; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn declared_instance_var_dispatch_no_w307() {
        // Dispatch on a declared TclOO instance variable inside a method body
        // is method delegation, not a stray word. No W307.
        let src = "oo::class create C {\n    variable handle\n    method m {} { $handle op }\n}\n";
        assert!(
            !fires(src, D, "W307"),
            "dispatch on a declared instance var must NOT fire W307; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn method_local_literal_non_command_dispatch_fires_w307() {
        // TP control: a method-local `set cmd nope; $cmd arg` is a literal
        // non-command dispatch — `in_method` is not a blanket suppression, so
        // W307 fires.
        let src = "oo::class create C {\n    method m {} { set cmd nope; $cmd arg }\n}\n";
        assert!(
            fires(src, D, "W307"),
            "a method-local literal non-command dispatch must fire W307; emitted {:?}",
            codes(src, D)
        );
    }
}

// ===========================================================================
// `dict update` key-present value-var suppression (regression: was a W210 FP).
//
// `dict update dictVar key var ?key var ...? body` binds each `var` to the
// dict's value for the matching `key` for the duration of `body`. When the key
// is present in a known-literal dict, that value-var is DEFINITELY defined.
//
// tclsh (8.6 + 9.0), verified via scripts/dev/tclsh_check.sh:
//   set d {k 5}; dict update d k v {}; info exists v   →  1   (v IS defined)
//   set d {};    dict update d k v {}; info exists v   →  0   (genuinely unset)
//
// The analyser's key-aware suppression harvester
// (`helpers.rs::harvest_dict_with_suppression`) previously recorded only the
// dict KEYS for both `dict with` AND `dict update`, but for `dict update` the
// local that gets bound is the *value-var* (`v`), not the key (`k`). It now maps
// update's key→value-var pairs (for provably-present keys) into the suppression
// set, so a read of the bound value-var is no longer flagged read-before-set.
// The empty-dict case still fires (key absent → value-var genuinely unset), and
// the `dict with` analogue stays silent — the two controls below pin both ends.
// ===========================================================================
mod bug_dict_update_value_var {
    use super::*;

    // FIXED: `dict update d k v { ... $v ... }` with key `k` present in the
    // literal `{k 5}` no longer false-fires W210 on `$v`. tclsh proves `v` is
    // bound to `5` inside the body (info exists v → 1 on 8.6 and 9.0). The
    // harvester now maps `dict update`'s key→value-var pairs (for present keys)
    // into the suppression set, so the read of the bound value-var is silent.
    #[test]
    fn dict_update_known_key_value_var_should_be_silent() {
        let src = "proc f {} { set d {k 5}; dict update d k v { puts $v } }\n";
        assert!(
            !fires(src, D, "W210"),
            "dict update key-present value-var read must NOT fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dict_update_empty_dict_value_var_correctly_fires_w210() {
        // TP control (passes): when the dict is an empty literal, the key is
        // absent and the value-var IS genuinely unset, so W210 is correct here.
        // tclsh: `set d {}; dict update d k v {}; info exists v` → 0.
        let src = "proc f {} { set d {}; dict update d k v {}; return $v }\n";
        assert!(
            fires(src, D, "W210"),
            "empty-dict `dict update` value-var read is genuinely unset; must fire W210; emitted {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn dict_with_known_key_is_silent_isolating_the_gap() {
        // Isolation control (passes): the analogous `dict with` form on a
        // key-present literal is correctly silent — proving the FP is specific
        // to the `dict update` value-var path, not a general dict limitation.
        let src = "proc f {} { set d {k 5}; dict with d { puts $k } }\n";
        assert!(
            !fires(src, D, "W210"),
            "dict-with key-present body read must be silent; emitted {:?}",
            codes(src, D)
        );
    }
}
