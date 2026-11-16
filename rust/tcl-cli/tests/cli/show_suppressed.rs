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

//! `tcl diag --show-suppressed`: every hidden finding and declared gap, with its reason.

use super::*;

/// Every `suppressed` row of a `diag --show-suppressed --json` report,
/// across all files — empty for a file with no `suppressed` key (the flag
/// was not given).
fn diag_suppressed_rows(out: &[u8]) -> Vec<serde_json::Value> {
    let report: serde_json::Value = serde_json::from_slice(out).expect("diag JSON");
    report
        .as_array()
        .expect("report array")
        .iter()
        .flat_map(|file| {
            file.get("suppressed")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

/// `--show-suppressed` lists every suppressed finding with its reason
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, CLI rows); the
/// plain report is unaffected — no `suppressed` key at all.
#[test]
fn diag_show_suppressed_lists_every_hidden_finding_with_its_reason() {
    let fixture = fixtures_dir().join("noqaSuppression.tcl");
    let path = fixture.to_str().unwrap();

    let plain: serde_json::Value =
        serde_json::from_slice(&run_tcl_allow_failure(&["diag", "--json", path]))
            .expect("diag JSON");
    for file in plain.as_array().expect("report array") {
        assert!(
            file.get("suppressed").is_none(),
            "without the flag the JSON carries no `suppressed` key: {file}"
        );
    }

    let rows = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--json",
        "--show-suppressed",
        path,
    ]));
    let find = |code: &str| {
        rows.iter()
            .find(|r| r["code"] == code)
            .unwrap_or_else(|| panic!("no suppressed {code} row: {rows:?}"))
    };
    let w210 = find("W210");
    assert_eq!(w210["reason"], "inline-directive", "{w210}");
    assert!(
        w210["message"]
            .as_str()
            .is_some_and(|m| m.contains("suppressedByCode")),
        "the W210 on the `suppressedByCode` line: {w210}"
    );
    let s100 = find("S100");
    assert_eq!(s100["reason"], "inline-directive", "{s100}");
    assert!(
        s100["message"]
            .as_str()
            .is_some_and(|m| m.contains("dictValue")),
        "the S100 on the `dictValue` line: {s100}"
    );
}

/// A code a layer disables is a declared gap when nothing computes a
/// finding for it — `--disable` occupies the invocation layer — and the
/// default-off seed (W242 here) is omitted: it is the catalogue's baseline,
/// identical for every file, and listing it on every file buries the answer.
#[test]
fn diag_show_suppressed_lists_a_disabled_analyser_code_as_a_gap() {
    let rows = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--disable",
        "W210",
        "--show-suppressed",
        "--json",
        "--source",
        "puts $y",
    ]));
    // The optimiser, which `diag` never runs, is one row on every document;
    // the rest is the one code the invocation turned off.
    let (not_run, rows): (Vec<&serde_json::Value>, Vec<&serde_json::Value>) =
        rows.iter().partition(|r| r.get("codes").is_some());
    assert_eq!(not_run.len(), 1, "{not_run:?}");
    assert_eq!(not_run[0]["producer"], "optimiser", "{not_run:?}");
    assert_eq!(not_run[0]["reason"], "optimiser-off", "{not_run:?}");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["line"], serde_json::Value::Null, "{rows:?}");
    assert_eq!(rows[0]["code"], "W210", "{rows:?}");
    assert_eq!(rows[0]["reason"], "disabled:invocation", "{rows:?}");
    assert!(
        !rows.iter().any(|r| r["code"] == "W242"),
        "the default-off seed is not listed as a gap: {rows:?}"
    );
}

/// The codes only the optimiser emits, which `tcl diag` declares as the one
/// producer it did not run: every catalogued optimisation code but those
/// the compiler checks and the O111 producer emit.
fn codes_only_the_optimiser_emits() -> Vec<String> {
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::PRODUCED_WITHOUT_THE_OPTIMISER;
    DiagCode::ALL
        .iter()
        .filter(|code| code.is_optimisation() && !PRODUCED_WITHOUT_THE_OPTIMISER.contains(*code))
        .map(ToString::to_string)
        .collect()
}

