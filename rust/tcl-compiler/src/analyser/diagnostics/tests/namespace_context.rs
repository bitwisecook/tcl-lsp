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

//! Source-site namespace seeding and the namespace-scope variable fallback of Tcl 8.x.

use super::*;

// Source-site namespace propagation (seeded analysis).

#[test]
fn seeded_analysis_homes_relative_defs_under_the_source_namespace_m9() {
    // `source` evaluates the file in the caller's namespace: analysing with
    // a seed must behave exactly like wrapping the file in
    // `namespace eval ::x { ... }` — relative names re-home, absolute stay.
    let mut a = Analyser::new();
    let r = a.analyse_with_source_namespace(
        "proc helper {} {}\nproc ::abs::keep {} {}\nnamespace eval rel { proc deep {} {} }\nhelper\n",
        "tcl",
        "::x",
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        r.all_procs.contains_key("::abs::keep"),
        "an absolute definition is not re-homed"
    );
    assert!(
        r.all_procs.contains_key("::x::rel::deep"),
        "a relative namespace nests under the seed: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.name == "helper"
                && i.resolved_qualified_name.as_deref() == Some("::x::helper")),
        "a bare call settles inside the seeded namespace first"
    );
}

#[test]
fn seeded_analysis_records_composed_source_site_namespaces_m9() {
    // A nested `source` inside the seeded file composes: its recorded site
    // namespace already carries the seed, so transitive re-homing needs no
    // separate composition step.
    let mut a = Analyser::new();
    let r = a.analyse_with_source_namespace(
        "source plain.tcl\nnamespace eval sub { source nested.tcl }\n",
        "tcl",
        "::x",
    );
    let sites: Vec<(&str, &str)> = r
        .source_targets
        .iter()
        .map(|s| (s.raw_path.as_str(), s.site_namespace.as_str()))
        .collect();
    assert!(sites.contains(&("plain.tcl", "::x")), "{sites:?}");
    assert!(sites.contains(&("nested.tcl", "::x::sub")), "{sites:?}");
}

#[test]
fn global_seed_is_the_plain_analysis_m9() {
    let mut a = Analyser::new();
    let r = a.analyse_with_source_namespace("proc helper {} {}\n", "tcl", "::");
    assert!(r.all_procs.contains_key("::helper"));
}

// M11 (TIP 278) — cross-version namespace-scope variable fallback.
//
// C-Tcl facts, pinned live against tclsh8.6 (8.6.16) and tclsh9.0 (9.0.4)
// (the same table `rust/tcl-vm-it/tests/it_4/cross_version_vars_e2e.rs` executes
// under both binaries):
//   * `set g 1; namespace eval foo { set g }`      8.6 → 1, 9.0 → error
//   * `set v 1; namespace eval bar { variable v; set v }`  both → error
//     (a declared-but-unset `variable` blocks the fallback)
//   * `namespace eval a { set x 1; namespace eval b { set x } }`
//     both → error (an *intermediate* namespace is never consulted)
//   * `set g 1; proc p {} { set g }; p`            both → error
//     (proc frames never had the fallback)

/// The recorded reference spans of the global-scope variable `name`.
fn global_var_refs(result: &crate::analyser::AnalysisResult, name: &str) -> usize {
    result
        .global_scope
        .variables
        .get(name)
        .map_or(0, |v| v.references.len())
}

#[test]
fn ns_scope_read_attaches_to_the_global_var_under_8x_only_m11() {
    let src = "set g 1\nnamespace eval foo { puts $g }\n";
    // TP: 8.x — the namespace-frame read is a genuine use of `::g`.
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "g"),
        1,
        "8.x: the `$g` read at namespace scope resolves to (and must be \
         recorded on) the global cell"
    );
    // TN: 9.0 removed the fallback — the read errors at runtime, so it must
    // NOT count as a reference of the global.
    let mut a = Analyser::new();
    let r90 = a.analyse(src, "tcl9.0");
    assert_eq!(
        global_var_refs(&r90, "g"),
        0,
        "9.0: no fallback — attaching the read would invent a reference"
    );
}

