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

//! Marshalling for the multi-release importer.
//!
//! `tcl_spec_studio::versions::import_package_versions` takes typed snapshots;
//! the browser has JSON. This module is the one place that conversion happens,
//! kept apart from `lib.rs` so it can be unit-tested on the host — the export
//! itself is then a two-line call with nothing left in it to get wrong.

use serde_json::Value;
use tcl_spec_studio::infer::SourceFile;
use tcl_spec_studio::versions::VersionedSnapshot;

/// Turn the front-end's `[{version, files: [{name, text}]}, …]` into snapshots.
///
/// A missing `version` is an error rather than a default: the whole derivation
/// is "what changed between *these* labels", so an unlabelled release would
/// silently corrupt every range it took part in. Missing per-file fields are
/// tolerated the way [`crate::import_package`] tolerates them — a nameless file
/// still analyses, it just reports under a placeholder.
pub fn parse(snapshots_json: &str) -> Result<Vec<VersionedSnapshot>, String> {
    let value: Value = serde_json::from_str(snapshots_json)
        .map_err(|e| format!("snapshots are not valid JSON: {e}"))?;
    let items = value
        .as_array()
        .ok_or_else(|| "snapshots must be a JSON array".to_owned())?;
    if items.is_empty() {
        return Err("no releases to import".to_owned());
    }
    let mut snapshots = Vec::with_capacity(items.len());
    for (at, item) in items.iter().enumerate() {
        let version = item
            .get("version")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("release {} has no version label", at + 1))?
            .to_owned();
        let files: Vec<SourceFile> = item
            .get("files")
            .and_then(Value::as_array)
            .map(|files| {
                files
                    .iter()
                    .map(|file| SourceFile {
                        name: file
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("untitled.tcl")
                            .to_owned(),
                        text: file
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        if files.is_empty() {
            return Err(format!("release {version} has no files"));
        }
        snapshots.push(VersionedSnapshot { version, files });
    }
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_labelled_releases_in_caller_order() {
        let parsed = parse(
            r#"[
                {"version": "1.0", "files": [{"name": "a.tcl", "text": "proc a {} {}"}]},
                {"version": " 1.1 ", "files": [{"name": "a.tcl", "text": "proc a {x} {}"}]}
            ]"#,
        )
        .expect("valid snapshots");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].version, "1.0");
        // The label is trimmed — a pasted tag often carries whitespace, and a
        // version comparison against " 1.1 " would not match "1.1".
        assert_eq!(parsed[1].version, "1.1");
        assert_eq!(parsed[0].files[0].name, "a.tcl");
    }

    #[test]
    fn a_file_missing_its_fields_still_analyses_under_a_placeholder() {
        let parsed = parse(r#"[{"version": "2", "files": [{}]}]"#).expect("valid snapshots");
        assert_eq!(parsed[0].files[0].name, "untitled.tcl");
        assert_eq!(parsed[0].files[0].text, "");
    }

    #[test]
    fn an_unlabelled_release_is_refused() {
        let err = parse(r#"[{"files": [{"name": "a.tcl", "text": ""}]}]"#).expect_err("no label");
        assert!(err.contains("release 1 has no version label"), "{err}");
        let err =
            parse(r#"[{"version": "  ", "files": [{"name": "a.tcl"}]}]"#).expect_err("blank label");
        assert!(err.contains("no version label"), "{err}");
    }

    #[test]
    fn an_empty_release_is_refused_by_name() {
        let err = parse(r#"[{"version": "3.1", "files": []}]"#).expect_err("no files");
        assert!(err.contains("release 3.1 has no files"), "{err}");
    }

    #[test]
    fn malformed_input_is_a_message_not_a_panic() {
        assert!(
            parse("nonsense")
                .expect_err("bad json")
                .contains("not valid JSON")
        );
        assert!(
            parse(r#"{"version": "1"}"#)
                .expect_err("not an array")
                .contains("must be a JSON array")
        );
        assert!(
            parse("[]")
                .expect_err("empty")
                .contains("no releases to import")
        );
    }
}
