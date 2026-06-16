//! The `registry-dump` verb — canonical JSON snapshots of the F5 registries.
//!
//! Port of `tooling/f5/verbs/registry.py`. Serialises the F5 registry / graph
//! snapshots from [`tcl_registry::snapshot`] as `json.dumps(indent=2,
//! sort_keys=True)`-parity JSON.
//!
//! ## Section coverage
//!
//! - `profiles` — fully ported, byte-identical to Python.
//! - `objects` — fully ported, byte-identical to Python.
//! - `events` — fully ported, byte-identical to Python (the per-event
//!   valid-command list is content-addressed via `validCommandsDigest`,
//!   exactly as Python does).
//! - `commands` — **deferred**. It embeds the full per-command traits/scalars
//!   dicts (mirroring the Python `CommandSpec` dataclass field layout) and the
//!   hover prose catalogue (`summary`), which have no clean, byte-identical
//!   Rust equivalent. It (and `all`, which contains it) reports a clear
//!   not-yet-ported error.

use std::path::Path;

use tcl_registry::snapshot::{
    event_graph_snapshot, object_graph_snapshot, profile_graph_snapshot,
};

/// Run the `registry-dump` verb for `section`, writing to `output`
/// (`None` = stdout).
pub fn run_registry_dump(section: &str, output: Option<&Path>) -> anyhow::Result<u8> {
    let json = match section {
        "profiles" => profile_graph_snapshot(),
        "objects" => object_graph_snapshot(),
        "events" => event_graph_snapshot(),
        "commands" | "all" => {
            anyhow::bail!(
                "`f5 registry-dump --section {section}` is not yet ported in the Rust port \
                 (the `commands` snapshot embeds the full per-command traits/scalars dicts \
                 and hover prose catalogue, which have no byte-identical Rust equivalent yet); \
                 the `profiles`, `objects`, and `events` sections are available"
            );
        }
        other => anyhow::bail!("unknown registry-dump section: {other}"),
    };

    // `json.dumps(payload, indent=2, sort_keys=True)` with a single trailing
    // newline (Python's `print` for stdout, `fh.write(text + "\n")` for files).
    let mut text = json.dumps_indent2();
    text.push('\n');

    if let Some(path) = output {
        std::fs::write(path, &text)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?;
    } else {
        use std::io::Write;
        std::io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .map_err(|e| anyhow::anyhow!("failed to write stdout: {e}"))?;
    }
    Ok(0)
}
