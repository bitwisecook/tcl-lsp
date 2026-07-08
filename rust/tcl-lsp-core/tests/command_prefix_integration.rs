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

//! `ArgRole::CommandPrefix` deep-integration: a command-prefix callback
//! (`lsort -command myCompare`) is a first-class command reference across the
//! call graph, find-references, and unknown-command (W123) — all registry
//! driven. Default dialect tcl9.0 (the ground-truth oracle).

use tcl_lsp_core::graphs;
use tcl_registry::CommandRegistry;

/// The nested form (`return [lsort …]`) — the common real shape.
const SRC: &str = "proc myCompare {a b} { expr {$a - $b} }\nproc doSort {items} {\n    return [lsort -command myCompare $items]\n}\n";

#[test]
fn call_graph_has_callback_edge() {
    let reg = CommandRegistry::build_default();
    let g = graphs::call_graph(SRC, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("doSort")
                && e["callee"].as_str().unwrap_or("").contains("myCompare")
        }),
        "expected a doSort→myCompare callback edge; got {g}"
    );
}

#[test]
fn command_invocations_record_callback_head_with_arity() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(SRC, "tcl9.0");
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.name == "myCompare" && i.callback_arity.is_some()),
        "the nested lsort callback must record a myCompare invocation with callback_arity"
    );
}

#[test]
fn find_references_includes_callback_site() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(SRC, "tcl9.0");
    // Cursor on `myCompare` in its definition (line 0, char 6).
    let refs = tcl_lsp_core::references::references(SRC, "tcl9.0", 0, 6, &r, true);
    assert!(
        refs.iter().any(|rg| rg.start_line == 2),
        "find-references must include the lsort -command callback on line 2; got {refs:?}"
    );
}

#[test]
fn w123_fires_on_unknown_callback_but_not_a_defined_one() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    // Unknown callback head → W123.
    let bad = "lsort -command nonexistentCb {a b}\n";
    let codes: Vec<_> = a
        .analyse(bad, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "W123"),
        "an unknown callback head must fire W123; got {codes:?}"
    );
    // Defined callback → no W123.
    let ok = "proc realCb {a b} {expr {$a-$b}}\nlsort -command realCb {a b}\n";
    let codes2: Vec<_> = a
        .analyse(ok, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        !codes2.iter().any(|c| c == "W123"),
        "a defined callback must NOT fire W123; got {codes2:?}"
    );
}

#[test]
fn dynamic_callback_head_is_not_recorded() {
    // `lsort -command $cb` — a dynamic head can't resolve to a proc, so it is
    // neither recorded (no W123 false-fire) nor a call-graph edge.
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {cb l} { return [lsort -command $cb $l] }\n",
        "tcl9.0",
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.callback_arity.is_some()),
        "a dynamic `$cb` callback head must not be recorded as a command-prefix invocation"
    );
}

/// The deferred core-Tcl callback surfaces are wired through the same generic
/// substrate — no per-command code — so a registry-declared prefix on
/// `namespace unknown` / `package unknown` / `regsub -command` /
/// `coroinject` lights up call-graph, references, and W123 automatically.
#[test]
fn namespace_unknown_handler_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc onUnknown {args} { puts $args }\nproc install {} {\n    namespace unknown onUnknown\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("install")
                && e["callee"].as_str().unwrap_or("").contains("onUnknown")
        }),
        "expected an install→onUnknown edge from `namespace unknown`; got {g}"
    );
}

#[test]
fn regsub_command_prefix_fires_w123_only_when_unknown() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    // `regsub -command` with an unknown head → W123 on the prefix.
    let bad = "regsub -command {(\\w+)} $s missingCb out\n";
    let codes: Vec<_> = a
        .analyse(bad, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "W123"),
        "an unknown regsub -command head must fire W123; got {codes:?}"
    );
    // A defined head → recorded, no W123. (Without `-command` the same word is a
    // replacement template and is never treated as a command.)
    let ok = "proc doSub {whole} { string toupper $whole }\nregsub -command {(\\w+)} $s doSub out\n";
    let r = a.analyse(ok, "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code.to_string() == "W123"),
        "a defined regsub -command head must not fire W123"
    );
    assert!(
        r.command_invocations.iter().any(|i| i.name == "doSub"),
        "regsub -command must record a reference to its head"
    );
    // No `-command`: the third positional is a template, not a command.
    let template = "regsub {(\\w+)} $s missingCb out\n";
    assert!(
        !a.analyse(template, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "a plain regsub subSpec must never be treated as a command"
    );
}

#[test]
fn coroinject_records_reference_but_never_arity_checks() {
    // `coroinject`'s appended arity is Unknown (depends on the yield point), so
    // the injected command is a reference/W123 surface but is never arity-checked.
    let reg = CommandRegistry::build_default();
    let src = "proc worker {args} { yield }\nproc kick {c} {\n    coroinject $c worker extra\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("kick")
                && e["callee"].as_str().unwrap_or("").contains("worker")
        }),
        "expected a kick→worker edge from `coroinject`; got {g}"
    );
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let inj = r
        .command_invocations
        .iter()
        .find(|i| i.name == "worker")
        .expect("coroinject records the injected command");
    assert_eq!(
        inj.callback_arity,
        Some(tcl_registry::AppendedArity::Unknown),
        "coroinject's callback arity is Unknown so it is never flagged too-few/too-many"
    );
}

/// tcllib callbacks flow through the same substrate.  This exercises two tcllib
/// paths at once: a sub-command-offset callback (`struct::list split`, arg after
/// the sub-command word) and a full-name math callback declared via the
/// `PREFIX_OVERRIDES` side table.
#[test]
fn tcllib_struct_list_split_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc isEven {n} { expr {$n % 2 == 0} }\nproc part {items} {\n    struct::list split $items isEven evens odds\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("part")
                && e["callee"].as_str().unwrap_or("").contains("isEven")
        }),
        "expected a part→isEven edge from `struct::list split`; got {g}"
    );
}

#[test]
fn tcllib_calculus_func_records_reference_with_fixed_arity() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "proc f {x} { expr {$x * $x} }\nmath::calculus::integral 0 1 100 f\n";
    let r = a.analyse(src, "tcl9.0");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "f")
        .expect("math::calculus::integral records its func callback");
    assert_eq!(
        inv.callback_arity,
        Some(tcl_registry::AppendedArity::Exactly(1)),
        "the calculus func callback carries its man-page-pinned Exactly(1) arity"
    );
}
