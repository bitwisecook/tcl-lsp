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

//! Tests of the variable-receiver operand form: fact projection, selected forms and original trace receivers.

use super::*;

#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn variable_receiver_fact_projection_preserves_form_precedence_and_member_offsets() {
    use VariableReceiverOperandForm::Combined;
    const FORMS: &[CommandForm] = &[
        CommandForm {
            name: "inherited",
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::VarWrite)],
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "withdrawn",
            arity: Arity::exact(2),
            arg_roles: &[(0, ArgRole::VarWrite)],
            variable_receivers: Some(&[]),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "selected",
            arity: Arity::exact(3),
            arg_roles: &[(2, ArgRole::VarWrite)],
            variable_receivers: Some(&[(2, Combined)]),
            ..CommandForm::DEFAULT
        },
    ];
    const MEMBERS: &[SubCommand] = &[SubCommand {
        name: "nested",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::VarRead)],
        variable_receivers: Some(&[(0, Combined)]),
        ..SubCommand::DEFAULT
    }];
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "receiver-forms",
        variable_receivers: Some(&[(0, Combined)]),
        command_forms: FORMS,
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "receiver-member",
        subcommands: MEMBERS,
        ..CommandSpec::DEFAULT
    });
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (args, position, expected) in [
        (vec!["x"], 0, Some(Combined)),
        (vec!["x", "V"], 0, None),
        (vec!["a", "b", "x"], 2, Some(Combined)),
    ] {
        let selected = registry
            .resolve_invocation("receiver-forms", &args, dialect.authoring_query())
            .unwrap();
        let facts = selected.facts();
        assert_eq!(facts.variable_receiver_operand_form(position), expected);
        assert!(facts.successful_handler.is_none());
        assert!(facts.native_compilation.is_none());
    }
    let selected = registry
        .resolve_invocation(
            "receiver-member",
            &["nested", "x"],
            dialect.authoring_query(),
        )
        .unwrap();
    let facts = selected.facts();
    assert_eq!(facts.argument_offset, 1);
    assert_eq!(facts.variable_receiver_operand_form(1), Some(Combined));
    assert!(facts.variable_receiver_operand_form(0).is_none());
}

#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn selected_variable_receiver_form_keeps_array_argv_combined_and_aliases_separate() {
    // Native proof: naming.array-source.combined-set-receiver
    // docs/design/analysis/name-resolution-proofs/array-source-combined-set-receiver.md
    // Native proof: naming.array-source.scalar-element-storage
    // docs/design/analysis/name-resolution-proofs/array-source-scalar-element-storage.md
    // Native proof: naming.array-source.combined-read-enumeration
    // docs/design/analysis/name-resolution-proofs/array-source-combined-read-enumeration.md
    // Native proof: naming.array-source.combined-unset
    // docs/design/analysis/name-resolution-proofs/array-source-combined-unset.md
    let registry = crate::CommandRegistry::build_default();
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let dialect = tcl_native_model::InvocationDialect::of_profile(
            crate::model::ingress::resolve_known_environment(profile)
                .unwrap()
                .unit_profile(),
        );
        let query = dialect.authoring_query();
        for (head, arguments, ordinal) in [
            ("set", vec!["a(b)", "VALUE"], 0),
            ("array", vec!["set", "a(b)", "k V"], 1),
            ("array", vec!["get", "a(b)"], 1),
            ("array", vec!["names", "a(b)"], 1),
            ("array", vec!["unset", "a(b)"], 1),
        ] {
            let selected = registry
                .resolve_invocation(head, &arguments, query)
                .unwrap();
            assert_eq!(
                selected.facts().variable_receiver_operand_form(ordinal),
                Some(VariableReceiverOperandForm::Combined),
                "{head} {arguments:?}"
            );
            assert!(
                selected
                    .facts()
                    .variable_receiver_operand_form(ordinal + 1)
                    .is_none()
            );
        }
    }
    let query = Some(SurfaceQuery::core(tcl_dialect::model::Family::Tcl, "8.6"));
    let global = registry
        .resolve_invocation("global", &["x"], query)
        .unwrap();
    assert!(global.facts().variable_receiver_operand_form(0).is_none());
    let array_get = registry
        .resolve_invocation("array", &["get", "a(b)"], query)
        .unwrap();
    assert!(
        array_get.facts().successful_handler.is_none(),
        "receiver metadata cannot donate a normal handler"
    );
    let mut explicit_withdrawal = array_get.facts();
    explicit_withdrawal.variable_receivers = Some(&[]);
    assert!(
        explicit_withdrawal
            .variable_receiver_operand_form(1)
            .is_none()
    );
    let wrong_arity = registry
        .resolve_invocation("array", &["get"], query)
        .unwrap();
    assert!(
        wrong_arity
            .facts()
            .variable_receiver_operand_form(1)
            .is_none()
    );
    let mut only_role = registry
        .resolve_invocation("set", &["x", "V"], query)
        .unwrap()
        .facts();
    only_role.native_compilation = None;
    only_role.successful_handler = None;
    assert!(
        only_role.variable_receiver_operand_form(0).is_none(),
        "a role alone cannot select native receiver semantics"
    );
}

