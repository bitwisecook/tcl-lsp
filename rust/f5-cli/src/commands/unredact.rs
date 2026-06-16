//! The `unredact` (`unmap`) verb — reverse a redaction using its map file.
//!
//! Port of `tooling/f5/verbs/unredact.py` (`_run_unredact`). Reads the TOML map
//! produced by `f5 redact` and rewrites every IPv4 / IPv6 literal back to its
//! original value via [`tcl_bigip::redact::apply_map`].
//!
//! `--format scf` (default) emits the recovered text verbatim, reaching
//! byte-for-byte parity. `tmsh` / `tmsh-delta` need the unported tmsh emitter
//! and are cleanly rejected.

use std::path::Path;

use tcl_bigip::redact::{RedactionMap, apply_map};
use tcl_bigip_io::read_path;

use crate::cli::FormatArgs;

/// `f5 unredact`.
pub fn run_unredact(
    map_file: &str,
    path: &str,
    format: &FormatArgs,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    if map_file == "-" && path == "-" {
        eprintln!("error: cannot read both map file and input from stdin");
        return Ok(2);
    }

    // Deferred: `tmsh` / `tmsh-delta` need the unported tmsh emitter.
    if format.format != "scf" {
        anyhow::bail!(
            "`f5 unredact --format {}` is not yet implemented in the Rust port (needs the tmsh emitter)",
            format.format
        );
    }

    let opts = crate::cli::PassphraseArgs::default().to_options();
    let (_map_uri, map_text) = match read_path(map_file, false, &opts) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };
    let (_uri, source) = match read_path(path, false, &opts) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let mut rm = match RedactionMap::from_toml(&map_text) {
        Ok(rm) => rm,
        Err(e) => {
            eprintln!("error: cannot load map: {e}");
            return Ok(2);
        }
    };

    let (out, count) = apply_map(&mut rm, &source, true);

    // `--format scf` => verbatim (`render_config`).
    if let Some(o) = output {
        std::fs::write(o, &out)?;
    } else {
        print!("{out}");
    }

    eprintln!("unredacted: {count} IP(s) recovered");
    Ok(0)
}
