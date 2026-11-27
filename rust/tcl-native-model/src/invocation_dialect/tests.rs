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
fn logical_numeric_simulation_requires_explicit_consistent_f5_contract() {
    use tcl_dialect::model::{DialectPoint, Family, Release};
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    let provider = AuthoredLogicalNumericSimulation::Tcl84Core;
    for release in [Release::F5_TCL_TMOS, Release::F5_IRULES_TMM] {
        let dialect = InvocationDialect::of_point(DialectPoint::canonical(release));
        assert_eq!(
            dialect.authored_logical_numeric_simulation(provider),
            Some(provider)
        );
        assert_eq!(dialect.native_scalar_getter_protocol(), None);
        assert_eq!(dialect.scalar_numeric_input_policy(), None);
        let mut conflict = dialect;
        conflict.core_point = Some(DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_0));
        assert_eq!(conflict.authored_logical_numeric_simulation(provider), None);
        conflict = dialect;
        conflict.tcl_version = Some(tcl_dialect::TclVersion::V9_0);
        assert_eq!(conflict.authored_logical_numeric_simulation(provider), None);
        conflict = dialect;
        conflict.numbers = tcl_dialect::NumberSyntax::Tcl90;
        assert_eq!(conflict.authored_logical_numeric_simulation(provider), None);
    }
    let mut unknown = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    assert_eq!(unknown.authored_logical_numeric_simulation(provider), None);
    unknown.native_family = None;
    assert_eq!(unknown.authored_logical_numeric_simulation(provider), None);
    unknown.native_family = Some(Family::F5Irules);
    assert_eq!(
        unknown.authored_logical_numeric_simulation(provider),
        Some(provider)
    );
    let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
    assert_eq!(jim.authored_logical_numeric_simulation(provider), None);
}

#[test]
fn execution_point_retains_actual_release_without_borrowing_a_vendor_base() {
    use tcl_dialect::model::{DialectPoint, Family, Release};
    for version in tcl_dialect::TclVersion::ALL {
        let mut dialect = InvocationDialect::for_version(version);
        assert_eq!(
            dialect.execution_point(),
            Some(DialectPoint::for_tcl_version(version))
        );
        dialect.native_family = Some(Family::F5Irules);
        assert_eq!(dialect.execution_point(), None);
        dialect.native_family = None;
        assert_eq!(dialect.execution_point(), None);
    }
    let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
    assert_eq!(jim.execution_point(), jim.core_point);
    let mut conflict = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    conflict.core_point = Some(DialectPoint::canonical(Release::TCL_9_0));
    assert_eq!(conflict.execution_point(), None);
}

#[test]
fn hosted_fork_profiles_keep_their_core_family_without_c_native_recipes() {
    // Implementation proof: naming.invocation.execution-policy-canonical-point
    // docs/design/analysis/name-resolution-proofs/invocation-execution-policy-canonical-point.md
    // These are profile/admission controls, not observations of a BIG-IP interpreter.
    use tcl_dialect::model::Family;
    for (name, family) in [
        ("f5-tmsh", Family::F5Tcl),
        ("f5-iapps", Family::F5Tcl),
        ("f5-irules", Family::F5Irules),
    ] {
        let profile = tcl_dialect::DialectProfile::find(name).expect("hosted fork profile");
        let selected = InvocationDialect::of_profile(profile);
        assert_eq!(profile.runtime_core_family(), Some(family));
        assert_eq!(selected.family(), Some(family));
        assert_eq!(selected.tcl_version, Some(tcl_dialect::TclVersion::V8_4));
        assert!(selected.execution_point().is_none());
        assert!(selected.native_string_protocol().is_none());
        assert!(selected.native_name_protocol().is_none());
        assert!(profile.f5_core_expr_grammar().is_some());
    }
    for version in tcl_dialect::TclVersion::ALL {
        let profile =
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).expect("C profile");
        assert_eq!(profile.runtime_core_family(), Some(Family::Tcl));
        assert!(
            InvocationDialect::of_profile(profile)
                .native_string_protocol()
                .is_some()
        );
    }
    let jim = crate::test_profiles::resolve_environment("jim");
    assert_eq!(jim.unit_profile().runtime_core_family(), Some(Family::Jim));
    assert!(
        InvocationDialect::of_profile(jim.unit_profile())
            .native_name_protocol()
            .is_some()
    );
}

