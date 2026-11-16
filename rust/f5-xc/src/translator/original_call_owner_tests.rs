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
fn translated_actions_use_original_source_owners_and_decline_shadowed_heads() {
    let available = translate_irule("when HTTP_REQUEST {pool real_pool}");
    assert!(
        available
            .origin_pools
            .iter()
            .any(|pool| pool.name == "real_pool")
    );
    let shadowed = translate_irule("proc pool args {return}; when HTTP_REQUEST {pool fake_pool}");
    assert!(
        !shadowed
            .origin_pools
            .iter()
            .any(|pool| pool.name == "fake_pool")
    );
    let dynamic = translate_irule("when HTTP_REQUEST {pool $selected_pool}");
    assert_eq!(dynamic.origin_pools, [] as [crate::model::XCOriginPool; 0]);
    let quoted = translate_irule("when HTTP_REQUEST {{pool} {literal pool}}");
    assert!(
        quoted
            .origin_pools
            .iter()
            .any(|pool| pool.name == "literal pool")
    );
}
