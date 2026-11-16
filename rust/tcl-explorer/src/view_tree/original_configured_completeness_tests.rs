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
fn original_structural_view_keeps_unknown_completeness_distinct_from_false() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Display of the actual source query; no parser or execution authority.
    let mut result = crate::run_pipeline("set x 1", "tcl8.6");
    let view = build_structural_index(&crate::serialise::serialise_result(&result));
    assert_eq!(view[0].label, "script complete: true");
    assert_eq!(view[0].style.as_deref(), Some("green"));
    let incomplete = crate::run_pipeline("set x {", "tcl8.6");
    let view = build_structural_index(&crate::serialise::serialise_result(&incomplete));
    assert_eq!(view[0].label, "script complete: false");
    assert_eq!(view[0].style.as_deref(), Some("yellow"));
    result.source.push_str("foreign");
    let view = build_structural_index(&crate::serialise::serialise_result(&result));
    assert_eq!(view[0].label, "script completeness unavailable");
    assert_eq!(view[0].style.as_deref(), Some("dim"));
}
