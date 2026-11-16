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

//! W220 dead stores and the H300 paste-error fingerprint.

use super::*;

#[test]
fn parameter_only_function_has_no_physical_store_warnings() {
    let source = "proc identity {value} {return $value}";
    let registry = tcl_registry::CommandRegistry::build_default();
    let unit = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
    let function = unit.function("::identity").expect("the procedure");
    assert!(
        function
            .def_use
            .chains
            .values()
            .all(|chain| { chain.definition.kind != crate::def_use::DefKind::Statement })
    );

    let mut analyser = Analyser::new();
    analyser.emit_document_cfg_ssa_diagnostics_for_test(source);
    assert!(
        analyser
            .result
            .diagnostics
            .iter()
            .all(|diagnostic| { !matches!(diagnostic.code, DiagCode::W211 | DiagCode::W220) })
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w220_on_set_once_never_read() {
    // ``set x 1`` set once and never read is a dead store *and* an unused
    // variable. Both checks anchor at the same assignment, so the co-located
    // W220 is deduped in favour of the more informative W211 — a single hint,
    // not two.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set x 1 }");
    let lifecycle: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::W211 | DiagCode::W220))
        .map(|d| d.code)
        .collect();
    assert_eq!(
        lifecycle,
        vec![DiagCode::W211],
        "set-once-never-read must yield only W211; got {:?}",
        a.result.diagnostics,
    );

    // A dead store of a variable that *is* used elsewhere still fires a
    // standalone W220 (no W211, so nothing to dedup against).
    let mut c = Analyser::new();
    c.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set x 1\nset x 2\nreturn $x }");
    assert!(
        c.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220)
            && !c
                .result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W211),
        "the overwritten `set x 1` fires W220 without W211; got {:?}",
        c.result.diagnostics,
    );

    let mut b = Analyser::new();
    b.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set x 1\nreturn $x }");
    assert!(
        !b.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220),
        "W220 must not fire when the assignment is read; got {:?}",
        b.result.diagnostics,
    );
}

