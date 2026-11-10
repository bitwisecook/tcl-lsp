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

//! The process's `TCL_*` environment, read once.
//!
//! Diagnostic trace switches are consulted on hot analysis paths; a libc
//! `getenv` per query (a lock and a scan of `environ`) was a measurable share
//! of analysis time. Nothing in the process sets these variables after start.

use std::collections::HashMap;
use std::env::VarError;
use std::ffi::OsString;
use std::sync::OnceLock;

fn snapshot() -> &'static HashMap<OsString, OsString> {
    static SNAPSHOT: OnceLock<HashMap<OsString, OsString>> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        std::env::vars_os()
            .filter(|(name, _)| name.as_encoded_bytes().starts_with(b"TCL"))
            .collect()
    })
}

/// [`std::env::var_os`] for a `TCL*` variable, from the start-up snapshot.
#[must_use]
pub fn var_os(name: &str) -> Option<OsString> {
    snapshot().get(std::ffi::OsStr::new(name)).cloned()
}

/// [`std::env::var`] for a `TCL*` variable, from the start-up snapshot.
///
/// # Errors
/// [`VarError::NotPresent`] when unset, [`VarError::NotUnicode`] when the
/// value is not Unicode.
pub fn var(name: &str) -> Result<String, VarError> {
    match var_os(name) {
        Some(value) => value.into_string().map_err(VarError::NotUnicode),
        None => Err(VarError::NotPresent),
    }
}
