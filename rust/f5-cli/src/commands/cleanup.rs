//! `f5 cleanup` (alias `clean`) — emit `tmsh delete` commands for objects
//! unreferenced by any virtual / wide-IP. Mirrors `tooling/f5/verbs/cleanup.py`.

use std::collections::HashSet;
use std::path::Path;

use tcl_bigip::cleanup::{compute_cleanup, report_to_json};
use tcl_bigip::graph::{build_bigip_object_graph, GraphContext};
use tcl_cli_support::{write_text_output, OutputTarget};

/// Build the reference graph from `inputs` (UCS-aware) and emit a cleanup script
/// (or JSON report). `--keep` entries ending in `/` are partition prefixes;
/// `/Common/` is kept implicitly unless `no_keep_common`.
pub fn run_cleanup(
    inputs: &[std::path::PathBuf],
    keep: &[String],
    no_keep_common: bool,
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

    // Split --keep into exact paths and partition prefixes (trailing '/').
    let mut keep_paths: HashSet<String> = HashSet::new();
    let mut keep_partitions: Vec<String> = Vec::new();
    for entry in keep {
        if entry.ends_with('/') {
            keep_partitions.push(entry.clone());
        } else {
            keep_paths.insert(entry.clone());
        }
    }
    if !no_keep_common {
        keep_partitions.push("/Common/".to_owned());
    }

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);
    let mut source_uris: Vec<String> = sources.iter().map(|(u, _)| u.clone()).collect();
    source_uris.sort();
    let report = compute_cleanup(&graph, &source_uris, &keep_paths, &keep_partitions);

    let mut rendered = if json {
        report_to_json(&report)
    } else {
        report.tmsh_script.clone()
    };
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    write_text_output(&OutputTarget::from_arg(output), &rendered)?;
    Ok(0)
}