#[test]
fn authored_receiver_form_does_not_require_native_procedure_acceptance() {
    // Implementation contract: naming.vendor.original-registry-metadata
    // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal, Opaque};
    use VariableReceiverOperandForm::Combined;
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "authored-definer",
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::VarWrite),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        variable_receivers: Some(&[(0, Combined)]),
        procedure_definition: Some(crate::native_procedure::NativeProcedureDefinitionSpec::Core),
        ..CommandSpec::DEFAULT
    });
    for (arguments, expected) in [
        (vec![Dynamic, Dynamic, Literal("")], Some(Combined)),
        (vec![Dynamic, Dynamic], None),
        (vec![Dynamic, Expanded, Literal("")], None),
        (vec![Opaque, Dynamic, Literal("")], None),
    ] {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("authored-definer"), &arguments),
                None,
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_variable_receiver_operand_form(0),
            expected
        );
        assert!(
            selected
                .authored_source_variable_receiver_operand_form(1)
                .is_none()
        );
        assert!(!selected.facts().arg_roles_complete);
        assert!(selected.facts().variable_receiver_operand_form(0).is_none());
        assert!(selected.facts().successful_handler.is_none());
        assert!(selected.facts().native_compilation.is_none());
    }
    registry.insert(CommandSpec {
        name: "role-only",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    });
    let selected = registry
        .resolve_invocation("role-only", &["x"], None)
        .unwrap();
    assert!(selected.authored_source_argument_roles().1);
    assert!(
        selected
            .authored_source_variable_receiver_operand_form(0)
            .is_none()
    );
}

