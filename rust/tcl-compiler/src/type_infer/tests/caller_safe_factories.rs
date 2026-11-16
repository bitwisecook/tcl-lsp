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

//! The caller-safe factory proof over original class declarations.

use super::*;

fn assert_unsupported_caller_factories(
    version: tcl_dialect::TclVersion,
    metadata: TypePropagationMetadata<'_>,
) {
    for (source, args) in [
        (
            "oo::class create C {}; C create object",
            vec!["create", "object"],
        ),
        ("oo::class create C {}; C new value", vec!["new", "value"]),
        (
            "oo::class create C {constructor {} {return}}; C new",
            vec!["new"],
        ),
        (
            "oo::class create C {self method new args {return}}; C new",
            vec!["new"],
        ),
        (
            "oo::class create C {method observe args {}; filter observe}; C new",
            vec!["new"],
        ),
        (
            "oo::class create C {}; rename C retired; proc C args {}; C new",
            vec!["new"],
        ),
    ] {
        let tokens = original_factory_tokens(source, version, "C ", "C");
        assert!(
            !caller_safe_factory_invocation(metadata, &tokens, "C", &args),
            "{version:?}: {source}"
        );
    }
}

#[test]
fn original_caller_safe_factory_keeps_selected_layout_and_withdraws_changed_owners() {
    // naming.tcloo.original-default-manufacture-source-shape
    // docs/design/analysis/name-resolution-proofs/tcloo-original-default-manufacture-source-shape.md
    // This checks source layout and the TypeInfer precision gate only. The
    // explicit standalone source carrier supplies no actual Native frame,
    // successful allocation, Normal result or execution equivalence.
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let metadata = standalone_factory_metadata(version);
        let source = "oo::class create C {}; C new";
        let tokens = original_factory_tokens(source, version, "C new", "C");
        assert!(caller_safe_factory_invocation(
            metadata,
            &tokens,
            "C",
            &["new"]
        ));
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(!caller_safe_factory_invocation(
            metadata,
            &missing,
            "C",
            &["new"]
        ));
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "create".to_owned();
        assert!(!caller_safe_factory_invocation(
            metadata,
            &changed,
            "C",
            &["new"]
        ));
        let mut changed_config = metadata.config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                config: changed_config,
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                context: None,
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        // Actual availability without complete source input does not become
        // the explicitly tagged standalone carrier used above.
        let availability = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                context: Some(availability.into()),
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        let mut foreign = CommandRegistry::build_default();
        let mut changed_spec = foreign.get("set").unwrap().clone();
        changed_spec.return_type = Some(TclType::List);
        foreign.insert(changed_spec);
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                registry: &foreign,
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        assert_unsupported_caller_factories(version, metadata);
    }
}

#[test]
fn original_caller_safe_factory_does_not_promote_abstract_exports_or_custom_metaclasses() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    // The C9 public-call proof permits explicit export of Abstract new.
    // That does not issue the ordinary Native default recipe required by
    // this separate precision gate, or establish a completed allocation.
    for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
        let metadata = standalone_factory_metadata(version);
        for source in [
            "oo::abstract create C {}; C new",
            "oo::abstract create C {}; oo::objdefine C export new; C new",
            "oo::class create Meta {superclass oo::class}; Meta create C {}; C new",
        ] {
            let tokens = original_factory_tokens(source, version, "C new", "C");
            assert!(
                !caller_safe_factory_invocation(metadata, &tokens, "C", &["new"]),
                "this source lacks the ordinary default manufacture owner: {source}"
            );
        }
        let source = "oo::class create C {}; C new";
        let tokens = original_factory_tokens(source, version, "oo::class create", "oo::class");
        assert_eq!(
            empty_source_class(metadata, &tokens),
            Some("::C".to_owned())
        );
        let source = "oo::abstract create C {}; C new";
        let tokens =
            original_factory_tokens(source, version, "oo::abstract create", "oo::abstract");
        assert_eq!(
            empty_source_class(metadata, &tokens),
            Some("::C".to_owned()),
            "factory own public create is separate from the manufactured class defaults"
        );
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(empty_source_class(metadata, &missing).is_none());
    }
}

#[test]
fn original_nested_factory_precision_requires_actual_child_tokens() {
    // naming.tcloo.original-default-manufacture-source-shape
    // docs/design/analysis/name-resolution-proofs/tcloo-original-default-manufacture-source-shape.md
    let version = tcl_dialect::TclVersion::V9_1;
    let metadata = standalone_factory_metadata(version);
    let source = "oo::class create C {}; set made [C new]";
    let parent = original_factory_tokens(source, version, "set made", "set");
    let calls = crate::word_subst::checked_lifted_calls(&parent, metadata.config)
        .expect("complete original child inventory");
    assert_eq!(calls.len(), 1);
    let child = calls[0].tokens.as_ref().expect("actual child tokens");
    assert!(caller_safe_factory_invocation(
        metadata,
        child,
        "C",
        &["new"]
    ));
    let statement = Statement::AssignValue {
        span: parent.words()[0].source().span,
        name: "made".to_owned(),
        name_braced: false,
        value: "[C new]".to_owned(),
        value_needs_backsubst: false,
        tokens: Some(parent.clone()),
    };
    assert!(substitutions_have_only_caller_safe_factories(
        &statement, metadata
    ));
    let mut missing = parent;
    missing.source_binding = None;
    let mut statement = statement;
    if let Statement::AssignValue { tokens, .. } = &mut statement {
        *tokens = Some(missing);
    }
    assert!(!substitutions_have_only_caller_safe_factories(
        &statement, metadata
    ));
}
