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

//! The `unredact` (`unmap`) verb — reverse a redaction using its map file.
//!
//! Reads the TOML map produced by `f5 redact` and rewrites every IPv4 / IPv6
//! literal back to its original value via [`tcl_bigip::redact::apply_map`].
//!
//! `--format scf` (default) emits the recovered text verbatim. `tmsh` /
//! `tmsh-delta` re-render the recovered config via the BIG-IP tmsh emit engine
//! (`tcl_bigip::tmsh_emit`); no pre-edit original is threaded, so `tmsh-delta`
//! treats every object as freshly created (and a non-config input falls back to
//! raw text).

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

    // `--format scf` => verbatim; `tmsh` / `tmsh-delta` re-render. No pre-edit
    // original is threaded (delta treats all as new).
    let out = crate::commands::emit::render_config(
        &out,
        &format.format,
        "modify",
        format.transaction,
        "",
    );
    if let Some(o) = output {
        std::fs::write(o, &out)?;
    } else {
        print!("{out}");
    }

    eprintln!("unredacted: {count} IP(s) recovered");
    Ok(0)
}
