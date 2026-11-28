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
use tcl_explorer::{run_pipeline, serialise_result};

mod support;

fn wasm_entries(src: &str) -> Vec<Value> {
    serialise_result(&run_pipeline(src, "tcl8.6"))["wasm"]
        .as_array()
        .expect("the wasm view is an array")
        .clone()
}

#[test]
fn source_function_rows_preserve_duplicate_names_and_named_top() {
    support::install_surfaces();
    let entries = wasm_entries(
        "proc p {} {return FIRST}; proc p {} {return LAST}; proc ::top {} {return NAMED}",
    );
    let procedure_rows: Vec<_> = entries
        .iter()
        .filter(|row| row["bodyKey"].is_string() && row["isTopLevel"] == false)
        .collect();
    assert_eq!(procedure_rows.len(), 3, "{entries:?}");
    let keys: std::collections::HashSet<_> = procedure_rows
        .iter()
        .map(|row| row["bodyKey"].as_str().unwrap())
        .collect();
    assert_eq!(keys.len(), 3);
    let roots: Vec<_> = entries
        .iter()
        .filter(|row| row["isTopLevel"].as_bool() == Some(true))
        .collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["name"], "::top");
    assert!(procedure_rows.iter().any(|row| row["name"] == "::top"));
    assert!(procedure_rows.iter().all(|row| row["isTopLevel"] == false));
}

#[test]
fn module_header_then_functions() {
    support::install_surfaces();
    let entries = wasm_entries("set x 1\nputs $x");
    assert_ne!(entries, [] as [serde_json::Value; 0]);
    assert_eq!(entries[0]["kind"], "module");
    assert_eq!(entries[0]["name"], "(module)");
    // At least the top-level function follows the module header.
    assert!(entries.len() >= 2);
    assert!(entries[1]["instructions"].is_array());
    assert_eq!(
        entries[1]["instrCount"],
        entries[1]["instructions"].as_array().unwrap().len()
    );
}

#[test]
fn module_header_carries_every_contract_field() {
    support::install_surfaces();
    let entries = wasm_entries("proc add {a b} { expr {$a + $b} }\nputs [add 1 2]");
    let header = &entries[0];
    assert_eq!(header["kind"], "module");
    for field in ["imports", "types", "dataSegments", "instructions"] {
        assert!(
            header[field].is_array(),
            "module header field `{field}` must be an array, got {}",
            header[field]
        );
    }
    for field in ["instrCount", "totalInstrCount", "functionCount"] {
        assert!(
            header[field].is_u64(),
            "module header field `{field}` must be a number, got {}",
            header[field]
        );
    }
    // The header's counters describe the whole module, not the (empty)
    // header body: `instrCount` stays 0 so tab badges don't double-count.
    assert_eq!(header["instrCount"], 0);
    let total: u64 = entries
        .iter()
        .skip(1)
        .map(|f| f["instrCount"].as_u64().unwrap())
        .sum();
    assert_eq!(header["totalInstrCount"].as_u64().unwrap(), total);
    assert_eq!(
        header["functionCount"].as_u64().unwrap(),
        entries.len() as u64 - 1
    );
}

#[test]
fn module_types_cover_imports_and_defined_functions() {
    // Every import's `typeIdx` must index into the emitted `types` list,
    // and each type entry carries WAT type names the GUI can print.
    support::install_surfaces();
    let entries = wasm_entries("proc add {a b} { expr {$a + $b} }\nputs [add 1 2]");
    let header = &entries[0];
    let types = header["types"].as_array().unwrap();
    assert!(!types.is_empty(), "a module with imports has types");
    for (i, t) in types.iter().enumerate() {
        assert_eq!(t["index"].as_u64().unwrap(), i as u64);
        assert!(t["params"].is_array());
        assert!(t["results"].is_array());
        for name in t["params"]
            .as_array()
            .unwrap()
            .iter()
            .chain(t["results"].as_array().unwrap())
        {
            assert!(
                ["i32", "i64", "f32", "f64"].contains(&name.as_str().unwrap()),
                "unexpected WAT type name {name}"
            );
        }
    }
    for imp in header["imports"].as_array().unwrap() {
        let idx = usize::try_from(imp["typeIdx"].as_u64().unwrap()).unwrap();
        assert!(
            idx < types.len(),
            "import {} points at type {idx}, but only {} types are emitted",
            imp["name"],
            types.len()
        );
    }
}

#[test]
fn calls_resolve_a_target() {
    // General lowering boxes unsupported leaf commands as a `call` to an
    // imported host function; every such call must resolve a target label.
    support::install_surfaces();
    let entries = wasm_entries("set x 1\nputs hello");
    let mut saw_call = false;
    for func in entries.iter().skip(1) {
        for instr in func["instructions"].as_array().unwrap() {
            if instr["op"] == "call" {
                saw_call = true;
                assert!(
                    instr["callTarget"].is_object(),
                    "call has no resolved target: {instr}"
                );
                assert!(instr["callTarget"]["name"].is_string());
            }
        }
    }
    assert!(saw_call, "no call instruction emitted for a leaf command");
}

#[test]
fn branches_and_blocks_pair_up() {
    // A control construct should produce paired open/end indices and any
    // branch should resolve a structured target (or a function return).
    support::install_surfaces();
    let entries = wasm_entries("set x 1\nif {$x > 0} {\n  puts yes\n}\n");
    for func in entries.iter().skip(1) {
        for instr in func["instructions"].as_array().unwrap() {
            if instr["op"] == "block" || instr["op"] == "loop" || instr["op"] == "if" {
                // A paired open carries an endIdx (possibly null only if the
                // body is genuinely unterminated, which the emitter avoids).
                assert!(instr.get("endIdx").is_some());
            }
            if instr["op"] == "br" || instr["op"] == "br_if" {
                assert!(instr["branchTarget"].is_object());
                assert!(instr["branchTarget"]["kind"].is_string());
            }
        }
    }
}
