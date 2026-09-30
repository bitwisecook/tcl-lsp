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

//! Environment selection end to end over LSP: a Jim Tcl document reaches the
//! `jim` environment through its language id, a shebang line or a directive
//! alias; the environment's own commands and `proc` form draw no false
//! diagnostics while the same text under Tcl 8.6 does; the dialect list and the
//! effective config describe the environments the registry holds; and a
//! session-scope name that selects nothing is reported and replaced by the
//! default.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::common::{Lsp, scaled_timeout, unique_uri};

/// Jim's class, static-variable `proc`, `loop` and `sleep` in one document.
const JIM_PROGRAM: &str = concat!(
    "class system {model \"\"}\n",
    "proc {system model} {} {{model \"\"}} {\n",
    "\tif {$model ne \"\"} { return $model }\n",
    "\tif {[catch {set fp [open /etc/model r]}]} {\n",
    "\t\tset model {HD[R]}\n",
    "\t} else {\n",
    "\t\tset model [string trim [read $fp]]\n",
    "\t\tclose $fp\n",
    "\t}\n",
    "\treturn $model\n",
    "}\n",
    "loop i 0 3 { puts $i }\n",
    "sleep 0.1\n",
    "set s [system new]\n",
    "puts [$s model]\n",
);

/// LSP `DiagnosticSeverity.Hint`.
const SEVERITY_HINT: i64 = 4;

/// LSP `MessageType.Warning`.
const MESSAGE_TYPE_WARNING: i64 = 2;

/// The hints the Jim program may draw: a variable or parameter nothing reads
/// (`W211`, `W214`, `W220`), and `catch` with no result variable (`W302`).
const ALLOWED_HINTS: &[&str] = &["W211", "W214", "W220", "W302"];

fn code_of(diagnostic: &Value) -> &str {
    diagnostic["code"].as_str().unwrap_or_default()
}

fn codes(diagnostics: &[Value]) -> BTreeSet<&str> {
    diagnostics.iter().map(code_of).collect()
}

/// The text `diagnostic` covers, cut from the single-line-per-command `source`.
fn covered_text<'a>(source: &'a str, diagnostic: &Value) -> &'a str {
    let line = usize::try_from(diagnostic["range"]["start"]["line"].as_u64().unwrap()).unwrap();
    let start =
        usize::try_from(diagnostic["range"]["start"]["character"].as_u64().unwrap()).unwrap();
    let end_line = usize::try_from(diagnostic["range"]["end"]["line"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(diagnostic["range"]["end"]["character"].as_u64().unwrap()).unwrap();
    let text = source.lines().nth(line).unwrap_or_default();
    if end_line == line {
        text.get(start..end).unwrap_or_default()
    } else {
        text.get(start..).unwrap_or_default()
    }
}

/// The resolved environment `uri` is analysed under.
fn environment_of(lsp: &mut Lsp, uri: &str) -> Value {
    let cfg = lsp.effective_config(uri);
    json!({
        "dialect_id": cfg["dialect_id"],
        "dialect_kind": cfg["dialect_kind"],
        "dialect_display_name": cfg["dialect_display_name"],
    })
}

fn jim_environment() -> Value {
    json!({
        "dialect_id": "jim",
        "dialect_kind": "language",
        "dialect_display_name": "Jim Tcl",
    })
}

#[test]
fn a_tcl_jim_document_is_analysed_as_jim() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready_lang(&uri, "puts hello\n", "tcl-jim");
    assert_eq!(environment_of(&mut lsp, &uri), jim_environment());
}

#[test]
fn a_jimsh_shebang_selects_jim_for_a_plain_tcl_document() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready_lang(&uri, "#!/usr/bin/env jimsh\nputs hello\n", "tcl");
    assert_eq!(environment_of(&mut lsp, &uri), jim_environment());
}

#[test]
fn a_dialect_directive_alias_selects_jim_for_a_plain_tcl_document() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready_lang(&uri, "# tcl-dialect: jimsh\nputs hello\n", "tcl");
    assert_eq!(environment_of(&mut lsp, &uri), jim_environment());
}

/// A plain Tcl document with no shebang or directive is not taken for Jim.
#[test]
fn a_plain_tcl_document_is_not_analysed_as_jim() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready_lang(&uri, "puts hello\n", "tcl");
    let environment = environment_of(&mut lsp, &uri);
    assert_ne!(environment["dialect_id"], json!("jim"), "{environment}");
}