/// The optimiser `diag` never runs is one `--show-suppressed` row per reason,
/// not one per code (`docs/design/compiler/diagnostic-policy.md` § Adapters):
/// the text row counts the codes, the JSON entry lists them, and a code the
/// compiler checks or the O111 producer emit is in neither, since those
/// producers ran.
#[test]
fn diag_show_suppressed_collapses_the_optimiser_into_one_row() {
    let only_the_optimisers = codes_only_the_optimiser_emits();
    let rows = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--show-suppressed",
        "--json",
        "--source",
        "set x 1   \nputs $y\n",
    ]));
    let not_run: Vec<&serde_json::Value> =
        rows.iter().filter(|r| r.get("codes").is_some()).collect();
    assert_eq!(not_run.len(), 1, "{rows:?}");
    let row = not_run[0];
    assert_eq!(row["producer"], "optimiser", "{row}");
    assert_eq!(row["reason"], "optimiser-off", "{row}");
    assert_eq!(row["message"], "optimiser not run on this surface", "{row}");
    assert_eq!(row["line"], serde_json::Value::Null, "{row}");
    assert!(row.get("code").is_none(), "{row}");
    assert_eq!(
        row["codes"],
        serde_json::json!(only_the_optimisers),
        "{row}"
    );
    for code in ["O100", "O105", "O106", "O111"] {
        assert!(
            !rows.iter().any(|r| r["code"] == code),
            "{code} is computed without the optimiser, so it is no gap: {rows:?}"
        );
    }

    let output = tcl()
        .args([
            "diag",
            "--show-suppressed",
            "--source",
            "set x 1   \nputs $y\n",
        ])
        .reused_output()
        .expect("failed to spawn tcl binary");
    let rendered = String::from_utf8(output.stdout).expect("utf-8 output");
    let sentence = format!(
        "optimiser not run on this surface ({} codes) [optimiser-off]",
        only_the_optimisers.len()
    );
    assert_eq!(
        rendered
            .lines()
            .filter(|line| line.contains(&sentence))
            .count(),
        1,
        "{rendered}"
    );
    assert_eq!(
        rendered.matches("[optimiser-off]").count(),
        1,
        "no per-code optimiser row: {rendered}"
    );
}

/// An abstaining document declares what `tcl diag` did not run, as any
/// document does: the analyser's skip and the optimiser, each for the
/// reason the policy gives — the abstention, the step that fires first. A
/// UTF-16 byte-order mark ahead of UTF-8 text is what abstains here.
#[test]
fn diag_show_suppressed_declares_what_an_abstaining_document_did_not_run() {
    let scratch = Scratch::new("abstain-declared");
    let bom = scratch.write_bytes("bom.tcl", b"\xFF\xFEset x 1   \nputs $y\n");
    let path = bom.to_str().expect("utf-8 path");

    let output = tcl()
        .args(["diag", "--show-suppressed", "--json", path])
        .reused_output()
        .expect("failed to spawn tcl binary");
    let codes: Vec<String> = diag_codes_by_file(&output.stdout)
        .into_iter()
        .map(|(_, code)| code)
        .collect();
    assert_eq!(codes, ["W109"], "the integrity code alone shows");
    let rows = diag_suppressed_rows(&output.stdout);
    let w242 = rows
        .iter()
        .find(|r| r["code"] == "W242")
        .unwrap_or_else(|| panic!("the analyser's declared skip: {rows:?}"));
    assert_eq!(w242["reason"], "encoding-abstention", "{w242}");
    let not_run: Vec<&serde_json::Value> =
        rows.iter().filter(|r| r.get("codes").is_some()).collect();
    assert_eq!(not_run.len(), 1, "{rows:?}");
    assert_eq!(not_run[0]["producer"], "optimiser", "{rows:?}");
    assert_eq!(not_run[0]["reason"], "encoding-abstention", "{rows:?}");
    assert_eq!(
        not_run[0]["codes"],
        serde_json::json!(codes_only_the_optimiser_emits()),
        "{rows:?}"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("suppressed=2 "), "{stderr}");
}

/// O111 pairs with every W100 the analyser finds; `tcl diag` keeps
/// the optimiser off by policy, so O111 is an `OptimiserOff`
/// suppression here rather than a finding that silently never existed.
#[test]
fn diag_show_suppressed_lists_o111_as_optimiser_off() {
    let src = "set a 1\nset b [expr $a + 1]\n";
    let shown = diag_codes_by_file(&run_tcl_allow_failure(&["diag", "--json", "--source", src]));
    assert!(shown.iter().any(|(_, code)| code == "W100"), "{shown:?}");
    let hidden = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--show-suppressed",
        "--json",
        "--source",
        src,
    ]));
    let o111 = hidden
        .iter()
        .find(|r| r["code"] == "O111")
        .unwrap_or_else(|| panic!("no suppressed O111 row: {hidden:?}"));
    assert_eq!(o111["reason"], "optimiser-off", "{o111}");
}

/// The text form's `--show-suppressed` rows carry `[reason]`, and a file
/// whose only findings are suppressed still exits `0` — the exit status
/// counts shown problems only.
#[test]
fn diag_show_suppressed_text_rows_keep_the_exit_status() {
    let scratch = Scratch::new("show-suppressed-text");
    let only_suppressed = scratch.write("noqa.tcl", "# noqa: W210\nputs $y\n");
    let output = tcl()
        .args([
            "diag",
            "--show-suppressed",
            only_suppressed.to_str().unwrap(),
        ])
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "a document whose only finding is suppressed must exit 0: {output:?}"
    );
    let rendered = String::from_utf8(output.stdout).expect("utf-8 output");
    assert!(
        rendered.contains("hidden") && rendered.contains("[inline-directive]"),
        "{rendered}"
    );
}