#[test]
fn w220_suppressed_for_interpreter_special_variable_writes() {
    // `set auto_path …` at top level configures the runtime
    // package/auto-loader; the write is observed by the interpreter even
    // though the script never reads `$auto_path` back, so it is not a dead
    // store.  The special-variable set is sourced from the dialect-aware
    // `tcl_registry::special_vars` registry.
    // Neither the dead-store (W220) nor the unused-variable (W211) hint may
    // fire on these writes.
    for src in [
        "set auto_path ../\n",
        "lappend auto_path /some/dir\n",
        "set env(FOO) bar\n",
        "set tcl_precision 12\n",
        "set errorInfo {}\n",
    ] {
        let mut a = Analyser::new();
        let res = a.analyse(src, "tcl8.6");
        assert!(
            !res.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
            "W220/W211 must not fire for special-var write {src:?}; got {:?}",
            res.diagnostics,
        );
    }

    // Control: a genuine top-level user variable written and never read is
    // still flagged (the same shape as the issue's screenshot).
    let mut a = Analyser::new();
    let res = a.analyse("set myUnusedVar ../\n", "tcl8.6");
    assert!(
        res.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
        "a genuine dead/unused store must still be flagged; got {:?}",
        res.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w220_suppressed_for_substitution_hidden_reads() {
    // A variable read only inside a command substitution the
    // version-precise SSA can't see — a branch condition, an `expr`
    // value, or a read-modify-write buried in a `[…]` — is not a dead
    // store. gate.
    // Uses the full `analyse` entry so the command registry (which the
    // hidden-read recovery consults) is populated.
    for src in [
        // read in an `if` condition substitution
        "proc f {} { set x 1\nif {[string length $x]} { puts hi } }",
        // read in an expr value substitution
        "proc f {} { set x 1\nset y [expr {[string length $x]}]\nreturn $y }",
        // read-modify-write buried in a substitution keeps the feed alive
        "proc f {} { set i 0\nforeach j {1 2 3} { lappend r [incr i $j] }\nreturn $r }",
    ] {
        let mut a = Analyser::new();
        let res = a.analyse(src, "tcl8.6");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::W220),
            "W220 must not fire for a substitution-hidden read; got {:?} for {src:?}",
            res.diagnostics,
        );
    }
    // Control: a normal later read does not hide anything, so an earlier
    // overwritten store is still a dead store.
    let mut a = Analyser::new();
    let res = a.analyse("proc f {} { set x 1\nset x 2\nputs $x }", "tcl8.6");
    assert!(
        res.diagnostics.iter().any(|d| d.code == DiagCode::W220),
        "W220 must still fire for a genuine overwrite; got {:?}",
        res.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w220_dead_store_overwritten() {
    // ``set x 1\nset x 2\nputs $x`` — the first ``set x 1``
    // is overwritten before being read.  W220 should fire
    // at the first assignment.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test("set x 1\nset x 2\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        !w220s.is_empty(),
        "W220 expected for overwritten ``set x 1``; got {:?}",
        a.result.diagnostics,
    );
    assert!(w220s.iter().any(|d| d.message.contains("'x'")));
    assert_eq!(w220s[0].severity, Severity::Hint);
}

#[test]
fn w220_array_element_overwrite_not_dead() {
    // `set a(k) 1` is NOT a dead store even
    // though the later `set a(j) 2` bumps the name-level SSA version of the
    // base `a`.  The place model sees `a(k)` is read by `puts $a(k)` and
    // that `a(k)` ≠ `a(j)`, so the false W220 on the first write is
    // suppressed.  Goes through `analyse` (the production path) so the
    // registry — which the place bridge needs — is bound.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }", "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W220),
        "no W220 expected — a(k) is read by `puts $a(k)`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w220_scalar_overwrite_still_fires_via_analyse() {
    // Regression guard for the element-granular scope of the suppression:
    // a genuine *scalar* overwrite must still fire W220 with the place
    // model active (scalars don't fold, so the name-level verdict stands).
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { set x 1; set x 2; puts $x }", "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 && d.message.contains("'x'")),
        "scalar dead store must still fire; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w220_braced_literal_arg_is_not_a_read() {
    // A braced word performs no `$`-substitution at this call — but the
    // name-level scan deliberately keeps `{$a(k)}` as a conservative
    // liveness read (the word may be `eval`-ed / scheduled later, e.g.
    // `after 100 {incr x}`), so under per-element SSA the mention keeps
    // `a(k)` alive and no dead store fires. The control: an element with
    // NO mention of any kind still reports.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {} { set a(k) 1; set a(j) 2; puts $a(j); puts {$a(k)} }",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
        "a braced mention conservatively keeps the element alive; got {:?}",
        r.diagnostics,
    );
    let mut b = Analyser::new();
    let r = b.analyse("proc f {} { set a(k) 1; set a(j) 2; puts $a(j) }", "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::W211),
        "an unmentioned dead element write still reports; got {:?}",
        r.diagnostics,
    );
}