#[test]
fn ns_scope_declared_variable_blocks_the_8x_attach_m11() {
    // FP guard: `variable v` creates the cell in the namespace's table, so
    // even under 8.x the read resolves namespace-locally (tclsh8.6 errors on
    // the read — it does NOT see the global `v`).
    let src = "set v 1\nnamespace eval bar { variable v; puts $v }\n";
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "v"),
        0,
        "the declared namespace `v` shadows the global in both versions"
    );
}

#[test]
fn proc_read_never_attaches_via_the_ns_fallback_m11() {
    // TN: the fallback is a **namespace-frame** rule; a proc frame resolves
    // bare names locally in every Tcl version (tclsh8.6 and 9.0 both error).
    let src = "set g 1\nproc p {} { puts $g }\n";
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "g"),
        0,
        "a proc-body read must not be recorded on the global via the \
         namespace fallback"
    );
}

#[test]
fn ns_scope_read_never_attaches_to_an_intermediate_namespace_m11() {
    // FP guard: `$x` at `::a::b` scope sees `::a::b::x` and (8.x only) `::x`
    // — never `::a::x`, in ANY version (both tclshs error on this script).
    let src = "namespace eval a { set x 1\nnamespace eval b { puts $x } }\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let mut a = Analyser::new();
        let r = a.analyse(src, dialect);
        let ns_a = r
            .global_scope
            .children
            .iter()
            .find(|s| s.name == "a")
            .expect("namespace scope `a` exists");
        let x_refs = ns_a.variables.get("x").map_or(0, |v| v.references.len());
        assert_eq!(
            x_refs, 0,
            "[{dialect}] `::a::x` must gain no reference from the read at \
             `::a::b` scope"
        );
    }
}

#[test]
fn ns_scope_array_element_read_attaches_under_8x_m11() {
    // The fallback covers array-element reads too (pinned vector: `set
    // arr(k) AV; namespace eval foo { set arr(k) }` reads AV on 8.6).
    let src = "set arr(k) AV\nnamespace eval foo { puts $arr(k) }\n";
    let mut a = Analyser::new();
    let r86 = a.analyse(src, "tcl8.6");
    assert_eq!(
        global_var_refs(&r86, "arr"),
        1,
        "8.x: element read attaches"
    );
    let mut a = Analyser::new();
    let r90 = a.analyse(src, "tcl9.0");
    assert_eq!(global_var_refs(&r90, "arr"), 0, "9.0: no fallback");
}

#[test]
fn ns_scope_write_reports_the_selected_global_cell_under_8x() {
    let source =
        "set x global\nnamespace eval ::n { set x nsevalset }\nnamespace eval ::n { set y $x }\n";
    let namespace_write = u32::try_from(source.find("x nsevalset").unwrap()).unwrap();
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let reaches_root = dialect.starts_with("tcl8.");
        assert_eq!(
            result.global_scope.variables["x"]
                .references
                .iter()
                .any(|span| span.start() == namespace_write),
            reaches_root,
            "{dialect}"
        );
        assert_eq!(
            result
                .global_scope
                .children
                .iter()
                .any(|scope| scope.variables.contains_key("x")),
            !reaches_root,
            "{dialect}"
        );
    }
}

#[test]
fn ns_scope_creative_write_leaves_no_namespace_cell_under_8x() {
    let source = "set x 1\nnamespace eval ::n { set x 2 }\nputs [catch {set ::n::x} m]:$m\n";
    let read = source.find("::n::x").unwrap();
    for (dialect, missing) in [("tcl8.6", true), ("tcl9.0", false)] {
        let result = Analyser::new().analyse(source, dialect);
        assert_eq!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagCode::W210
                    && diagnostic.span.as_range() == (read..read + "::n::x".len())
            }),
            missing,
            "{dialect}: {:?}",
            result.diagnostics
        );
    }
}
