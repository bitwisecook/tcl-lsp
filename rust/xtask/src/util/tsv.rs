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

/// Flat, unquoted TSV rows used by the xtask evidence ledgers.
pub(crate) fn rows(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

pub(crate) fn fields(row: &str) -> impl Iterator<Item = &str> {
    row.split('\t')
}

pub(crate) fn record<'a>(fields: impl IntoIterator<Item = &'a str>) -> String {
    fields.into_iter().collect::<Vec<_>>().join("\t")
}
