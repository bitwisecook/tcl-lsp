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

//! Which paths are Tcl sources: extensions, globs and the directory walk.

use std::path::{Path, PathBuf};

use tcl_registry::dialects::TCL_SOURCE_EXTENSIONS;
pub(in super::super) use tcl_registry::dialects::{is_skipped_scan_dir, is_tcl_source};

use crate::vfs;

/// Default upper bound on the number of files the on-disk workspace scan
/// will analyse, so a pathologically large tree can't stall
/// start-up.  Open documents are always indexed regardless of
/// this cap (they flow through `publish_analyser_diagnostics`).
///
/// Only the **default**: the effective bound is
/// `tclLsp.workspaceScan.maxFiles` (INI `[workspaceScan] max_files`), held in
/// `crate::BackendState::workspace_scan_max_files` and read per scan.  A workspace larger
/// than the bound is silently only partly indexed, which is why the bound is
/// configurable (issue #2021: a 3217-file Quartus `ip/altera` tree lost a
/// third of itself to the fixed 2000).
pub(in super::super) const WORKSPACE_SCAN_FILE_CAP: usize = 2000;

/// Upper bound on the number of directories the package-database tree scan
/// visits per workspace root, so a huge tree can't stall the package scan.
pub(in super::super) const WORKSPACE_SCAN_DIR_CAP: usize = 4000;

/// The `**/*.{…}` glob naming exactly [`TCL_SOURCE_EXTENSIONS`], for the
/// LSP registrations that take a glob rather than a predicate.
///
/// Lower-case only, and paired with the protocol's own
/// `FileOperationPatternOptions.ignoreCase` (see
/// [`rename_file_operation_options`]) — the `willRename` / `didRename` filters
/// are the only registrations that carry that option.  Registrations without
/// it use [`tcl_source_watch_glob`], which folds case in the glob itself.
pub(in super::super) fn tcl_source_glob() -> String {
    format!("**/*.{{{}}}", TCL_SOURCE_EXTENSIONS.join(","))
}

/// The same extension set as [`tcl_source_glob`], written so it matches **any
/// casing** — `**/*.{[tT][cC][lL],…}`.
///
/// `workspace/didChangeWatchedFiles` has no `ignoreCase` option, and `VS Code`
/// matches watcher globs against the platform file system: case-insensitively
/// on Windows and macOS, case-**sensitively** on Linux.  A single-cased glob
/// therefore missed every external create/change/delete of an `UPPER.TCL` on
/// Linux — the file was scanned, indexed and renameable, but its on-disk
/// changes produced no watch event, so the index only caught up on the next
/// full scan or an explicit open.
///
/// The set is the same one [`is_tcl_source`] case-folds, and the pattern is
/// built by the registry so the `VS Code` activation glob
/// (`cargo xtask gen-vscode-package`) is the same string by construction
/// rather than a second hand-maintained copy.
pub(in super::super) fn tcl_source_watch_glob() -> String {
    tcl_registry::dialects::tcl_source_glob_any_case()
}

/// Iteratively walk `root`, appending the paths of Tcl source
/// files to `out` until `out` reaches `cap` entries.  Skips the
/// directories [`is_skipped_scan_dir`] rejects. Unreadable directories are
/// omitted from the collected paths and make the returned completeness false,
/// so absence-sensitive consumers cannot treat a partial walk as exhaustive.
/// Iterative (explicit stack) to avoid unbounded recursion on deep trees.
pub(in super::super) fn collect_tcl_files(
    store: &dyn vfs::SourceStore,
    root: &Path,
    cap: usize,
    out: &mut Vec<PathBuf>,
) -> bool {
    let mut complete = true;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= cap {
            return false;
        }
        let Ok(entries) = store.read_dir(&dir) else {
            complete = false;
            continue;
        };
        for entry in entries {
            if entry.is_dir {
                if !is_skipped_scan_dir(&entry.path) {
                    stack.push(entry.path);
                }
            } else if entry.is_file && is_tcl_source(&entry.path) {
                if out.len() >= cap {
                    return false;
                }
                out.push(entry.path);
            }
        }
    }
    complete
}

/// `true` when `uri` sits under the workspace folder `folder`.
///
/// Performs the directory-boundary check the prefix-only
/// `target.starts_with(folder)` form gets wrong — without
/// the boundary check `file:///workspace/app` would also
/// match `file:///workspace/app2/...`.
/// The match succeeds when:
///
/// * `uri == folder`, OR
/// * `uri == folder` with the folder's trailing slash trimmed, OR
/// * `uri` starts with `folder + "/"`.
pub(in super::super) fn uri_under_folder(uri: &str, folder: &str) -> bool {
    let folder_trimmed = folder.trim_end_matches('/');
    if uri == folder_trimmed || uri == folder {
        return true;
    }
    // The boundary check: the byte immediately after the
    // folder prefix in `uri` must be `/`.  Compare against the
    // trimmed form so a trailing slash on `folder` doesn't
    // change the behaviour.
    let needle = format!("{folder_trimmed}/");
    uri.starts_with(&needle)
}
