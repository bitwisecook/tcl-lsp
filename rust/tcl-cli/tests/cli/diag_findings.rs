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

//! What `tcl diag` reports: cross-file call sites, release-dependent findings, source-style rows and the inline and file directives.

use super::*;

/// `tcl diag` over several inputs is a multi-file compilation, so a call in
/// one file must be visible to another file's interprocedural constant seed.
/// On its own, the library's two agreeing callers make `$mode eq "prod"`
/// fold (I230); adding the file that calls it with `dev` must retract that.
#[test]
pub(super) fn diag_shares_call_sites_across_inputs() {
    let lib = fixtures_dir().join("issue977Lib.tcl");
    let main = fixtures_dir().join("issue977Main.tcl");
    let alone = String::from_utf8(run_tcl_allow_failure(&["diag", lib.to_str().unwrap()]))
        .expect("utf-8 output");
    assert!(
        alone.contains("I230"),
        "the library alone has only agreeing callers, so the fold is expected: {alone}"
    );
    let together = String::from_utf8(run_tcl_allow_failure(&[
        "diag",
        lib.to_str().unwrap(),
        main.to_str().unwrap(),
    ]))
    .expect("utf-8 output");
    assert!(
        !together.contains("I230"),
        "issue977Main.tcl calls the helper with \"dev\": {together}"
    );
}

/// `lset x 3 Z` on a three-element list appends from Tcl 8.6 and raises
/// `list index out of range` on tclsh 8.4.20 and 8.5.19, so `tcl diag` reports
/// W231 only before 8.6 (#2428). The two releases either side of the change
/// prove `--dialect` reaches the check; every release and index spelling is
/// `tcl-compiler`'s `w231_reads_the_length_as_an_append_only_from_8_6`.
#[test]
fn diag_reports_an_lset_at_the_length_under_tcl_8_5_and_not_8_6() {
    let source = "set x {a b c}\nlset x 3 Z\nputs $x\n";
    for (dialect, reported) in [("tcl8.5", true), ("tcl8.6", false)] {
        let output = String::from_utf8(run_tcl_allow_failure(&[
            "diag",
            "--dialect",
            dialect,
            "--source",
            source,
        ]))
        .expect("utf-8 output");
        assert_eq!(output.contains("W231"), reported, "{dialect}: {output}");
    }
}

/// `# noqa` silences a diagnostic for `tcl diag` exactly as it does in the
/// editor (`docs/kcs/kcs-howto-suppress-diagnostics.md`): the directive covers
/// the analyser families (`W210`) and the compiler-check families (`S100`)
/// alike, because both surfaces ask the one shared `line_suppressed` helper.
///
/// A comment that merely mentions the word is not a directive, so the finding
/// below it still fires.
///
/// The control is the same fixture with its directive lines stripped: every
/// code the markers silence must come back, or this test would pass on a
/// `diag` that had simply stopped reporting.
#[test]
fn diag_honours_noqa_directives_the_way_the_editor_does() {
    let fixture = fixtures_dir().join("noqaSuppression.tcl");
    let source = std::fs::read_to_string(&fixture).expect("fixture is readable");

    let marked = diag_messages(&["diag", "--json", fixture.to_str().unwrap()]);
    for silenced in [
        // `# noqa: W210` — the named analyser code.
        "suppressedByCode",
        // bare `# noqa` — every code on the following command.
        "suppressedByBareNoqa",
        // `# noqa: S100` — a compiler-check code from the other lift.
        "dictValue",
    ] {
        assert!(
            !marked.iter().any(|m| m.contains(silenced)),
            "a preceding noqa must silence the finding on `{silenced}`: {marked:?}"
        );
    }
    assert!(
        marked.iter().any(|m| m.contains("reportedWithoutAMarker")),
        "an unmarked W210 must still be reported: {marked:?}"
    );
    assert!(
        marked.iter().any(|m| m.contains("reportedBesideProse")),
        "a comment that only mentions the word is not a directive: {marked:?}"
    );
    assert!(
        marked.iter().any(|m| m.contains("otherDict")),
        "an unmarked S100 must still be reported: {marked:?}"
    );

    let unmarked: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("# noqa"))
        .collect::<Vec<_>>()
        .join("\n");
    let without_markers = diag_messages(&["diag", "--json", "--source", &unmarked]);
    for reported in [
        "suppressedByCode",
        "suppressedByBareNoqa",
        "reportedWithoutAMarker",
        "reportedBesideProse",
        "dictValue",
        "otherDict",
    ] {
        assert!(
            without_markers.iter().any(|m| m.contains(reported)),
            "without its marker the finding on `{reported}` must fire: {without_markers:?}"
        );
    }
}

