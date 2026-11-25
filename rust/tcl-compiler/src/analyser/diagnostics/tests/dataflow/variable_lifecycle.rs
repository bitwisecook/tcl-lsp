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

//! How a variable comes to exist and stops existing: startup bindings, the commands that bind their targets, `unset` and guards.

use super::*;

#[test]
fn w210_uses_registry_owned_startup_lifecycle_facts() {
    // Availability in SPECIAL_VARS is intentionally broader than startup
    // readability.  Build this probe from the central table so a new
    // default global gains W210 coverage automatically; the registry's own
    // exact fixture guards the release-by-release inventory.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let context = tcl_registry::model::ingress::static_context_for(dialect);
        let registry = context.commands();
        let query = Some(context.context().authoring_query());
        let source: String = registry
            .special_vars_for_dialect(query)
            .filter(|spec| registry.startup_existence(spec.name, query).readable())
            .filter_map(|spec| match spec.kind {
                tcl_registry::special_vars::SpecialVarKind::Scalar => {
                    Some(format!("puts ${}\n", spec.name))
                }
                // Array elements are normalised to their registry-owned base
                // by the SSA model.  The synthetic key need not exist at
                // runtime; the assertion is specifically about W210's entry
                // binding knowledge, not an array-key value claim.
                tcl_registry::special_vars::SpecialVarKind::Array => {
                    Some(format!("puts ${}(__tcl_lsp_startup_probe__)\n", spec.name))
                }
                tcl_registry::special_vars::SpecialVarKind::Namespace => None,
            })
            .collect();
        let result = Analyser::new().analyse(&source, dialect);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "startup-readable special vars must not emit W210 for {dialect}: {:?}",
            result.diagnostics
        );
    }

    // Controls make the test mutation-sensitive: these names remain special
    // / documented but are not default-readable in the cited lifecycle.
    for name in ["auto_index", "auto_execs", "auto_noexec", "auto_noload"] {
        let result = Analyser::new().analyse(&format!("puts ${name}\n"), "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "lazy special variable {name} must still emit W210: {:?}",
            result.diagnostics
        );
    }
    let result = Analyser::new().analyse("puts $tcl_precision\n", "tcl9.0");
    assert!(result.diagnostics.iter().any(|d| d.code == DiagCode::W210));
    let result = Analyser::new().analyse("puts $argv\n", "f5-irules");
    assert!(result.diagnostics.iter().any(|d| d.code == DiagCode::W210));

    // Startup facts are global-only and version-zero-only.  They must not
    // leak into a fresh procedure local, nor survive an explicit deletion.
    for src in [
        "proc local {} { puts $argv }\n",
        "proc local_return {} { return $argv }\n",
        "unset argv\nputs $argv\n",
        "unset ::argv\nputs $::argv\n",
    ] {
        let result = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "{src:?} must retain W210 after scope/lifecycle handling: {:?}",
            result.diagnostics
        );
    }
    let result = Analyser::new().analyse("proc global {} { global argv; puts $argv }\n", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "an explicit global alias must retain the startup binding: {:?}",
        result.diagnostics
    );

    // The startup version can also be one incoming of an SSA phi.  A
    // conditional write must not turn its untouched global incoming into a
    // synthetic undef; this covers both ordinary statement reads and return
    // values, whose phi walk is a separate consumer of the shared fact.
    for src in [
        "if {[info exists tcl_interactive]} { set argv replacement }\nputs $argv\n",
        "if {[info exists tcl_interactive]} { set argv replacement }\nreturn $argv\n",
    ] {
        let result = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "a phi including startup-bound argv must remain defined for {src:?}: {:?}",
            result.diagnostics
        );
    }
    let result = Analyser::new().analyse(
        "if {[info exists tcl_interactive]} { unset argv }\nputs $argv\n",
        "tcl8.6",
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "an unset incoming must keep the phi possibly undefined: {:?}",
        result.diagnostics
    );

    // `tcl_precision` is the Tcl 8.x registry-owned read-trace exception:
    // unlike an eager startup binding, reading it after `unset` materialises
    // its value again. Exercise both W210 consumers and the Tcl 9 boundary.
    for src in [
        "unset tcl_precision\nputs $tcl_precision\n",
        "unset tcl_precision\nreturn $tcl_precision\n",
    ] {
        let result = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "Tcl 8 read trace must rematerialise after unset for {src:?}: {:?}",
            result.diagnostics
        );
        let result = Analyser::new().analyse(src, "tcl9.0");
        assert!(
            result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "Tcl 9 must not inherit Tcl 8 read-trace behaviour for {src:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn i230_existence_fold_abstains_on_interpreter_globals_at_top_level() {
    // The `[info exists X]` / `[array exists X]` fold would otherwise decide
    // "never assigned in this body, therefore absent".  In the initial global
    // frame that body *is* the interpreter's global namespace, so every name
    // the special-variable registry owns there is out-of-frame runtime state:
    // startup-bound (`argv`, `env`, `tcl_platform`), materialised by a later
    // event (`errorInfo` after a `catch`, `auto_index` after an auto-load), or
    // read-traced (`tcl_precision` on Tcl 8.x).  Folding them false produced a
    // false I230 and an O101 rewrite of `if {[info exists argv]} …` to
    // `if {0} …`.  Drive the probe from the table so a new global is covered
    // automatically.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let source: String = tcl_registry::special_vars::special_vars_for_dialect(
            Some(tcl_registry::special_vars::surface_query_for_profile(Some(
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        ))),
        )
            .filter_map(|spec| match spec.kind {
                tcl_registry::special_vars::SpecialVarKind::Scalar => {
                    Some(format!("if {{[info exists {}]}} {{ puts hit }}\n", spec.name))
                }
                tcl_registry::special_vars::SpecialVarKind::Array => Some(format!(
                    "if {{[info exists {0}(k)]}} {{ puts hit }}\nif {{[array exists {0}]}} {{ puts hit }}\n",
                    spec.name
                )),
                tcl_registry::special_vars::SpecialVarKind::Namespace => None,
            })
            .collect();
        let result = Analyser::new().analyse(&source, dialect);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::I230),
            "an interpreter global's existence must not fold in {dialect}: {:?}",
            result.diagnostics
        );
    }

    // Mutation controls.  The abstention is dialect-versioned and frame-local:
    // a release that never had the variable keeps folding it (tclsh 9.0.4 /
    // 9.1b0: `info exists tcl_precision` → 0), a dialect that never had it
    // keeps folding it (iRules has no `argv`), a procedure-local name of the
    // same spelling is an ordinary fresh Tcl variable (tclsh 8.4.20 / 8.5.19 /
    // 8.6.14 / 9.0.4 / 9.1b0: `proc p {} {info exists argv}` → 0), and an
    // ordinary never-assigned global still folds.
    for (src, dialect) in [
        ("if {[info exists tcl_precision]} { puts hit }\n", "tcl9.0"),
        ("if {[info exists tcl_precision]} { puts hit }\n", "tcl9.1"),
        ("if {[info exists tcl_libPath]} { puts hit }\n", "tcl8.5"),
        (
            "proc p {} { if {[info exists argv]} { puts hit } }\n",
            "tcl8.6",
        ),
        ("if {[info exists no_such_global]} { puts hit }\n", "tcl8.6"),
    ] {
        let result = Analyser::new().analyse(src, dialect);
        assert!(
            result.diagnostics.iter().any(|d| d.code == DiagCode::I230),
            "{src:?} must still fold in {dialect}: {:?}",
            result.diagnostics
        );
    }

    // `tcl_libPath` is the Tcl 8.4-only `Tcl_Init` global (`tclUnixInit.c` /
    // `tclWinInit.c`; Tcl 8.5's `tclInterp.c` records it as OBSOLETE), so the
    // 8.4 side must abstain where 8.5+ folds.
    let result = Analyser::new().analyse("if {[info exists tcl_libPath]} { puts hit }\n", "tcl8.4");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::I230),
        "Tcl 8.4 tcl_libPath must not fold: {:?}",
        result.diagnostics
    );
}

