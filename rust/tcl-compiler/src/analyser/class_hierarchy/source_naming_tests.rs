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
use crate::signature_scan::scope::{SignatureSourceCommand, SignatureSourceLookup};
use tcl_names::namespace::SignatureNamespaceScope;
use tcl_syntax::naming::NamePolicyProtocol;

fn source_class(
    policy: NamePolicyProtocol,
    scope: &SignatureNamespaceScope,
    written: &str,
) -> ClassDef {
    ClassDef {
        source_name: SignatureSourceCommand::object_in_context(policy, scope, written),
        ..ClassDef::default()
    }
}

#[test]
fn class_lookup_requires_original_context_and_does_not_borrow_unique_tail() {
    let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
    let root = SignatureNamespaceScope::root(Some(policy));
    let left = root.child("left", Some(policy)).unwrap();
    let right = root.child("right", Some(policy)).unwrap();
    let classes = HashMap::from([(
        "::left::Base".to_owned(),
        source_class(policy, &left, "Base"),
    )]);
    assert_eq!(
        resolve_written_class_name_in_context("Base", &left, policy, &classes).as_deref(),
        Some("::left::Base")
    );
    assert_eq!(
        resolve_written_class_name_in_context("Base", &right, policy, &classes),
        None
    );
    assert_eq!(resolve_written_class_name("Base", &classes), None);
    let caller = SignatureSourceLookup::new(policy, left, "Base".to_owned()).unwrap();
    assert_eq!(
        resolve_class_lookup(&caller, &classes).as_deref(),
        Some("::left::Base")
    );
    assert!(
        SignatureSourceLookup::new(
            NamePolicyProtocol::authored_jim084(),
            right,
            "Base".to_owned()
        )
        .is_none()
    );
}

#[test]
fn class_report_collisions_withdraw_instead_of_selecting_another_slot() {
    let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
    let root = SignatureNamespaceScope::root(Some(policy));
    let left = root
        .child("a:", Some(policy))
        .unwrap()
        .child("b", Some(policy))
        .unwrap();
    let right = root
        .child("a", Some(policy))
        .unwrap()
        .child(":b", Some(policy))
        .unwrap();
    assert_eq!(left.display(), right.display());
    let mut result = super::super::types::AnalysisResult::default();
    result.retain_class_declaration(
        "::a:::b::Base".to_owned(),
        source_class(policy, &left, "Base"),
    );
    result.retain_class_declaration(
        "::a:::b::Base".to_owned(),
        source_class(policy, &right, "Base"),
    );
    assert!(
        result.all_classes["::a:::b::Base"]
            .source_name_ambiguous
            .is_observed()
    );
    assert_eq!(
        resolve_written_class_name_in_context("Base", &right, policy, &result.all_classes),
        None
    );
}