/// Every diagnostic message a `diag --json` run reports, across all its files.
fn diag_messages(args: &[&str]) -> Vec<String> {
    let report: serde_json::Value =
        serde_json::from_slice(&run_tcl_allow_failure(args)).expect("diag JSON");
    report
        .as_array()
        .expect("diag reports an array of files")
        .iter()
        .flat_map(|file| {
            file["diagnostics"]
                .as_array()
                .expect("diagnostics array")
                .iter()
                .map(|d| {
                    format!(
                        "{} {}",
                        d["code"].as_str().unwrap_or_default(),
                        d["message"].as_str().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// `tcl diag` runs the source-text pass the editor publishes, so a style
/// finding is not something you have to open an editor to see.
///
/// W111 / W112 / W115 / W118 come from the same `source_style` orchestrator the
/// server's style lift calls, and the byte-backed W107 / W109 ride with them.
#[test]
fn diag_reports_the_source_style_findings_the_editor_publishes() {
    let messages = diag_messages(&["diag", "--json", "--source", "set x 1   \nset y $x\n"]);
    assert!(
        messages.iter().any(|m| m.starts_with("W112")),
        "trailing whitespace must be reported: {messages:?}"
    );

    let long = format!("set x \"{}\"\nputs $x\n", "a".repeat(200));
    let long_messages = diag_messages(&["diag", "--json", "--source", &long]);
    assert!(
        long_messages.iter().any(|m| m.starts_with("W111")),
        "an over-long line must be reported: {long_messages:?}"
    );
}

/// A top-of-file `# tcl-lsp: disable=…` silences a code for every pass, not
/// only the analyser's own.
///
/// The analyser folds the directive into its internal disabled set, so its
/// codes obeyed it already; the source-text and compiler-check passes are
/// filtered by the caller, which is where the directive has to reach them.
#[test]
fn diag_honours_a_file_directive_across_every_pass() {
    let source = "# tcl-lsp: disable=W112\nset x 1   \nputs $x\n";
    let messages = diag_messages(&["diag", "--json", "--source", source]);
    assert!(
        !messages.iter().any(|m| m.starts_with("W112")),
        "a file-level directive must silence the style pass too: {messages:?}"
    );

    let without = diag_messages(&["diag", "--json", "--source", "set x 1   \nputs $x\n"]);
    assert!(
        without.iter().any(|m| m.starts_with("W112")),
        "without the directive the same document reports it: {without:?}"
    );
}

/// A file whose bytes are not UTF-8 text reports the integrity code alone, and
/// a file-level directive silences it by name or by the `*` wildcard.
///
/// `*` is the spelling `# tcl-lsp: disable=*` records, and it governs this
/// family as it governs every other. The document below is NUL-interleaved,
/// which is what makes the analysis abstain: everything derived from the
/// decoded text would describe positions the file does not have.
#[test]
fn diag_honours_a_file_directive_on_an_abstaining_document() {
    let nul_run = "\u{0}".repeat(80);
    let plain = format!("set x 1\n{nul_run}");
    let by_name = format!("# tcl-lsp: disable=W109\nset x 1\n{nul_run}");
    let by_wildcard = format!("# tcl-lsp: disable=*\nset x 1\n{nul_run}");

    let plain_rows = tcl_diag_rows("abstain-plain", &plain);
    assert_eq!(
        plain_rows
            .iter()
            .map(|(code, _)| code.as_str())
            .collect::<Vec<_>>(),
        ["W109"],
        "an abstaining document reports the integrity code and nothing else"
    );
    assert!(
        tcl_diag_rows("abstain-named", &by_name).is_empty(),
        "`disable=W109` must silence it"
    );
    assert!(
        tcl_diag_rows("abstain-wildcard", &by_wildcard).is_empty(),
        "`disable=*` must silence it too"
    );
}
