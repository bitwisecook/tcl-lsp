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

//! Issue #1281 — renaming an ensemble `-map` target must not rewrite the
//! **subcommand word** at the dispatch site, while a `-subcommands` target
//! must still rewrite it.
//!
//! Every case here **applies** the emitted `WorkspaceEdit` back onto the
//! source and asserts the exact resulting program text, because the bug is
//! invisible at the "some edits came back" layer: the pre-fix edit set looked
//! complete and confident, and only the applied result — run under a real
//! `tclsh` — showed it had broken the dispatch.
//!
//! Oracle for every expected text below (tclsh 8.6.14 and 9.0.4, identical):
//!
//! ```tcl
//! # -map: the key is arbitrary, unrelated to the target's tail
//! namespace eval ::app::widget {}
//! proc ::app::widget::Show {} { return "showing" }
//! namespace ensemble create -command ::app::widget -map {show ::app::widget::Show}
//! puts [::app::widget show]     ;# -> showing
//! puts [::app::widget Show]     ;# -> unknown or ambiguous subcommand "Show": must be show
//!
//! # -subcommands: the entry IS the target's tail
//! namespace eval ::app::widget {
//!     proc beta {} { return "alpha!" }
//!     namespace ensemble create -command ::app::widget -subcommands {alpha}
//! }
//! puts [::app::widget alpha]    ;# -> invalid command name "alpha"
//! ```
//!
//! So a `-map` rename that rewrites `show` produces `::app::widget Display`
//! against a map that still says `show` (the reported corruption), and a
//! `-subcommands` rename that *doesn't* rewrite `alpha` produces an ensemble
//! whose entry names a proc that no longer exists.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// Apply **every** edit the rename returned for `uri` to `source`, rightmost
/// first so earlier ranges stay valid — the whole edit set, not a
/// hand-picked one, since the hazard is an edit that should not be in it.
fn apply_all(edits: &[Value], source: &str) -> String {
    fn offset(source: &str, pos: &Value) -> usize {
        let line = usize::try_from(pos["line"].as_u64().unwrap()).unwrap();
        let character = usize::try_from(pos["character"].as_u64().unwrap()).unwrap();
        let line_start: usize = source.split_inclusive('\n').take(line).map(str::len).sum();
        line_start + character
    }
    let mut resolved: Vec<(usize, usize, String)> = edits
        .iter()
        .map(|e| {
            let rng = &e["range"];
            (
                offset(source, &rng["start"]),
                offset(source, &rng["end"]),
                e["newText"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    resolved.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = source.to_owned();
    for (start, end, new_text) in resolved {
        out = format!("{}{}{}", &out[..start], new_text, &out[end..]);
    }
    out
}

/// The rename edit set for `uri`, applied to `src`.
fn renamed_source(lsp: &mut Lsp, uri: &str, src: &str, pos: (u32, u32), new_name: &str) -> String {
    let result = lsp.rename(uri, pos.0, pos.1, new_name);
    let edits = rename_edits(&result);
    apply_all(&edits.get(uri).cloned().unwrap_or_default(), src)
}

const MAP_SRC: &str = concat!(
    "namespace eval ::app::widget {}\n",
    "proc ::app::widget::Show {} { return \"showing\" }\n",
    "namespace ensemble create -command ::app::widget -map {show ::app::widget::Show}\n",
    "puts [::app::widget show]\n",
    "puts [::app::widget::Show]\n",
);

const SUBCOMMANDS_SRC: &str = concat!(
    "namespace eval ::app::widget {\n",
    "    proc alpha {} { return \"alpha!\" }\n",
    "    namespace ensemble create -command ::app::widget -subcommands {alpha}\n",
    "}\n",
    "puts [::app::widget alpha]\n",
);

#[test]
fn renaming_a_map_target_leaves_the_dispatch_word_alone_end_to_end() {
    // The reported bug. Renaming `::app::widget::Show` → `Display` must
    // rewrite the declaration and the `-map` *value* (that span really does
    // carry the target's name) and must leave the `show` of
    // `::app::widget show` untouched. Applied, the program still prints
    // `showing` twice under tclsh 8.6.14 / 9.0.4; the pre-fix edit set turned
    // line 3 into `puts [::app::widget Display]`, which errors with
    // `unknown or ambiguous subcommand "Display": must be show`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, MAP_SRC);
    // Cursor on the `Show` of the `proc ::app::widget::Show` declaration.
    let out = renamed_source(&mut lsp, &uri, MAP_SRC, (1, 20), "Display");
    assert_eq!(
        out,
        concat!(
            "namespace eval ::app::widget {}\n",
            "proc ::app::widget::Display {} { return \"showing\" }\n",
            "namespace ensemble create -command ::app::widget -map {show ::app::widget::Display}\n",
            "puts [::app::widget show]\n",
            "puts [::app::widget::Display]\n",
        ),
        "the -map key is arbitrary and must survive the target's rename",
    );
}

#[test]
fn renaming_a_map_target_from_the_dispatch_site_leaves_the_dispatch_word_alone_end_to_end() {
    // Same verdict from the other trigger position — the gate must not depend
    // on *which* occurrence the rename was invoked from (the discipline the
    // idx 79 verification pass established for the member-rename hazard).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, MAP_SRC);
    // Cursor on the `show` dispatch word of `puts [::app::widget show]`.
    let out = renamed_source(&mut lsp, &uri, MAP_SRC, (3, 20), "Display");
    assert!(
        out.contains("puts [::app::widget show]\n"),
        "the dispatch word must be intact however the rename was triggered: {out}",
    );
    assert!(
        !out.contains("::app::widget Display"),
        "the -map key must never become the target's new name: {out}",
    );
}

#[test]
fn renaming_a_subcommands_target_still_rewrites_the_dispatch_word_end_to_end() {
    // The true negative: the fix must not over-correct into never touching
    // ensemble dispatch. With `-subcommands`, the word IS the target's tail,
    // so the declaration, the `-subcommands` entry and the dispatch word all
    // move together — applied, the program still prints `alpha!` under
    // tclsh 8.6.14 / 9.0.4. Leaving the word behind would be
    // `invalid command name "alpha"`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SUBCOMMANDS_SRC);
    // Cursor on the `alpha` of the `proc alpha` declaration.
    let out = renamed_source(&mut lsp, &uri, SUBCOMMANDS_SRC, (1, 9), "beta");
    assert_eq!(
        out,
        concat!(
            "namespace eval ::app::widget {\n",
            "    proc beta {} { return \"alpha!\" }\n",
            "    namespace ensemble create -command ::app::widget -subcommands {beta}\n",
            "}\n",
            "puts [::app::widget beta]\n",
        ),
        "a -subcommands entry is the target's own tail and must follow it",
    );
}

#[test]
fn renaming_a_subcommands_target_from_the_dispatch_site_rewrites_it_too_end_to_end() {
    // The `-subcommands` half of the trigger-position independence check.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SUBCOMMANDS_SRC);
    // Cursor on the `alpha` dispatch word of `puts [::app::widget alpha]`.
    let out = renamed_source(&mut lsp, &uri, SUBCOMMANDS_SRC, (4, 20), "beta");
    assert!(
        out.contains("puts [::app::widget beta]\n"),
        "the -subcommands dispatch word must be rewritten from here too: {out}",
    );
}

#[test]
fn find_references_reports_the_dispatch_site_under_both_provenances_end_to_end() {
    // The gate is a *rename* gate. A dispatch site is a genuine reference
    // either way, so find-references must keep reporting it — the two share
    // `invocation_references_proc`, which is exactly why fixing rename by
    // weakening the reference rule would have been the easy wrong fix.
    let mut lsp = Lsp::tcl();
    let map_uri = unique_uri("tcl");
    lsp.open_ready(&map_uri, MAP_SRC);
    let map_refs = start_lines(&lsp.references(&map_uri, 1, 20, false));
    assert!(
        map_refs.contains(&3),
        "the -map dispatch site is still a reference: {map_refs:?}",
    );

    let sub_uri = unique_uri("tcl");
    lsp.open_ready(&sub_uri, SUBCOMMANDS_SRC);
    let sub_refs = start_lines(&lsp.references(&sub_uri, 1, 9, false));
    assert!(
        sub_refs.contains(&4),
        "the -subcommands dispatch site is still a reference: {sub_refs:?}",
    );
}

#[test]
fn a_dynamic_map_keeps_its_dispatch_word_unattributed_end_to_end() {
    // The abstention the issue asks to preserve: a `-map` built from a
    // variable records no mapping at all, so the dispatch word is never
    // attributed to the target and rename cannot rewrite it — the same answer
    // the `-map` gate gives, reached by never having the fact rather than by
    // discarding it. (References agrees: it reports only the declaration.)
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = concat!(
        "namespace eval ::app::widget {}\n",
        "proc ::app::widget::Show {} { return \"showing\" }\n",
        "set m [list show ::app::widget::Show]\n",
        "namespace ensemble create -command ::app::widget -map $m\n",
        "puts [::app::widget show]\n",
    );
    lsp.open_ready(&uri, src);
    let out = renamed_source(&mut lsp, &uri, src, (1, 20), "Display");
    assert!(
        out.contains("puts [::app::widget show]\n"),
        "a dynamic map leaves the dispatch word unattributed: {out}",
    );
}
