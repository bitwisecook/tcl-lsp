//! The `convert` verb — `ucs2scf` (UCS → SCF, same shape as `extract`) and
//! `scf2as3` (bigip.conf → AS3 declaration JSON).
//!
//! `ucs2scf` reuses the UCS-extraction path; `scf2as3` runs the
//! `tcl_bigip::convert` AS3 engine and emits the declaration as
//! two-space-indented JSON.

use std::path::Path;

use tcl_bigip::convert::scf_to_as3;
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_io::ucs::{is_pgp_bytes, is_ucs_bytes, ucs_archive_to_scf};
use tcl_bigip_io::{read_path, resolve_passphrase};
use tcl_cli_support::{OutputTarget, write_text_output};

use crate::cli::PassphraseArgs;

/// `f5 convert <format> <path>` — dispatch to the requested conversion.
pub fn run_convert(
    format: &str,
    path: &str,
    output: Option<&Path>,
    tenant: &str,
    application: &str,
    report: bool,
    passphrase: &PassphraseArgs,
) -> anyhow::Result<u8> {
    match format {
        "ucs2scf" => run_ucs2scf(path, output, passphrase),
        "scf2as3" => run_scf2as3(path, output, tenant, application, report, passphrase),
        other => {
            eprintln!("error: unknown format {other:?}");
            Ok(2)
        }
    }
}

/// `ucs2scf` — unpack a local UCS archive to SCF text (same shape as `extract`).
fn run_ucs2scf(
    path: &str,
    output: Option<&Path>,
    passphrase: &PassphraseArgs,
) -> anyhow::Result<u8> {
    if path == "-" {
        eprintln!("error: UCS input must be a file path, not stdin");
        return Ok(2);
    }
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };
    if !(is_ucs_bytes(&data) || is_pgp_bytes(&data)) {
        eprintln!("error: {path}: not a UCS archive");
        return Ok(2);
    }
    let opts = passphrase.to_options();
    let provider = || resolve_passphrase(&opts);
    let text = match ucs_archive_to_scf(&data, &provider, false, path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };
    write_text_output(&OutputTarget::from_arg(output), &text)?;
    Ok(0)
}

/// `scf2as3` — convert a bigip.conf to an AS3 declaration (with `--report`).
fn run_scf2as3(
    path: &str,
    output: Option<&Path>,
    tenant: &str,
    application: &str,
    report: bool,
    passphrase: &PassphraseArgs,
) -> anyhow::Result<u8> {
    let opts = passphrase.to_options();
    let (_uri, source) = match read_path(path, false, &opts) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };
    let cfg = parse_bigip_conf(&source, "Common");
    let report_data = scf_to_as3(&cfg, tenant, application);
    let output_text = report_data.declaration.dumps_indent2() + "\n";

    write_text_output(&OutputTarget::from_arg(output), &output_text)?;

    if report {
        eprintln!(
            "as3: mapped {} object(s); {} unmapped",
            report_data.mapped,
            report_data.unmapped.len()
        );
        for entry in &report_data.unmapped {
            eprintln!("  unmapped: {entry}");
        }
    }
    Ok(0)
}
