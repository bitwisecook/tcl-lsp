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

//! Variable read and write projections of a call.

use super::*;

#[test]
fn variable_effect_footprints_keep_actual_availability_over_argument_dialect() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // This is authored metadata, not a Native read/store or command result.
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "gated-write",
        surface: registry.get("dict").unwrap().surface,
        ..registry.get("set").unwrap().clone()
    });
    let store = std::sync::Arc::new(registry);
    let arguments = [
        crate::InvocationWord::Literal("$scalar(open"),
        crate::InvocationWord::Literal("VALUE"),
    ];
    let words = crate::InvocationWords::structured(
        crate::InvocationWord::Literal("gated-write"),
        &arguments,
    )
    .with_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
        let context = crate::model::ingress::static_context_for(environment)
            .with_command_store(std::sync::Arc::clone(&store));
        let projection = store.variable_write_projection_in_resolved_context(
            context.context(),
            words,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert_eq!(
            projection
                .literal_names
                .contains(&"$scalar(open".to_owned()),
            available,
            "{environment}"
        );
        assert_eq!(
            projection.opaque_variable_frame, !available,
            "{environment}"
        );
    }
}

#[test]
fn variable_write_projection_keeps_repeated_literal_targets() {
    let reg = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Literal("{(a)(b)}"),
        crate::InvocationWord::Literal("ab"),
        crate::InvocationWord::Literal("whole"),
        crate::InvocationWord::Literal("first"),
        crate::InvocationWord::Literal("second"),
    ];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("regexp"),
        &arguments,
    ));

    assert_eq!(projection.literal_names, ["whole", "first", "second"]);
    assert!(!projection.opaque_variable_frame);
}

#[test]
fn variable_write_projection_roots_registry_global_option_targets() {
    let reg = crate::model::semantic::SemanticContext::for_environment("tk").commands();
    let arguments = [
        crate::InvocationWord::Literal(".e"),
        crate::InvocationWord::Literal("-textvariable"),
        crate::InvocationWord::Literal("value"),
    ];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("entry"),
        &arguments,
    ));

    assert_eq!(projection.literal_names, ["::value"]);
    assert!(!projection.opaque_variable_frame);
    assert!(
        CommandRegistry::build_default()
            .variable_write_projection(InvocationWords::structured(
                crate::InvocationWord::Literal("entry"),
                &arguments,
            ))
            .literal_names
            .is_empty(),
        "an unprovided catalogue widget cannot establish its option targets"
    );
}

#[test]
fn variable_write_projection_respects_registry_write_arity_floor() {
    let reg = CommandRegistry::build_default();
    for (head, arguments) in [
        (
            "::regexp",
            vec![
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Dynamic,
            ],
        ),
        (
            "::regsub",
            vec![
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Dynamic,
            ],
        ),
    ] {
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal(head),
            &arguments,
        ));
        assert_eq!(projection, VariableWriteProjection::default(), "{head}");
    }

    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("::regexp"),
        &arguments,
    ));
    assert_eq!(projection.literal_names.len(), 0);
    assert!(projection.opaque_variable_frame);
}

#[test]
fn variable_write_projection_marks_dynamic_targets_and_argv_opaque() {
    let reg = CommandRegistry::build_default();
    for arguments in [
        vec![
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("value"),
        ],
        vec![crate::InvocationWord::Expanded],
    ] {
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("set"),
            &arguments,
        ));
        assert_eq!(projection.literal_names.len(), 0);
        assert!(projection.opaque_variable_frame);
    }
}

#[test]
fn variable_write_projection_marks_computed_heads_opaque() {
    let reg = CommandRegistry::build_default();
    let no_arguments = [];
    for head in [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Expanded,
        crate::InvocationWord::Opaque,
    ] {
        let projection =
            reg.variable_write_projection(InvocationWords::structured(head, &no_arguments));
        assert_eq!(projection.literal_names.len(), 0);
        assert!(projection.opaque_variable_frame);
    }
}

#[test]
fn variable_write_projection_distinguishes_aliases_from_value_writes() {
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        for (head, arguments) in [
            ("global", vec![crate::InvocationWord::Literal("item")]),
            (
                "upvar",
                vec![
                    crate::InvocationWord::Literal("1"),
                    crate::InvocationWord::Literal("outer"),
                    crate::InvocationWord::Literal("local"),
                ],
            ),
            (
                "trace",
                vec![
                    crate::InvocationWord::Literal("add"),
                    crate::InvocationWord::Literal("variable"),
                    crate::InvocationWord::Literal("item"),
                    crate::InvocationWord::Literal("write"),
                    crate::InvocationWord::Literal("callback"),
                ],
            ),
        ] {
            let projection = reg.variable_write_projection(
                InvocationWords::structured(crate::InvocationWord::Literal(head), &arguments)
                    .with_dialect(dialect),
            );
            assert_eq!(projection, VariableWriteProjection::default(), "{head}");
        }

        let arguments = [
            crate::InvocationWord::Literal("first"),
            crate::InvocationWord::Literal("value"),
            crate::InvocationWord::Literal("declared_only"),
        ];
        let projection = reg.variable_write_projection(
            InvocationWords::structured(crate::InvocationWord::Literal("variable"), &arguments)
                .with_dialect(dialect),
        );
        assert_eq!(projection.literal_names, ["first"]);
        assert!(!projection.opaque_variable_frame);
    }
    let unknown = [crate::InvocationWord::Literal("item")];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("global"),
        &unknown,
    ));
    assert_eq!(projection.literal_names.len(), 0);
    assert!(
        projection.opaque_variable_frame,
        "missing alias naming policy remains unknown"
    );
}

#[test]
fn variable_write_projection_does_not_infer_roles_from_an_unrelated_resolver() {
    let reg = CommandRegistry::build_default();
    let arguments = [crate::InvocationWord::Dynamic];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("puts"),
        &arguments,
    ));
    assert_eq!(projection, VariableWriteProjection::default());
}

/// An unbind reads its place's existence: the
/// read projection names a destroyer's targets, past `-nocomplain` and
/// `--`, where the write projection names none, and a substituted target
/// widens the frame.
#[test]
fn variable_read_projection_names_a_destroyers_targets() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Dynamic, Literal};
    let reg = CommandRegistry::build_default();
    for (arguments, expected) in [
        (vec![Literal("x")], vec!["x"]),
        (
            vec![
                Literal("-nocomplain"),
                Literal("--"),
                Literal("a"),
                Literal("b"),
            ],
            vec!["a", "b"],
        ),
    ] {
        let words = InvocationWords::structured(Literal("unset"), &arguments);
        assert_eq!(reg.variable_read_projection(words).literal_names, expected);
        assert_eq!(
            reg.variable_write_projection(words),
            VariableWriteProjection::default()
        );
    }
    let computed = [Dynamic];
    let projection =
        reg.variable_read_projection(InvocationWords::structured(Literal("unset"), &computed));
    assert_eq!(projection.literal_names.len(), 0);
    assert!(projection.opaque_variable_frame);
    let query = [Literal("exists"), Literal("x")];
    assert_eq!(
        reg.variable_read_projection(InvocationWords::structured(Literal("info"), &query))
            .literal_names,
        vec!["x"]
    );
}
