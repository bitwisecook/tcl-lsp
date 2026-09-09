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
/// Every diagnostic code `source` draws.
fn codes(source: &str) -> Vec<String> {
    Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

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

// ───────────────────────── ArgRole::Expr → call graph ─────────────────────

/// The registry baseline: brace quoting suppresses substitution at the word
/// level, but the expression engine re-evaluates the operand, so a `[cmd …]`
/// inside `expr {…}` is a real call.
#[test]
fn registry_expr_role_is_a_call_graph_edge() {
    let source = "proc score {} { return 1 }\nproc main {} { expr {[score] > 0} }\n";
    assert!(
        calls(source, "::main", "::score"),
        "an expression operand's substitutions are calls; got {:?}",
        edges(source)
    );
}

/// The same fact declared by a stub: `cond:expr` makes the word an
/// expression, so the command substituted inside it is a call.
#[test]
fn stub_expr_role_is_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub assert_that {cond:expr}\n",
        "# tcl-lsp: stubs-end\n",
        "proc score {} { return 1 }\n",
        "proc main {} { assert_that {[score] > 0} }\n",
    );
    assert!(
        calls(source, "::main", "::score"),
        "a stub-declared expression operand's substitutions are calls; got {:?}",
        edges(source)
    );
}

/// A deferred script is reachable code too: the callback a registry command
/// schedules for later is an edge of the procedure that schedules it.
#[test]
fn a_deferred_registry_body_is_a_call_graph_edge() {
    let source = "proc on_row {} { puts row }\nproc main {} {\n    after 100 { on_row }\n}\n";
    assert!(
        calls(source, "::main", "::on_row"),
        "a scheduled script's calls are the scheduler's own; got {:?}",
        edges(source)
    );
}

/// An expression operand draws the expression diagnostics whichever source
/// declared it: an unbraced one is a double-substitution risk either way.
#[test]
fn a_stub_declared_expression_draws_the_unbraced_warning() {
    let registry = "proc main {x} { if $x { puts hi } }\n";
    assert!(
        codes(registry).contains(&"W100".to_owned()),
        "the registry baseline warns on an unbraced expression; got {:?}",
        codes(registry)
    );
    let stubbed = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub calculate {condition:expr}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {x} { calculate $x }\n",
    );
    assert!(
        codes(stubbed).contains(&"W100".to_owned()),
        "a stub-declared expression warns the same way; got {:?}",
        codes(stubbed)
    );
}

// ────────────────────────── optional argument slots ───────────────────────

/// A declared position is not a call position. With the optional slot
/// omitted, the `var` role lands on the word the call actually passed.
#[test]
fn an_omitted_optional_slot_shifts_the_var_role() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch {?table? row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch out\n",
        "    puts $out\n",
        "}\n",
    );
    assert!(
        w210(source).is_empty(),
        "the write lands on the supplied word, not the declared index; got {:?}",
        w210(source)
    );
}

/// The same shape for a body role: the script is still found when the
/// optional slot before it is omitted.
#[test]
fn an_omitted_optional_slot_shifts_the_body_role() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub visit {?context? script:body}\n",
        "# tcl-lsp: stubs-end\n",
        "proc on_row {} { puts row }\n",
        "proc main {} { visit { on_row } }\n",
    );
    assert!(
        calls(source, "::main", "::on_row"),
        "the body lands on the supplied word; got {:?}",
        edges(source)
    );
}

/// Supplying the optional slot puts the role back where the declaration
/// writes it, so the shift tracks the call rather than being a fixed offset.
#[test]
fn a_supplied_optional_slot_keeps_the_declared_position() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch {?table? row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch t out\n",
        "    puts $out\n",
        "}\n",
    );
    assert!(
        w210(source).is_empty(),
        "with the optional word present the write is at index 1; got {:?}",
        w210(source)
    );
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
