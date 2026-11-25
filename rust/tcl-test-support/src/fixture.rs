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

//! Shared readers for authored conformance fixtures.

use std::borrow::Cow;
use tcl_dialect::TclVersion;

/// One filesystem materialisation of the common workspace scan fixture.
pub struct WorkspaceScan(std::path::PathBuf);

impl WorkspaceScan {
    /// Materialise the file names in a private temporary workspace.
    ///
    /// # Panics
    /// Panics when the fixture cannot be created or written.
    #[must_use]
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let temp = std::env::temp_dir()
            .canonicalize()
            .expect("canonical temp directory");
        let workspace = temp.join(format!(
            "tcl-workspace-scan-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&workspace).expect("new workspace scan fixture");
        let fixture = Self(workspace);
        for name in include_str!("../data/workspace-scan-inputs.txt").lines() {
            let path = fixture.0.join(name);
            std::fs::create_dir_all(path.parent().expect("fixture file has a parent")).unwrap();
            let content = if name.ends_with(".tclspec") {
                "speclib scan_fixture 1 {}\n"
            } else if name.ends_with("/tclpkg.tcl") {
                "package require scan_fixture\n"
            } else {
                "set scan_fixture 1\n"
            };
            std::fs::write(path, content).expect("workspace fixture file");
        }
        fixture
    }

    /// Root of the fixture's native directory tree.
    #[must_use]
    pub fn root(&self) -> &std::path::Path {
        &self.0
    }

    /// The sorted Tcl-family paths expected from a recursive workspace scan.
    #[must_use]
    pub fn expected_sources(&self) -> Vec<std::path::PathBuf> {
        let mut paths: Vec<_> = include_str!("../data/workspace-scan-sources.txt")
            .lines()
            .map(|path| self.0.join(path))
            .collect();
        paths.sort();
        paths
    }
}

impl Default for WorkspaceScan {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WorkspaceScan {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Physical TSV rows with source line numbers, excluding blank and comment rows.
pub fn tsv_lines(fixture: &str) -> impl Iterator<Item = (usize, &str)> {
    fixture.lines().enumerate().filter_map(|(index, line)| {
        (!line.is_empty() && !line.starts_with('#')).then_some((index + 1, line))
    })
}

/// Split one row, preserving leading, internal and trailing empty cells.
pub fn tsv_fields(row: &str) -> std::str::Split<'_, char> {
    row.split('\t')
}

/// Read fixed-width TSV fixture rows, preserving empty cells and row order.
/// Empty lines and lines starting with `#` are skipped.
/// The accompanying physical line number is one-based for diagnostics.
///
/// # Panics
/// Iteration panics if a data row does not have exactly `COLUMNS` cells.
pub fn tsv_rows<const COLUMNS: usize>(
    fixture: &str,
) -> impl Iterator<Item = (usize, [&str; COLUMNS])> {
    tsv_lines(fixture).map(|(number, line)| {
        let mut cells = tsv_fields(line);
        let fields = std::array::from_fn(|_| {
            cells
                .next()
                .unwrap_or_else(|| panic!("TSV line {number} requires {COLUMNS} fields"))
        });
        assert!(
            cells.next().is_none(),
            "TSV line {number} requires {COLUMNS} fields"
        );
        (number, fields)
    })
}

/// Decode `\t`, `\n` and `\\` in a fixture field that declares escaped text.
/// Other sequences stay literal, including a trailing backslash.
#[must_use]
pub fn decode_escaped_field(field: &str) -> Cow<'_, str> {
    if !field.contains('\\') {
        return Cow::Borrowed(field);
    }
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') | None => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    Cow::Owned(out)
}

/// Read a C Tcl oracle fixture's canonical release key, such as `tcl8.6`.
#[must_use]
pub fn oracle_release(key: &str) -> Option<TclVersion> {
    TclVersion::ALL
        .into_iter()
        .find(|release| release.dialect_profile_name() == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_rows_keep_empty_cells_and_decode_annotated_fields() {
        let data = "# Fixture\nname\t\ta\\tb\\nc\\\\d\n";
        let mut rows = tsv_rows::<3>(data);
        let (line, fields) = rows.next().unwrap();
        assert_eq!(line, 2);
        assert!(rows.next().is_none());
        assert_eq!(&fields[..2], &["name", ""]);
        assert_eq!(tsv_rows::<3>("\t\t\n").next(), Some((1, ["", "", ""])));
        assert_eq!(decode_escaped_field(fields[2]), "a\tb\nc\\d");
        assert_eq!(decode_escaped_field(r"\q\"), r"\q\");
    }

    #[test]
    fn oracle_keys_use_the_canonical_release_vocabulary() {
        assert_eq!(oracle_release("tcl8.6"), Some(TclVersion::V8_6));
        for key in ["8.6", "8.6.18", "tcl8.6.18"] {
            assert_eq!(oracle_release(key), None, "{key}");
        }
    }
}
