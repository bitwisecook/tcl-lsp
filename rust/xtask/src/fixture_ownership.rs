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

//! Check authored test fixture readers and persisted oracle release keys.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use regex::Regex;
use tcl_test_support::fixture::{oracle_release, tsv_fields, tsv_lines};

// Direct tab splits in fixture-bearing Rust modules. Lower the limit when
// readers move to tcl_test_support::fixture. Protocol readers outside those
// modules and byte payloads inside oracle rows have separate owners.
const MAX_DIRECT_TSV_SPLITS: usize = 271;
// Native capture directories still use pinned patchlevels on work. The
// release-key migration shrinks this count as their readers move together.
const MAX_LEGACY_RELEASE_DIRECTORIES: usize = 1198;

pub(crate) fn run() -> Result<ExitCode> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let crates = std::fs::read_dir(root.join("rust"))?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .chain([root.join("runtime/rust")]);
    let tabs = Regex::new(
        r"\.(?:split|split_once|rsplit|rsplit_once|split_terminator)\(\s*'\\t'\s*\)|\.(?:splitn|rsplitn)\([^,\n]+,\s*'\\t'\s*\)",
    )?;
    let mut direct_splits = 0;
    let mut bad_keys = Vec::new();
    let mut legacy_directories = Vec::new();
    for project in crates {
        for relative in ["src", "tests"] {
            walk(&project.join(relative), |path, directory| {
                if !directory && path.extension().is_some_and(|extension| extension == "rs") {
                    let text = std::fs::read_to_string(path)?;
                    if text.contains("include_str!") && text.contains(".tsv") {
                        direct_splits += tabs.find_iter(&text).count();
                    }
                }
                Ok(())
            })?;
        }
        for relative in ["tests/data", "testdata"] {
            walk(&project.join(relative), |path, directory| {
                if directory
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(noncanonical_release_dir)
                {
                    legacy_directories.push(path.strip_prefix(&root).unwrap_or(path).to_path_buf());
                } else if path.extension().is_some_and(|extension| extension == "tsv")
                    && !path.ends_with("tcl-regex/tests/data/reg_cases.tsv")
                {
                    // In reg_cases.tsv, the first column is a regexp test ID (e.g. 8.4).
                    let text = std::fs::read_to_string(path)?;
                    for (line, row) in tsv_lines(&text) {
                        if row.contains('\t')
                            && let Some(key) = tsv_fields(row)
                                .next()
                                .filter(|key| noncanonical_release_key(key))
                        {
                            bad_keys.push(format!(
                                "{}:{line}: noncanonical release key {key}",
                                path.strip_prefix(&root).unwrap_or(path).display()
                            ));
                        }
                    }
                }
                Ok(())
            })?;
        }
    }
    bad_keys.sort_unstable();
    for fault in &bad_keys {
        eprintln!("{fault}");
    }
    println!(
        "fixture-ownership: {direct_splits} direct TSV fixture splits, {} noncanonical release keys, {} legacy release directories",
        bad_keys.len(),
        legacy_directories.len()
    );
    if direct_splits > MAX_DIRECT_TSV_SPLITS {
        eprintln!(
            "fixture TSV split count rose above {MAX_DIRECT_TSV_SPLITS}. Use tcl_test_support::fixture instead."
        );
    }
    if legacy_directories.len() > MAX_LEGACY_RELEASE_DIRECTORIES {
        eprintln!(
            "legacy release directory count rose above {MAX_LEGACY_RELEASE_DIRECTORIES}. Use canonical Tcl release keys for new fixtures."
        );
        legacy_directories.sort_unstable();
        for path in legacy_directories.iter().take(10) {
            eprintln!("{}", path.display());
        }
    }
    Ok(
        if bad_keys.is_empty()
            && direct_splits <= MAX_DIRECT_TSV_SPLITS
            && legacy_directories.len() <= MAX_LEGACY_RELEASE_DIRECTORIES
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        },
    )
}

fn noncanonical_release_key(key: &str) -> bool {
    if let Some(release) = key.strip_prefix("tcl") {
        tcl_dialect::TclVersion::parse(release).is_some() && oracle_release(key).is_none()
    } else {
        // A full unprefixed patchlevel is an observed measurement field.
        // Bare major.minor in the release-key column is an obsolete key.
        tcl_dialect::TclVersion::from_version_string(key).is_some()
    }
}

fn noncanonical_release_dir(key: &str) -> bool {
    let release = key.strip_prefix("tcl").unwrap_or(key);
    tcl_dialect::TclVersion::parse(release).is_some() && oracle_release(key).is_none()
}

fn walk(root: &Path, mut visit: impl FnMut(&Path, bool) -> Result<()>) -> Result<()> {
    let mut pending: Vec<PathBuf> = root
        .is_dir()
        .then(|| root.to_path_buf())
        .into_iter()
        .collect();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .with_context(|| format!("reading {}", directory.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() && !tcl_registry::dialects::is_skipped_scan_dir(&path) {
                visit(&path, true)?;
                pending.push(path);
            } else if kind.is_file() {
                visit(&path, false)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_fixture_release_keys_from_patchlevel_observations() {
        assert!(noncanonical_release_key("8.6"));
        assert!(noncanonical_release_key("tcl8.6.18"));
        assert!(!noncanonical_release_key("tcl8.6"));
        assert!(!noncanonical_release_key("8.6.18"));
        assert!(noncanonical_release_dir("8.6.18"));
        assert!(noncanonical_release_dir("tcl8.6.18"));
        assert!(!noncanonical_release_dir("tcl8.6"));
    }
}