/// Jim's own commands, the `proc` form with a static-variable list and `class`
/// with its two-word method definitions draw nothing worse than hints: the
/// style hint on `catch` with no result variable and hints about variables
/// nothing reads.
///
/// The server loads its bundled packs, which carry Jim's own commands, after
/// it starts, and republishes the open documents once they are loaded; the
/// test waits for that publish.
#[test]
fn the_jim_program_draws_only_hints_as_a_tcl_jim_document() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready_lang(&uri, JIM_PROGRAM, "tcl-jim");
    let diagnostics = lsp.await_diagnostics_settled(&uri, Duration::from_secs(30), |published| {
        !codes(published).contains("E003") && !codes(published).contains("W123")
    });
    let present = codes(&diagnostics);
    for code in ["E003", "W002", "W123", "W210"] {
        assert!(!present.contains(code), "{code} reported: {diagnostics:#?}");
    }
    for diagnostic in &diagnostics {
        assert_eq!(
            diagnostic["severity"],
            json!(SEVERITY_HINT),
            "only hints are drawn: {diagnostic}"
        );
        assert!(
            ALLOWED_HINTS.contains(&code_of(diagnostic)),
            "{} is not an unused-variable hint or the `catch` style hint: {diagnostics:#?}",
            code_of(diagnostic)
        );
    }
}

/// Whether the server has logged a message containing `needle`.
fn logged(lsp: &Lsp, needle: &str) -> bool {
    lsp.notifications().iter().any(|note| {
        note["method"] == "window/logMessage"
            && note["params"]["message"]
                .as_str()
                .is_some_and(|message| message.contains(needle))
    })
}

/// The first diagnostics a `tcl-jim` document is published with are already
/// right: Jim's own commands and `proc` form are part of the surface the
/// server starts from, not something a later pack reload supplies.
///
/// The startup pack reload is held on its snapshot, so no pack set is
/// published while the document is opened and analysed, and the test reads the
/// first publish rather than the settled one.
#[test]
fn the_first_publish_for_a_tcl_jim_document_needs_no_pack_reload() {
    let hold = scaled_timeout(Duration::from_secs(5))
        .as_millis()
        .to_string();
    let mut lsp = Lsp::tcl_with_env(&[("TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS", hold.as_str())]);
    let uri = unique_uri("tcl");
    lsp.open_document_lang(&uri, JIM_PROGRAM, "tcl-jim", 1);
    let first = lsp.await_first_diagnostics(&uri, Duration::from_secs(30));
    assert!(
        !logged(&lsp, "pack(s)"),
        "the start-up reload published before the first diagnostics arrived, \
         so this run proves nothing: {first:#?}"
    );
    let present = codes(&first);
    for code in ["E003", "W002", "W123", "W210"] {
        assert!(!present.contains(code), "{code} reported: {first:#?}");
    }
}

/// The same text under Tcl 8.6 has a four-word `proc` and commands Tcl does not
/// have.
#[test]
fn the_jim_program_draws_arity_and_unknown_command_errors_as_a_tcl86_document() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diagnostics = lsp.open_ready_lang(&uri, JIM_PROGRAM, "tcl86");
    assert_eq!(
        environment_of(&mut lsp, &uri)["dialect_id"],
        json!("tcl8.6"),
        "{diagnostics:#?}"
    );

    let arity: Vec<&Value> = diagnostics
        .iter()
        .filter(|d| code_of(d) == "E003")
        .collect();
    assert!(
        arity
            .iter()
            .any(|d| d["range"]["start"]["line"] == json!(1)),
        "the four-word `proc` on line 2 is an arity error: {diagnostics:#?}"
    );

    let unknown: BTreeSet<&str> = diagnostics
        .iter()
        .filter(|d| code_of(d) == "W123")
        .map(|d| covered_text(JIM_PROGRAM, d))
        .collect();
    for command in ["loop", "sleep"] {
        assert!(
            unknown.contains(command),
            "`{command}` is unknown under Tcl 8.6: {unknown:?} in {diagnostics:#?}"
        );
    }
}

