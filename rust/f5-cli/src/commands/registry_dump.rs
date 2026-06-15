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
//! - `commands` / `events` — **deferred**. Both embed the full
//!   event-validity cross-product (`validCommandsDigest` /
//!   `validEventsDigest`) and, for commands, the hover prose catalogue
//!   (`summary`); these reflect Python-internal derivation machinery without a
//!   clean, byte-identical Rust equivalent. They (and `all`, which contains
//!   them) report a clear not-yet-ported error.

use std::path::Path;

use tcl_registry::snapshot::{object_graph_snapshot, profile_graph_snapshot};

/// Run the `registry-dump` verb for `section`, writing to `output`
/// (`None` = stdout).
pub fn run_registry_dump(section: &str, output: Option<&Path>) -> anyhow::Result<u8> {
    let json = match section {
        "profiles" => profile_graph_snapshot(),
        "objects" => object_graph_snapshot(),
        "commands" | "events" | "all" => {
            anyhow::bail!(
                "`f5 registry-dump --section {section}` is not yet ported in the Rust port \
                 (the `commands` / `events` snapshots embed the event-validity cross-product \
                 and hover prose catalogue, which have no byte-identical Rust equivalent yet); \
                 the `profiles` and `objects` sections are available"
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
