//! `registry-dump` verb: serialise the command registry as canonical JSON.
//!
//! Port of `_run_registry_dump` in `tooling/tcl/verbs/registry.py`, driving
//! the snapshot builders in `tcl_registry::command_snapshot`. The output is
//! `json.dumps(indent=2, sort_keys=True)`-faithful.

use std::path::Path;

use tcl_cli_support::{OutputTarget, registry_for_dialect, write_text_output};
use tcl_registry::command_snapshot::{command_registry_snapshot, command_registry_snapshots};

/// The Tcl dialects `--all-dialects` snapshots, in stable order — Python
/// `TCL_DIALECTS`.
const TCL_DIALECTS: [&str; 4] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"];

/// `tcl registry-dump` — dump the command registry for one dialect (or
/// every Tcl dialect with `--all-dialects`) as canonical JSON.
pub fn run_registry_dump(dialect: &str, all_dialects: bool, output: Option<&Path>) -> anyhow::Result<u8> {
    let target = OutputTarget::from_arg(output);
    // `build_default` already carries every Tcl dialect's commands, so the
    // `tcl8.6` registry serves all four Tcl dialects (and `--all-dialects`).
    let json = if all_dialects {
        let registry = registry_for_dialect("tcl8.6");
        command_registry_snapshots(registry, &TCL_DIALECTS)
    } else {
        let registry = registry_for_dialect(dialect);
        command_registry_snapshot(registry, dialect)
    };
    write_text_output(&target, &json.dumps_indent2())?;
    Ok(0)
}
