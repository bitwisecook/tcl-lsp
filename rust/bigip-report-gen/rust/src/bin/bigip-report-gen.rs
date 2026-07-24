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

//! `bigip-report-gen` — the native command-line BIG-IP report generator.
//!
//! The Rust twin of the Python `f5-report` CLI (`f5report/__main__.py`): turns
//! one or more BIG-IP configs or UCS archives into a single self-contained HTML
//! report, using the in-process `f5-query` engine for every fact. Unlike the
//! wasm/browser form, this runs entirely off the filesystem — and because it has
//! the native UCS reader ([`tcl_bigip_io`]) it also recovers the certificate
//! filestore (for the certs tab) and the forensic file inventory (for the
//! Forensics tab) straight out of a `.ucs`, which the library/PyO3 path can't.
//!
//! Gated behind the `cli` feature so the default build — and the wasm / `PyO3`
//! consumers, which render in-process and are time-free — stay lean.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine as _;
use chrono::Utc;

use bigip_report_gen_rust::{
    RenderOptions, Source, build_report, collect_model_full, decrypt_secrets,
};
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::driver::parse_bigip_conf;
use tcl_bigip_io::{
    MemberScope, PassphraseOptions, is_pgp_bytes, is_ucs_bytes, list_ucs_members, read_ucs_member,
    resolve_passphrase, ucs_archive_to_scf,
};

/// Cap on embedded content per forensic file (256 KiB) — mirrors the wasm
/// generator's `MAX_FORENSIC_CONTENT`. Auth/config files are small; a runaway
/// log-config include shouldn't bloat the page.
const MAX_FORENSIC_CONTENT: u64 = 256 * 1024;

const USAGE: &str = "\
usage: bigip-report-gen [options] PATH [PATH ...]

Generate an interactive, single-file HTML report from F5 BIG-IP configs or UCS
archives, powered by the in-process f5-query engine.

positional arguments:
  PATH                   bigip.conf / SCF files or .ucs archives (plain or encrypted)