#[test]
fn execution_policy_comparison_normalises_only_the_selected_point() {
    // Implementation proof: naming.invocation.execution-policy-canonical-point
    // docs/design/analysis/name-resolution-proofs/invocation-execution-policy-canonical-point.md
    use tcl_dialect::model::{BuildProfileId, DialectPoint, Family, Release};
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name())
            .expect("explicit C profile");
        let selected = InvocationDialect::of_profile(profile);
        let mut retained = selected;
        retained.core_point = selected.execution_point();
        assert!(selected.core_point.is_none());
        assert!(retained.core_point.is_some());
        assert!(retained.has_same_execution_policy(selected));
        assert!(selected.has_same_execution_policy(retained));

        let mut changed = selected;
        changed.characters = None;
        assert!(!retained.has_same_execution_policy(changed));
        changed = selected;
        changed.lexer_grammar.expand_syntax = !changed.lexer_grammar.expand_syntax;
        assert!(!retained.has_same_execution_policy(changed));
        changed = selected;
        changed.namespace_import_binding = None;
        assert!(!retained.has_same_execution_policy(changed));
        changed = selected;
        changed.native_family = Some(Family::F5Irules);
        assert!(!retained.has_same_execution_policy(changed));
        changed = selected;
        changed.core_point = Some(DialectPoint::for_tcl_version(
            if version == tcl_dialect::TclVersion::V9_1 {
                tcl_dialect::TclVersion::V8_4
            } else {
                tcl_dialect::TclVersion::V9_1
            },
        ));
        assert!(!retained.has_same_execution_policy(changed));
        changed = selected;
        changed.native_family = None;
        assert!(!retained.has_same_execution_policy(changed));
    }
    let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
    assert!(jim.has_same_execution_policy(jim));
    let mut different_build = jim;
    different_build.core_point = Some(DialectPoint::new(
        Release::JIM_0_84,
        BuildProfileId::JimFull,
    ));
    assert!(!jim.has_same_execution_policy(different_build));
}

#[test]
fn scalar_numeric_input_retains_actual_family_without_an_explicit_point() {
    use tcl_syntax::number::NativeScalarNumericInputPolicy as Policy;
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        assert!(dialect.core_point.is_none());
        assert_eq!(
            dialect.scalar_numeric_input_policy(),
            Some(Policy::LengthDelimited)
        );
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert_eq!(
        jim.scalar_numeric_input_policy(),
        Some(Policy::NulTerminatedJim084)
    );
    for family in [
        tcl_dialect::model::Family::F5Tcl,
        tcl_dialect::model::Family::F5Irules,
    ] {
        let mut foreign = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        foreign.native_family = Some(family);
        assert_eq!(foreign.scalar_numeric_input_policy(), None);
    }
    let mut unknown = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    unknown.native_family = None;
    assert_eq!(unknown.scalar_numeric_input_policy(), None);
}

#[test]
fn primitive_getter_selection_rejects_conflicting_actual_axes() {
    use tcl_dialect::{
        TclVersion,
        model::{DialectPoint, Family, Release},
    };
    let mut dialect = InvocationDialect::for_version(TclVersion::V8_6);
    dialect.core_point = Some(DialectPoint::for_tcl_version(TclVersion::V9_1));
    assert_eq!(dialect.native_scalar_getter_protocol(), None);
    dialect.tcl_version = Some(TclVersion::V9_1);
    assert_eq!(
        dialect
            .native_scalar_getter_protocol()
            .unwrap()
            .tcl_version(),
        Some(TclVersion::V9_1)
    );
    dialect.native_family = Some(Family::F5Irules);
    assert_eq!(dialect.native_scalar_getter_protocol(), None);

    let mut jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
    assert!(jim.native_scalar_getter_protocol().unwrap().is_jim084());
    jim.tcl_version = Some(TclVersion::V8_6);
    assert_eq!(jim.native_scalar_getter_protocol(), None);
    jim.tcl_version = None;
    jim.native_family = Some(Family::Tcl);
    assert_eq!(jim.native_scalar_getter_protocol(), None);
}

