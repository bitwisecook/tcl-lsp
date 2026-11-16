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

use super::*;

#[test]
fn w214_namespace_grouping_keeps_literal_reporting_prefix() {
    // Implementation contract: naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Literal String grouping only; overlapping pairs are not native paths.
    for (qname, expected) in [
        ("", "::"),
        ("p", "::"),
        ("::p", "::"),
        ("n::p", "n"),
        (":::p", ":"),
        ("::a::::p", "::a::"),
        ("n::", "n"),
        ("é::λ", "é"),
        ("n\0::p", "n\0"),
    ] {
        assert_eq!(namespace_of(qname), expected);
    }
}
