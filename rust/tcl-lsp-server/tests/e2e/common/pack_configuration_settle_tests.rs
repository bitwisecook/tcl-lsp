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
fn nonempty_old_roster_cannot_settle_a_new_requested_pack() {
    let requested = json!({"specPacks": ["new/provider"]});
    let mut effective = json!({
        "spec_packs": ["new/provider"],
        "spec_packs_loaded": [{"name": "old-provider"}],
    });
    assert!(!config_reflected(&requested, &effective));
    effective["spec_pack_publication"] = json!({
        "status": "complete", "configured": ["old/provider"],
    });
    assert!(!config_reflected(&requested, &effective));
    effective["spec_pack_publication"] = json!({
        "status": "complete", "configured": ["new/provider"],
    });
    assert!(config_reflected(&requested, &effective));
}
