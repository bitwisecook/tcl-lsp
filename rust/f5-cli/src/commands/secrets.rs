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

//! The `encrypt-secrets` / `decrypt-secrets` verbs — master-key crypto for the
//! credential-bearing values in a bigip.conf / SCF.
//!
//! Both verbs walk every secret-bearing property (`passphrase`, `password`,
//! `secret`, `shared-secret`, `auth-password`, `privacy-password`) via
//! [`tcl_bigip::secrets::rewrite_secrets`] and run its value through the F5 unit
//! master key supplied by the user — the base64 key `f5mku -K` prints on the
//! device. `encrypt-secrets` wraps clear-text values in the `$M$<salt>$<base64>`
//! envelope BIG-IP stores; `decrypt-secrets` recovers the clear text. Values
//! already in the target form are left untouched, so both verbs are idempotent
//! and safe to re-run.

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use tcl_bigip::secrets::rewrite_secrets;
use tcl_bigip_io::read_path;
use tcl_cli_support::secret_input::{SecretRequest, resolve_secret};

use crate::cli::{FormatArgs, MasterKeyArgs, PassphraseArgs};
use crate::commands::emit::render_config;
use crate::f5mku;

const F5MKU_ENV: &str = "F5MKU";
const KEY_PROMPT: &str = "F5 MKU Key: ";

/// A value already in a `$scheme$...` encoded form — an F5 `$M$` master-key
/// secret or an OS crypt(3) hash (`$1$` / `$5$` / `$6$` / `$2y$` …).
/// `encrypt-secrets` never re-wraps these.
fn already_encoded() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\$[A-Za-z0-9]+\$").expect("valid already-encoded regex"))
}

/// The two crypto directions the shared handler runs in.
#[derive(Clone, Copy)]
pub enum Mode {
    Encrypt,
    Decrypt,
}

impl Mode {
    /// Past-tense label for the stderr report line.
    fn past(self) -> &'static str {
        match self {
            Mode::Encrypt => "encrypted",
            Mode::Decrypt => "decrypted",
        }
    }
}

/// Resolve the master key from `--f5mku` / `--f5mku-file` / `$F5MKU` / prompt.
///
/// The base64 key is whitespace-trimmed (`strip: true`) so a file or env value
/// carrying a trailing newline — `f5mku -K > key.txt` — still works.
fn resolve_key(key_args: &MasterKeyArgs) -> Result<String, String> {
    let req = SecretRequest {
        explicit: key_args.f5mku.as_deref(),
        file: key_args.f5mku_file.as_deref(),
        env_var: Some(F5MKU_ENV),
        allow_prompt: !key_args.no_key_prompt,
        prompt: Some(KEY_PROMPT),
        strip: true,
    };
    let key = resolve_secret(&req).map_err(|e| e.to_string())?;
    key.filter(|k| !k.is_empty()).ok_or_else(|| {
        format!(
            "no master key supplied; pass --f5mku KEY, --f5mku-file FILE, set ${F5MKU_ENV}, \
             or run in an interactive terminal to be prompted"
        )
    })
}

/// `f5 encrypt-secrets` / `f5 decrypt-secrets`.
pub fn run_secrets(
    mode: Mode,
    path: &str,
    key_args: &MasterKeyArgs,
    salt: Option<&str>,
    passphrase: &PassphraseArgs,
    format: &FormatArgs,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let key = match resolve_key(key_args) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    // Fail fast on a bad key before touching the file.
    if let Err(e) = f5mku::encrypt("probe", &key, Some("00")) {
        eprintln!("error: {e}");
        return Ok(2);
    }

    let opts = passphrase.to_options();
    let (_uri, source) = match read_path(path, false, &opts) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    // The cipher closure may surface an F5MkuError (e.g. a non-`$M$` value that
    // slipped the decrypt guard, or a wrong key on decrypt); capture the first
    // one so the rewrite aborts with a clean error.
    let mut crypto_error: Option<String> = None;
    let report = {
        let key = &key;
        let crypto_error = &mut crypto_error;
        rewrite_secrets(&source, |value| {
            if crypto_error.is_some() {
                return None;
            }
            match mode {
                Mode::Encrypt => {
                    // Skip anything already in a `$scheme$...` encoded form: an
                    // existing `$M$` ciphertext (idempotent re-run) or a stored
                    // crypt(3) hash like `$6$...` that slipped past the key set —
                    // double-encrypting either would corrupt the credential.
                    if already_encoded().is_match(value) {
                        return None;
                    }
                    match f5mku::encrypt(value, key, salt) {
                        Ok(ct) => Some(ct),
                        Err(e) => {
                            *crypto_error = Some(e.to_string());
                            None
                        }
                    }
                }
                Mode::Decrypt => {
                    if !f5mku::is_ciphertext(value) {
                        return None; // not encrypted
                    }
                    match f5mku::decrypt(value, key) {
                        Ok(pt) => Some(pt),
                        Err(e) => {
                            *crypto_error = Some(e.to_string());
                            None
                        }
                    }
                }
            }
        })
    };

    if let Some(e) = crypto_error {
        eprintln!("error: {e}");
        return Ok(2);
    }

    let rendered = render_config(
        &report.new_source,
        &format.format,
        "modify",
        format.transaction,
        &source,
    );
    if let Some(out) = output {
        std::fs::write(out, rendered)?;
    } else {
        print!("{rendered}");
    }

    eprintln!(
        "{}: {} secret(s); {} left untouched",
        mode.past(),
        report.transformed,
        report.skipped
    );
    Ok(0)
}