#[test]
fn invocation_character_model_retains_embedding_policy() {
    let profile = tcl_dialect::DialectProfile::irules();
    assert_eq!(
        InvocationDialect::of_profile(profile).characters,
        Some(tcl_dialect::StringCharacterModel::Utf16CodeUnits)
    );
    for version in tcl_dialect::TclVersion::ALL {
        assert_eq!(
            InvocationDialect::for_version(version).characters,
            Some(version.string_character_model())
        );
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert_eq!(
        jim.characters,
        Some(tcl_dialect::StringCharacterModel::Jim084Utf8)
    );
}

#[test]
fn private_ensemble_implementations_follow_native_engine_versions() {
    use tcl_dialect::TclVersion;
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        assert_eq!(
            dialect
                .ensemble_implementation_namespace(EnsembleImplementationFamily::Namespace)
                .is_some(),
            version >= TclVersion::V8_6
        );
        assert_eq!(
            dialect
                .ensemble_implementation_namespace(EnsembleImplementationFamily::Dict)
                .is_some(),
            version >= TclVersion::V8_5
        );
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert_eq!(
        jim.ensemble_implementation_namespace(EnsembleImplementationFamily::Namespace),
        None
    );
    assert_eq!(
        jim.ensemble_implementation_namespace(EnsembleImplementationFamily::Dict),
        None
    );
}
#[test]
fn private_ensemble_addresses_preserve_selected_tail_bytes_and_engine_refusals() {
    use tcl_dialect::TclVersion;
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for (family, namespace, first) in [
            (
                EnsembleImplementationFamily::String,
                "::tcl::string",
                TclVersion::V8_5,
            ),
            (
                EnsembleImplementationFamily::Dict,
                "::tcl::dict",
                TclVersion::V8_5,
            ),
            (
                EnsembleImplementationFamily::Namespace,
                "::tcl::namespace",
                TclVersion::V8_6,
            ),
            (
                EnsembleImplementationFamily::Array,
                "::tcl::array",
                TclVersion::V8_6,
            ),
        ] {
            for tail in [b"worker".as_slice(), b"::literal", b"\xff\0tail", b""] {
                let address = dialect.ensemble_implementation_command_address(family, tail);
                if version < first {
                    assert!(address.is_none());
                } else {
                    let mut expected = namespace.as_bytes().to_vec();
                    expected.extend_from_slice(b"::");
                    expected.extend_from_slice(tail);
                    assert_eq!(address, Some(expected));
                }
            }
        }
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    for family in [
        EnsembleImplementationFamily::String,
        EnsembleImplementationFamily::Dict,
        EnsembleImplementationFamily::Namespace,
        EnsembleImplementationFamily::Array,
    ] {
        assert!(
            jim.ensemble_implementation_command_address(family, b"worker")
                .is_none()
        );
    }
}

#[test]
fn source_word_materialisation_selects_actual_or_explicit_logical_recipes() {
    let provider = super::LogicalSourceWordProvider::Tcl84CoreSimulation;
    let logical = tcl_dialect::DialectProfile::irules();
    let dialect = crate::InvocationDialect::of_profile(logical);
    assert!(dialect.native_source_string_protocol().is_none());
    assert_eq!(
        dialect.logical_source_string_protocol(provider, logical),
        Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(dialect.execution_point().is_none());
    for version in tcl_dialect::TclVersion::ALL {
        let actual = crate::InvocationDialect::for_version(version);
        assert_eq!(
            actual.native_source_string_protocol(),
            Some(tcl_syntax::native_string::NativeStringProtocol::C(version))
        );
        assert!(
            actual
                .logical_source_string_protocol(provider, logical)
                .is_none()
        );
        let mut changed = actual;
        changed.lexer_grammar.escapes = tcl_dialect::EscapeSyntax::Jim;
        assert!(changed.native_source_string_protocol().is_none());
    }
}

#[test]
fn logical_parser_recipe_does_not_select_a_native_vendor_engine() {
    let profile = tcl_dialect::DialectProfile::irules();
    let dialect = crate::InvocationDialect::of_profile(profile);
    let provider = super::LogicalExpressionParseProvider::Tcl84CoreSimulation;
    assert_eq!(
        dialect
            .expression_parse_context(Some(profile))
            .native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Unknown
    );
    let context = dialect
        .logical_expression_parse_context(provider, profile)
        .expect("explicit authored F5 parser recipe");
    assert_eq!(context.lexer_grammar, profile.grammar);
    assert_eq!(context.f5_word_grammar, profile.f5_core_expr_grammar());
    assert_eq!(
        context.native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4)
    );
    assert_eq!(
        dialect.expression_parse_context_with(Some(provider), Some(profile)),
        context
    );
    assert_eq!(
        dialect.expression_parse_context_with(None, Some(profile)),
        dialect.expression_parse_context(Some(profile))
    );
    assert!(dialect.execution_point().is_none());
    assert!(crate::TemplateParseErrors::for_dialect(dialect).is_none());
    assert!(
        dialect
            .logical_expression_template_policy(provider, profile)
            .is_some()
    );
    for version in tcl_dialect::TclVersion::ALL {
        let actual = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        assert!(
            crate::InvocationDialect::of_profile(actual)
                .logical_expression_parse_context(provider, actual)
                .is_none()
        );
        assert_eq!(
            crate::InvocationDialect::of_profile(actual)
                .expression_parse_context_with(Some(provider), Some(actual)),
            crate::InvocationDialect::of_profile(actual).expression_parse_context(Some(actual))
        );
    }
    assert!(
        crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl())
            .logical_expression_parse_context(provider, profile)
            .is_none()
    );
}
