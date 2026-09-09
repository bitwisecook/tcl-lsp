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

//! A `# tcl-lsp: stub` declaration states the same kind of fact a registry
//! `CommandSpec` does, so its argument roles must reach the same consumers by
//! the same path: `arg_role_resolver` → `arg_roles` → the query every
//! consumer asks, widened by the document's own `DeclaredSurface`.
//!
//! Two consumers are pinned here — the call-graph builder (a `script:body`
//! word is a script whose calls are edges of the caller) and the variable
//! analyser (a `var` word is a definition, so a later read is not W210) —
//! each beside the registry command whose behaviour the stub must match.

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::graphs;
use tcl_registry::CommandRegistry;

const DIALECT: &str = "tcl8.6";

/// The call-graph edges of `source`, as `caller → callee` pairs.
fn edges(source: &str) -> Vec<(String, String)> {
    let registry = CommandRegistry::build_default();
    let graph = graphs::call_graph(
        source,
        &registry,
        tcl_registry::model::ingress::resolve_environment(DIALECT).analyser_profile(),
    );
    graph["edges"]
        .as_array()
        .expect("edges array")
        .iter()
        .map(|edge| {
            (
                edge["caller"].as_str().unwrap_or_default().to_owned(),
                edge["callee"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The `W210` (read before set) messages `source` draws.
fn w210(source: &str) -> Vec<String> {
    Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W210")
        .map(|d| d.message.clone())
        .collect()
}

/// Whether the call graph of `source` records `from → to`.
fn calls(source: &str, from: &str, to: &str) -> bool {
    edges(source)
        .iter()
        .any(|(caller, callee)| caller == from && callee == to)
}

// ───────────────────────── ArgRole::Body → call graph ─────────────────────

/// The registry baseline: a shipped command whose spec marks an argument
/// `ArgRole::Body` makes the procedures that body calls edges of the caller.
#[test]
fn registry_body_role_is_a_call_graph_edge() {
    let source = "proc on_row {} { puts row }\nproc main {} {\n    time { on_row }\n}\n";
    assert!(
        calls(source, "::main", "::on_row"),
        "a registry body command's script is the caller's own calls; got {:?}",
        edges(source)
    );
}

/// The same fact declared by a stub reaches the same builder: `script:body`
/// makes `db_eval`'s second word a script, so `on_row` is reachable from
/// `main` exactly as it is through `time` above.
#[test]
fn stub_body_role_is_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub db_eval {sql script:body} -barrier\n",
        "# tcl-lsp: stubs-end\n",
        "proc on_row {} { puts row }\n",
        "proc main {} {\n",
        "    db_eval \"select 1\" { on_row }\n",
        "}\n",
    );
    assert!(
        calls(source, "::main", "::on_row"),
        "a stub-declared body must contribute the same edge a registry body does; got {:?}",
        edges(source)
    );
}

/// Without the role the word is an opaque string, so there is no edge — the
/// declaration is what does the work, not the braces.
#[test]
fn an_undeclared_body_word_is_not_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub db_eval {sql script}\n",
        "# tcl-lsp: stubs-end\n",
        "proc on_row {} { puts row }\n",
        "proc main {} {\n",
        "    db_eval \"select 1\" { on_row }\n",
        "}\n",
    );
    assert!(
        !calls(source, "::main", "::on_row"),
        "a role-less word is data, not a script; got {:?}",
        edges(source)
    );
}

/// A body that runs in another namespace is still left to the body unit that
/// owns it: walking `namespace eval ::b { helper }` here would invent an edge
/// to a same-named procedure in the *caller's* namespace (issues #977/#980).
#[test]
fn a_namespace_shifted_body_invents_no_caller_namespace_edge() {
    let source = concat!(
        "proc helper {} { puts global }\n",
        "namespace eval ::a {\n",
        "    proc helper {} { puts inner }\n",
        "    proc run_it {} { namespace eval ::b { helper } }\n",
        "}\n",
    );
    assert!(
        !calls(source, "::a::run_it", "::a::helper"),
        "a `namespace eval ::b` body does not call the caller's namespace; got {:?}",
        edges(source)
    );
}

/// The body word is walked as a script, not merely counted as one: a command
/// the workspace does not know, written inside it, draws the same W123 it
/// would inside a registry body command's script.
#[test]
fn a_stub_declared_body_is_analysed_as_a_script() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub db_eval {sql script:body}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    db_eval \"select 1\" { totally_unknown_cmd }\n",
        "}\n",
    );
    let found: Vec<String> = Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W123")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        found.len(),
        1,
        "the stubbed body's own commands are resolved; got {found:?}"
    );
    assert!(found[0].contains("totally_unknown_cmd"), "got {found:?}");
}

// ──────────────────────── ArgRole::VarWrite → W210 ────────────────────────

/// The registry baseline: `scan`'s output word is `ArgRole::VarWrite`, so the
/// variable it names is defined and reading it afterwards is not W210.
#[test]
fn registry_var_role_suppresses_w210() {
    let source = "proc main {} {\n    scan \"1\" %d out\n    puts $out\n}\n";
    assert!(
        w210(source).is_empty(),
        "a registry-declared variable write defines the name; got {:?}",
        w210(source)
    );
}

/// The same fact declared by a stub: `row:var` says `fetch_row` writes the
/// variable its second word names, so the later read is not read-before-set.
#[test]
fn stub_var_role_suppresses_w210() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch_row {table row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch_row t out\n",
        "    puts $out\n",
        "}\n",
    );
    assert!(
        w210(source).is_empty(),
        "a stub-declared variable write must suppress W210 as a registry one does; got {:?}",
        w210(source)
    );
}

/// The suppression is the declared role's, not a blanket amnesty: a variable
/// the stub never names is still read before it is set.
#[test]
fn stub_var_role_suppresses_only_the_name_it_declares() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch_row {table row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch_row t out\n",
        "    puts $other\n",
        "}\n",
    );
    let found = w210(source);
    assert_eq!(
        found.len(),
        1,
        "only the undeclared `other` is read before set; got {found:?}"
    );
    assert!(found[0].contains("other"), "got {found:?}");
}
