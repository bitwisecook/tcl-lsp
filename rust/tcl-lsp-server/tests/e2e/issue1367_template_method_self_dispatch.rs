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

//! Issue #1367 — the template-method pattern across the workspace boundary.
//!
//! ruff's `::ruff::formatter::Formatter` (the #1181 corpus) is an abstract
//! base whose bodies call `my AddProgramElementHeading` / `my Escape`,
//! written only by the concrete formatters in **sibling files**. `my`
//! late-binds on the actual receiver — always a subclass instance — and
//! bypasses export filtering, so the capitalised (unexported) subclass
//! method is genuinely reachable and the eight W308s the corpus drew were
//! false positives.
//!
//! The single-file legs of the abstention live in
//! `rust/tcl-compiler/tests/analyser.rs` (`issue_1367_…`). This suite pins
//! the workspace half: the defining subclass lives in a *different
//! document*, so the abstention only works if the cross-file class index
//! feeds the base document's re-publish — the issue #977 "converged, not
//! first, publish" discipline.

use crate::common::{Lsp, unique_uri};
use serde_json::Value;
use std::time::Duration;

/// How long to let the cross-file class index converge — same scale as the
/// metaclass-chain suite's `CONVERGE`.
const CONVERGE: Duration = Duration::from_secs(25);

/// The base: one template dispatch a sibling will justify (`my Render`) and
/// one typo no file defines (`my Rendr`), so the converged state is exactly
/// one W308 and the test can tell abstention from a blanket suppression in
/// a single settle.
const BASE: &str = "namespace eval ::TM {}\n\
                    oo::class create ::TM::Formatter {\n\
                    \x20   method run {} { my Render }\n\
                    \x20   method oops {} { my Rendr }\n\
                    }\n";

/// The sibling document that makes `my Render` the template-method pattern.
const CONCRETE: &str = "oo::class create ::TM::Html {\n\
                        \x20   superclass ::TM::Formatter\n\
                        \x20   method Render {} { return ok }\n\
                        }\n";

fn w308_methods(diags: &[Value]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some("W308"))
        .filter_map(|d| d.get("message").and_then(Value::as_str))
        .filter_map(|m| {
            m.split_once('\'')
                .and_then(|(_, rest)| rest.split_once('\''))
                .map(|(name, _)| name.to_owned())
        })
        .collect()
}

/// TN + TP in one converged state: once the sibling that defines `Render`
/// is indexed, the base document re-publishes with the template dispatch
/// abstained and the typo still flagged.
#[test]
fn a_sibling_file_subclass_clears_w308_on_the_base_template_dispatch() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let concrete = unique_uri("tcl");

    // Alone, the base has no evidence for either name — both warn.
    let first = lsp.open_ready(&base, BASE);
    assert_eq!(
        {
            let mut m = w308_methods(&first);
            m.sort();
            m
        },
        vec!["Render".to_string(), "Rendr".to_string()],
        "single-file view has no defining subclass: {first:?}"
    );

    lsp.open_ready(&concrete, CONCRETE);

    // Converged: `Render` is justified by the sibling subclass, `Rendr`
    // is still a typo nothing defines.
    let settled =
        lsp.await_diagnostics_settled(&base, CONVERGE, |diags| w308_methods(diags) == ["Rendr"]);
    assert_eq!(
        w308_methods(&settled),
        vec!["Rendr".to_string()],
        "the template dispatch abstains, the typo stays: {settled:?}"
    );
}
