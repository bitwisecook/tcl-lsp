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
fn tagged_wrong_kind_decodes_without_becoming_legacy_authority() {
    let source = "proc p {} {}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let document = OriginalDeclarationDocument {
        uri: "file:///tagged-call.tcl",
        source,
        analysis: &analysis,
    };
    let identity =
        OriginalDeclarationIdentity::for_document(document.uri, source, &analysis).unwrap();
    let mut registry = OriginalDeclarationRegistry::default();
    let native = registry.issue(identity).unwrap();
    let nonce: EditorWireSession =
        serde_json::from_value(serde_json::json!(vec![255; 16])).unwrap();
    for schema in [1, 2, 4] {
        let wire = serde_json::json!({"schema":schema,"token":1,"session":nonce,"wirePurpose":"logicalSourceRelationship"});
        let inert: OriginalDeclarationHandle = serde_json::from_value(wire).unwrap();
        assert!(
            registry
                .resolve(inert, document.uri, source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve_logical_class(inert, document.uri, source, &analysis)
                .is_none()
        );
    }
    assert!(
        registry
            .resolve(native, document.uri, source, &analysis)
            .is_some()
    );
    assert_eq!(
        serde_json::to_value(native).unwrap(),
        serde_json::json!({"schema":1,"token":1})
    );
}