#[test]
fn w210_startup_read_trace_respects_global_alias_and_unset_lifecycle() {
    // The trace belongs to the global storage cell even when that cell is
    // reached through `global` or an explicit `::` spelling.  A same-named
    // procedure local remains an ordinary local and must not inherit it.
    for src in [
        "proc global_lazy {} { global tcl_precision; set seen $tcl_precision; unset tcl_precision; puts $tcl_precision }\n",
        "proc global_lazy_return {} { global tcl_precision; set seen $tcl_precision; unset tcl_precision; return $tcl_precision }\n",
        "set seen $::tcl_precision\nunset ::tcl_precision\nputs $::tcl_precision\n",
    ] {
        let result = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
            "Tcl 8 global read trace must rematerialise for {src:?}: {:?}",
            result.diagnostics
        );
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W213),
            "a prior Tcl 8 trace read makes its unset valid for {src:?}: {:?}",
            result.diagnostics
        );
    }
    let result = Analyser::new().analyse(
        "proc local_lazy {} { unset tcl_precision; puts $tcl_precision }\n",
        "tcl8.6",
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "a same-named procedure local must remain undefined: {:?}",
        result.diagnostics
    );

    // Readability through a trace is not eager existence.  In a fresh Tcl 8
    // interpreter `unset tcl_precision` itself errors; once an ordinary read
    // has materialised it, the unset is valid.  Eager argv is valid to unset
    // immediately, yet its later read remains W210.
    let fresh_unset = Analyser::new().analyse("unset tcl_precision\n", "tcl8.6");
    assert!(
        fresh_unset
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "a fresh lazy trace must not be mistaken for an eager binding: {:?}",
        fresh_unset.diagnostics
    );
    let materialised_unset = Analyser::new().analyse(
        "set observed $tcl_precision\nunset tcl_precision\n",
        "tcl8.6",
    );
    assert!(
        !materialised_unset
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "a prior trace read must make its unset valid: {:?}",
        materialised_unset.diagnostics
    );
    let eager_unset = Analyser::new().analyse("unset argv\nputs $argv\n", "tcl8.6");
    assert!(
        !eager_unset
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "an eagerly-bound startup global is safe to unset: {:?}",
        eager_unset.diagnostics
    );
    assert!(
        eager_unset
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "the later argv read must still see the deletion: {:?}",
        eager_unset.diagnostics
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_suppressed_when_proc_writes_global() {
    // A helper proc ``init`` writes ``::counter`` via ``set``, so a
    // top-level read after ``init`` runs does not flag W210. Without the
    // call tclsh 8.6 fails with `can't read "counter"`.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc init {} { set ::counter 0 }\nputs $counter");
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "the uncalled writer leaves ::counter unset; got {:?}",
        a.result.diagnostics,
    );
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "proc init {} { set ::counter 0 }\ninit\nputs $counter",
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must be suppressed for globals written by procs; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_suppressed_via_global_alias() {
    // ``proc init {} { global counter; set counter 0 }`` — the
    // ``global`` declaration aliases the proc-local ``counter``
    // to the global.  Top-level read should not flag W210.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "proc init {} { global counter; set counter 0 }\ninit\nputs $counter",
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must be suppressed via global-alias case; got {:?}",
        a.result.diagnostics,
    );
}

