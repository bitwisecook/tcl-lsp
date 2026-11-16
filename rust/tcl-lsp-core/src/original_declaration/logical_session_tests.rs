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
fn missing_random_allocation_withholds_logical_call_without_changing_legacy_wire() {
    let source = "proc helper {} {}\nhelper\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    let document = OriginalDeclarationDocument {
        uri: "file:///session-failure.tcl",
        source,
        analysis: &analysis,
    };
    let advice =
        crate::call_hierarchy::prepare_logical_from_inventory(document.uri, 0, 5, &[document])
            .unwrap();
    let identity = advice.possible_source_items()[0].identity().clone();
    let publication = LogicalCallHostPublication::retain(());
    let mut registry = OriginalDeclarationRegistry {
        session: None,
        ..OriginalDeclarationRegistry::default()
    };
    assert!(
        registry
            .issue_logical_call(identity.clone(), publication.clone())
            .is_none()
    );
    assert!(registry.rows.is_empty());
    let legacy = registry
        .issue(OriginalDeclarationIdentity::for_document(document.uri, source, &analysis).unwrap())
        .unwrap();
    assert_eq!(
        serde_json::to_value(legacy).unwrap(),
        serde_json::json!({"schema":1,"token":1})
    );
    assert!(
        registry
            .resolve(legacy, document.uri, source, &analysis)
            .is_some()
    );
    // A genuine retained CALL from an entropy-available issuer cannot be
    // reborrowed once the current allocation owner is unavailable.
    let mut issuer = OriginalDeclarationRegistry::default();
    let handle = issuer
        .issue_logical_call(identity, publication.clone())
        .unwrap();
    assert!(
        issuer
            .resolve_logical_call(handle, document.uri, &[document], &publication)
            .is_some()
    );
    issuer.session = None;
    assert!(
        issuer
            .resolve_logical_call(handle, document.uri, &[document], &publication)
            .is_none()
    );
}

#[test]
fn session_wire_bytes_are_lossless_and_legacy_shapes_stay_strict() {
    let nonce: EditorWireSession =
        serde_json::from_value(serde_json::json!(vec![255; 16])).unwrap();
    for schema in [3, 4] {
        let handle = OriginalDeclarationHandle {
            schema,
            token: 1,
            session: Some(nonce),
        };
        let value = serde_json::to_value(handle).unwrap();
        assert_eq!(value["session"], serde_json::json!(vec![255; 16]));
        assert_eq!(
            serde_json::from_value::<OriginalDeclarationHandle>(value).unwrap(),
            handle
        );
    }
    for schema in [1, 2] {
        let legacy = serde_json::json!({"schema":schema,"token":1});
        let handle: OriginalDeclarationHandle = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(serde_json::to_value(handle).unwrap(), legacy);
        for session in [serde_json::json!(nonce), serde_json::Value::Null] {
            assert!(
                serde_json::from_value::<OriginalDeclarationHandle>(
                    serde_json::json!({"schema":schema,"token":1,"session":session})
                )
                .is_err()
            );
        }
    }
    for session in [
        serde_json::json!([0]),
        serde_json::json!(vec![256; 16]),
        serde_json::Value::Null,
    ] {
        assert!(
            serde_json::from_value::<OriginalDeclarationHandle>(
                serde_json::json!({"schema":3,"token":1,"session":session})
            )
            .is_err()
        );
    }
}
