//! `f5 stats` (alias `summary`) — object counts, partition breakdown, iRule
//! stats, and top-referenced objects. Mirrors `tooling/f5/verbs/stats.py`.

use std::path::Path;

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::stats::{compute_stats, report_to_json};
use tcl_cli_support::{OutputTarget, write_text_output};

/// Build the reference graph from `inputs` (UCS-aware) and print aggregate
/// statistics as text or JSON.
pub fn run_stats(
    inputs: &[std::path::PathBuf],
    top: Option<usize>,
    json: bool,
    output: Option<&Path>,
    passphrase: &crate::cli::PassphraseArgs,
) -> anyhow::Result<u8> {
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
    let report = compute_stats(&graph, &configs, top.unwrap_or(10));

    let mut rendered = if json {
        report_to_json(&report)
    } else {
        report.text_report.clone()
    };
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    write_text_output(&OutputTarget::from_arg(output), &rendered)?;
    Ok(0)
}
