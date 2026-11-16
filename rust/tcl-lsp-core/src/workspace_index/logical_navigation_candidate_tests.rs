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
use tcl_compiler::analyser::Analyser;

#[test]
fn logical_workspace_navigation_matches_original_coordinates_and_retains_ambiguity() {
    let caller = "namespace eval A {proc caller {} {helper}}";
    let analysis = Analyser::new().analyse(caller, "tcl");
    let request =
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
            caller,
            &analysis,
            u32::try_from(caller.find("helper").unwrap()).unwrap(),
        )
        .unwrap();
    let a = Analyser::new().analyse("namespace eval A {proc helper {} {}}", "tcl");
    let b = Analyser::new().analyse("namespace eval B {proc helper {} {}}", "tcl");
    let root = Analyser::new().analyse("proc helper {} {}", "tcl");
    let duplicate = Analyser::new().analyse("namespace eval A {proc helper {arg} {}}", "tcl");
    let mut index = WorkspaceIndex::from_documents([("a", &a), ("b", &b), ("root", &root)]);
    let candidates = index.logical_command_candidates(&request, "caller");
    assert_eq!(candidates.len(), 1, "{candidates:?}");
    assert_eq!(candidates[0].uri, "a");
    assert!(candidates[0].matches_source("namespace eval A {proc helper {} {}}"));
    assert!(!candidates[0].matches_source("namespace eval A {proc other {} {}}"));
    assert!(index.logical_command_candidate_is_current(&candidates[0]));
    index.replace_document("a", &a);
    assert!(!index.logical_command_candidate_is_current(&candidates[0]));
    index.add_document("duplicate", &duplicate);
    assert_eq!(
        index.logical_command_candidates(&request, "caller").len(),
        2
    );
    assert_eq!(index.logical_command_candidates(&request, "a").len(), 1);
    index.remove_document("a");
    index.remove_document("duplicate");
    let candidates = index.logical_command_candidates(&request, "caller");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].uri, "root");
}

#[test]
fn logical_workspace_navigation_keeps_source_occupants_and_deleted_alternatives() {
    let local = Analyser::new().analyse("namespace eval A {proc helper {} {}}", "tcl");
    let root = Analyser::new().analyse("proc helper {} {}", "tcl");
    let occupants = Analyser::new().analyse(
        "proc puts {} {}; proc helper {} {}; proc delegated {} {}",
        "tcl",
    );
    let index = WorkspaceIndex::from_documents([
        ("local", &local),
        ("root", &root),
        ("occupants", &occupants),
    ]);
    for (source, needle) in [
        ("puts ONE", "puts ONE"),
        ("proc helper {x} {}; helper ONE", "helper ONE"),
        (
            "interp alias {} delegated {} puts; delegated ONE",
            "delegated ONE",
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let request = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
                source,
                &analysis,
                u32::try_from(source.find(needle).unwrap()).unwrap(),
            )
            .unwrap();
        assert!(
            index
                .logical_command_candidates(&request, "caller")
                .is_empty(),
            "{source}"
        );
    }
    let source =
        "namespace eval A {proc helper {x} {}; rename helper {}; proc caller {} {helper ONE}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let request =
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
            source,
            &analysis,
            u32::try_from(source.find("helper ONE").unwrap()).unwrap(),
        )
        .unwrap();
    let candidates = index.logical_command_candidates(&request, "occupants");
    assert_eq!(candidates.len(), 1, "{candidates:?}");
    assert_eq!(candidates[0].uri(), "root");
}
