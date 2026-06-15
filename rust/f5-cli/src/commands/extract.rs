//! The `extract` (`ucs2scf`) verb — convert a local UCS archive to SCF text.
//!
//! Port of `_run_extract` (`tooling/f5/verbs/extract.py`) + `extract_ucs_file`
//! (`tooling/f5/f5_remote/ucs.py`). A UCS is a gzip-tar of `/config` (optionally
//! OpenPGP-encrypted); this writes the concatenated `bigip*.conf` members as a
//! Single Configuration File for the rest of the f5 CLI.
//!
//! For the default `--format scf` the SCF text passes through verbatim, so this
//! reaches byte-for-byte parity with the Python CLI. `tmsh` / `tmsh-delta`
//! rendering needs the BIG-IP tmsh emitter (a later engine port).

use std::path::Path;

use tcl_bigip_io::resolve_passphrase;
use tcl_bigip_io::ucs::{is_pgp_bytes, is_ucs_bytes, ucs_archive_to_scf};
use tcl_cli_support::{OutputTarget, write_text_output};

use crate::cli::{FormatArgs, PassphraseArgs};

/// `f5 extract <ucs>` — decrypt (if needed), extract, and emit SCF text.
pub fn run_extract(
    ucs: &Path,
    include_extras: bool,
    format: &FormatArgs,
    passphrase: &PassphraseArgs,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    if !ucs.is_file() {
        anyhow::bail!("not a file: {}", ucs.display());
    }
    let raw = std::fs::read(ucs).map_err(|e| anyhow::anyhow!("{}: {e}", ucs.display()))?;

    // `extract_ucs_file` rejects anything that is neither gzip nor OpenPGP up
    // front, so a mistyped path fails clearly rather than parsing as garbage.
    if !(is_ucs_bytes(&raw) || is_pgp_bytes(&raw)) {
        anyhow::bail!(
            "{}: not a UCS archive (gzip/OpenPGP magic missing)",
            ucs.display()
        );
    }

    let opts = passphrase.to_options();
    let provider = || resolve_passphrase(&opts);

    let label = ucs.display().to_string();
    let scf = ucs_archive_to_scf(&raw, &provider, include_extras, &label)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if format.format != "scf" {
        anyhow::bail!(
            "`f5 extract --format {}` is not yet implemented in the Rust port (needs the tmsh emitter)",
            format.format
        );
    }

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &scf)?;
    Ok(0)
}
