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

//! What is published when: an edit clears or introduces a diagnostic, and a large file publishes its fast tier before its deep tier.

use super::*;

#[test]
fn fixing_the_source_clears_the_diagnostic() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if $a {puts x}\n");
    assert!(has_code(&diags, "W100"));
    // Wrap the expression in braces via an incremental edit: `$a` -> `{$a}`.
    lsp.change_document(
        &uri,
        2,
        serde_json::json!([
            {
                "range": {
                    "start": { "line": 0, "character": 3 },
                    "end": { "line": 0, "character": 3 },
                },
                "text": "{",
            },
            {
                "range": {
                    "start": { "line": 0, "character": 6 },
                    "end": { "line": 0, "character": 6 },
                },
                "text": "}",
            },
        ]),
    );
    let cleared = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(!has_code(&cleared, "W100"));
}

#[test]
fn introducing_an_error_publishes_it() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert_eq!(
        lsp.open_ready(&uri, "puts hello\n"),
        [] as [serde_json::Value; 0]
    );
    // Replace the whole line with an arity error.
    lsp.replace_document(&uri, 2, "set\n");
    let diags = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(has_code(&diags, "E002"));
}

// Progressive (two-tier) diagnostics.

/// Build a large Tcl document (~`n` procs, ~10×`n` lines), plus one
/// proc whose unbraced `expr` yields a fast-tier **W100** (an analyser code) and
/// a deep-tier-only **O111** (the optimiser hint paired with W100) — so the two
/// progressive publishes are distinguishable regardless of package-database
/// state.
fn big_tcl_with_split_markers(n: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(n * 200);
    s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
    for i in 0..n {
        let _ = write!(
            s,
            "proc ::bench::step{i} {{a b}} {{\n\
             \x20   set v{i} [expr {{$a + $b}}]\n\
             \x20   if {{$v{i} > 10}} {{\n\
             \x20       set v{i} [expr {{$v{i} + 1}}]\n\
             \x20   }}\n\
             \x20   return $v{i}\n\
             }}\n\n"
        );
    }
    // The distinguishing pair: an unbraced `expr` → analyser W100 (fast tier),
    // whose paired optimiser hint O111 is produced only by the deep pass.
    s.push_str("proc ::bench::w100_probe {x} {\n    return [expr $x + 1]\n}\n");
    s
}

/// On a large / cold file the client sees the
/// workspace-independent **fast tier** first (analyser syntax / structural /
/// style diagnostics) and then the **deep tier** (adding compiler / optimiser
/// diagnostics), which is a strict superset and replaces it for the same
/// version.  The unbraced `expr` gives a fast-tier `W100`; its paired optimiser
/// hint `O111` is produced only by the deep pass, so the two publishes are
/// distinguishable regardless of package-database state.
///
/// A small / warm file settles inside `DIAGNOSTICS_FAST_TIER_BUDGET` and skips
/// the fast tier entirely (a single publish — the debounce-skip guarded by the
/// existing `diagnostics_delivery_smoke` single-publish tests). This test uses
/// a one-shot child-process seam to choose the same fast-tier branch without
/// making its coverage depend on host speed; other tests retain the actual
/// 40 ms budget race.
#[test]
fn large_file_publishes_fast_tier_before_deep_tier() {
    let mut lsp = Lsp::tcl_with_env(&[("TCL_LSP_TEST_FORCE_DIAGNOSTICS_FAST_TIER", "1")]);
    let uri = unique_uri("tcl");
    // 70 procs remain above the production 500-line progressive-tier floor,
    // while avoiding the unrelated scaling cost of the former 600-proc input.
    let big = big_tcl_with_split_markers(70);
    assert!(
        big.lines().count() >= 500,
        "fixture must enter the fast tier"
    );

    let since = lsp.notification_cursor();
    lsp.open_document_lang(&uri, &big, "tcl", 1);
    // The `[timing] deep diagnostics` log fires only from the deep publish, so it
    // is the reliable "deep pass finished" barrier; both publishes are buffered
    // by the time it arrives. This deadline observes eventual convergence rather
    // than imposing a deep-tier latency budget: the fast-tier deadline above is
    // the user-visible latency contract, while a loaded CI runner can legitimately
    // take longer to finish the deliberately large deep analysis.
    lsp.await_log(
        &["deep diagnostics", uri.as_str()],
        Duration::from_secs(75),
        since,
    );

    let pubs: Vec<Vec<Value>> = lsp
        .notifications()
        .into_iter()
        .skip(since)
        .filter(|n| {
            n.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
                && n.get("params")
                    .and_then(|p| p.get("uri"))
                    .and_then(Value::as_str)
                    == Some(uri.as_str())
        })
        .map(|n| {
            n.get("params")
                .and_then(|p| p.get("diagnostics"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect();

    let all_codes: Vec<BTreeSet<String>> = pubs.iter().map(|p| codes(p)).collect();
    let Some(deep_idx) = pubs.iter().position(|p| codes(p).contains("O111")) else {
        panic!(
            "no deep-tier publish (carrying the optimiser O111) arrived; publishes: {all_codes:?}"
        );
    };
    assert!(
        deep_idx >= 1,
        "the fast tier must be published before the deep tier on a large file, \
         but the first publish already carried the deep-only O111: {all_codes:?}",
    );

    let fast = codes(&pubs[deep_idx - 1]);
    let deep = codes(&pubs[deep_idx]);
    assert!(
        fast.contains("W100"),
        "the fast tier must carry the workspace-independent analyser W100: {fast:?}",
    );
    assert!(
        !fast.contains("O111") && !fast.contains("W120") && !fast.contains("W123"),
        "the fast tier must exclude deep-only optimiser and workspace-refined \
         codes: {fast:?}",
    );
    assert!(
        deep.contains("W100") && deep.contains("O111"),
        "the deep tier must carry both the analyser W100 and the optimiser O111: {deep:?}",
    );
    assert!(
        fast.is_subset(&deep),
        "the deep tier must be a strict superset of the fast tier (no fast-tier \
         diagnostic is ever removed by the deep pass): fast={fast:?} deep={deep:?}",
    );
}
