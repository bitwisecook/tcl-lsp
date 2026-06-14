//! The `merge` verb — concatenate split per-partition SCFs back into one
//! bigip.conf. Port of `_run_merge` (`tooling/f5/verbs/merge.py`) +
//! `emit_merged` (`dialects/f5/bigip/emit.py`).
//!
//! For the default `--format scf` this is pure file I/O (the SCF passes through
//! `render_config` verbatim), so it reaches byte-for-byte parity. `tmsh` /
//! `tmsh-delta` rendering needs the BIG-IP tmsh emitter (a later engine port).

use std::path::{Path, PathBuf};

use anyhow::Context;
use tcl_cli_support::{write_text_output, OutputTarget};

use crate::cli::FormatArgs;

/// `f5 merge` — concatenate per-partition `.conf` files (or a directory).
pub fn run_merge(
    paths: &[PathBuf],
    format: &FormatArgs,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    // Collect input files: a directory contributes its sorted `*.conf`
    // children; anything else is taken verbatim (mirrors the Python glob).
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

    // Read each file via the UCS-aware resolver (mirrors `read_path`) so a
    // `.ucs` member is transparently extracted to SCF before concatenation.
    let opts = tcl_bigip_io::PassphraseOptions::default();
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

    if format.format != "scf" {
        anyhow::bail!(
            "`f5 merge --format {}` is not yet implemented in the Rust port (needs the tmsh emitter)",
            format.format
        );
    }

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &merged)?;
    Ok(0)
}

/// Concatenate split sources back into a single SCF text (mirrors
/// `emit_merged`): drop blank chunks, trim each to a single trailing newline,
/// and join the chunks with a blank line.
fn emit_merged(chunks: &[String]) -> String {
    chunks
        .iter()
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("{}\n", p.trim_end()))
        .collect::<Vec<_>>()
        .join("\n")
}
