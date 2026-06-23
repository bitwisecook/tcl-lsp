//! The `redact` (`sanitize`) verb — strip secrets and remap public IPs.
//!
//! Strips secret-bearing property values + PEM blocks via
//! [`tcl_bigip::redact::redact_secrets`] and consistently remaps public IPs into
//! a configurable target-CIDR pool, writing a sidecar map file so `f5 unredact`
//! can reverse it.
//!
//! `--format scf` (default) emits the rewritten text verbatim. `tmsh` /
//! `tmsh-delta` re-render the rewritten config via the BIG-IP tmsh emit engine
//! (`tcl_bigip::tmsh_emit`); no pre-edit original is threaded, so `tmsh-delta`
//! treats every object as freshly created.

use std::path::{Path, PathBuf};

use tcl_bigip::redact::{
    DEFAULT_TARGET_CIDRS_V4, DEFAULT_TARGET_CIDRS_V6, RedactOptions, RedactionMap, redact_secrets,
};
use tcl_bigip_io::read_path;

use crate::cli::FormatArgs;

/// Partition CIDRs into v4 and v6 pools (defaults if none supplied).
fn split_pools(cidrs: &[String]) -> (Vec<String>, Vec<String>) {
    let mut v4: Vec<String> = Vec::new();
    let mut v6: Vec<String> = Vec::new();
    for raw in cidrs {
        if raw.contains(':') {
            v6.push(raw.clone());
        } else {
            v4.push(raw.clone());
        }
    }
    let v4 = if v4.is_empty() {
        DEFAULT_TARGET_CIDRS_V4
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    } else {
        v4
    };
    let v6 = if v6.is_empty() {
        DEFAULT_TARGET_CIDRS_V6
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    } else {
        v6
    };
    (v4, v6)
}

/// Resolve the sidecar map path.
fn resolve_map_path(explicit: Option<&Path>, output: Option<&Path>) -> Option<PathBuf> {
    if let Some(e) = explicit {
        return Some(e.to_owned());
    }
    if let Some(o) = output {
        let mut s = o.as_os_str().to_owned();
        s.push(".redact.toml");
        return Some(PathBuf::from(s));
    }
    None
}

/// `f5 redact`.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn run_redact(
    path: &str,
    keep_ips: bool,
    target_cidr: &[String],
    shuffle: bool,
    seed: &str,
    remap_private: bool,
    map_file: Option<&Path>,
    source_cidr: &[String],
    format: &FormatArgs,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let (_uri, source) = match read_path(path, false, &opts) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let (target_v4, target_v6) = split_pools(target_cidr);

    let map_path = resolve_map_path(map_file, output);
    let mut existing_map: Option<RedactionMap> = None;
    if let Some(mp) = map_path.as_ref().filter(|mp| mp.is_file()) {
        let loaded = std::fs::read_to_string(mp)
            .map_err(|e| e.to_string())
            .and_then(|text| RedactionMap::from_toml(&text).map_err(|e| e.to_string()));
        match loaded {
            Ok(rm) => existing_map = Some(rm),
            Err(e) => {
                eprintln!("error: cannot load existing map {}: {e}", mp.display());
                return Ok(2);
            }
        }
    }

    let report = match redact_secrets(
        &source,
        RedactOptions {
            remap_ips: !keep_ips,
            target_pool_v4: target_v4,
            target_pool_v6: target_v6,
            mode: if shuffle {
                "shuffle".to_owned()
            } else {
                "direct".to_owned()
            },
            seed: seed.to_owned(),
            explicit_source_cidrs: source_cidr.to_vec(),
            existing_map,
            remap_private,
        },
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    // `--format scf` => verbatim; `tmsh` / `tmsh-delta` re-render. No pre-edit
    // original is threaded (delta treats all as new).
    let rendered = crate::commands::emit::render_config(
        &report.new_source,
        &format.format,
        "modify",
        format.transaction,
        "",
    );
    if let Some(out) = output {
        std::fs::write(out, &rendered)?;
    } else {
        print!("{rendered}");
    }

    if let (false, Some(mp), Some(rm)) = (keep_ips, &map_path, &report.redaction_map) {
        if let Some(parent) = mp.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(mp, rm.to_toml())?;
    }

    let map_suffix = map_path
        .as_ref()
        .map_or_else(String::new, |mp| format!("; map -> {}", mp.display()));
    eprintln!(
        "redacted: {} secret(s), {} PEM block(s), {} IP(s) remapped{}",
        report.redactions_count, report.pem_blocks_replaced, report.ips_remapped, map_suffix
    );
    Ok(0)
}