options:
  -o, --output PATH      output HTML path (default: bigip-report.html; '-' for stdout)
  -t, --title TEXT       report title (default: F5 BIG-IP Configuration Report)
      --passphrase TEXT  passphrase for an encrypted UCS (or set $F5_UCS_PASSPHRASE)
      --f5mku KEY        base64 unit master key (f5mku -K) — decrypts the config's
                         $M$ secrets (SSL key passphrases, monitor/RADIUS/SNMP secrets)
      --f5mku-file FILE  read the base64 master key from FILE
      --include-extras   fold every config/*.conf UCS member (partitions, GTM) in
      --no-console       omit the in-browser WASM query console (smaller page)
      --copyright TEXT   copyright / confidentiality notice shown in the footer
      --copyright-file FILE  read the footer copyright notice from FILE
      --front-matter FILE    Markdown shown in a 'Front matter' tab (rendered at build time)
      --logo FILE        image (PNG/JPEG/SVG/GIF/WebP) inlined as the header logo
      --json             emit the report model as JSON instead of HTML
  -h, --help             show this help and exit
      --version          show version and exit
";

/// Parsed command line. Mirrors the Python `argparse` surface field-for-field.
#[derive(Default)]
struct Args {
    inputs: Vec<String>,
    output: Option<String>,
    title: Option<String>,
    passphrase: Option<String>,
    f5mku: Option<String>,
    f5mku_file: Option<String>,
    include_extras: bool,
    no_console: bool,
    copyright: Option<String>,
    copyright_file: Option<String>,
    front_matter: Option<String>,
    logo: Option<String>,
    json: bool,
}

/// Read the value for an option that takes one, erroring cleanly if missing.
fn need(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, ExitCode> {
    it.next().ok_or_else(|| {
        eprintln!("bigip-report-gen: {flag} requires a value\n\n{USAGE}");
        ExitCode::from(2)
    })
}

/// A tiny hand-rolled parser (no `clap` dep, matching the Python CLI's argparse
/// surface). Returns `Err(exit_code)` for `--help`/`--version` (0) and usage
/// errors (2).
fn parse_args() -> Result<Args, ExitCode> {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Err(ExitCode::SUCCESS);
            }
            "--version" => {
                println!(
                    "bigip-report-gen {} (engine {})",
                    bigip_report_gen_rust::GIT_DESCRIBE,
                    bigip_report_gen_rust::ENGINE_VERSION
                );
                return Err(ExitCode::SUCCESS);
            }
            "-o" | "--output" => a.output = Some(need(&mut it, &arg)?),
            "-t" | "--title" => a.title = Some(need(&mut it, &arg)?),
            "--passphrase" => a.passphrase = Some(need(&mut it, &arg)?),
            "--f5mku" => a.f5mku = Some(need(&mut it, &arg)?),
            "--f5mku-file" => a.f5mku_file = Some(need(&mut it, &arg)?),
            "--include-extras" => a.include_extras = true,
            "--no-console" => a.no_console = true,
            "--copyright" => a.copyright = Some(need(&mut it, &arg)?),
            "--copyright-file" => a.copyright_file = Some(need(&mut it, &arg)?),
            "--front-matter" => a.front_matter = Some(need(&mut it, &arg)?),
            "--logo" => a.logo = Some(need(&mut it, &arg)?),
            "--json" => a.json = true,
            "--" => a.inputs.extend(it.by_ref()),
            s if s.starts_with('-') && s != "-" => {
                eprintln!("bigip-report-gen: unknown option: {s}\n\n{USAGE}");
                return Err(ExitCode::from(2));
            }
            _ => a.inputs.push(arg),
        }
    }
    if a.inputs.is_empty() {
        eprintln!("bigip-report-gen: at least one input PATH is required\n\n{USAGE}");
        return Err(ExitCode::from(2));
    }
    Ok(a)
}

/// Guess a `data:` URI MIME type from a file extension, inlining an image so the
/// report stays a single file. Mirrors Python's `_logo_data_uri`.
fn logo_data_uri(path: &str) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

/// Is `name` a UCS by content + extension? (`OpenPGP` magic is unambiguous; plain
/// gzip is only a UCS for a `.ucs` path.) Mirrors `read_one` in the `PyO3` binding
/// and `extract_source` in the wasm binding.
fn looks_like_ucs(name: &str, bytes: &[u8]) -> bool {
    let is_ucs_ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ucs"));
    is_pgp_bytes(bytes) || (is_ucs_bytes(bytes) && is_ucs_ext)
}

/// Everything one input file contributes to the report.
struct Loaded {
    source: Source,
    /// `{cache-path: pem}` recovered from the UCS filestore (for the certs tab).
    cert_pems: HashMap<String, String>,
    /// Forensic member inventory `[{path,size,sha256,isText,content?}]`.
    files: Vec<serde_json::Value>,
}

/// Load one path into SCF plus, for a UCS, its cert filestore and forensic file
/// inventory (the native CLI's edge over the library path). A bare config
/// contributes only its text.
fn load_one(path: &str, passphrase: Option<&str>, include_extras: bool) -> Result<Loaded, String> {
    let raw = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if !looks_like_ucs(path, &raw) {
        let scf = String::from_utf8_lossy(&raw).into_owned();
        return Ok(Loaded {
            source: (path.to_owned(), scf),
            cert_pems: HashMap::new(),
            files: Vec::new(),
        });
    }

    let opts = PassphraseOptions {
        explicit: passphrase.map(str::to_owned),
        allow_prompt: false,
        ..PassphraseOptions::default()
    };
    let provider = || resolve_passphrase(&opts);

    let scf =
        ucs_archive_to_scf(&raw, &provider, include_extras, path).map_err(|e| e.to_string())?;
    let cert_pems = extract_cert_pems(&raw, &scf, path, &provider);
    let files = extract_forensic_files(&raw, path, &provider);
    Ok(Loaded {
        source: (path.to_owned(), scf),
        cert_pems,
        files,
    })
}

/// Recover certificate PEMs from a UCS filestore, keyed by `cache-path`. Mirrors
/// the wasm `extract_cert_files`; failures for an individual member are skipped
/// so a partial filestore still yields what it can.
fn extract_cert_pems(
    raw: &[u8],
    scf: &str,
    label: &str,
    provider: &impl Fn() -> Result<String, tcl_bigip_io::UcsError>,
) -> HashMap<String, String> {
    let config = parse_bigip_conf(scf, "Common");
    let mut map: HashMap<String, String> = HashMap::new();
    for placed in &config.objects {
        if let ModelObject::SysFileSslCert(c) = &placed.object {
            let member = if c.cache_path.is_empty() {
                c.source_path
                    .strip_prefix("file://")
                    .unwrap_or(&c.source_path)
                    .to_string()
            } else {
                c.cache_path.clone()
            };
            if member.is_empty() || map.contains_key(&member) {
                continue;
            }
            if let Ok(pem) = read_ucs_member(raw, &member, provider, label) {
                map.insert(member, String::from_utf8_lossy(&pem).into_owned());
            }
        }
    }
    map
}

/// Recover the forensic member inventory from a UCS, reading back small text
/// files' content. Mirrors the wasm `extract_files`. A read failure yields an
/// empty inventory (the Forensics tab then stays hidden).
fn extract_forensic_files(
    raw: &[u8],
    label: &str,
    provider: &impl Fn() -> Result<String, tcl_bigip_io::UcsError>,
) -> Vec<serde_json::Value> {
    let Ok(entries) = list_ucs_members(raw, provider, label, MemberScope::Forensic) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let content = if e.is_text && e.size <= MAX_FORENSIC_CONTENT {
            read_ucs_member(raw, &e.path, provider, label)
                .ok()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        } else {
            None
        };
        out.push(serde_json::json!({
            "path": e.path,
            "size": e.size,
            "sha256": e.sha256,
            "isText": e.is_text,
            "content": content,
        }));
    }
    out
}

/// Read a whole file to a trimmed string, wrapping the error with the flag name.
fn read_text_arg(path: &str, flag: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_owned())
        .map_err(|e| format!("could not read {flag}: {e}"))
}

fn run() -> Result<(), (u8, String)> {
    let args = parse_args().map_err(|code| {
        // help/version already printed; propagate the exit code with no message.
        (if code == ExitCode::SUCCESS { 0 } else { 2 }, String::new())
    })?;

    let title = args
        .title
        .unwrap_or_else(|| "F5 BIG-IP Configuration Report".to_owned());

    let master_key = match (&args.f5mku, &args.f5mku_file) {
        (_, Some(f)) => Some(read_text_arg(f, "--f5mku-file").map_err(|e| (2, e))?),
        (Some(k), None) => Some(k.clone()),
        (None, None) => None,
    };

    let copyright = match (&args.copyright, &args.copyright_file) {
        (_, Some(f)) => read_text_arg(f, "--copyright-file").map_err(|e| (2, e))?,
        (Some(c), None) => c.clone(),
        (None, None) => String::new(),
    };

    let front_matter = match &args.front_matter {
        Some(f) => std::fs::read_to_string(f)
            .map_err(|e| (2, format!("could not read --front-matter: {e}")))?,
        None => String::new(),
    };

    let logo = match &args.logo {
        Some(f) => logo_data_uri(f).map_err(|e| (2, format!("could not read --logo: {e}")))?,
        None => String::new(),
    };

    // Load every input, extracting UCS certs + forensic files along the way.
    let mut loaded = Vec::with_capacity(args.inputs.len());
    for path in &args.inputs {
        loaded.push(
            load_one(path, args.passphrase.as_deref(), args.include_extras)
                .map_err(|e| (2, format!("failed to load inputs: {e}")))?,
        );
    }

    // Decrypt `$M$…` secrets up front when a master key was supplied, so the
    // whole model (Secrets tab, monitor/RADIUS/SNMP values) reflects clear text.
    if let Some(key) = &master_key {
        for l in &mut loaded {
            let (clear, _n) = decrypt_secrets(&l.source.1, key).map_err(|e| (2, e.to_string()))?;
            l.source.1 = clear;
        }
    }

    let sources: Vec<Source> = loaded.iter().map(|l| l.source.clone()).collect();
    let cert_pems: HashMap<String, HashMap<String, String>> = loaded
        .iter()
        .filter(|l| !l.cert_pems.is_empty())
        .map(|l| (l.source.0.clone(), l.cert_pems.clone()))
        .collect();
    let files: HashMap<String, Vec<serde_json::Value>> = loaded
        .iter()
        .filter(|l| !l.files.is_empty())
        .map(|l| (l.source.0.clone(), l.files.clone()))
        .collect();

    let out = if args.json {
        let model = collect_model_full(&sources, &title, &cert_pems, &files, None);
        serde_json::to_string_pretty(&model).map_err(|e| (2, e.to_string()))?
    } else {
        let opts = RenderOptions {
            title,
            generated_at: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            embed_console: !args.no_console,
            cert_pems,
            files,
            architecture: None,
            report_id: String::new(),
            copyright,
            front_matter,
            logo,
        };
        build_report(&sources, &opts).map_err(|e| (2, e.to_string()))?
    };

    let output = args.output.as_deref().unwrap_or("bigip-report.html");
    if output == "-" {
        use std::io::Write as _;
        std::io::stdout()
            .write_all(out.as_bytes())
            .map_err(|e| (2, e.to_string()))?;
    } else {
        std::fs::write(output, &out).map_err(|e| (2, format!("{output}: {e}")))?;
        let kind = if args.json { "model" } else { "report" };
        eprintln!(
            "bigip-report-gen: wrote {kind} for {} device(s) to {output} ({} bytes)",
            sources.len(),
            out.len(),
        );
    }
    Ok(())
}

/// Stack budget for the dedicated worker thread the CLI runs on.
///
/// Matches `WORKER_STACK_SIZE` in `f5-cli`/`tcl-lsp-server`/`tcl-mcp`/
/// `tcl-cli`'s entry points — see `f5-cli/src/lib.rs`'s doc comment for the
/// full rationale. Short version: this binary calls straight into the
/// `tcl-bigip-query` DSL parser/evaluator, `tcl-bigip` config parser, and
/// `tcl_diagram::irule_flowchart_graph` (IR/CFG-based) for every iRule body
/// in the report — the same depth-capped-but-stack-hungry recursion chain
/// that crashed `tcl-lsp-server` in issue #996. The OS-provided main-thread
/// stack this binary would otherwise inherit is outside this crate's
/// control (8 MiB by default on Linux, far less guaranteed elsewhere), so
/// the CLI runs on an explicitly-sized thread instead.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

fn main() -> ExitCode {
    let result = std::thread::Builder::new()
        .stack_size(WORKER_STACK_SIZE)
        .spawn(run)
        .expect("failed to spawn the bigip-report-gen CLI worker thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    match result {
        Ok(()) | Err((0, _)) => ExitCode::SUCCESS,
        Err((_, msg)) => {
            if !msg.is_empty() {
                eprintln!("bigip-report-gen: {msg}");
            }
            ExitCode::from(2)
        }
    }
}
