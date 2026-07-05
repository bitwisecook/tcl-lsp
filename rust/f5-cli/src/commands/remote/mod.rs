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

//! Remote-access helpers for the `f5` CLI.
//!
//! Hosts the credential resolver (`auth`), the iControl REST transport (`rest`),
//! the single-object pull/push request shaping (`object_io`), and the SSH/scp
//! transport (`ssh`) shared by the `fetch` / `push` / `pull` verbs.

pub mod auth;
pub mod json_compat;
pub mod object_io;
pub mod rest;
pub mod ssh;

/// Render a file-I/O error in the `[Errno N] <strerror>: '<path>'` format.
/// Used by `f5 push` for its missing-file / permission errors.
#[must_use]
pub fn os_error_string(err: &std::io::Error, path: &str) -> String {
    if let Some(errno) = err.raw_os_error() {
        let strerror = strerror(errno);
        format!("[Errno {errno}] {strerror}: '{path}'")
    } else {
        format!("{err}: '{path}'")
    }
}

/// The libc `strerror` text for the handful of errnos a file read surfaces.
/// Matches the GNU/Linux messages of the CI / dev target.
fn strerror(errno: i32) -> &'static str {
    match errno {
        2 => "No such file or directory",
        13 => "Permission denied",
        21 => "Is a directory",
        20 => "Not a directory",
        _ => "I/O error",
    }
}
