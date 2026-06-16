//! `f5 grep` (alias `related`) — list every BIG-IP object related to a given
//! object path or pattern. Mirrors `tooling/f5/verbs/grep.py`.

use std::path::Path;

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::grep::{GrepArgs, GrepReport, compute_grep, report_to_json};
use tcl_cli_support::{OutputTarget, write_text_output};

/// Build the reference graph from `inputs` (UCS-aware) and walk it from the
/// objects matching `pattern`, emitting the text / JSON / tmsh report.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn run_grep(
    pattern: &str,
    inputs: &[std::path::PathBuf],
    use_regex: bool,
    use_cidr: bool,
    direction: &str,
    max_depth: Option<usize>,
    recurse: bool,
    max_nodes: usize,
    include_body: bool,
    json: bool,
    output_format: &str,
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
    let source_uris: Vec<String> = sources.iter().map(|(uri, _)| uri.clone()).collect();

    // The graph builder loads the iRules dialect (matching the Python
    // `configure_signatures(dialect="f5-irules")` wrap in `compute_grep`).
    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    let report = compute_grep(
        &graph,
        &source_uris,
        &GrepArgs {
            pattern,
            use_regex,
            use_cidr,
            use_exact: false,
            direction,
            max_depth,
            max_nodes,
            include_body,
            recurse,
        },
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut rendered = if json {
        report_to_json(&report, include_body)
    } else if output_format == "tmsh" {
        matched_stanzas_as_tmsh(&report)
    } else {
        report.text_report.clone()
    };
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    write_text_output(&OutputTarget::from_arg(output), &rendered)?;

    // Exit 0 when at least one object matched, else 1 (the `grep` convention).
    Ok(u8::from(report.seeds.is_empty()))
}

/// Render every matched object as a one-line `tmsh create` command (port of
/// `_matched_stanzas_as_tmsh`). Dedup is on `node_id`; order is seeds then
/// related.
fn matched_stanzas_as_tmsh(report: &GrepReport) -> String {
    use std::fmt::Write as _;
    let mut parts = String::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for obj in report.seeds.iter().chain(report.related.iter()) {
        if !seen.insert(obj.node_id.as_str()) {
            continue;
        }
        let header = obj.header.trim();
        let body: String = obj.body.split_whitespace().collect::<Vec<_>>().join(" ");
        if body.is_empty() {
            let _ = writeln!(parts, "tmsh create {header} {{ }}");
        } else {
            let _ = writeln!(parts, "tmsh create {header} {{ {body} }}");
        }
    }
    parts
}
