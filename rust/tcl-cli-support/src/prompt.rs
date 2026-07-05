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

//! Secure interactive terminal prompts for the CLIs.
//!
//! Reads a secret (e.g. a UCS decryption passphrase) from the controlling
//! terminal without echoing it, via [`rpassword`] — which restores terminal
//! echo on every exit path and falls back to `/dev/tty` when stdin is
//! redirected. Returns `Err` when there is no usable terminal, so callers fall
//! back to a non-interactive error rather than hanging.

/// Securely read the UCS decryption passphrase from the terminal (no echo). The
/// prompt text matches `getpass` provider (`"Enter UCS passphrase: "`).
///
/// # Errors
/// Returns the underlying I/O error string when there is no controlling
/// terminal or the read fails — the caller then falls through to a
/// non-interactive error.
pub fn read_ucs_passphrase() -> Result<String, String> {
    rpassword::prompt_password("Enter UCS passphrase: ").map_err(|e| e.to_string())
}
