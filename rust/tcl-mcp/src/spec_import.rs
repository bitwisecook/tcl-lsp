// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `spec_import` — derive version ranges from local release snapshots.
//!
//! The MCP half of `tcl spec import`. Given several *labelled* snapshots of
//! one package's sources, it drafts each release, diffs them, and returns the
//! rendered `.tclspec` pack together with the per-command ranges the releases
//! witness and every warning the derivation raised.
//!
//! **Local artefacts only.** Every path is read from this machine; the tool
//! has no fetcher and cannot be made to reach the network. That is a property
//! of the code, not a policy: the shared pipeline lives in
//! [`tcl_cli_support::spec_import`], which links no HTTP client of its own, and
//! the tag enumeration and downloads of the CLI's `--github` mode stay in
//! `tcl-cli`. A caller who wants release history from a forge fetches it with
//! `tcl spec import --github`, or with `git`, and points this tool at what
//! landed on disk.
//!
//! The result is meant to be handed straight to
//! [`spectcl_check`](crate::spectcl::spectcl_check): the derivation is a
//! starting point carrying its evidence, not an assertion.

use std::path::Path;

use serde_json::{Value, json};
use tcl_cli_support::spec_import::{
    SpecImportError, SpecImportOptions, import_snapshots, load_snapshot,
};
use tcl_spec_studio::versions::VersionedSnapshot;

/// The MCP `spec_import` handler.
pub fn spec_import(args: &Value) -> Value {
    let snapshots = match read_snapshots(args) {
        Ok(snapshots) => snapshots,
        Err(message) => return json!({ "error": message }),
    };

    let dialect = crate::tools::declared_dialect(args);
    let package = args.get("package").and_then(Value::as_str);
    // Absent means "not claimed", which is the modest reading: presence in the
    // earliest snapshot is then not recorded as an introduction.
    let complete_history = args
        .get("complete_history")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let import = import_snapshots(
        &snapshots,
        &SpecImportOptions {
            dialect: tcl_lsp_core::profile_for_dialect(&dialect),
            package,
            complete_history,
        },
    );

    let mut result = import.to_json();
    if let Some(object) = result.as_object_mut() {
        object.insert("dialect".to_owned(), json!(dialect));
        object.insert("complete_history".to_owned(), json!(complete_history));
        object.insert(
            "summary".to_owned(),
            json!({
                "releases": import.versions.len(),
                "commands": import.commands.len(),
                "introduced": import.introduced_count(),
                "retired": import.retired_count(),
                "warnings": import.warnings.len(),
            }),
        );
    }
    result
}

