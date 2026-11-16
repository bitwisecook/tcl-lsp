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
fn original_loader_notice_geometry_refuses_helper_and_changed_evaluated_parent_bodies() {
    // Genuine evaluated captures disagree with an existing same-line source
    // declaration. The established reader may keep its verbatim text, but
    // neither that preference nor a later row match may restore ownership.
    for source in [
        "speclib demo 1 {proc emit {} {command demo {arty 9}}; emit; command demo {arty 1}}",
        "speclib demo 1 {set body {arty 9}; command demo $body; command demo {arty 1}}",
    ] {
        let pack = evaluate_pack(source);
        let notices: Vec<_> = pack
            .notices
            .iter()
            .filter(|notice| notice.message.contains("unknown property `arty`"))
            .collect();
        assert!(
            !notices.is_empty(),
            "the real loader must still report: {:?}",
            pack.notices
        );
        assert!(
            notices.iter().all(|notice| notice.subject.is_none()),
            "a changed/helper parent cannot donate original source geometry: {:?}",
            pack.notices
        );
    }
}
