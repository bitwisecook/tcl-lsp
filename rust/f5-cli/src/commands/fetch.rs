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

//! `f5 fetch` — pull SCF/UCS from a live BIG-IP device.
//!
//! Handles credential resolution, transport dispatch, and the arg surfaces; the
//! live REST flow (UCS save → poll-download → `ucs_to_scf` →
//! cache-dir/`latest`/`fetch.meta`) and the SSH flow are both implemented but
//! exercised only against a live device.
//!
//! Two transports are supported: `rest` (iControl REST over HTTPS) and `ssh`
//! (the in-process pure-Rust `russh` client + SFTP subsystem — see
//! [`super::remote::ssh`]). `auto` (the default) tries REST first and falls back
//! to SSH when REST cannot connect.

use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};

use super::remote::auth::{Credentials, ResolveOptions, resolve_credentials, xdg_cache_dir};
use super::remote::rest::RestError;
use super::remote::{rest, ssh};

/// Parameters for [`run_fetch`].
pub struct FetchArgs<'a> {
    pub host: Option<&'a str>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    pub port: Option<u16>,
    pub ssh_port: Option<u16>,
    pub transport: &'a str,
    pub fmt: &'a str,
    pub insecure: bool,
    pub timeout: f64,
    pub output: Option<&'a str>,
    pub no_prompt: bool,
    pub print_path: bool,
}

/// A fetched config bundle + provenance.
struct FetchResult {
    host: String,
    transport: String,
    fmt: String,
    fetched_at: chrono::DateTime<Utc>,
    scf: String,
    ucs: Option<Vec<u8>>,
}

/// Run `f5 fetch`, returning the process exit code (0 / 2).
#[must_use]
pub fn run_fetch(args: &FetchArgs) -> u8 {
    let creds = match resolve_credentials(&ResolveOptions {
        host: args.host,
        user: args.user,
        password: args.password,
        port: args.port,
        ssh_port: args.ssh_port,
        interactive: !args.no_prompt,
    }) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let result = match fetch_scf(&creds, args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: fetch failed: {e}");
            return 2;
        }
    };

    if args.output == Some("-") {
        print!("{}", result.scf);
        return 0;
    }

    let out_dir = match resolve_output_dir(args.output, &result) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: fetch failed: {e}");
            return 2;
        }
    };
    if let Err(e) = write_result(&result, &out_dir) {
        eprintln!("error: fetch failed: {e}");
        return 2;
    }
    if args.print_path {
        println!("{}", out_dir.display());
    } else {
        eprintln!(
            "fetched {} from {} via {} -> {}",
            result.fmt,
            result.host,
            result.transport,
            out_dir.display()
        );
    }
    0
}

/// Dispatch the transport. `auto` tries REST first and falls back to SSH only
/// when REST cannot connect ([`RestError::Connection`]) — any other REST failure
/// (a non-2xx status, a read error) propagates rather than masking the device's
/// response behind an SSH attempt.
fn fetch_scf(creds: &Credentials, args: &FetchArgs) -> Result<FetchResult, String> {
    match args.transport {
        "rest" => via_rest(creds, args).map_err(String::from),
        "ssh" => via_ssh(creds, args),
        _ => match via_rest(creds, args) {
            Ok(r) => Ok(r),
            Err(RestError::Connection(_)) => via_ssh(creds, args),
            Err(other) => Err(other.to_string()),
        },
    }
}

/// Build the remote artefact name for a fetch started at `fetched_at`.
///
/// Millisecond (not second) resolution: two fetches against the same device
/// started within the same second would otherwise derive the identical
/// remote filename and one save/download could clobber or read the other's
/// artefact.
fn fetch_artefact_name(fetched_at: chrono::DateTime<Utc>) -> String {
    format!("f5_fetch_{}", fetched_at.timestamp_millis())
}

/// The live SSH flow: save the config on the device via `tmsh`, pull the
/// artefact back over the SFTP subsystem, and (for a UCS-only fetch) reconstruct
/// the SCF locally.
fn via_ssh(creds: &Credentials, args: &FetchArgs) -> Result<FetchResult, String> {
    let fetched_at = Utc::now();
    let name = fetch_artefact_name(fetched_at);
    let (scf_text, ucs_data) = ssh::fetch(creds, args.fmt, args.timeout, &name)?;
    Ok(FetchResult {
        host: creds.host.clone(),
        transport: "ssh".to_owned(),
        fmt: args.fmt.to_owned(),
        fetched_at,
        scf: scf_text,
        ucs: ucs_data,
    })
}

