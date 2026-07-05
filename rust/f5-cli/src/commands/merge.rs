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

//! The `merge` verb — concatenate split per-partition SCFs back into one
//! bigip.conf.
//!
//! For the default `--format scf` this is pure file I/O (the SCF passes through
//! `render_config` verbatim). `--format tmsh` / `tmsh-delta` re-render the
//! merged config via the BIG-IP tmsh emit engine (`tcl_bigip::tmsh_emit`).

use std::path::{Path, PathBuf};

use anyhow::Context;
use tcl_cli_support::{OutputTarget, write_text_output};

use crate::cli::FormatArgs;

/// `f5 merge` — concatenate per-partition `.conf` files (or a directory).
pub fn run_merge(
    paths: &[PathBuf],
    format: &FormatArgs,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    // Collect input files: a directory contributes its sorted `*.conf`
    // children; anything else is taken verbatim.
    let mut files: Vec<PathBuf> = Vec::new();
    for raw in paths {
        if raw.is_dir() {
            let mut confs: Vec<PathBuf> = std::fs::read_dir(raw)
                .with_context(|| format!("failed to read directory {}", raw.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("conf"))
                .collect();
            confs.sort();
            files.extend(confs);
        } else {
            files.push(raw.clone());
        }
    }

    // Read each file via the UCS-aware resolver so a
    // `.ucs` member is transparently extracted to SCF before concatenation.
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let mut chunks: Vec<String> = Vec::with_capacity(files.len());
    for f in &files {
        let (_uri, src) = tcl_bigip_io::read_path(&f.to_string_lossy(), false, &opts)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        chunks.push(src);
    }

    let mut merged = emit_merged(&chunks);
    if !merged.ends_with('\n') {
        merged.push('\n');
    }

    let rendered = crate::commands::emit::render_config(
        &merged,
        &format.format,
        "create",
        format.transaction,
        "",
    );

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &rendered)?;
    Ok(0)
}

/// Concatenate split sources back into a single SCF text: drop blank chunks,
/// trim each to a single trailing newline, and join the chunks with a blank
/// line.
fn emit_merged(chunks: &[String]) -> String {
    chunks
        .iter()
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("{}\n", p.trim_end()))
        .collect::<Vec<_>>()
        .join("\n")
}
