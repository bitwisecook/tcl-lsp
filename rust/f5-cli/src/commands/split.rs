//! The `split` verb — slice an SCF into per-partition files under a directory
//! (the inverse of `merge`).
//!
//! For `--format scf` the per-partition text is written verbatim (the SCF passes
//! through `render_config`). `--format tmsh` re-renders each partition via the
//! BIG-IP tmsh emit engine (`tcl_bigip::tmsh_emit`) and writes `.tmsh` files.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use tcl_bigip::parser::header::{ObjectTypeIndex, parse_generic_header};
use tcl_bigip::parser::helpers::extract_blocks;
use tcl_registry::BigipRegistry;

use crate::cli::FormatArgs;

/// Partition bucket for stanzas without an identifier-style partition.
const DEFAULT_PARTITION: &str = "_no_partition";

/// `f5 split` — write one `.conf` per partition under `output`.
pub fn run_split(input: &Path, output: &Path, format: &FormatArgs) -> anyhow::Result<u8> {
    let bytes =
        std::fs::read(input).with_context(|| format!("failed to read {}", input.display()))?;
    let source = String::from_utf8_lossy(&bytes).into_owned();

    std::fs::create_dir_all(output)
        .with_context(|| format!("failed to create {}", output.display()))?;

    let registry = BigipRegistry::build();
    let index = ObjectTypeIndex::build(&registry);

    // Group stanzas by partition, preserving first-seen order.
    let mut order: Vec<String> = Vec::new();
    let mut by_partition: HashMap<String, Vec<String>> = HashMap::new();

    for block in extract_blocks(&source) {
        let Some((_module, _object_type, identifier)) = parse_generic_header(&block.header, &index)
        else {
            continue;
        };
        let partition = {
            let trimmed = partition_of(&identifier);
            let trimmed = trimmed.trim_matches('/');
            if trimmed.is_empty() {
                DEFAULT_PARTITION.to_owned()
            } else {
                trimmed.to_owned()
            }
        };
        let text = slice_for_block(&source, block.start_offset, block.end_offset);
        let chunk = format!("{}\n", text.trim_end());
        if !by_partition.contains_key(&partition) {
            order.push(partition.clone());
        }
        by_partition.entry(partition).or_default().push(chunk);
    }

    let suffix = if format.format == "tmsh" {
        "tmsh"
    } else {
        "conf"
    };
    for partition in &order {
        let text = by_partition[partition].join("\n");
        let rendered = crate::commands::emit::render_config(
            &text,
            &format.format,
            "create",
            format.transaction,
            "",
        );
        let path = output.join(format!("{partition}.{suffix}"));
        std::fs::write(&path, rendered)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    eprintln!(
        "split into {} partition file(s) under {}",
        order.len(),
        output.display()
    );
    Ok(0)
}

/// Return the `/Common/` etc. partition prefix of an identifier, or `""`.
fn partition_of(identifier: &str) -> String {
    if identifier.is_empty() || !identifier.starts_with('/') {
        return String::new();
    }
    let parts: Vec<&str> = identifier.splitn(3, '/').collect();
    if parts.len() >= 2 {
        format!("/{}/", parts[1])
    } else {
        String::new()
    }
}

/// Full text of a stanza including its header line: walk back from the opening
/// `{` to the start of its line.
fn slice_for_block(source: &str, start_offset: usize, end_offset: usize) -> &str {
    let bytes = source.as_bytes();
    let mut header_start = start_offset;
    while header_start > 0 && bytes[header_start - 1] != b'\n' {
        header_start -= 1;
    }
    &source[header_start..end_offset]
}