/// W220-IR-paths.  Variables prefixed with ``::`` are
/// externally consumed (other namespaces, the global frame
/// outside this file) — the dead-store check skips them.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_global_qualified_var() {
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("set ::x 1\nset ::x 2\nputs $::x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220s.len(),
        0,
        "W220 must skip ``::``-prefixed globals; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``set x [foo]`` is a side-effecting
/// store: dropping the assignment would also drop the call
/// to ``foo``.  ``Statement::AssignValue`` values
/// containing ``[`` are filtered out.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_command_substitution_value() {
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("set x [clock seconds]\nset x 2\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220s.len(),
        0,
        "W220 must skip ``set x [cmd]`` side-effecting stores; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``set x [expr {[foo]}]`` lowers as
/// ``Statement::AssignExpr`` with a command call inside —
/// same side-effecting reasoning as command-substitution
/// values.  ``Statement::AssignExpr`` whose tree contains
/// an ``ExprNode::Command`` is filtered out.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_expr_with_command_call() {
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "set x [expr {[clock seconds] + 1}]\nset x 2\nputs $x",
    );
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220s.len(),
        0,
        "W220 must skip ``Statement::AssignExpr`` containing a command call; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``incr x`` is a side-effecting write
/// (it reads the current value first).  The dead-store
/// check only matches ``Statement::AssignConst`` /
/// ``Statement::AssignValue`` / ``Statement::AssignExpr`` —
/// ``Statement::Incr`` and ``Statement::Call.defs`` are
/// skipped by exclusion.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_incr_writes() {
    let mut a = Analyser::new();
    // ``incr x`` reads x then writes x+1; even when later
    // overwritten, dropping the incr would also drop the
    // implicit read.  Of the three writes to ``x``, only
    // the ``incr`` qualifies as overwritten-before-read
    // (``set x 0`` is read by incr, ``set x 5`` is read
    // by puts), so any W220 on x must be from the incr,
    // and the IR-statement-type filter must drop it.
    a.emit_document_cfg_ssa_diagnostics_for_test("set x 0\nincr x\nset x 5\nputs $x");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220 && d.message.contains("'x'"))
        .collect();
    assert_eq!(
        w220s.len(),
        0,
        "W220 must skip ``incr`` side-effecting writes; got {w220s:?}",
    );
}

/// W220-IR-paths.  ``lassign $list a b`` defines ``a`` and
/// ``b`` via ``Statement::Call.defs`` — a side-effecting
/// write that can't be dropped without also dropping the
/// call.  The dead-store check only matches the three
/// pure-assign IR shapes; ``Statement::Call`` is skipped by
/// exclusion.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_call_defs() {
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("lassign {1 2} a b\nset a 5\nputs $a");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.iter().all(|d| !d.message.contains("'a'")),
        "W220 must skip ``Statement::Call.defs`` side-effecting writes; got {w220s:?}",
    );
}

/// W220-IR-paths.  In a ``pkgIndex.tcl`` file, ``$dir`` is
/// set by the Tcl package loader before the script body
/// runs — even when the script reassigns it, the original
/// store can't be considered dead (the loader-supplied
/// value is the relevant initial state).
#[test]
fn emit_cfg_ssa_diagnostics_w220_pkgindex_dir_var_suppressed() {
    let mut a = Analyser::new();
    a.file_path = Some("/some/path/pkgIndex.tcl".to_string());
    a.emit_document_cfg_ssa_diagnostics_for_test("set dir foo\nset dir bar\nputs $dir");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220s.len(),
        0,
        "W220 must suppress ``$dir`` in pkgIndex.tcl; got {w220s:?}",
    );
}

