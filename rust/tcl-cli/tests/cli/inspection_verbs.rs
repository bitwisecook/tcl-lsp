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

//! The verbs that print what the toolchain knows about a command or a snippet: `command-info` and `explore`.

use super::*;

#[test]
fn command_info_preserves_exact_command_names() {
    for (name, found) in [
        ("puts", true),
        ("::puts", true),
        ("", false),
        ("Puts", false),
        ("PUTS", false),
        (" puts", false),
        ("puts ", false),
        (" puts ", false),
        ("\tputs", false),
        ("puts\n", false),
        (" ", false),
    ] {
        let output = tcl()
            .args(["command-info", name, "--dialect", "tcl8.6", "--json"])
            .reused_output()
            .expect("failed to spawn tcl binary");
        assert_eq!(output.status.code(), Some(i32::from(!found)), "{name:?}");
        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("command-info JSON");
        assert_eq!(payload["command"], name, "{name:?}");
        assert_eq!(payload["found"], found, "{name:?}");
    }
}

#[test]
fn explore_json_emits_the_contract_keys() {
    let out = run_tcl(&["explore", "--source", "set x 1\nputs $x", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("explore --json must emit valid JSON");
    let obj = value.as_object().expect("top-level object");
    // A representative spread of views is present.
    for key in [
        "meta",
        "ir",
        "cfgPreSsa",
        "cfgPostSsa",
        "segments",
        "asm",
        "stats",
    ] {
        assert!(obj.contains_key(key), "missing explorer key {key:?}");
    }
}

/// The SCCP view and the interprocedural view spell one constant the same
/// way, as the fold spells it (issue #2405).
#[test]
fn explore_sccp_and_interprocedural_spell_a_constant_alike() {
    let source = "proc p {} {set x [expr {1e300 * 10.0}]; return $x}\nputs [p]";
    let out = run_tcl(&[
        "explore",
        "--source",
        source,
        "--json",
        "--show",
        "sccp,interprocedural",
    ]);
    let value: serde_json::Value = serde_json::from_slice(&out).expect("explore JSON");
    let p_sccp = value["sccp"]
        .as_array()
        .and_then(|units| units.iter().find(|u| u["name"] == "::p"))
        .expect("::p in the SCCP view");
    let lattice = p_sccp["values"][0]["lattice"].as_str().expect("lattice");
    let shape = value["interprocedural"][0]["returnShape"]
        .as_str()
        .expect("returnShape");
    assert_eq!(lattice, "const(1e+301)");
    assert_eq!(lattice, shape);
}

#[test]
fn explore_summary_lists_views() {
    let out = run_tcl(&["explore", "--source", "set x 1"]);
    let text = String::from_utf8(out).expect("utf-8 summary");
    assert!(text.contains("Compiler explorer summary"));
    assert!(text.contains("ir:"));
}

/// Smoke variant of [`explore_summary_lists_views`]: the cheapest possible
/// "the `tcl` binary still starts, parses its args, and reports success" CLI
/// invocation, on the default-features surface only (no `--tui`).
#[test]
fn smoke_explore_reports_summary_for_tiny_snippet() {
    let out = run_tcl(&["explore", "--source", "set x 1"]);
    let text = String::from_utf8(out).expect("utf-8 summary");
    assert!(text.contains("Compiler explorer summary"));
}

#[test]
fn explore_text_renders_box_drawing_trees() {
    let out = run_tcl(&[
        "explore",
        "--source",
        "set x 1\nputs $x",
        "--text",
        "--show",
        "ir",
        "--no-colour",
    ]);
    let text = String::from_utf8(out).expect("utf-8 text render");
    assert!(text.contains("=== ir ==="), "section header present");
    assert!(
        text.contains("├── ") || text.contains("└── "),
        "box-drawing tree connectors present"
    );
    assert!(!text.contains('\x1b'), "no ANSI escapes with --no-colour");
}
