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

use super::dynamic_variable_word_can_spell as can_spell;
use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};

/// The wildcard extents come from the shared owner, so the
/// *literal* characters that bound what a dynamic word can spell move with
/// the release.
///
/// Under the default (9.x) rule `${a{b}c}` is one wildcard and can spell
/// any cell; under 8.x the name ends at the first `}` and the literal `c}`
/// remains, which no ordinary cell name ends with. A narrower pattern is
/// the unsafe direction: it judges a cell the word really can reach as out
/// of reach, and rename proceeds.
#[test]
fn wildcard_extent_follows_the_release_close_rule() {
    assert!(can_spell("${a{b}c}", "::v", Tcl9Nesting));
    assert!(!can_spell("${a{b}c}", "::v", FirstClose));
    // With the 8.x remainder actually present in the cell name, the
    // narrower pattern does match — the literal run is the whole bound.
    assert!(can_spell("${a{b}c}", "::xc}", FirstClose));
}
