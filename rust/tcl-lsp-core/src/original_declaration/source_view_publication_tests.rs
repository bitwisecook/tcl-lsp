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

// A storage-layer control, not a genuine server publication lease. The
// protocol/host tests must retire actual physical-parent publications.
#[derive(Debug, PartialEq, Eq)]
struct TestView {
    parent: &'static str,
    epoch: u64,
}

#[test]
fn equal_source_identity_cannot_renew_another_saved_view_stamp() {
    let uri = "file:///shared.tcl";
    let source = "proc run {} {}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let identity = OriginalDeclarationIdentity::for_document(uri, source, &analysis).unwrap();
    let a = OriginalSourceViewPublication::retain(TestView {
        parent: "file:///A/root.tcl",
        epoch: 1,
    });
    let b = OriginalSourceViewPublication::retain(TestView {
        parent: "file:///B/root.tcl",
        epoch: 1,
    });
    let restored_a = OriginalSourceViewPublication::retain(TestView {
        parent: "file:///A/root.tcl",
        epoch: 3,
    });
    let mut registry = OriginalDeclarationRegistry::default();
    let old = registry
        .issue_in_source_view(identity.clone(), a.clone())
        .unwrap();
    let current = registry
        .issue_in_source_view(identity.clone(), b.clone())
        .unwrap();
    let restored = registry
        .issue_in_source_view(identity.clone(), restored_a.clone())
        .unwrap();
    assert_ne!(old, current);
    assert_ne!(old, restored);
    assert_eq!(
        registry.source_view_publication::<TestView>(old),
        Some(&TestView {
            parent: "file:///A/root.tcl",
            epoch: 1
        })
    );
    assert!(registry.source_view_publication::<String>(old).is_none());
    assert!(
        registry
            .resolve_in_source_view(old, uri, source, &analysis, &b)
            .is_none()
    );
    assert!(
        registry
            .resolve_in_source_view(old, uri, source, &analysis, &restored_a)
            .is_none()
    );
    assert_eq!(
        registry.resolve_in_source_view(current, uri, source, &analysis, &b),
        Some(identity.clone())
    );
    assert!(registry.resolve(old, uri, source, &analysis).is_none());
    let legacy = registry.issue(identity).unwrap();
    assert!(
        registry
            .source_view_publication::<TestView>(legacy)
            .is_none()
    );
    assert!(
        registry
            .resolve_in_source_view(legacy, uri, source, &analysis, &a)
            .is_none()
    );
    let wire = serde_json::to_value(old).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), 4);
    assert_eq!(wire["schema"], 5);
    assert_eq!(wire["wirePurpose"], "originalSourceViewRelationship");
    assert_eq!(
        serde_json::from_value::<OriginalDeclarationHandle>(wire.clone()).unwrap(),
        old
    );
    assert_eq!(
        wire["session"],
        serde_json::to_value(old.session.unwrap()).unwrap()
    );
    for schema in [1, 2, 3, 4] {
        let wrong = OriginalDeclarationHandle { schema, ..old };
        assert!(
            registry
                .source_view_publication::<TestView>(wrong)
                .is_none()
        );
        assert!(
            registry
                .resolve_in_source_view(wrong, uri, source, &analysis, &a)
                .is_none()
        );
    }
}

#[test]
fn source_view_wire_rejects_restarted_registry_and_missing_session() {
    // Real wire-session allocation protects counter collisions only; it
    // grants no source, parent-publication or declaration authority.
    let uri = "file:///shared.tcl";
    let source = "proc run {} {}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let identity = OriginalDeclarationIdentity::for_document(uri, source, &analysis).unwrap();
    let stamp = OriginalSourceViewPublication::retain(TestView {
        parent: "file:///A/root.tcl",
        epoch: 1,
    });
    let mut first = OriginalDeclarationRegistry::default();
    let old = first
        .issue_in_source_view(identity.clone(), stamp.clone())
        .unwrap();
    let mut restarted = OriginalDeclarationRegistry::default();
    let new = restarted
        .issue_in_source_view(identity.clone(), stamp.clone())
        .unwrap();
    assert_eq!(
        old.token, new.token,
        "exercise an actual reset-counter collision"
    );
    assert_ne!(
        old.session, new.session,
        "independent genuine wire allocations"
    );
    assert!(restarted.source_view_publication::<TestView>(old).is_none());
    assert!(
        restarted
            .resolve_in_source_view(old, uri, source, &analysis, &stamp)
            .is_none()
    );
    assert!(
        restarted
            .resolve_in_source_view(new, uri, source, &analysis, &stamp)
            .is_some()
    );
    let missing = OriginalDeclarationHandle {
        session: None,
        ..old
    };
    assert!(first.source_view_publication::<TestView>(missing).is_none());
    assert!(
        first
            .resolve_in_source_view(missing, uri, source, &analysis, &stamp)
            .is_none()
    );
    let mut unavailable = OriginalDeclarationRegistry {
        session: None,
        next: 0,
        rows: BTreeMap::new(),
        order: VecDeque::new(),
    };
    assert!(
        unavailable
            .issue_in_source_view(identity.clone(), stamp)
            .is_none()
    );
    assert!(
        unavailable.issue(identity).is_some(),
        "legacy Native issuance stays independent"
    );
}

#[test]
fn source_view_transport_keeps_its_exact_session_relationship_purpose() {
    let session = EditorWireSession::allocate().unwrap();
    let locator = OriginalDeclarationHandle {
        schema: 5,
        token: 1,
        session: Some(session),
    };
    let wire = serde_json::to_value(locator).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), 4);
    assert_eq!(
        serde_json::from_value::<OriginalDeclarationHandle>(wire.clone()).unwrap(),
        locator
    );
    let mut changed = wire.clone();
    changed["wirePurpose"] = serde_json::json!("logicalSourceRelationship");
    assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
    let mut changed = wire.clone();
    changed.as_object_mut().unwrap().remove("session");
    assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
    let mut changed = wire.clone();
    changed.as_object_mut().unwrap().remove("wirePurpose");
    assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
    for schema in [1, 2, 3, 4] {
        let mut changed = wire.clone();
        changed["schema"] = serde_json::json!(schema);
        assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
    }
}
