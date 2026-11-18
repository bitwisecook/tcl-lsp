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

use super::{
    BARRIER_TIMEOUT_MARKER, Duration, Lsp, ScratchDir, latency_barrier_timeout_note, load_factor,
    scaled_timeout,
};

/// The footer every barrier timeout carries must keep classifying the
/// failure as a *timeout* (not an oracle divergence), render the live
/// scheduling probe, and point at the `heavy-lsp-e2e` isolation — so a
/// future edit that guts the message is caught here rather than in a
/// midnight CI triage. (Regression guard for the self-diagnosing barrier.)
#[test]
fn barrier_timeout_note_is_self_classifying() {
    let note = latency_barrier_timeout_note();
    assert!(
        note.contains(BARRIER_TIMEOUT_MARKER),
        "note must carry the greppable marker; got:\n{note}"
    );
    assert!(
        note.contains("TIMEOUT") && note.contains("NOT an oracle/content divergence"),
        "note must distinguish a timeout from a content divergence; got:\n{note}"
    );
    assert!(
        note.contains("PROBE:"),
        "note must render the live scheduling-health probe verdict; got:\n{note}"
    );
    assert!(
        note.contains("heavy-lsp-e2e"),
        "note must point at the nextest isolation; got:\n{note}"
    );
    assert!(
        note.contains("load_factor"),
        "note must say the barrier was already load-scaled, so a reader does not \
             re-derive 'just widen the timeout'; got:\n{note}"
    );
}

/// The measured capacity factor must never *shrink* a barrier: a machine
/// that is faster than the constants assume still gets exactly the deadline
/// the source says, and a slower one gets proportionally more. (Regression
/// guard: a factor below 1 would silently tighten every deadline in the
/// harness and manufacture the flakes this exists to remove.)
#[test]
fn load_factor_never_tightens_a_barrier() {
    let factor = load_factor();
    assert!(
        factor >= 1.0,
        "the capacity factor must be a multiplier >= 1, got {factor}"
    );
    let base = Duration::from_secs(30);
    assert!(
        scaled_timeout(base) >= base,
        "a scaled barrier must never be shorter than the constant it scales"
    );
}

#[test]
fn a_scratch_dir_is_removed_when_it_drops_even_on_panic() {
    let dir = std::env::temp_dir().join(format!("tcl-lsp-e2e-guard-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("nested")).unwrap();
    std::fs::write(dir.join("nested/file"), b"x").unwrap();
    drop(ScratchDir::at(dir.clone()));
    assert!(!dir.exists(), "dropped guard must remove the tree");

    let file = std::env::temp_dir().join(format!("tcl-lsp-e2e-guard-file-{}", std::process::id()));
    std::fs::write(&file, b"x").unwrap();
    drop(ScratchDir::at(file.clone()));
    assert!(!file.exists(), "dropped guard must remove a file path too");

    let panicked =
        std::env::temp_dir().join(format!("tcl-lsp-e2e-guard-panic-{}", std::process::id()));
    std::fs::create_dir_all(&panicked).unwrap();
    let path = panicked.clone();
    let outcome = std::panic::catch_unwind(move || {
        let _guard = ScratchDir::at(path);
        panic!("the test body fails");
    });
    assert!(outcome.is_err());
    assert!(
        !panicked.exists(),
        "unwinding must still remove the directory"
    );
}

#[test]
fn the_shared_workspace_root_exists_while_a_server_lives_and_is_gone_after() {
    let uri = super::harness_root_uri().to_owned();
    let root = std::path::PathBuf::from(uri.trim_start_matches("file://"));
    let lsp = Lsp::tcl();
    assert!(root.exists(), "a live server's workspace root is on disk");
    drop(lsp);
    // Other tests in this process may hold their own servers; the root is
    // removed only once the last one drops, so only assert absence when none
    // is alive in this process.
    if super::live_harness_servers() == 0 {
        assert!(!root.exists(), "the last server's drop removes the root");
    }
}
