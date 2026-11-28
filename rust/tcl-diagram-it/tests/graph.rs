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

use serde_json::Value;
use tcl_diagram::irule_flowchart_graph;

fn install_surfaces() {
    tcl_registry::install_shipped().unwrap();
}

#[test]
fn flowchart_handles_irule_brace_chain() {
    install_surfaces();
    // `if {expr}{body}` — `}{` valid in TMM. `irule_flowchart_graph` must
    // lex with the f5-irules preset, so both branches (and their `pool`
    // actions) appear. Under stock-Tcl lexing the `}{` derails the parse and
    // the flowchart comes out empty.
    let src = "when HTTP_REQUEST {\n  if { [HTTP::uri] eq \"/a\" }{\n    pool a_pool\n  } else {\n    pool b_pool\n  }\n}";
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let out = irule_flowchart_graph(src, registry);
    // Valid JSON graph carrying both branch actions.
    let v: Value = serde_json::from_str(&out).expect("valid JSON graph");
    assert!(
        v.get("nodes").and_then(Value::as_array).is_some(),
        "no nodes:\n{out}"
    );
    assert!(out.contains("a_pool"), "missing then-branch pool:\n{out}");
    assert!(out.contains("b_pool"), "missing else-branch pool:\n{out}");
}

#[test]
fn flowchart_empty_source_is_blank() {
    install_surfaces();
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    assert_eq!(irule_flowchart_graph("# just a comment\n", registry), "");
}
