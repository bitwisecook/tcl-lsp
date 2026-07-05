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

//! The `tmsh` (`scf2tmsh`) verb — emit a tmsh script that recreates an SCF.
//!
//! Reads the input (`-` for stdin / UCS), validates `--include` kinds, parses
//! the config, and renders it via [`tcl_bigip::tmsh_emit::emit_tmsh`].
//! `--modify` swaps `create` for `modify`. The `tmsh: emitted N command(s)`
//! summary goes to stderr.

use std::path::Path;

use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip::tmsh_emit::emit_tmsh;
use tcl_bigip_io::read_path;
use tcl_cli_support::OutputTarget;

/// The kinds accepted by `--include`.
const VALID_KINDS: [&str; 10] = [
    "generic-foundation",
    "node",
    "monitor",
    "data-group",
    "profile",
    "snatpool",
    "persistence",
    "pool",
    "rule",
    "virtual",
];

/// `f5 tmsh <path>` — render an SCF as a tmsh create/modify script.
pub fn run_tmsh(
    path: &Path,
    output: Option<&Path>,
    modify: bool,
    include: &[String],
) -> anyhow::Result<u8> {
    let path_str = path.to_string_lossy();
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let (_uri, source) = match read_path(&path_str, false, &opts) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let include: Vec<&str> = if include.is_empty() {
        VALID_KINDS.to_vec()
    } else {
        for kind in include {
            if !VALID_KINDS.contains(&kind.as_str()) {
                eprintln!(
                    "error: unknown kind '{kind}' (expected one of {})",
                    VALID_KINDS.join(", ")
                );
                return Ok(2);
            }
        }
        include.iter().map(String::as_str).collect()
    };

    let cfg = parse_bigip_conf(&source, "Common");
    let script = emit_tmsh(&cfg, &source, &include, modify);

    let target = OutputTarget::from_arg(output);
    match &target {
        // Write verbatim, with no trailing-newline coercion; `script.text`
        // already ends in `\n` when non-empty.
        OutputTarget::File(p) => std::fs::write(p, &script.text)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", p.display()))?,
        OutputTarget::Stdout => {
            use std::io::Write;
            std::io::stdout()
                .lock()
                .write_all(script.text.as_bytes())
                .map_err(|e| anyhow::anyhow!("failed to write stdout: {e}"))?;
        }
    }

    eprintln!("tmsh: emitted {} command(s)", script.command_count);
    Ok(0)
}
