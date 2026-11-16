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
fn original_class_inventory_keeps_equal_source_owners_and_current_occupied_headers() {
    // Implementation contract: naming.consumer.original-type-and-implementation-navigation
    // docs/design/analysis/name-resolution-proofs/original-type-and-implementation-navigation.md
    let source = "oo::class create C {}\nproc occupied {} {}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let documents = [
            OriginalDeclarationDocument {
                uri: "file:///one.tcl",
                source,
                analysis: &analysis,
            },
            OriginalDeclarationDocument {
                uri: "file:///two.tcl",
                source,
                analysis: &analysis,
            },
        ];
        let inventory = OriginalClassInventory::from_documents(&documents).unwrap();
        assert_eq!(inventory.records().len(), 2, "{dialect}");
        assert_eq!(inventory.occupied_non_classes().len(), 2);
        assert_eq!(inventory.owner(0).unwrap().uri, "file:///one.tcl");
        assert_eq!(inventory.owner(1).unwrap().uri, "file:///two.tcl");
        assert!(inventory.owner(2).is_none());
        assert!(OriginalClassInventory::from_documents(&[documents[0], documents[0]]).is_none());
        assert!(
            OriginalClassInventory::from_documents(&[OriginalDeclarationDocument {
                source: "# stale",
                ..documents[0]
            }])
            .is_none()
        );
        let mut different_config = analysis.clone();
        let mut config = analysis.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        different_config.body_lexer_config = Some(config);
        assert!(
            OriginalClassInventory::from_documents(&[OriginalDeclarationDocument {
                analysis: &different_config,
                ..documents[0]
            }])
            .is_none()
        );
    }
}

#[test]
fn original_declaration_reference_matching_keeps_opaque_slots_and_actual_lookup_owners() {
    // Implementation contract: naming.editor.original-declaration-wire-identity
    // docs/design/analysis/name-resolution-proofs/original-declaration-wire-identity.md
    let source = r"proc p\uD800 {} {}; p\uD800; proc p\uD801 {} {}; p\uD801";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let declarations = analysis
        .original_procedure_declarations()
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    let first = analysis
        .command_invocations
        .iter()
        .find(|inv| {
            inv.original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
        })
        .unwrap();
    assert!(invocation_targets_declaration(
        source,
        &analysis,
        first,
        declarations[0],
        false
    ));
    assert!(!invocation_targets_declaration(
        source,
        &analysis,
        first,
        declarations[1],
        false
    ));
    let mut missing = first.clone();
    missing.original_lookup = None;
    assert!(!invocation_targets_declaration(
        source,
        &analysis,
        &missing,
        declarations[0],
        true
    ));
    assert!(!invocation_targets_declaration(
        &format!("{source} # changed"),
        &analysis,
        first,
        declarations[0],
        true
    ));
    let mut presentation = first.clone();
    presentation.name = "p\\uD801".to_owned();
    presentation.resolved_qualified_name = Some("::unrelated".to_owned());
    assert!(invocation_targets_declaration(
        source,
        &analysis,
        &presentation,
        declarations[0],
        false
    ));
}

#[test]
fn original_declaration_handles_reselect_complete_source_uri_and_scanner() {
    // Implementation contract: naming.editor.original-declaration-wire-identity
    // docs/design/analysis/name-resolution-proofs/original-declaration-wire-identity.md
    let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; proc p\uD800 {a} {}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    let identities = declarations("file:///a.tcl", source, &analysis).unwrap();
    assert_eq!(identities.len(), 3);
    assert_ne!(identities[0], identities[2]);
    let mut registry = OriginalDeclarationRegistry::default();
    let handle = registry.issue(identities[0].clone()).unwrap();
    let wire = serde_json::to_value(handle).unwrap();
    let decoded = serde_json::from_value(wire).unwrap();
    assert_eq!(
        registry.resolve(decoded, "file:///a.tcl", source, &analysis),
        Some(identities[0].clone())
    );
    assert!(
        registry
            .resolve(handle, "file:///b.tcl", source, &analysis)
            .is_none()
    );
    assert!(
        registry
            .resolve(
                handle,
                "file:///a.tcl",
                &format!("{source}\n# changed"),
                &analysis
            )
            .is_none()
    );
    let changed = Analyser::new().analyse(source, "tcl9.0");
    assert!(
        registry
            .resolve(handle, "file:///a.tcl", source, &changed)
            .is_none()
    );
    let forged = serde_json::from_value(serde_json::json!({"schema":1,"token":999_999})).unwrap();
    assert!(
        registry
            .resolve(forged, "file:///a.tcl", source, &analysis)
            .is_none()
    );
}