/// Only ``dir`` is loader-supplied: tclsh 8.6/9.0 `can't read "missing"` for
/// the substituted read below, so W210 still reports it in pkgIndex.tcl.
#[test]
fn w210_pkgindex_reports_other_reads_inside_command_substitutions() {
    let mut failures = Vec::new();
    for (path, source, names) in [
        (
            "/proj/pkgIndex.tcl",
            "set f [file join $dir $missing]\nsource $f\n",
            &["missing"][..],
        ),
        // Outside a package index `$dir` is unset, so the substitution
        // stops at it: tclsh 9.0.4 reports `can't read "dir": no such
        // variable` and never reads `$missing`.
        (
            "/proj/other.tcl",
            "set f [file join $dir $missing]\nsource $f\n",
            &["dir"][..],
        ),
        (
            "/proj/pkgIndex.tcl",
            "set f [file join $missing]\n",
            &["missing"][..],
        ),
        ("/proj/pkgIndex.tcl", "set f $missing\n", &["missing"][..]),
        (
            "/proj/pkgIndex.tcl",
            "set f [file join $dir $missing]\n",
            &["missing"][..],
        ),
        (
            "/proj/pkgIndex.tcl",
            "set f [list $missing]\nsource $f\n",
            &["missing"][..],
        ),
        (
            "/proj/pkgIndex.tcl",
            "set f $missing\nsource $f\n",
            &["missing"][..],
        ),
    ] {
        let mut analyser = Analyser::new().with_file_path(Some(path.to_owned()));
        let result = analyser.analyse(source, "tcl8.6");
        let w210 = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>();
        for name in names {
            if !w210
                .iter()
                .any(|message| message.contains(&format!("'{name}'")))
            {
                failures.push(format!("{path}: {source:?}: {name}: {w210:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// W220-IR-paths.  Outside ``pkgIndex.tcl``, ``$dir`` is
/// just a regular variable — no special suppression.
/// Negative control for the pkgIndex special-case.
#[test]
fn emit_cfg_ssa_diagnostics_w220_dir_var_not_suppressed_outside_pkgindex() {
    let mut a = Analyser::new();
    a.file_path = Some("/some/path/script.tcl".to_string());
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test("set dir foo\nset dir bar\nputs $dir");
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        !w220s.is_empty(),
        "W220 must fire on ``$dir`` outside pkgIndex.tcl; got {:?}",
        a.result.diagnostics,
    );
    assert!(w220s.iter().any(|d| d.message.contains("'dir'")));
}

/// W220-IR-paths.  Variables shared across iRule events
/// via ``::when::*`` procs (collected in
/// ``ConnectionScope::cross_event_imports``) may be read
/// in a different event from where they're set — the
/// local "no use" verdict is unsafe.
#[test]
fn emit_cfg_ssa_diagnostics_w220_irules_cross_event_var_suppressed() {
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::irules();
    // ``HTTP_REQUEST`` writes ``v``, ``HTTP_RESPONSE``
    // reads ``v`` — ``v`` is a cross-event def.  The
    // ``set v 1\nset v 2`` shape inside ``HTTP_REQUEST``
    // would normally fire W220 on the first ``set v 1``,
    // but cross-event suppression should drop it.
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "when HTTP_REQUEST {\n  set v 1\n  set v 2\n}\nwhen HTTP_RESPONSE {\n  log local0. $v\n}",
    );
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.iter().all(|d| !d.message.contains("'v'")),
        "W220 must suppress vars shared across iRule events; got {w220s:?}",
    );
}

/// W220-IR-paths.  Negative control: a proc-local variable
/// (NOT shared across events) inside a ``::when::*`` proc
/// is still subject to W220.  Confirms the cross-event
/// filter is targeted, not a blanket
/// "skip everything in `::when::`*" rule.
#[test]
fn emit_cfg_ssa_diagnostics_w220_irules_proc_local_still_flagged() {
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::irules();
    // ``local`` is only used inside HTTP_REQUEST — not a
    // cross-event var, so W220 should still fire on the
    // overwritten first assignment.
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "when HTTP_REQUEST {\n  set local 1\n  set local 2\n  log local0. $local\n}",
    );
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert!(
        w220s.iter().any(|d| d.message.contains("'local'")),
        "W220 must still fire for proc-local vars in ::when::*; got {:?}",
        a.result.diagnostics,
    );
}

/// W220-IR-paths.  Dead stores in SCCP-unreachable blocks
/// are reported as O107 by the optimiser; the analyser
/// must not double-report them as W220.
#[test]
fn emit_cfg_ssa_diagnostics_w220_skips_unreachable_block() {
    let mut a = Analyser::new();
    // ``if {0} { ... }`` makes the then-branch unreachable
    // under SCCP.  Any dead store inside is suppressed.
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "if {0} {\n  set x 1\n  set x 2\n  puts $x\n}\nputs done",
    );
    let w220s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220s.len(),
        0,
        "W220 must skip dead stores in SCCP-unreachable blocks; got {w220s:?}",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_h300_repeated_assignment() {
    // ``proc foo {} { set x 1; set x 1 }`` — same var,
    // same literal value, consecutive statements.  The
    // first is a dead store; H300 fires on the second.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set x 1\nset x 1 }");
    let h300s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::H300)
        .collect();
    assert!(
        !h300s.is_empty(),
        "H300 expected for repeated ``set x 1``; got {:?}",
        a.result.diagnostics,
    );
    assert!(h300s[0].message.contains("'x'"));
    assert!(h300s[0].message.contains("Possible paste error"));
}

#[test]
fn emit_cfg_ssa_diagnostics_h300_skips_underscore_vars() {
    // Vars starting with ``_`` are excluded (the convention
    // for "intentionally unused").
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set _x 1\nset _x 1 }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::H300),
        "H300 must not fire on underscore-prefixed vars",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_h300_skips_distinct_values() {
    // Same var, different literal → not a paste error.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set x 1\nset x 2 }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::H300),
        "H300 must not fire when literal values differ",
    );
}

