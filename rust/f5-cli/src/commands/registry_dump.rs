//! The `registry-dump` verb — canonical JSON snapshots of the F5 registries.
//!
//! Serialises the F5 registry / graph snapshots from [`tcl_registry::snapshot`]
//! as two-space-indented, key-sorted JSON.
//!
//! ## Section coverage
//!
//! - `profiles` — emits the profile graph snapshot.
//! - `objects` — emits the object graph snapshot.
//! - `events` — emits the event graph snapshot (the per-event valid-command
//!   list is content-addressed via `validCommandsDigest`).
//! - `commands` — not implemented. It would embed the full per-command
//!   traits/scalars dicts and the hover prose catalogue (`summary`).
//!   It (and `all`, which contains it) exits with a
//!   not-implemented error.

use std::path::Path;

use tcl_registry::snapshot::{event_graph_snapshot, object_graph_snapshot, profile_graph_snapshot};

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

    // Two-space-indented, key-sorted JSON with a single trailing
    // newline appended (to stdout, or to the file).
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
