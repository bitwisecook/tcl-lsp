//! `f5 graph` (alias `deps`) — emit the BIG-IP object reference graph as
//! DOT / JSON / Mermaid. Mirrors `tooling/f5/verbs/graph.py`.

use std::path::Path;

use tcl_bigip::graph::{build_bigip_object_graph, export_graph, GraphContext};
use tcl_cli_support::{write_text_output, OutputTarget};

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
) -> anyhow::Result<u8> {
    // Resolve inputs via the UCS-aware loader (mirrors `load_paths`) — each
    // loaded config carries its uri + original source text, both of which the
    // graph builder needs (the source for the node/edge walk, the parsed config
    // for reference resolution).
    let opts = tcl_bigip_io::PassphraseOptions::default();
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
