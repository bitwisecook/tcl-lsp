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

use super::*;

/// Two namespaces defining a `dup` proc with distinct parameter names, so a
/// docstring reveals which one was resolved.
const TWO_NS_DUP: &str = "namespace eval a {\n    proc dup {alphaparam} { return $alphaparam }\n}\nnamespace eval z {\n    proc dup {omegaparam} { return $omegaparam }\n}\n";

fn docstring_for(proc_name: &str) -> String {
    let args = json!({
        "source": TWO_NS_DUP,
        "proc_name": proc_name,
        "style": "doxygen",
    });
    let res = generate_docstring(&args);
    res["docstring"].as_str().unwrap_or_default().to_owned()
}

#[test]
fn qualified_name_resolves_to_that_namespace() {
    // `z::dup` names `::z::dup` exactly — its `omegaparam` shows, never
    // `::a::dup`'s `alphaparam`.
    let ds = docstring_for("z::dup");
    assert!(ds.contains("omegaparam"), "{ds}");
    assert!(!ds.contains("alphaparam"), "{ds}");
}

#[test]
fn bare_name_without_a_namespace_does_not_choose_an_unrelated_declaration() {
    let result = generate_docstring(&json!({
        "source": TWO_NS_DUP,
        "proc_name": "dup",
    }));
    assert!(result["docstring"].is_null());
    assert!(result["error"].as_str().unwrap().contains("qualified name"));
}

#[test]
fn original_doc_tools_preserve_opaque_names_and_repeated_declarations() {
    let source = r"proc p\uD800 {} {}
proc p\uD801 {} {}
proc repeated {first} {}
proc repeated {second} {}";
    let result = read_proc_docs(&json!({ "source": source, "dialect": "tcl8.6" }));
    let rows = result["procs"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["name_source"], r"p\uD800");
    assert_eq!(rows[1]["name_source"], r"p\uD801");
    assert_ne!(rows[0]["name_start"], rows[1]["name_start"]);
    let ambiguous = generate_docstring(&json!({
        "source": source, "dialect": "tcl8.6", "proc_name": "repeated",
    }));
    assert!(ambiguous["docstring"].is_null());
    assert_eq!(ambiguous["declarations"].as_array().unwrap().len(), 2);
    let updated = update_docstrings(&json!({ "source": source, "dialect": "tcl8.6" }));
    assert_eq!(updated["total_procs"], 4);
    assert_eq!(updated["procs_documented"], 4);
    let after = updated["source"].as_str().unwrap();
    assert!(after.contains(r"proc p\uD800"));
    assert!(after.contains(r"proc p\uD801"));
}
