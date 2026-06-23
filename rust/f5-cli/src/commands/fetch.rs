//! `f5 fetch` — pull SCF/UCS from a live BIG-IP device.
//!
//! Handles credential resolution, the SSH-deferral, and the arg surfaces; the
//! live REST flow (UCS save → poll-download → `ucs_to_scf` →
//! cache-dir/`latest`/`fetch.meta`) is implemented but exercised only against a
//! live device.
//!
//! SSH transport is not supported: `--transport ssh` returns the deferral
//! error, and `auto` falls back to it only when REST is unavailable.

use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};

use super::remote::auth::{Credentials, ResolveOptions, resolve_credentials, xdg_cache_dir};
use super::remote::rest;
use super::remote::ssh::SSH_DEFERRAL;

/// Parameters for [`run_fetch`].
#[allow(clippy::struct_excessive_bools)]
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

/// A fetched config bundle + provenance (Rust analogue of `FetchResult`).
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

/// Dispatch the transport: REST is the default and the only live one; SSH (and
/// the `auto` fallback to it) returns the deferral error.
fn fetch_scf(creds: &Credentials, args: &FetchArgs) -> Result<FetchResult, String> {
    match args.transport {
        "rest" => via_rest(creds, args),
        "ssh" => Err(SSH_DEFERRAL.to_owned()),
        // auto: try REST, fall back to the SSH deferral only when REST is
        // unavailable (connection error).
        _ => match via_rest(creds, args) {
            Ok(r) => Ok(r),
            Err(rest_err) => {
                if rest_err.starts_with("REST connection to") {
                    Err(SSH_DEFERRAL.to_owned())
                } else {
                    Err(rest_err)
                }
            }
        },
    }
}

/// The live REST flow: trigger a UCS save, poll-download it, reconstruct the
/// SCF locally via `tcl_bigip_io::ucs_to_scf`.
fn via_rest(creds: &Credentials, args: &FetchArgs) -> Result<FetchResult, String> {
    let fetched_at = Utc::now();
    let name = format!("f5_fetch_{}", fetched_at.timestamp());
    rest::save_ucs(creds, &name, args.insecure, args.timeout)?;
    let ucs_data = rest::download(
        creds,
        &format!("/mgmt/shared/file-transfer/ucs-downloads/{name}.ucs"),
        args.insecure,
        args.timeout,
    )?;
    let scf_text = tcl_bigip_io::ucs_to_scf(&ucs_data, false).map_err(|e| e.to_string())?;
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
