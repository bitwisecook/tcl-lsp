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

//! `f5 graph` (alias `deps`) — emit the BIG-IP object reference graph as
//! DOT / JSON / Mermaid.

use std::path::Path;

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph, export_graph};
use tcl_cli_support::{OutputTarget, write_text_output};

/// Build the reference graph from `inputs` (UCS-aware, via `load_paths`) and
/// serialise it to `format`, optionally filtered to the subgraph reachable from
/// `seeds`.
pub fn run_graph(
    inputs: &[std::path::PathBuf],
    format: &str,
    seeds: &[String],
    reverse: bool,
    max_depth: Option<usize>,
    output: Option<&Path>,
    passphrase: &crate::cli::PassphraseArgs,
) -> anyhow::Result<u8> {
    // Resolve inputs via the UCS-aware loader — each
    // loaded config carries its uri + original source text, both of which the
    // graph builder needs (the source for the node/edge walk, the parsed config
    // for reference resolution).
    let opts = passphrase.to_options();
    let paths: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let loaded = tcl_bigip_io::load_paths(&paths, &opts).map_err(|e| anyhow::anyhow!("{e}"))?;

    let sources: Vec<(String, String)> = loaded
        .iter()
        .map(|l| (l.uri.clone(), l.source.clone()))
        .collect();
    let configs: Vec<(String, &tcl_bigip::parser::BigipConfig)> =
        loaded.iter().map(|l| (l.uri.clone(), &l.config)).collect();

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);
    let export = export_graph(&graph, format, seeds, reverse, max_depth)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    write_text_output(&OutputTarget::from_arg(output), &export.text)?;
    Ok(0)
}
