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

//! Issue #1556 — `tclLsp.diagnostics.exclude`: files matching a configured
//! glob publish **no** diagnostics at all.
//!
//! Both assertions key on the server's own `diagnostics excluded` timing
//! marker (via [`Lsp::await_diagnostics_excluded`]) rather than a
//! version-tagged publish, for the same reason the master-switch test does
//! (issue #1135): a config change never bumps the document version, so a
//! pre-exclusion publish and the exclusion's empty one are otherwise
//! indistinguishable.

use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};
use std::time::Duration;

/// Source that reliably yields at least one on-by-default diagnostic
/// (trailing whitespace → W112).
const WARNY_SRC: &str = "set x 1  \n";

/// A name-only pattern (`*.ruff` — the motivating report's documentation
/// files) from the editor-config layer clears an open matching document's
/// squiggles, while a non-matching sibling keeps its diagnostics.  Follows
/// the master-switch test's shape: open first, flip the config, await the
/// marker (a config change never bumps the document version, so the marker
/// is the only reliable barrier).
#[test]
fn editor_config_name_pattern_excludes_open_document() {
    let mut lsp = Lsp::tcl();
    let excluded = unique_uri("ruff");
    assert!(
        !lsp.open_ready(&excluded, WARNY_SRC).is_empty(),
        "baseline diagnostics expected before the exclusion is configured"
    );

    let since = lsp.notification_cursor();
    lsp.apply_configuration(json!({ "diagnostics": { "exclude": ["*.ruff"] } }));
    assert_eq!(
        lsp.await_diagnostics_excluded(&excluded, Duration::from_secs(15), since),
        Vec::<Value>::new(),
        "an excluded document must publish an empty diagnostic set"
    );

    let control = unique_uri("tcl");
    assert!(
        !lsp.open_ready(&control, WARNY_SRC).is_empty(),
        "a document matching no exclude pattern must keep its diagnostics"
    );
}

/// A `.tcl-lsp.ini [diagnostics] exclude` path glob live-reloads: writing the
/// file clears an open matching document's squiggles (empty publish + marker),
/// and removing the exclusion brings them back — no restart either way.
#[test]
fn project_ini_exclude_live_reloads() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-diag-exclude-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(root.join("docs")).expect("mk workspace root");
    let doc_path = root.join("docs").join("gen.tcl");
    std::fs::write(&doc_path, WARNY_SRC).expect("write fixture");
    let mut lsp = Lsp::at_workspace_root(&root);
    let uri = format!("file://{}", doc_path.to_string_lossy());

    // Baseline: no project config yet, the document diagnoses normally.
    assert!(
        !lsp.open_ready(&uri, WARNY_SRC).is_empty(),
        "baseline diagnostics expected before any exclusion is configured"
    );

    let ini_path = root.join(".tcl-lsp.ini");
    let ini_uri = format!("file://{}", ini_path.to_string_lossy());
    let since = lsp.notification_cursor();
    std::fs::write(&ini_path, "[diagnostics]\nexclude =\n    docs/**\n").expect("write ini");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": ini_uri, "type": 1 }] }),
    );
    assert_eq!(
        lsp.await_diagnostics_excluded(&uri, Duration::from_secs(20), since),
        Vec::<Value>::new(),
        "the live-reloaded exclusion must clear the open document's diagnostics"
    );

    // Dropping the exclusion restores the diagnostics on the next reload.
    std::fs::write(&ini_path, "[project]\n").expect("rewrite ini");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": ini_uri, "type": 2 }] }),
    );
    lsp.await_diagnostics_settled(&uri, Duration::from_secs(20), |d| !d.is_empty());

    std::fs::remove_dir_all(&root).ok();
}