/// Read the `snapshots` argument: `[{"version": "1.2", "path": "…"}, …]`.
///
/// Every entry must carry both fields — a snapshot without a label cannot be
/// placed in the history, and guessing one from a directory name would be the
/// kind of invention this whole pipeline exists to avoid.
fn read_snapshots(args: &Value) -> Result<Vec<VersionedSnapshot>, String> {
    let Some(entries) = args.get("snapshots").and_then(Value::as_array) else {
        return Err(
            "snapshots is required: [{\"version\": \"1.2\", \"path\": \"/path/to/release\"}, …]"
                .to_owned(),
        );
    };
    if entries.is_empty() {
        return Err(SpecImportError::NoSnapshots.to_string());
    }

    let mut snapshots = Vec::with_capacity(entries.len());
    for entry in entries {
        let version = entry
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let path = entry
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if version.is_empty() || path.is_empty() {
            return Err(format!(
                "each snapshot needs a non-empty `version` and `path`; got {entry}"
            ));
        }
        snapshots.push(load_snapshot(version, Path::new(path)).map_err(|e| e.to_string())?);
    }
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::dispatch;

    /// A throwaway directory tree, removed when the guard drops.
    struct Tree(std::path::PathBuf);

    impl Tree {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("tcl-mcp-spec-import-{tag}-{nanos}"));
            std::fs::create_dir_all(&dir).expect("scratch dir");
            Self(dir)
        }

        /// Write one release's single source file, returning its directory.
        fn release(&self, version: &str, text: &str) -> std::path::PathBuf {
            let dir = self.0.join(version);
            std::fs::create_dir_all(&dir).expect("release dir");
            std::fs::write(dir.join("pkg.tcl"), text).expect("write source");
            dir
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const V1: &str = "package provide demo 1.0\n\n# Greet someone.\nproc demo::greet {name} {\n    return \"hello $name\"\n}\n";
    const V2: &str = "package provide demo 2.0\n\n# Greet someone.\nproc demo::greet {name} {\n    return \"hello $name\"\n}\n\n# Say goodbye.\nproc demo::farewell {name} {\n    return \"bye $name\"\n}\n";

    fn run(snapshots: &Value, extra: &[(&str, Value)]) -> Value {
        let mut args = json!({ "snapshots": snapshots.clone(), "dialect": "tcl8.6" });
        for (key, value) in extra {
            args.as_object_mut()
                .expect("object")
                .insert((*key).to_owned(), value.clone());
        }
        dispatch("spec_import", &args).expect("spec_import tool")
    }

    fn command<'a>(result: &'a Value, name: &str) -> &'a Value {
        result["commands"]
            .as_array()
            .expect("commands array")
            .iter()
            .find(|c| c["name"] == name)
            .unwrap_or_else(|| panic!("no command {name} in {result}"))
    }

    #[test]
    fn a_command_added_in_the_second_release_is_introduced_there() {
        let tree = Tree::new("added");
        let v1 = tree.release("1.0", V1);
        let v2 = tree.release("2.0", V2);
        let result = run(
            &json!([
                { "version": "1.0", "path": v1.to_string_lossy() },
                { "version": "2.0", "path": v2.to_string_lossy() },
            ]),
            &[],
        );

        assert_eq!(result["package"], "demo", "{result}");
        assert_eq!(result["versions"], json!(["1.0", "2.0"]));
        assert_eq!(result["summary"]["commands"], 2, "{result}");
        assert_eq!(
            command(&result, "demo::farewell")["introduced_version"],
            "2.0"
        );
        // Without a complete-history claim, a command present in the oldest
        // snapshot has no derivable introduction.
        assert_eq!(
            command(&result, "demo::greet")["introduced_version"],
            Value::Null,
            "{result}"
        );
        let pack = result["pack"].as_str().expect("pack source");
        assert!(pack.contains("introduced_version 2.0"), "{pack}");
    }

    #[test]
    fn a_complete_history_pins_the_oldest_release_as_the_introduction() {
        let tree = Tree::new("complete");
        let v1 = tree.release("1.0", V1);
        let v2 = tree.release("2.0", V2);
        let result = run(
            &json!([
                { "version": "1.0", "path": v1.to_string_lossy() },
                { "version": "2.0", "path": v2.to_string_lossy() },
            ]),
            &[("complete_history", json!(true))],
        );
        assert_eq!(command(&result, "demo::greet")["introduced_version"], "1.0");
        assert_eq!(result["complete_history"], json!(true));
    }

    /// The snapshots are given newest first; version order decides, not the
    /// caller's order, and the reordering is reported.
    #[test]
    fn snapshots_out_of_order_are_sorted_and_reported() {
        let tree = Tree::new("unordered");
        let v1 = tree.release("1.0", V1);
        let v2 = tree.release("2.0", V2);
        let result = run(
            &json!([
                { "version": "2.0", "path": v2.to_string_lossy() },
                { "version": "1.0", "path": v1.to_string_lossy() },
            ]),
            &[],
        );
        assert_eq!(result["versions"], json!(["1.0", "2.0"]));
        let warnings = result["warnings"].as_array().expect("warnings");
        assert!(
            warnings
                .iter()
                .any(|w| w.as_str().is_some_and(|w| w.contains("ascending"))),
            "{result}"
        );
    }

    #[test]
    fn a_missing_snapshot_path_is_an_error_not_a_panic() {
        let result = run(
            &json!([{ "version": "1.0", "path": "/definitely/not/here" }]),
            &[],
        );
        assert!(
            result["error"]
                .as_str()
                .is_some_and(|e| e.contains("1.0") && e.contains("not a directory")),
            "{result}"
        );
    }

    #[test]
    fn snapshots_are_required() {
        let result =
            dispatch("spec_import", &json!({ "dialect": "tcl8.6" })).expect("spec_import tool");
        assert!(result["error"].as_str().is_some(), "{result}");

        let empty = run(&json!([]), &[]);
        assert!(empty["error"].as_str().is_some(), "{empty}");
    }

    /// The rendered pack is what `spectcl_check` is meant to be handed next,
    /// so it has to survive the real loader.
    #[test]
    fn the_rendered_pack_loads_through_the_real_loader() {
        let tree = Tree::new("roundtrip");
        let v1 = tree.release("1.0", V1);
        let v2 = tree.release("2.0", V2);
        let result = run(
            &json!([
                { "version": "1.0", "path": v1.to_string_lossy() },
                { "version": "2.0", "path": v2.to_string_lossy() },
            ]),
            &[],
        );
        let pack = result["pack"].as_str().expect("pack source").to_owned();
        let checked = dispatch(
            "spectcl_check",
            &json!({ "source": pack, "dialect": "tcl8.6" }),
        )
        .expect("spectcl_check tool");
        assert_eq!(checked["pack"], "demo", "{checked}");
        assert_eq!(checked["summary"]["commands"], 2, "{checked}");
        assert_eq!(checked["notices"], json!([]), "{checked}");
    }
}
