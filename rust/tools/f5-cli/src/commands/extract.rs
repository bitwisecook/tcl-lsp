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

//! The `extract` (`ucs2scf`) verb — convert a local UCS archive to SCF text.
//!
//! A UCS is a gzip-tar of `/config` (optionally OpenPGP-encrypted); this writes
//! the concatenated `bigip*.conf` members as a Single Configuration File for the
//! rest of the f5 CLI.
//!
//! For the default `--format scf` the SCF text passes through verbatim. `tmsh`
//! / `tmsh-delta` re-render the extracted config via the BIG-IP tmsh emit
//! engine (`tcl_bigip::tmsh_emit`).

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

    // Reject anything that is neither gzip nor OpenPGP up front, so a mistyped
    // path fails clearly rather than parsing as garbage.
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

    let rendered = crate::commands::emit::render_config(
        &scf,
        &format.format,
        "create",
        format.transaction,
        "",
    );

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &rendered)?;
    Ok(0)
}
