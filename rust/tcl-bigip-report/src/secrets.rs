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

//! Master-key (`f5mku`) secret decryption for a loaded config.
//!
//! BIG-IP keeps credential-bearing values — SSL key passphrases, monitor
//! passwords, RADIUS / TACACS / SNMP secrets — encrypted under the unit master
//! key as `$M$<salt>$<base64>` envelopes. Given that key (the base64 string
//! `f5mku -K` prints), [`decrypt_secrets`] rewrites every such value in the SCF
//! to its clear text, so the report — and its embedded query console — show the
//! real secrets instead of opaque ciphertext. This runs entirely locally (pure
//! AES, no device access); the master key never leaves the browser.

use serde_json::Value as J;
use tcl_bigip::secrets::rewrite_secrets;
use tcl_f5mku::{decrypt, is_ciphertext};

use crate::query::ReportError;

/// Inventory every secret-bearing field in `scf` as `{object, field, value,
/// encrypted}` JSON (powers the report's Secrets tab).
///
/// Delegates to the shared [`tcl_bigip::secrets::collect_secrets`] scanner so
/// the Rust and Python report generators agree on the inventory. When the
/// config was decrypted upstream (via [`decrypt_secrets`]) the `value` is clear
/// text (hidden behind a reveal button in the report); otherwise `encrypted` is
/// true.
#[must_use]
pub fn collect_secrets(scf: &str) -> Vec<J> {
    tcl_bigip::secrets::collect_secrets(scf)
        .iter()
        .map(tcl_bigip::secrets::FoundSecret::to_json)
        .collect()
}

/// Count the `f5mku` `$M$…` encrypted secrets in `scf`.
///
/// Used to decide whether a master key is *needed* at all — the UI only asks for
/// one when the config actually carries encrypted secrets.
#[must_use]
pub fn count_encrypted_secrets(scf: &str) -> usize {
    let mut n = 0usize;
    rewrite_secrets(scf, |val| {
        if is_ciphertext(val) {
            n += 1;
        }
        None
    });
    n
}

/// Decrypt every `f5mku` `$M$…` secret in `scf` with the base64 `master_key`.
///
/// Returns `(new_scf, decrypted_count)`. Non-ciphertext values are left as-is.
/// If the config contains encrypted secrets but none could be decrypted (a
/// wrong or malformed master key), the underlying crypto error is surfaced so
/// the caller can tell the user the key was wrong — rather than silently
/// returning the config unchanged.
pub fn decrypt_secrets(scf: &str, master_key: &str) -> Result<(String, usize), ReportError> {
    let master_key = master_key.trim();
    if master_key.is_empty() {
        return Ok((scf.to_owned(), 0));
    }

    let mut first_err: Option<String> = None;
    let report = rewrite_secrets(scf, |val| {
        if !is_ciphertext(val) {
            return None;
        }
        match decrypt(val, master_key) {
            Ok(plain) => Some(plain),
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e.to_string());
                }
                None
            }
        }
    });

    if report.transformed == 0
        && let Some(e) = first_err
    {
        return Err(ReportError(format!("f5mku decryption failed: {e}")));
    }
    Ok((report.new_source, report.transformed))
}
