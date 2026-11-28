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

mod support;

use serde_json::{Value, json};
use tcl_explorer::{run_pipeline, serialise_result};

/// Collect every `kind == "instr"` row across all asm entries.
fn instr_rows(asm: &Value) -> Vec<&Value> {
    asm.as_array()
        .expect("asm view is an array")
        .iter()
        .flat_map(|entry| entry["instructions"].as_array().unwrap().iter())
        .filter(|row| row["kind"] == "instr")
        .collect()
}

#[test]
fn instructions_carry_per_op_source_range() {
    support::install_surfaces();
    // Two statements on two lines: the second `set` lowers on line 2.
    let source = "set x 1\nset y 2";
    let result = run_pipeline(source, "tcl8.6");
    let asm = serialise_result(&result)["asm"].clone();

    let rows = instr_rows(&asm);
    assert!(!rows.is_empty(), "expected emitted instructions");

    // At least one instruction must now carry a real (non-null) range
    // with a 1-based sourceLine — the per-op span plumbing.
    let mapped: Vec<&Value> = rows
        .iter()
        .filter(|r| !r["range"].is_null())
        .copied()
        .collect();
    assert!(
        !mapped.is_empty(),
        "no instruction carried a source range: {rows:#?}"
    );
    for row in &mapped {
        let range = &row["range"];
        let source_line = row["sourceLine"].as_u64().expect("sourceLine is a number");
        assert!(source_line >= 1, "mapped op has 1-based sourceLine");
        // sourceLine is the 1-based startLine of the range.
        assert_eq!(
            source_line,
            range["startLine"].as_u64().unwrap() + 1,
            "sourceLine matches range startLine+1"
        );
    }

    // The second statement (`set y 2`) starts on line 2 (0-based line
    // 1), so some instruction must map there.
    assert!(
        mapped
            .iter()
            .any(|r| r["range"]["startLine"].as_u64() == Some(1)),
        "expected an op mapped to the second source line"
    );
}

#[test]
fn synthetic_ops_have_null_range() {
    support::install_surfaces();
    // An empty proc body emits only synthetic instructions (push "" /
    // done) with no source construct — their range stays null.
    let source = "proc p {} {}\n";
    let result = run_pipeline(source, "tcl8.6");
    let asm = serialise_result(&result)["asm"].clone();

    let proc_entry = asm
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "::p")
        .expect("proc ::p present");
    for row in proc_entry["instructions"].as_array().unwrap() {
        if row["kind"] == "instr" {
            assert!(
                row["range"].is_null(),
                "synthetic op range is null: {row:#?}"
            );
            assert_eq!(row["sourceLine"], json!(0), "synthetic op sourceLine is 0");
        }
    }
}
