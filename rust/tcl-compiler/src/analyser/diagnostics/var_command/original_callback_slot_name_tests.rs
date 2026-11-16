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
fn callback_slot_hints_use_shared_resolved_element_name_boundaries() {
    // naming.diagnostics.original-variable-name-anchor
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
    // Callback-shaped names are suppression hints only; these controls
    // assert neither current callback occupancy nor native variable reads.
    for name in [
        "state(-command)",
        "$state(-command)",
        "é(doneCallback)",
        "(-command)",
        "state(a)(-command)",
    ] {
        assert!(is_callback_array_slot(name), "{name}");
    }
    for name in [
        "state(-command",
        "state(doneCallback",
        "state()",
        "doneCallback",
        "state(-command)tail",
        "state(a)(data)",
    ] {
        assert!(!is_callback_array_slot(name), "{name}");
    }
}
