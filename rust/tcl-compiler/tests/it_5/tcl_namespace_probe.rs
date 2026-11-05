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

//! Probes for the `::tcl` namespace surfaces: the prefix-form operator
//! commands (`::tcl::mathop::*`), user-extensible math functions
//! (`::tcl::mathfunc::*`), and `tcl::unsupported` — none may draw a W123
//! unknown-command under a dialect that actually has the `::tcl::`
//! namespace (real commands in every Tcl 8.5+), and a workspace-defined
//! mathfunc proc must be a legitimate definition.
//!
//! The `::tcl::` namespace itself is a Tcl 8.5+ addition (TIP 174 added
//! `::tcl::mathop` in 8.5; there is no `::tcl` namespace at all in a real
//! Tcl 8.4, nor in F5 iRules, which embeds a genuine Tcl 8.4.6 core) — so
//! the dialect-negative tests below assert the mirror image: under
//! `tcl8.4` / `f5-irules`, these commands must read as unavailable, not
//! silently resolve.

use tcl_compiler::analyser::Analyser;

fn codes(source: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let result = a.analyse(source, dialect);
    result
        .diagnostics
        .iter()
        .map(|d| format!("{:?}", d.code))
        .collect()
}

#[test]
fn mathop_prefix_commands_resolve() {
    // tclsh: `::tcl::mathop::+ 1 2` → 3; both spellings are real commands
    // from Tcl 8.5 onward.
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for src in [
            "set x [::tcl::mathop::+ 1 2]\nputs $x\n",
            "set x [tcl::mathop::+ 1 2]\nputs $x\n",
            "set ops [list ::tcl::mathop::* ::tcl::mathop::-]\nputs $ops\n",
        ] {
            let got = codes(src, dialect);
            assert!(
                !got.iter().any(|c| c.contains("W123")),
                "{dialect}: mathop command must be known for {src:?}: {got:?}"
            );
        }
    }
}

#[test]
fn mathop_prefix_commands_are_unavailable_before_tcl85() {
    // Real tclsh 8.4 has no `::tcl` namespace at all, and F5 iRules embeds
    // a genuine Tcl 8.4.6 — `::tcl::mathop` is TIP 174, added in 8.5.  A
    // registered-but-wrong-dialect command draws W002 ("disabled in the
    // active dialect profile"), which is the correct, more precise
    // diagnostic here — not a W123 "never heard of it" unknown-command.
    for dialect in ["tcl8.4", "f5-irules"] {
        for src in [
            "set x [::tcl::mathop::+ 1 2]\nputs $x\n",
            "set x [tcl::mathop::+ 1 2]\nputs $x\n",
        ] {
            let got = codes(src, dialect);
            assert!(
                got.iter().any(|c| c.contains("W002")),
                "{dialect}: mathop command must be disabled-in-dialect for {src:?}: {got:?}"
            );
        }
    }
}

#[test]
fn mathfunc_proc_definition_extends_expr() {
    // tclsh: `proc ::tcl::mathfunc::double2 {x} {expr {$x*2}}` makes
    // `expr {double2(4)}` valid — the definition must not be flagged, and
    // the proc must count as used/known.
    let src = "proc ::tcl::mathfunc::double2 {x} { expr {$x * 2} }\nputs [expr {double2(4)}]\n";
    let got = codes(src, "tcl9.0");
    assert!(
        !got.iter().any(|c| c.contains("W123") || c.contains("E00")),
        "user mathfunc must be clean: {got:?}"
    );
}

#[test]
fn tcl_unsupported_representation_is_not_unknown() {
    // `tcl::unsupported::representation` exists in 8.6+; the analyser must
    // stay conservative (no W123 hard-claim) even if the registry lacks a
    // spec for it.
    let got = codes("puts [tcl::unsupported::representation abc]\n", "tcl9.0");
    assert!(
        !got.iter().any(|c| c.contains("W123")),
        "tcl::unsupported must not be flagged unknown: {got:?}"
    );
}

#[test]
fn tcl_unsupported_corotype_is_gated_to_tcl86_plus() {
    // `::tcl::unsupported::corotype` ships with coroutines (Tcl 8.6, TIP
    // 396) — empirically confirmed present against a real tclsh 8.6.14, so
    // the floor is 8.6, not 9.0. Below that floor it must draw W002, not
    // silently resolve nor read as flat-out unknown (W123).
    for dialect in ["tcl8.4", "tcl8.5", "f5-irules"] {
        let got = codes("puts [::tcl::unsupported::corotype foo]\n", dialect);
        assert!(
            got.iter().any(|c| c.contains("W002")),
            "{dialect}: corotype must be disabled-in-dialect: {got:?}"
        );
    }
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let got = codes("puts [::tcl::unsupported::corotype foo]\n", dialect);
        assert!(
            !got.iter().any(|c| c.contains("W002") || c.contains("W123")),
            "{dialect}: corotype must resolve cleanly: {got:?}"
        );
    }
}

#[test]
fn tcl_build_info_is_gated_to_tcl90_plus() {
    // `::tcl::build-info` / `tcl::build-info` are Tcl 9.0+ — absent from a
    // real tclsh 8.6.14. Both the bare and fully-qualified spellings carry
    // their own spec (one in `tcl__build_info.rs`, one embedded in
    // `mathop_generated.rs`), so both must agree.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules"] {
        for src in [
            "puts [::tcl::build-info version]\n",
            "puts [tcl::build-info version]\n",
        ] {
            let got = codes(src, dialect);
            assert!(
                got.iter().any(|c| c.contains("W002")),
                "{dialect}: build-info must be disabled-in-dialect for {src:?}: {got:?}"
            );
        }
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        let got = codes("puts [::tcl::build-info version]\n", dialect);
        assert!(
            !got.iter().any(|c| c.contains("W002") || c.contains("W123")),
            "{dialect}: build-info must resolve cleanly: {got:?}"
        );
    }
}

#[test]
fn tcl_tm_path_is_gated_to_tcl85_plus() {
    // Tcl Modules (`tcl::tm::path`, TIP 189) are an 8.5+ addition.
    for dialect in ["tcl8.4", "f5-irules"] {
        let got = codes("tcl::tm::path add /foo\n", dialect);
        assert!(
            got.iter().any(|c| c.contains("W002")),
            "{dialect}: tcl::tm::path must be disabled-in-dialect: {got:?}"
        );
    }
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let got = codes("tcl::tm::path add /foo\n", dialect);
        assert!(
            !got.iter().any(|c| c.contains("W002") || c.contains("W123")),
            "{dialect}: tcl::tm::path must resolve cleanly: {got:?}"
        );
    }
}