// Registry-wide VarWrite binding (handle_var_binding_command).
//
// The binder takes no command-name gate: every command whose registry spec
// marks VarWrite-role arguments binds its literal targets.

#[test]
fn var_binding_gets_defines_target_and_read_is_clean() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {chan} {\n    gets $chan line\n    puts $line\n}\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "gets writes its line variable — reading it back must not fire W210; got {:?}",
        r.diagnostics
    );
    assert!(
        r.all_variables.contains_key("f::line"),
        "gets' line target must be a recorded definition; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

#[test]
fn var_binding_binary_scan_defines_targets() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {d} {\n    binary scan $d c v\n    puts $v\n}\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "binary scan can consume no fields; a possible target is not a definite write: {:?}",
        r.diagnostics
    );
    assert!(
        r.all_variables.contains_key("f::v"),
        "binary scan's target must be a recorded definition; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
    assert!(
        !Analyser::new()
            .analyse(
                "proc f {d} {set v 0; binary scan $d c v; puts $v}",
                "tcl8.6",
            )
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W210),
        "an existing capture remains defined when binary scan consumes no fields"
    );
}

#[test]
fn var_binding_a_nested_loop_command_binds_its_loop_variables_in_the_enclosing_scope() {
    // A loop command nested in a `[…]` substitution binds its loop variables
    // in the scope the substitution runs in, through the same role binder as
    // the top-level path.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {l d} {\n    set r [lmap x $l {string length $x}]\n    \
         set s [foreach y $l {}]\n    set t [dict for {k v} $d {}]\n    \
         return [list $r $s $t $x $y $k $v]\n}\n",
        "tcl8.6",
    );
    for name in ["x", "y", "k", "v"] {
        assert!(
            r.all_variables.contains_key(&format!("f::{name}")),
            "{name} must be bound in `f`; got {:?}",
            r.all_variables.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn var_binding_vwait_defines_target() {
    // `vwait done` returns only after an event handler wrote `done`
    // (`Tcl_VwaitObjCmd` traces WRITES|UNSETS and never reads the value),
    // so the operand is a definition site, and a read after the wait is
    // clean.
    let mut a = Analyser::new();
    let r = a.analyse("vwait done\nputs $done\n", "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W210),
        "a read after `vwait done` is guaranteed defined; got {:?}",
        r.diagnostics
    );
    assert!(
        r.all_variables.contains_key("::::done"),
        "vwait's operand must be a recorded definition; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

#[test]
fn var_binding_double_bind_with_set_is_idempotent() {
    // `set` has a dedicated handler that binds first; the generic
    // VarWrite binder re-binding the same token must not push a duplicate
    // self-reference or downgrade the dedicated handler's
    // `warn_if_unused = true`.
    let mut a = Analyser::new();
    let r = a.analyse("set x 5\n", "tcl8.6");
    let var = r
        .global_scope
        .variables
        .get("x")
        .expect("set must define x");
    assert_eq!(
        var.references.len(),
        0,
        "the definition site must not be double-recorded as a reference; got {:?}",
        var.references
    );
    assert!(
        var.warn_if_unused,
        "the generic binder's warn_if_unused=false must not downgrade set's true"
    );
}

#[test]
fn var_binding_scope_alias_and_destroy_families_are_excluded() {
    // CREATES_SCOPE_ALIAS: `global ::ns::v` binds the local alias tail
    // `v` (its dedicated handler's layout), never the qualified name a
    // flat VarWrite walk would record.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {} {\n    global ::ns::v\n    set v 1\n}\n",
        "tcl8.6",
    );
    assert!(
        r.all_variables.contains_key("f::v"),
        "global's dedicated handler binds the alias tail; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_variables.keys().any(|k| k.contains("ns::v")),
        "the generic binder must not record the qualified name; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
    // DESTROYS_VARIABLE: `unset`'s VarWrite role marks a removal target,
    // not a binding.
    let mut a = Analyser::new();
    let r = a.analyse("proc g {} {\n    unset zombie\n}\n", "tcl8.6");
    assert!(
        !r.all_variables.contains_key("g::zombie"),
        "unset must not record a definition for its removal target; got {:?}",
        r.all_variables.keys().collect::<Vec<_>>()
    );
}

/// A second `unset` after the first killed the version reads an unbound
/// place, so its W213 is definite: tclsh 8.4 to 9.1 raise `can't
/// unset "x": no such variable` there.
#[test]
fn a_second_unset_is_a_definite_w213() {
    let found = lifecycle_findings("proc f {} {\n    set x 1\n    unset x\n    unset x\n}\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, DiagCode::W213);
    assert!(found[0].1.contains("does not exist"), "{found:?}");
}

/// An `unset` on one path leaves the read after the merge may-unbound, so
/// it draws W210; tclsh raises `can't read "x"` when `c` is true.
#[test]
fn a_conditional_unset_gives_w210() {
    let found =
        lifecycle_findings("proc f {c} {\n    set x 1\n    if {$c} {unset x}\n    puts $x\n}\n");
    assert!(
        found
            .iter()
            .any(|(code, message)| *code == DiagCode::W210 && message.contains("'x'")),
        "{found:?}"
    );
}

/// `unset -nocomplain` raises nothing, whatever the place holds, so it
/// draws neither W213 nor W210.
#[test]
fn nocomplain_never_reports_w213() {
    for src in [
        "proc f {} {\n    unset -nocomplain x\n}\n",
        "proc f {} {\n    set x 1\n    unset x\n    unset -nocomplain x\n}\n",
        "proc f {c} {\n    if {$c} {set x 1}\n    unset -nocomplain x\n}\n",
    ] {
        let found = lifecycle_findings(src);
        assert!(found.is_empty(), "{src}: {found:?}");
    }
}

/// W210 is one per variable: a `return` of a variable an `unset` killed is
/// reported once, where the read pass and the `return` pass had each
/// reported it.
#[test]
fn a_killed_return_read_reports_once() {
    let found = lifecycle_findings("proc f {} {set v 1; unset v; return $v}\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, DiagCode::W210);
}

/// A write a condition's command substitution makes suppresses the reads
/// after it, not the ones before: the first `puts $x` runs before the
/// `catch` sets `x` and raises in every release, the second does not.
#[test]
fn a_read_before_the_conditions_write_still_reports() {
    let found = lifecycle_findings(
        "proc g {} {\n    puts $x\n    if {[catch {set x 1}]} {}\n    puts $x\n}\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, DiagCode::W210);
    let expr = lifecycle_findings(
        "proc h {} {\n    puts $t\n    set e [expr {[catch {error x} t] || $t}]\n    puts $t\n}\n",
    );
    assert_eq!(expr.len(), 1, "{expr:?}");
}

/// The script a concatenating command runs writes a name only through a
/// command the document's registry knows: `lassign` exists from 8.5, so
/// `eval lassign {1 2} a b` binds `a` under a Tcl 8.6 profile and binds
/// nothing under 8.4, where tclsh raises `invalid command name "lassign"`
/// and the read after it is a read before set.
#[test]
fn a_concatenated_script_writes_through_the_documents_registry() {
    let src = "proc p {} {\n    eval lassign {1 2} a b\n    puts $a\n}\n";
    let w210 = |dialect: &str| {
        Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W210)
            .count()
    };
    assert_eq!(w210("tcl8.6"), 0, "8.6: lassign writes a");
    // 8.4 has no `lassign`: the `eval` fails first (tclsh8.4: `invalid
    // command name "lassign"`), so the read of `a` is never reached.
    assert_eq!(w210("tcl8.4"), 0, "8.4: lassign is no command");
}

/// An existence read and an unbind are uses of their place: a
/// variable only asked about or unset draws no W211, and a parameter only
/// asked about or unset — its entry state read — draws no W214.
#[test]
fn an_existence_read_or_an_unbind_is_a_use() {
    let codes_of = |src: &str| -> Vec<DiagCode> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .into_iter()
            .map(|d| d.code)
            .collect()
    };
    for src in [
        "proc f {} {\n    set x 1\n    if {[info exists x]} {puts yes}\n}\n",
        "proc f {} {\n    set x 1\n    puts [array exists x]\n}\n",
        "proc f {} {\n    set x 1\n    unset x\n}\n",
    ] {
        assert!(!codes_of(src).contains(&DiagCode::W211), "{src}");
    }
    for src in [
        "proc f {a} {\n    if {[info exists a]} {puts yes}\n}\n",
        "proc f {a} {\n    unset a\n}\n",
    ] {
        assert!(!codes_of(src).contains(&DiagCode::W214), "{src}");
    }
}

/// A nested unbind reads its place's existence: the store `puts
/// [unset x]` observes is used, so no W211, and the read is no value read,
/// so an unbind of a never-set name — `-nocomplain` or not — draws no W210.
#[test]
fn a_nested_unbind_is_an_existence_read() {
    let codes_of = |src: &str| -> Vec<DiagCode> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .into_iter()
            .map(|d| d.code)
            .collect()
    };
    for src in [
        "proc f {} {\n    set x 1\n    puts [unset x]\n}\n",
        "proc f {} {\n    set x 1\n    set y [unset x]\n    return $y\n}\n",
    ] {
        assert!(!codes_of(src).contains(&DiagCode::W211), "{src}");
    }
    for src in [
        "proc f {} {\n    puts [unset x]\n}\n",
        "proc f {} {\n    puts [unset -nocomplain x]\n}\n",
    ] {
        assert!(!codes_of(src).contains(&DiagCode::W210), "{src}");
    }
}

/// A guard's refinement narrows the read under `&&` too: `x` set on one
/// path reads bound on the true edge of `[info exists x] &&
/// $flag`, so no W210; the read past the `if` still draws one.
/// The guarded read of a global draws no W210 although the existence rung
/// never refines an externally mutable place: the qualified-name and
/// scope-alias filters keep `if {[info exists ::errorInfo]} {puts
/// $::errorInfo}` silent at the top level and in a procedure, and so does
/// the `global` spelling.
#[test]
fn the_guarded_global_idiom_draws_no_w210() {
    for src in [
        "if {[info exists ::errorInfo]} {puts $::errorInfo}\n",
        "proc f {} {\n    if {[info exists ::errorInfo]} {puts $::errorInfo}\n}\n",
        "proc f {} {\n    global errorInfo\n    if {[info exists errorInfo]} {puts $errorInfo}\n}\n",
    ] {
        let found = lifecycle_findings(src);
        assert!(found.is_empty(), "{src}: {found:?}");
    }
}

/// A guard holds over the region its edge enters where the existence rung
/// refines nothing: a callback the module cannot read (`after idle [list
/// $cmd 1]`) may write any name, so the rung refines none in the file, and
/// `return $x` after `if {![info exists x]} {set x 0}` still draws no W210 —
/// tclsh 8.4 to 9.1 return 0 for `p 0` — while the same shape without the
/// `set` still draws one, `q 0` raising `can't read "y"`.
#[test]
fn a_guard_holds_where_the_rung_refines_nothing() {
    let found = lifecycle_findings(
        "proc p {c} {\n    if {$c} {set x 1}\n    if {![info exists x]} {set x 0}\n    return $x\n}\n\
         proc q {c} {\n    if {$c} {set y 1}\n    if {![info exists y]} {puts none}\n    return $y\n}\n\
         proc cb {cmd} {\n    after idle [list $cmd 1]\n}\n",
    );
    assert_eq!(
        found,
        vec![(
            DiagCode::W210,
            "Variable 'y' is read before it is set".to_owned()
        )]
    );
}

#[test]
fn a_guard_under_and_narrows_the_read() {
    let found = lifecycle_findings(
        "proc f {c flag} {\n    if {$c} {set x 1}\n    if {[info exists x] && $flag} {puts $x}\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
    let past = lifecycle_findings(
        "proc f {c flag} {\n    if {$c} {set x 1}\n    if {[info exists x] && $flag} {puts $x}\n    puts $x\n}\n",
    );
    assert_eq!(past.len(), 1, "{past:?}");
    assert_eq!(past[0].0, DiagCode::W210);
}

#[test]
fn w210_reads_after_lassign_and_dict_with_are_set_2501_2502() {
    let w210 = |source: &str| {
        crate::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::W210)
            .map(|d| d.message)
            .collect::<Vec<_>>()
    };
    for source in [
        "proc f {line} {\n    set pos 0\n    set re {\\{([A-Za-z0-9_]+)\\|([^{}]*)\\}}\n    \
         while {[regexp -start $pos -indices $re $line all nameIdx textIdx]} {\n        \
         lassign $all start end\n        if {$start > $pos} {\n            incr pos\n        }\n    }\n}\n",
        "proc f {} {\n    set style [dict create bg #ffffff radius {8 8 8 8}]\n    \
         dict with style {}\n    set radius [lmap r $radius {expr {$r / 2.0}}]\n    \
         if {$bg ne \"\"} {\n        set bg \"\"\n    }\n    return $radius\n}\n",
    ] {
        assert!(w210(source).is_empty(), "{source}: {:?}", w210(source));
    }
    assert_eq!(w210("proc f {} { puts $never }\n").len(), 1);
}

/// An element store makes its array exist, so reading that element right
/// after draws no read-before-set warning; tclsh 8.4 to 9.1 print `1`.
#[test]
fn an_element_store_defines_its_array_for_a_later_element_read() {
    let source = "proc q {} {\n set c(x) 1\n puts $c(x)\n}\nq\n";
    assert_eq!(w210_codes(source), [] as [String; 0], "{source}");
}