/// The live REST flow: trigger a UCS save, poll-download it, reconstruct the
/// SCF locally via `tcl_bigip_io::ucs_to_scf`. Returns a [`RestError`] so
/// `fetch_scf` can tell a failed connection (fall back to SSH) from any other
/// failure (propagate).
fn via_rest(creds: &Credentials, args: &FetchArgs) -> Result<FetchResult, RestError> {
    let fetched_at = Utc::now();
    let name = fetch_artefact_name(fetched_at);
    rest::save_ucs(creds, &name, args.insecure, args.timeout)?;
    let ucs_data = rest::download(
        creds,
        &format!("/mgmt/shared/file-transfer/ucs-downloads/{name}.ucs"),
        args.insecure,
        args.timeout,
    )?;
    let scf_text =
        tcl_bigip_io::ucs_to_scf(&ucs_data, false).map_err(|e| RestError::Other(e.to_string()))?;
    let keep_ucs = matches!(args.fmt, "ucs" | "both");
    Ok(FetchResult {
        host: creds.host.clone(),
        transport: "rest".to_owned(),
        fmt: args.fmt.to_owned(),
        fetched_at,
        scf: scf_text,
        ucs: keep_ucs.then_some(ucs_data),
    })
}

/// Resolve the artefact output directory. An explicit `-o DIR` is created
/// verbatim; otherwise the XDG cache layout
/// (`<cache>/<safe-host>/<UTC-timestamp>/`) is used, with a `latest` symlink.
fn resolve_output_dir(explicit: Option<&str>, result: &FetchResult) -> Result<PathBuf, String> {
    if let Some(explicit) = explicit {
        let out = PathBuf::from(explicit);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        return Ok(out);
    }
    let timestamp = result.fetched_at.format("%Y%m%dT%H%M%SZ").to_string();
    let base = xdg_cache_dir().join(safe_host_dir(&result.host));
    let out = base.join(&timestamp);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    update_latest_symlink(&base.join("latest"), &out);
    Ok(out)
}

/// Sanitise a host into a directory-safe component.
fn safe_host_dir(host: &str) -> String {
    host.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Point `link` at `target` (by basename). Silently skips on filesystems
/// without symlink support.
fn update_latest_symlink(link: &Path, target: &Path) {
    let Some(name) = target.file_name() else {
        return;
    };
    if link.is_symlink() || link.exists() {
        let _ = std::fs::remove_file(link);
    }
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(name, link);
    #[cfg(not(unix))]
    let _ = name;
}

/// Write `config.scf`, optionally `config.ucs`, and `fetch.meta` into `out_dir`.
fn write_result(result: &FetchResult, out_dir: &Path) -> Result<(), String> {
    std::fs::write(out_dir.join("config.scf"), &result.scf).map_err(|e| e.to_string())?;
    if let Some(ucs) = &result.ucs {
        std::fs::write(out_dir.join("config.ucs"), ucs).map_err(|e| e.to_string())?;
    }
    let meta = format!(
        "host = {}\ntransport = {}\nformat = {}\nfetched_at = {}\n",
        result.host,
        result.transport,
        result.fmt,
        result
            .fetched_at
            .to_rfc3339_opts(SecondsFormat::AutoSi, true),
    );
    std::fs::write(out_dir.join("fetch.meta"), meta).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_artefact_name_differs_within_the_same_second() {
        let base = Utc::now();
        let a = fetch_artefact_name(base);
        let b = fetch_artefact_name(base + chrono::Duration::milliseconds(1));
        assert_ne!(
            a, b,
            "two fetches within the same second must not derive the same remote artefact name"
        );
    }

    #[test]
    fn fetch_artefact_name_is_stable_for_the_same_instant() {
        let at = Utc::now();
        assert_eq!(fetch_artefact_name(at), fetch_artefact_name(at));
    }
}
