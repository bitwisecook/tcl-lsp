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

//! Issue #2021 — `tclLsp.workspaceScan.maxFiles`: the on-disk workspace
//! scan's file budget is a user setting, not a constant.
//!
//! The reporter's Quartus `ip/altera` tree holds 3217 Tcl files against a
//! fixed cap of 2000, so a third of it was silently never indexed. The
//! observable here is the server's own scan marker —
//! `[timing] workspace_folders_scan …ms (roots=R, files=N)` — which reports
//! how many files the scan actually read and merged, and is the same line the
//! startup-cost work measures.
//!
//! Three facts: a configured budget bounds the scan, an unconfigured session
//! keeps the built-in 2000 (so the same tree is scanned whole), and a budget
//! pushed *after* start-up re-runs the scan rather than waiting for a restart.

use crate::common::{Lsp, scaled_timeout};

use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

/// A throwaway workspace root holding `count` tiny Tcl files, removed on drop.
struct ScanWorkspace {
    root: PathBuf,
}

impl ScanWorkspace {
    fn new(tag: &str, count: usize) -> Self {
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-e2e-scan-cap-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).expect("mk workspace root");
        for i in 0..count {
            std::fs::write(
                root.join(format!("file{i:03}.tcl")),
                format!("proc scan_probe_{i} {{}} {{ return {i} }}\n"),
            )
            .expect("write fixture");
        }
        Self { root }
    }
}

impl Drop for ScanWorkspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

/// The `files=N` count from the next `workspace_folders_scan` marker logged
/// after `since`.
fn scanned_files(lsp: &Lsp, since: usize) -> usize {
    let line = lsp.await_log(
        &["[timing] workspace_folders_scan"],
        scaled_timeout(Duration::from_secs(30)),
        since,
    );
    let tail = line
        .split_once("files=")
        .unwrap_or_else(|| panic!("scan marker carries no file count: {line}"))
        .1;
    tail.trim_end_matches(')')
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("unparsable file count in {line}: {e}"))
}

/// A budget below the tree's size bounds the scan to exactly that many files.
#[test]
fn configured_budget_bounds_the_startup_scan() {
    let ws = ScanWorkspace::new("capped", 30);
    let lsp = Lsp::with_config_at_root(json!({ "workspaceScan": { "maxFiles": 10 } }), &ws.root);
    assert_eq!(
        scanned_files(&lsp, 0),
        10,
        "the scan must stop at the configured budget"
    );
}

/// With nothing configured — the XDG defaults path, since the harness gives
/// each server an empty config home — the built-in 2000 applies, so a 30-file
/// tree is scanned whole.
#[test]
fn default_budget_scans_the_whole_tree() {
    let ws = ScanWorkspace::new("default", 30);
    let mut lsp = Lsp::at_workspace_root(&ws.root);
    assert_eq!(
        scanned_files(&lsp, 0),
        30,
        "an unconfigured session scans every file in a small tree"
    );
    assert_eq!(
        lsp.effective_config("")["workspace_scan_max_files"],
        json!(2000),
        "the reported budget is the built-in default"
    );
}

/// Raising the budget through `didChangeConfiguration` re-runs the scan, the
/// same treatment a `libraryPaths` change gets — a user who discovers the cap
/// mid-session does not have to restart the server to index the rest.
#[test]
fn raising_the_budget_rescans_without_a_restart() {
    let ws = ScanWorkspace::new("reload", 30);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "workspaceScan": { "maxFiles": 10 } }), &ws.root);
    assert_eq!(scanned_files(&lsp, 0), 10);

    let since = lsp.notification_cursor();
    lsp.apply_configuration_settle(json!({ "workspaceScan": { "maxFiles": 30 } }), "", |cfg| {
        cfg["workspace_scan_max_files"] == json!(30)
    });
    assert_eq!(
        scanned_files(&lsp, since),
        30,
        "the raised budget must re-run the scan, not wait for a restart"
    );
}