#[test]
fn list_dialects_offers_jim_tk_and_the_tool_shells_with_their_kinds() {
    let mut lsp = Lsp::tcl();
    let listed = lsp.execute_command("tcl-lsp.listDialects", json!([]));
    let entries = listed.as_array().expect("listDialects answers an array");
    let entry = |name: &str| {
        entries
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("`{name}` is not listed: {listed}"))
    };

    assert_eq!(entry("jim")["kind"], json!("language"));
    assert_eq!(entry("tk")["kind"], json!("packages"));
    let vivado = entry("xilinx-eda-tcl");
    assert_eq!(vivado["kind"], json!("packages"));
    assert_eq!(
        vivado["description"],
        json!("Xilinx Vivado \u{2014} Tcl 8.5 + vivado, sdc, upf")
    );
}

/// The listing is the selectable set: every entry names a distinct environment
/// and the lenient fallback is not one of them.
#[test]
fn list_dialects_names_each_selectable_environment_once() {
    let mut lsp = Lsp::tcl();
    let listed = lsp.execute_command("tcl-lsp.listDialects", json!([]));
    let names: Vec<&str> = listed
        .as_array()
        .expect("listDialects answers an array")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    let distinct: BTreeSet<&str> = names.iter().copied().collect();
    assert_eq!(distinct.len(), names.len(), "{names:?}");
    assert_eq!(names.len(), 21, "{names:?}");
    assert!(!distinct.contains("tcl"), "{names:?}");
}

/// A Vivado constraints file is a Tcl release plus the tool's packages, from the
/// bundled pack.
#[test]
fn an_xdc_document_is_analysed_as_the_vivado_tool_shell() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("xdc");
    lsp.open_ready_lang(&uri, "create_clock -period 10 [get_ports clk]\n", "tcl");
    let cfg = lsp.effective_config(&uri);
    assert_eq!(cfg["dialect_id"], json!("xilinx-eda-tcl"), "{cfg}");
    assert_eq!(cfg["dialect_kind"], json!("packages"), "{cfg}");
    assert_eq!(cfg["dialect_provenance"], json!("bundled-pack"), "{cfg}");
}

/// Wait for a `window/logMessage` of type `Warning` containing every needle.
fn await_warning(lsp: &Lsp, needles: &[&str]) -> String {
    let budget = scaled_timeout(Duration::from_secs(10));
    let deadline = Instant::now() + budget;
    loop {
        let found = lsp.notifications().into_iter().find_map(|note| {
            if note["method"] != "window/logMessage"
                || note["params"]["type"] != json!(MESSAGE_TYPE_WARNING)
            {
                return None;
            }
            let message = note["params"]["message"].as_str()?.to_owned();
            needles
                .iter()
                .all(|needle| message.contains(needle))
                .then_some(message)
        });
        if let Some(message) = found {
            return message;
        }
        assert!(
            Instant::now() < deadline,
            "no warning logMessage containing {needles:?} within {budget:?}; saw {:#?}",
            lsp.notifications()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A session-scope `tclLsp.dialect` that names no environment is reported as a
/// warning listing the valid ids, and the document is analysed as Tcl 8.6 —
/// where `loop` is unknown, which the lenient fallback would not report.
#[test]
fn an_unknown_session_dialect_is_reported_and_the_document_is_analysed_as_tcl86() {
    let mut lsp = Lsp::with_config(json!({ "dialect": "tcl9.0" }));
    lsp.apply_configuration_settle(json!({ "dialect": "nonsense" }), "", |cfg| {
        cfg["dialect"] == json!("tcl8.6")
    });
    let warning = await_warning(
        &lsp,
        &["`nonsense`", "Valid dialects", "jim", "xilinx-eda-tcl"],
    );
    assert!(warning.contains("tcl8.6"), "{warning}");

    let uri = unique_uri("tcl");
    let diagnostics = lsp.open_ready_lang(&uri, "loop i 0 3 { puts $i }\n", "tcl");
    let cfg = lsp.effective_config(&uri);
    assert_eq!(cfg["dialect_id"], json!("tcl8.6"), "{cfg}");
    assert_eq!(cfg["dialect_explicitly_set"], json!(false), "{cfg}");
    assert!(
        codes(&diagnostics).contains("W123"),
        "`loop` is unknown under Tcl 8.6: {diagnostics:#?}"
    );
}
