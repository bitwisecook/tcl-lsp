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

#[test]
fn completed_world_survives_ui_clear_and_retains_moved_implementation() {
    let source = "proc P {} {}; rename P Q";
    let mut result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let declaration = result
        .original_procedure_declarations()
        .next()
        .unwrap()
        .clone();
    let policy = declaration.name().policy();
    let former_slot = declaration.name().slot().clone();
    result.all_procs.clear();
    result.superseded_procs.clear();
    let world = result
        .original_completed_command_world()
        .expect("independently completed Normal root");
    assert_eq!(world.source_image().bytes(), source.as_bytes());
    assert!(world.declaration_at(&former_slot, policy).is_none());
    let moved = world
        .declarations()
        .find(|publication| publication.slot().simple.as_bytes() == b"Q")
        .unwrap();
    assert_eq!(moved.declaration_site(), declaration.declaration_site());
    assert!(moved.definition().is_some());
}

#[test]
fn abrupt_or_unknown_roots_cannot_transport_completed_world() {
    for source in [
        "proc P {} {}; error BOOM",
        "proc P {} {}; eval $unknown",
        "proc P {} {}; rename $unknown Q",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result.original_completed_command_world().is_none(),
            "{source}"
        );
    }
    assert!(
        super::AnalysisResult::default()
            .original_completed_command_world()
            .is_none()
    );
}