/// The `(code, message)` of each W220 and W211 hint a program draws.
type Hints = Vec<(DiagCode, String)>;

/// The W220 and W211 hints a program draws on the whole-file path and on the
/// per-item path.
fn dead_store_hints_on_both_paths(src: &str) -> (Hints, Hints) {
    let hints = |per_item: bool| -> Hints {
        let mut a = Analyser::new();
        let result = if per_item {
            a.analyse_per_item(src, "tcl8.6")
        } else {
            a.analyse(src, "tcl8.6")
        };
        result
            .diagnostics
            .into_iter()
            .filter(|d| matches!(d.code, DiagCode::W220 | DiagCode::W211))
            .map(|d| (d.code, d.message))
            .collect()
    };
    (hints(false), hints(true))
}

/// A call to a command the module cannot see may read a name the store before
/// it leaves, as it may read `::g`, and a procedure's local through `upvar 1`,
/// so the store is no dead one (W220) and the name no unused one (W211): the
/// optimiser's O109 and O126 already said so. A procedure the module defines
/// is seen, and a store nothing reads before the next is still dead.
#[test]
fn a_store_a_call_the_module_cannot_see_may_read_draws_neither_w220_nor_w211() {
    for src in [
        "set g 5\nfoo\nset g 6\nputs $g\n",
        "set g 5\n$cmd\nset g 6\nputs $g\n",
        "set g 5\ncatch {foo}\nset g 6\nputs $g\n",
        "set g 5\nputs [foo]\nset g 6\nputs $g\n",
        "set h 6\nsource other.tcl\n",
        "set h 6\nputs [foo]\n",
        "proc p {} {\n set h 6\n source other.tcl\n}\n",
        "proc p {} {\n set g 5\n foo\n set g 6\n return $g\n}\n",
    ] {
        let (whole, per_item) = dead_store_hints_on_both_paths(src);
        assert!(whole.is_empty(), "whole file: {src}\n{whole:?}");
        assert_eq!(per_item, whole, "per item: {src}");
    }
    for (src, code) in [
        (
            "proc foo {} { puts hi }\nset g 5\nfoo\nset g 6\nputs $g\n",
            DiagCode::W220,
        ),
        ("set g 5\nset g 6\nputs $g\n", DiagCode::W220),
        // The store after the last such call has no call to read it.
        ("set h 6\nfoo\nset h 7\n", DiagCode::W220),
        ("proc p {} {\n set h 6\n}\n", DiagCode::W211),
        ("proc foo {} { puts hi }\nset h 6\nfoo\n", DiagCode::W211),
    ] {
        let (whole, per_item) = dead_store_hints_on_both_paths(src);
        assert_eq!(
            whole.iter().map(|(found, _)| *found).collect::<Vec<_>>(),
            [code],
            "{src}: {whole:?}"
        );
        assert_eq!(per_item, whole, "per item: {src}");
    }
}
