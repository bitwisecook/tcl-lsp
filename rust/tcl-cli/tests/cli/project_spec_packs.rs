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

//! The current project's spec pack reaches every verb that reads the registry.

use super::*;

#[test]
fn command_info_discovers_the_current_projects_spec_pack() {
    let project = spec_pack_project();
    let with_pack = tcl()
        .current_dir(&project)
        .args(["command-info", "::tcl_lsp_fixture::collect", "--json"])
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(
        with_pack.status.success(),
        "project pack was not discovered: {}",
        String::from_utf8_lossy(&with_pack.stderr)
    );
    let found: serde_json::Value =
        serde_json::from_slice(&with_pack.stdout).expect("command-info JSON");
    assert_eq!(found["found"], true);
    assert_eq!(
        found["summary"],
        "Evaluate a script while collecting a result in a caller variable."
    );

    let without_pack = tcl()
        .current_dir(project.join("lib"))
        .args(["command-info", "::tcl_lsp_fixture::collect", "--json"])
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert_eq!(without_pack.status.code(), Some(1));
    let missing: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("command-info JSON");
    assert_eq!(missing["found"], false);
}

#[test]
fn diag_analysis_changes_when_the_current_projects_spec_pack_is_present() {
    let project = spec_pack_project();
    let args = [
        "diag",
        "--source",
        "package require tcl_lsp_fixture\n::tcl_lsp_fixture::collect output",
        "--json",
    ];

    let with_pack = tcl()
        .current_dir(&project)
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert_eq!(with_pack.status.code(), Some(1));
    let analysed: serde_json::Value =
        serde_json::from_slice(&with_pack.stdout).expect("diag JSON with pack");
    let codes: Vec<&str> = analysed[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert_eq!(codes, ["E002"]);

    let without_pack = tcl()
        .current_dir(project.join("lib"))
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(without_pack.status.success());
    let opaque: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("diag JSON without pack");
    assert_eq!(opaque[0]["diagnostics"], serde_json::json!([]));
}

#[test]
fn analyser_only_verbs_install_the_current_projects_spec_pack_overlay() {
    let project = spec_pack_project();
    let source = "package require tcl_lsp_fixture\n::tcl_lsp_fixture::collect output { proc nested {} { expr $input + 1 } }";

    let symbols = run_tcl_in(&project, &["symbols", "--source", source, "--json"]);
    assert!(symbols.status.success());
    let symbols: serde_json::Value =
        serde_json::from_slice(&symbols.stdout).expect("symbols JSON with pack");
    assert!(
        symbols["symbols"]
            .as_array()
            .expect("symbols array")
            .iter()
            .any(|symbol| symbol["name"] == "nested"),
        "the Body role must expose the nested procedure"
    );

    let legacy = run_tcl_in(&project, &["find-legacy", "--source", source, "--json"]);
    assert!(legacy.status.success());
    let legacy: serde_json::Value =
        serde_json::from_slice(&legacy.stdout).expect("find-legacy JSON with pack");
    assert_eq!(legacy["issues"][0]["code"], "W100");

    let minimized = run_tcl_in(
        &project,
        &["minimize", "--source", source, "W100", "--json"],
    );
    assert!(minimized.status.success());
    let minimized: serde_json::Value =
        serde_json::from_slice(&minimized.stdout).expect("minimize JSON with pack");
    assert_eq!(minimized[0]["reproduces"], true);

    let without_pack = run_tcl_in(
        &project.join("lib"),
        &["symbols", "--source", source, "--json"],
    );
    assert!(without_pack.status.success());
    let without_pack: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("symbols JSON without pack");
    assert_eq!(without_pack["count"], 0);
}

#[test]
fn explore_uses_the_current_projects_active_spec_pack_registry() {
    let project = spec_pack_project();
    let source =
        "package require tcl_lsp_fixture\n::tcl_lsp_fixture::collect output { set value 1 }";
    let args = ["explore", "--source", source, "--json"];

    let with_pack = run_tcl_in(&project, &args);
    assert!(with_pack.status.success());
    let with_pack: serde_json::Value =
        serde_json::from_slice(&with_pack.stdout).expect("explore JSON with pack");
    assert_eq!(
        with_pack["semantic"][0]["invocations"][1]["resolution"],
        "resolved"
    );
    assert_eq!(
        with_pack["worldSsa"][0]["invocations"][1]["command"],
        "::tcl_lsp_fixture::collect"
    );

    let without_pack = run_tcl_in(&project.join("lib"), &args);
    assert!(without_pack.status.success());
    let without_pack: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("explore JSON without pack");
    assert_eq!(
        without_pack["semantic"][0]["invocations"][1]["resolution"],
        "unresolved-unproved-binding"
    );
}

#[test]
fn cli_publishes_project_pack_hooks() {
    let project = spec_pack_project();
    let args = [
        "opt",
        "--source",
        "package require tcl_lsp_fixture\nset length [::tcl_lsp_fixture::strlen abcde]",
    ];

    let with_pack = run_tcl_in(&project, &args);
    assert!(with_pack.status.success());
    let with_pack = String::from_utf8(with_pack.stdout).expect("UTF-8 opt output");
    assert!(
        with_pack.contains("set length 5"),
        "hook did not fold: {with_pack}"
    );
    assert!(
        with_pack.contains("O129"),
        "hook rewrite was not reported: {with_pack}"
    );

    let without_pack = run_tcl_in(&project.join("lib"), &args);
    assert!(without_pack.status.success());
    let without_pack = String::from_utf8(without_pack.stdout).expect("UTF-8 opt output");
    assert!(without_pack.contains("::tcl_lsp_fixture::strlen abcde"));
    assert!(!without_pack.contains("O129"));
}

#[test]
fn highlight_uses_the_current_projects_spec_pack_registry() {
    let project = spec_pack_project();
    let args = [
        "highlight",
        "--source",
        "::tcl_lsp_fixture::catalog names",
        "--format",
        "html",
    ];

    let with_pack = run_tcl_in(&project, &args);
    assert!(with_pack.status.success());
    let with_pack = String::from_utf8(with_pack.stdout).expect("UTF-8 highlight output");
    assert!(
        with_pack.contains("color:#2c5282;\">names</span>"),
        "pack subcommand was not highlighted: {with_pack}"
    );

    let without_pack = run_tcl_in(&project.join("lib"), &args);
    assert!(without_pack.status.success());
    let without_pack = String::from_utf8(without_pack.stdout).expect("UTF-8 highlight output");
    assert!(!without_pack.contains("color:#2c5282;\">names</span>"));
}