#[cfg(test)]
fn assert_original_trace_computed_receivers(
    registry: &crate::CommandRegistry,
    dialect: tcl_native_model::InvocationDialect,
) {
    use VariableReceiverOperandForm::TraceSubject;
    for (subject, expected) in [
        (crate::InvocationWord::Dynamic, Some(TraceSubject)),
        (
            crate::InvocationWord::KnownBytes(b"v\xff"),
            Some(TraceSubject),
        ),
        (crate::InvocationWord::Expanded, None),
    ] {
        let words = [
            crate::InvocationWord::Literal("info"),
            crate::InvocationWord::Literal("variable"),
            subject,
        ];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(crate::InvocationWord::Literal("trace"), &words),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(
            invocation.authored_source_variable_receiver_operand_form(2),
            expected
        );
    }
    let dynamic = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("trace"),
                &[
                    crate::InvocationWord::Literal("info"),
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Literal("::v"),
                ],
            ),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert!(
        dynamic
            .authored_source_variable_receiver_operand_form(2)
            .is_none()
    );
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn original_trace_receiver_forms_select_variable_grammar_ordinals_and_release() {
    use VariableReceiverOperandForm::TraceSubject;
    let registry = crate::CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        for arguments in [
            vec!["add", "variable", "::v(k)", "read", "callback"],
            vec!["remove", "var", "::v(k)", "read", "callback"],
            vec!["info", "variable", "::v(k)"],
        ] {
            let invocation = registry
                .resolve_invocation("trace", &arguments, dialect.authoring_query())
                .unwrap();
            assert_eq!(
                invocation.authored_source_variable_receiver_operand_form(2),
                Some(TraceSubject),
                "{version:?}: {arguments:?}"
            );
            assert_eq!(
                invocation.facts().variable_receiver_operand_form(2),
                Some(TraceSubject)
            );
            assert!(
                invocation
                    .authored_source_variable_receiver_operand_form(1)
                    .is_none()
            );
        }
        for arguments in [
            vec!["add", "command", "set", "rename", "callback"],
            vec!["remove", "execution", "set", "enter", "callback"],
            vec!["info", "command", "set"],
            vec!["info", "execution", "set"],
            vec!["info", "var"],
        ] {
            let invocation = registry
                .resolve_invocation("trace", &arguments, dialect.authoring_query())
                .unwrap();
            assert!(
                invocation
                    .authored_source_variable_receiver_operand_form(2)
                    .is_none(),
                "{version:?}: {arguments:?}"
            );
        }
        if version < tcl_dialect::TclVersion::V9_0 {
            for arguments in [
                vec!["variable", "::v(k)", "r", "callback"],
                vec!["vdelete", "::v(k)", "r", "callback"],
                vec!["vinfo", "::v(k)"],
            ] {
                let invocation = registry
                    .resolve_invocation("trace", &arguments, dialect.authoring_query())
                    .unwrap();
                assert_eq!(
                    invocation.authored_source_variable_receiver_operand_form(1),
                    Some(TraceSubject),
                    "{version:?}: {arguments:?}"
                );
                assert!(
                    invocation
                        .authored_source_variable_receiver_operand_form(0)
                        .is_none()
                );
            }
        } else if let Some(invocation) =
            registry.resolve_invocation("trace", &["vinfo", "::v"], dialect.authoring_query())
        {
            assert!(
                invocation
                    .authored_source_variable_receiver_operand_form(1)
                    .is_none()
            );
        }
        assert_original_trace_computed_receivers(&registry, dialect);
    }
}

#[test]
fn original_trace_subject_extent_preserves_counted_original_and_independent_array_form() {
    // Native proof: naming.variable.trace-subject-counted-zero-address
    // docs/design/analysis/name-resolution-proofs/trace-subject-counted-zero-address.md
    use tcl_syntax::naming::{NativeNameProtocol, NativeVariableInputForm};
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = NativeNameProtocol::C(version);
        let NativeVariableInputForm::Combined(selected) = VariableReceiverOperandForm::TraceSubject
            .input_form(protocol, b"::v\0tail(k)")
            .unwrap()
        else {
            panic!("trace subject is combined after its selected ingress");
        };
        assert_eq!(selected, b"::v");
        assert!(
            protocol
                .combined_variable_input(selected)
                .element()
                .is_none()
        );
        let NativeVariableInputForm::Combined(selected) = VariableReceiverOperandForm::TraceSubject
            .input_form(protocol, b"::v\xc0\x80tail(k)")
            .unwrap()
        else {
            panic!("trace subject");
        };
        let name = protocol.combined_variable_input(selected);
        assert_eq!(name.root().selected(), b"::v\xc0\x80tail");
        assert_eq!(name.element().unwrap().selected(), b"k");
    }
    assert!(
        VariableReceiverOperandForm::TraceSubject
            .input_form(NativeNameProtocol::Jim084, b"::v")
            .is_none()
    );
}
