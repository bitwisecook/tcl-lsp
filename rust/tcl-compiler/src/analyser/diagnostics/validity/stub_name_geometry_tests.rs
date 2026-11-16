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

use super::UserResolutionFacts;

#[test]
fn document_stub_suppression_uses_written_tail_geometry() {
    let facts = UserResolutionFacts {
        non_proc_qnames: rustc_hash::FxHashSet::default(),
        proc_offsets: rustc_hash::FxHashMap::default(),
        rename_offsets: rustc_hash::FxHashMap::default(),
        stub_names: ["known", "a:b"].into_iter().map(str::to_owned).collect(),
    };
    for name in ["known", "::ns:::known", "::ns::::known", "::ns::a:b"] {
        assert!(
            facts.resolves_to_user(name, "::other", &[], true, 0),
            "{name}"
        );
    }
    for name in ["::ns::other", "::ns::", "::ns::a::b"] {
        assert!(
            !facts.resolves_to_user(name, "::other", &[], true, 0),
            "{name}"
        );
    }
}
