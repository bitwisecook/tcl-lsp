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

use tcl_dialect::model::{Family, SpecProvider, SurfaceQuery, surface_admits};

use tcl_dialect::BracedVarStyle;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::{BuildProfileId, EnvironmentKind, Provenance, Release, WorldPolicy};

use super::*;

#[test]
fn subcommand_source_configuration_reference_loads_and_omission_abstains() {
    let pack = evaluate_pack(
        "speclib probe 2.0 { command sample { \
             subcommand compare { \
               source_configuration_reference DataGroupComparison \
             }; \
             subcommand plain { arity 0 } } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("sample").unwrap().spec;
    assert_eq!(
        spec.subcommands[0].source_configuration_reference,
        Some(tcl_registry::SourceConfigurationReferenceLayout::DataGroupComparison)
    );
    assert!(spec.subcommands[1].source_configuration_reference.is_none());
}

#[test]
fn invalid_source_configuration_reference_withdraws_the_descriptor() {
    let pack = evaluate_pack(
        "speclib probe 2.0 { command sample { subcommand compare { \
             source_configuration_reference Invented } } }",
    );
    assert!(
        pack.command("sample").unwrap().spec.subcommands[0]
            .source_configuration_reference
            .is_none()
    );
    assert!(
        pack.notices.iter().any(|notice| {
            notice.message == "unknown source configuration reference `Invented` dropped"
        }),
        "{:?}",
        pack.notices
    );
}

#[test]
fn source_configuration_reference_has_static_and_evaluated_loader_parity() {
    let source = "speclib probe 2.0 { command sample { subcommand compare { \
                      source_configuration_reference DataGroupComparison } } }";
    let fast = evaluate_pack(source);
    let evaluated = evaluate_pack_with(
        source,
        &EvalOptions {
            static_fast_path: false,
            ..EvalOptions::default()
        },
    );
    let templated = evaluate_pack(
        "speclib probe 2.0 { set layout DataGroupComparison; \
             command sample { subcommand compare { \
               source_configuration_reference $layout } } }",
    );
    for pack in [&fast, &evaluated, &templated] {
        assert!(pack.load_error.is_none(), "{:?}", pack.load_error);
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert_eq!(
            pack.command("sample").unwrap().spec.subcommands[0].source_configuration_reference,
            Some(tcl_registry::SourceConfigurationReferenceLayout::DataGroupComparison)
        );
    }
    assert_eq!(
        format!("{:?}", fast.command("sample").unwrap().spec),
        format!("{:?}", evaluated.command("sample").unwrap().spec)
    );
    assert_eq!(
        format!("{:?}", fast.command("sample").unwrap().spec),
        format!("{:?}", templated.command("sample").unwrap().spec)
    );
}

#[test]
fn original_abstract_definition_grammar_loads_without_changing_factory_visibility() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    let pack = evaluate_pack(
        "speclib probe 1.1 { command makeAbstract { definition_body tcloo-abstract; \
             manufacturer create -names-instance-at 1 -definition-body-at 2 -constructor-args-from 2 } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("makeAbstract").unwrap().spec;
    let grammar = spec.definition_body.unwrap();
    for selector in ["create", "new", "createWithNamespace"] {
        assert_eq!(
            grammar.manufacturer(selector).unwrap().visibility,
            MemberVisibility::Unexported
        );
    }
    assert_eq!(
        spec.manufacturer_methods[0].visibility,
        MemberVisibility::Exported
    );
    assert!(spec.native_result.is_none());
}

#[test]
fn expression_representation_coercion_loads_without_a_native_hook() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command evaluate { \
             representation_effect {CoerceExpressionValues 0} } }",
    );
    assert_eq!(
        pack.command("evaluate").unwrap().spec.representation_effect,
        Some(RepresentationEffect::CoerceExpressionValues { arguments_from: 0 })
    );
}

#[test]
fn numeric_object_conversion_requirements_load_without_a_result_contract() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command convert { \
             representation_effect {CoerceNumericValues 0} } }",
    );
    let spec = &pack.command("convert").unwrap().spec;
    assert_eq!(
        spec.representation_effect,
        Some(RepresentationEffect::CoerceNumericValues { arguments_from: 0 })
    );
    assert!(spec.native_result.is_none());
}

/// A pack can declare `DEFERS_BODY` for its own definer, and — the half
/// that matters — a pack that says nothing leaves the trait **unset**.
///
/// The flag tells a static walk that an unreadable body word costs it
/// nothing about the call's completion, so the silent
/// default has to be the abstaining one: unset means "this body may run
/// here". Traits resolve by name against the registry's own flag list, so
/// there is no loader table to keep in step — but there is also nothing
/// stopping a future edit from inventing one, which is what this pins.
#[test]
fn a_pack_declares_defers_body_and_omitting_it_stays_abstaining() {
    let declared = evaluate_pack(
        "speclib probe 1.1 { command mydefiner { \
             arity 3; traits {DEFERS_BODY} } }",
    );
    assert!(
        declared
            .command("mydefiner")
            .unwrap()
            .spec
            .traits
            .contains(tcl_registry::Traits::DEFERS_BODY),
        "a pack must be able to declare its own definer dormant: {:?}",
        declared.notices
    );
    assert!(declared.notices.is_empty(), "{:?}", declared.notices);

    let silent = evaluate_pack("speclib probe 1.1 { command mydefiner { arity 3 } }");
    assert!(
        !silent
            .command("mydefiner")
            .unwrap()
            .spec
            .traits
            .contains(tcl_registry::Traits::DEFERS_BODY),
        "an undeclared body must stay material, never dormant-by-default"
    );
}

/// Every plain case descriptor field has an authored row and a nondefault
/// assertion. The compile-time field count catches omitted DSL coverage.
#[test]
fn case_list_rows_author_every_descriptor_field_issue_2140() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command demo { case_list { \
             subject_args 2; \
             two_arg_optionless_surface tcl8.5+; \
             fallthrough_body -; \
             value_options_require_regex {-indexvar -matchvar}; \
             special_match_options {-sorted}; \
             clause_flags {-re -gl -ex}; clause_regex_flag -re; \
             clause_value_flags {-timeout}; clause_end_options_flag --; \
             clause_force_inline_flag -nobrace; clause_force_list_flag -brace; \
             clause_force_list_shape first_arg_only_remainder; \
             allow_omitted_final_body 1; \
             keyword_patterns {default} -final-only; exhaustive_keyword_patterns {default}; \
             warn_unbraced_bodies 1; \
             optional_subject_separator --; \
             default_mode glob; pattern_words lists } } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let case = pack.command("demo").unwrap().spec.case_list.unwrap();

    // One assertion per field of `CaseListSpec`. Adding a field to the
    // struct without a row to author it leaves this list short of the
    // struct, which the count assertion below catches.
    assert_eq!(case.subject_args, 2);
    assert_eq!(
        case.two_arg_optionless_surface,
        Some(SpecSurface::TCL85_PLUS)
    );
    assert_eq!(case.fallthrough_body, Some("-"));
    assert_eq!(case.value_options_require_regex, ["-indexvar", "-matchvar"]);
    assert_eq!(case.special_match_options, ["-sorted"]);
    assert_eq!(case.clause_flags, ["-re", "-gl", "-ex"]);
    assert_eq!(case.clause_regex_flag, Some("-re"));
    assert_eq!(case.clause_value_flags, ["-timeout"]);
    assert_eq!(case.clause_end_options_flag, Some("--"));
    assert_eq!(case.clause_force_inline_flag, Some("-nobrace"));
    assert_eq!(case.clause_force_list_flag, Some("-brace"));
    assert_eq!(
        case.clause_force_list_shape,
        Some(tcl_registry::CaseForceListShape::FirstArgOnlyRemainder)
    );
    assert!(case.allow_omitted_final_body);
    assert_eq!(case.keyword_patterns, ["default"]);
    assert!(case.keyword_patterns_require_final);
    assert_eq!(case.exhaustive_keyword_patterns, ["default"]);
    assert!(case.warn_unbraced_bodies);
    assert_eq!(case.optional_subject_separator, Some("--"));
    assert_eq!(case.default_mode, tcl_registry::spec::CaseMatchMode::Glob);
    assert_eq!(case.pattern_words, tcl_registry::spec::PatternWords::Lists);

    // …and the list above is the whole struct. Counted from the
    // definition rather than from `Debug`, whose field *values*
    // contain `": "` of their own.
    let spec_source = include_str!("../../../tcl-registry/src/spec.rs");
    let from = spec_source
        .find("pub struct CaseListSpec")
        .expect("`CaseListSpec` by that name");
    let to = spec_source[from..]
        .find("\n}\n")
        .expect("a struct body that ends");
    let fields = spec_source[from..from + to]
        .lines()
        .filter(|line| line.starts_with("    pub ") && line.contains(':'))
        .count();
    assert_eq!(
        fields, 20,
        "`CaseListSpec` has {fields} fields; the assertions above cover 20 — a new field needs a `case_list` row and an assertion here"
    );
}

#[test]
fn incomplete_effect_and_transition_blocks_cannot_claim_empty_semantics() {
    // Implementation contract: naming.dsl.incomplete-effect-descriptors
    // docs/design/analysis/name-resolution-proofs/dsl-incomplete-effect-descriptors.md

    for (field, rows) in [
        (
            "state_transitions",
            "composition Replace; argument_shape whatever; resolver none; widen -operands 1; covers x; commit yes",
        ),
        (
            "world_effects",
            "composition Replace; access read; dynamic_fallback yes; resolver none",
        ),
        ("state_transitions", "composition invalid"),
        ("world_effects", "composition invalid"),
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 1.1 {{ command demo {{ {field} {{ {rows} }} }} }}"
        ));
        assert!(
            pack.command("demo").is_none(),
            "{field}: an incomplete descriptor must not become EMPTY"
        );
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.class == VocabularyClass::Semantic),
            "{field}: {:?}",
            pack.notices
        );
    }
    let pack = evaluate_pack(
        "speclib probe 1.1 { command demo { state_transitions { composition Replace }; world_effects none } }",
    );
    let spec = pack.command("demo").unwrap().spec;
    assert_eq!(
        spec.state_transitions.unwrap().composition,
        tcl_registry::state_transition::StateTransitionComposition::Replace
    );
    let effects = spec.world_effects.unwrap();
    assert!(effects.static_footprint.accesses.is_empty());
    assert_eq!(
        effects.static_footprint.callback,
        WorldEffectDescriptor::EMPTY.static_footprint.callback
    );
    assert!(effects.resolver.is_none());
}

/// The command-level switch rows `case_list` once read are the option
/// rows' effects now: a pack still writing one is told so, and the row
/// changes nothing.
#[test]
fn a_retired_case_list_switch_row_is_dropped_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command demo { case_list { \
             subject_args 1; regex_option -regexp } } }",
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("`regex_option` is retired")),
        "{:?}",
        pack.notices
    );
    let case = pack.command("demo").unwrap().spec.case_list.unwrap();
    assert_eq!(case.subject_args, 1);
}

/// `default_mode` names one of the three comparisons a case list's
/// clauses make, and `pattern_words` one of the two readings of a
/// pattern word; any other word — the specialised `other` comparison
/// among them, which only an option row selects — is told so and the
/// block keeps the exact comparison and the single pattern.
#[test]
fn an_unknown_case_list_reading_keeps_the_default() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command demo { case_list { \
             subject_args 1; default_mode other; pattern_words many } } }",
    );
    for row in ["`default_mode` takes", "`pattern_words` takes"] {
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains(row)),
            "{row}: {:?}",
            pack.notices
        );
    }
    let case = pack.command("demo").unwrap().spec.case_list.unwrap();
    assert_eq!(case.default_mode, tcl_registry::spec::CaseMatchMode::Exact);
    assert_eq!(case.pattern_words, tcl_registry::spec::PatternWords::Single);
}

#[test]
fn state_transition_blocks_preserve_complete_authored_rows() {
    let pack = evaluate_pack(
        "speclib probe 2.2 { command demo { state_transitions { \
             composition Replace; argument_shape Positional; resolver none; \
             widen -operands {Strided 0 2} -domains {VariableCells}; \
             covers DeclaredWorldEffect -domains {VariableStore}; \
             commit MayCommitBeforeAbruptCompletion } } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let descriptor = pack
        .command("demo")
        .unwrap()
        .spec
        .state_transitions
        .unwrap();
    assert_eq!(descriptor.composition, StateTransitionComposition::Replace);
    assert_eq!(
        descriptor.argument_shape,
        StateTransitionArgumentShape::Positional
    );
    assert_eq!(
        descriptor.commit,
        StateTransitionCommit::MayCommitBeforeAbruptCompletion
    );
    assert!(descriptor.resolver.is_none());
    assert_eq!(descriptor.dynamic_widening.len(), 1);
    assert_eq!(
        descriptor.dynamic_widening[0].operands,
        StateTransitionOperandLayout::Strided {
            first: 0,
            stride: 2
        }
    );
    assert_eq!(
        descriptor.dynamic_widening[0].domains,
        [StateTransitionDomain::VariableCells]
    );
    assert_eq!(descriptor.effect_coverage.len(), 1);
    assert_eq!(
        descriptor.effect_coverage[0].source,
        WorldEffectWriteSource::DeclaredWorldEffect
    );
    assert_eq!(
        descriptor.effect_coverage[0].domains,
        [WorldStateDomain::VariableStore]
    );
}

// The original row-reading regression also covers malformed input. Invalid
// rows now withdraw the complete descriptor instead of exposing a partial
// EMPTY descriptor; the valid composition control remains independent.
#[test]
fn state_transition_blocks_read_every_row_world_effects_only_composition_issue_2140() {
    // Implementation contract: naming.dsl.incomplete-effect-descriptors
    // docs/design/analysis/name-resolution-proofs/dsl-incomplete-effect-descriptors.md
    assert_transition_composition_controls();
    assert_each_malformed_transition_row_is_read();
    assert_world_effects_rows_withdraw_complete_descriptor();
}

fn assert_transition_composition_controls() {
    let pack = evaluate_pack(
        "speclib probe 2.2 { command demo { state_transitions { \
             composition Replace; resolver none }; \
             world_effects { composition Replace } } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").unwrap().spec;
    let descriptor = spec.state_transitions.expect("a descriptor");
    assert_eq!(
        descriptor.composition,
        tcl_registry::state_transition::StateTransitionComposition::Replace,
    );
    assert!(descriptor.resolver.is_none(), "`resolver none`");
    assert_eq!(
        spec.world_effects.expect("a descriptor").composition,
        tcl_registry::world_effect::WorldEffectComposition::Replace
    );
}

fn assert_each_malformed_transition_row_is_read() {
    let transitions = evaluate_pack(
        "speclib probe 2.2 { command demo { state_transitions { \
             composition Replace; argument_shape whatever; resolver none; \
             widen -operands 1; covers x; commit yes } } }",
    );
    assert!(
        transitions.command("demo").is_none(),
        "incomplete transitions must not publish a partial descriptor"
    );
    assert!(
        transitions
            .notices
            .iter()
            .all(|notice| !notice.message.contains("is not yet loadable; dropped")),
        "every transition row is read"
    );
    let notices: Vec<&str> = transitions
        .notices
        .iter()
        .map(|notice| notice.message.as_str())
        .collect();
    for unreadable in [
        "unknown state-transition argument shape `whatever` dropped",
        "unreadable operand layout `1` dropped",
        "a `covers` row needs a write source and `-domains`; row dropped",
        "unknown state-transition commit `yes` dropped",
    ] {
        assert!(
            notices.contains(&unreadable),
            "`{unreadable}` expected; got {notices:#?}"
        );
    }
    assert!(transitions.notices.iter().any(|notice| {
        notice.class == VocabularyClass::Semantic
            && notice.message
                == "incomplete state-transition row excludes this command from strong analysis"
    }));
}

fn assert_world_effects_rows_withdraw_complete_descriptor() {
    let effects = evaluate_pack(
        "speclib probe 2.2 { command demo { world_effects { \
             composition Replace; access read; dynamic_fallback yes; \
             resolver none } } }",
    );
    assert!(
        effects.command("demo").is_none(),
        "unloadable world effects must not publish a partial descriptor"
    );
    for row in ["access", "dynamic_fallback", "resolver"] {
        let expected = format!(
            "`world_effects` row `{row}` is not loadable; the incomplete semantic descriptor excludes this command from strong analysis"
        );
        assert!(
            effects.notices.iter().any(|notice| {
                notice.class == VocabularyClass::Semantic && notice.message == expected
            }),
            "`{row}` should be rejected with a notice; got {:?}",
            effects.notices
        );
    }
}

#[test]
fn case_list_loader_preserves_the_clause_shape_flags() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command demo { case_list { \
             two_arg_optionless_surface tcl8.5+; \
             clause_end_options_flag --; clause_force_inline_flag -nobrace; \
             clause_force_list_flag -brace; clause_force_list_shape first_arg_only_remainder; \
             allow_omitted_final_body 1; \
             warn_unbraced_bodies 1 } } }",
    );
    let case = pack.command("demo").unwrap().spec.case_list.unwrap();
    assert_eq!(
        case.two_arg_optionless_surface,
        Some(SpecSurface::TCL85_PLUS)
    );
    assert_eq!(case.clause_end_options_flag, Some("--"));
    assert_eq!(case.clause_force_inline_flag, Some("-nobrace"));
    assert_eq!(case.clause_force_list_flag, Some("-brace"));
    assert_eq!(
        case.clause_force_list_shape,
        Some(tcl_registry::CaseForceListShape::FirstArgOnlyRemainder)
    );
    assert!(case.allow_omitted_final_body);
    assert!(case.warn_unbraced_bodies);
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);

    let disabled = evaluate_pack(
        "speclib probe 1.1 { command demo { case_list { \
             allow_omitted_final_body no; warn_unbraced_bodies false } } }",
    );
    let case = disabled.command("demo").unwrap().spec.case_list.unwrap();
    assert!(!case.allow_omitted_final_body);
    assert!(!case.warn_unbraced_bodies);
}

/// Native folder IDs select their actual command/subcommand callbacks.
/// Unknown or mismatched IDs retain a notice and abstaining placeholder.
#[test]
fn a_native_fold_id_installs_the_shipped_folder() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n\
             command string {\n\
                 subcommand range {\n\
                     arity 3\n\
                     const_fold -native string::range::const_fold\n\
                 }\n\
                 subcommand is {\n\
                     arity 2..\n\
                     const_fold_versioned -native string::is::const_fold_versioned\n\
                 }\n\
             }\n\
             command probe::short {\n\
                 arity 3\n\
                 const_fold -native string::range\n\
             }\n\
             command probe::unknown {\n\
                 arity 1\n\
                 const_fold -native probe::unknown::const_fold\n\
             }\n\
             }",
    );
    let string = pack.command("string").expect("declared");
    let sub = |name: &str| {
        string
            .spec
            .subcommands
            .iter()
            .find(|sub| sub.name == name)
            .expect("declared")
    };
    let range = sub("range").const_fold.expect("a folder");
    assert_eq!(range(&["abcdef", "1", "3"]).as_deref(), Some("bcd"));
    let is = sub("is").const_fold_versioned.expect("a folder");
    assert_eq!(
        is(&["integer", "42"], Some(tcl_dialect::TclVersion::V9_0)).as_deref(),
        Some("1")
    );
    for name in ["probe::short", "probe::unknown"] {
        let fold = pack
            .command(name)
            .expect("declared")
            .spec
            .const_fold
            .expect("the placeholder");
        assert_eq!(fold(&["abcdef", "1", "3"]), None, "{name}");
    }
    let notices: Vec<&str> = pack
        .notices
        .iter()
        .map(|notice| notice.message.as_str())
        .collect();
    assert!(
        notices.iter().any(|notice| notice.contains(
            "`const_fold -native string::range` is not this scope's id; spell it \
                 `probe::short::const_fold`"
        )),
        "{notices:#?}"
    );
    assert!(
        notices.iter().any(|notice| notice.contains(
            "`const_fold -native probe::unknown::const_fold` names nothing this build ships"
        )),
        "{notices:#?}"
    );
    assert_eq!(notices.len(), 2, "{notices:#?}");
}

/// The loader resolves a `-native ID` by matching the catalogue's own
/// spelling, so its value tables have to name every variant the catalogue
/// does. A registry that grows a hook and a studio catalogue that lists it
/// would otherwise leave the loader silently dropping the new id.
fn assert_table_covers(what: &str, mine: &[String], catalogue: &[catalogue::Variant]) {
    let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
    let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
    expected.sort_unstable();
    mine.sort_unstable();
    assert_eq!(mine, expected, "the {what}-hook table is out of step");
}

/// The five closed compiler-hook catalogues, by Rust variant name.
#[test]
fn compiler_hook_catalogues_cover_their_tables() {
    fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
        all.iter().map(catalogue::variant_name).collect()
    }
    for (what, mine, catalogue) in [
        ("lowering", names(LOWERING_HOOKS), catalogue::LOWERING_HOOKS),
        ("codegen", names(CODEGEN_HOOKS), catalogue::CODEGEN_HOOKS),
        (
            "inline codegen",
            names(INLINE_CODEGEN_HOOKS),
            catalogue::INLINE_CODEGEN_HOOKS,
        ),
        ("analyser", names(ANALYSER_HOOKS), catalogue::ANALYSER_HOOKS),
        (
            "return type",
            names(RETURN_TYPE_HOOKS),
            catalogue::RETURN_TYPE_HOOKS,
        ),
    ] {
        assert_table_covers(what, &mine, catalogue);
    }
}

/// Every `HookFamily` variant and the `semantics` / `evaluate` / `facts`
/// fields each own a `SCOPE::FIELD`-keyed native table in
/// `tcl_registry::pack_hooks`
/// (`docs/design/compiler/value-evaluation.md` § *`-native ID`, and the
/// per-family catalogues*); this is the same obligation as
/// [`compiler_hook_catalogues_cover_their_tables`], read from the table's
/// own id rather than a Rust variant name.
#[test]
fn native_hook_tables_cover_their_catalogues() {
    fn ids<T>(table: &[(&str, T)]) -> Vec<String> {
        table.iter().map(|(id, _)| (*id).to_owned()).collect()
    }
    for (what, mine, catalogue) in [
        (
            "arg-role-resolver native",
            ids(tcl_registry::pack_hooks::ARG_ROLE_RESOLVER_NATIVE),
            catalogue::ARG_ROLE_RESOLVER_NATIVE,
        ),
        (
            "command-prefix-resolver native",
            ids(tcl_registry::pack_hooks::COMMAND_PREFIX_RESOLVER_NATIVE),
            catalogue::COMMAND_PREFIX_RESOLVER_NATIVE,
        ),
        (
            "script-timing-resolver native",
            ids(tcl_registry::pack_hooks::SCRIPT_TIMING_RESOLVER_NATIVE),
            catalogue::SCRIPT_TIMING_RESOLVER_NATIVE,
        ),
        (
            "const-fold native",
            ids(tcl_registry::pack_hooks::CONST_FOLD_NATIVE),
            catalogue::CONST_FOLD_NATIVE,
        ),
        (
            "const-fold-versioned native",
            ids(tcl_registry::pack_hooks::CONST_FOLD_VERSIONED_NATIVE),
            catalogue::CONST_FOLD_VERSIONED_NATIVE,
        ),
        (
            "taint-sink-gate native",
            ids(tcl_registry::pack_hooks::TAINT_SINK_GATE_NATIVE),
            catalogue::TAINT_SINK_GATE_NATIVE,
        ),
        (
            "context-gate native",
            ids(tcl_registry::pack_hooks::CONTEXT_GATE_NATIVE),
            catalogue::CONTEXT_GATE_NATIVE,
        ),
        (
            "literal-argument-validator native",
            ids(tcl_registry::pack_hooks::LITERAL_ARGUMENT_VALIDATOR_NATIVE),
            catalogue::LITERAL_ARGUMENT_VALIDATOR_NATIVE,
        ),
        (
            "clause-shape-check native",
            ids(tcl_registry::pack_hooks::CLAUSE_SHAPE_CHECK_NATIVE),
            catalogue::CLAUSE_SHAPE_CHECK_NATIVE,
        ),
        (
            "option-arity native",
            ids(tcl_registry::pack_hooks::OPTION_ARITY_NATIVE),
            catalogue::OPTION_ARITY_NATIVE,
        ),
        (
            "constraints native",
            ids(tcl_registry::pack_hooks::CONSTRAINTS_NATIVE),
            catalogue::CONSTRAINTS_NATIVE,
        ),
        (
            "state-transition-resolver native",
            ids(tcl_registry::pack_hooks::STATE_TRANSITION_RESOLVER_NATIVE),
            catalogue::STATE_TRANSITION_RESOLVER_NATIVE,
        ),
        (
            "semantics native",
            ids(tcl_registry::pack_hooks::SEMANTICS_NATIVE),
            catalogue::SEMANTICS_NATIVE,
        ),
        (
            "evaluate native",
            ids(tcl_registry::pack_hooks::EVALUATE_NATIVE),
            catalogue::EVALUATE_NATIVE,
        ),
        (
            "facts native",
            ids(tcl_registry::pack_hooks::FACTS_NATIVE),
            catalogue::FACTS_NATIVE,
        ),
    ] {
        assert_table_covers(what, &mine, catalogue);
    }
}

/// Same obligation for the vocabularies a row flag resolves.
/// [`value_transfer_tables_cover_their_catalogues`] does the same for
/// the `value_transfer` vocabulary.
#[test]
fn value_tables_cover_their_catalogues() {
    fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
        all.iter().map(catalogue::variant_name).collect()
    }
    for (what, mine, catalogue) in [
        ("argument role", names(ArgRole::ALL), catalogue::ARG_ROLES),
        (
            "source deprecation advice",
            names(&tcl_registry::SourceDeprecationAdvice::ALL),
            catalogue::SOURCE_DEPRECATION_ADVICE,
        ),
        ("Tcl type", names(TCL_TYPES), catalogue::TCL_TYPES),
        ("body kind", names(BODY_KINDS), catalogue::BODY_KINDS),
        (
            "executable lookup frame",
            names(&ScriptLookupScope::ALL),
            catalogue::SCRIPT_LOOKUP_SCOPES,
        ),
        (
            "variable scope",
            names(VARIABLE_SCOPES),
            catalogue::VARIABLE_SCOPES,
        ),
        (
            "argument presentation",
            names(PRESENTATIONS),
            catalogue::ARG_PRESENTATIONS,
        ),
        (
            "side-effect target",
            names(SIDE_EFFECT_TARGETS),
            catalogue::SIDE_EFFECT_TARGETS.as_slice(),
        ),
        (
            "connection side",
            names(CONNECTION_SIDES),
            catalogue::CONNECTION_SIDES,
        ),
        ("form kind", names(FORM_KINDS), catalogue::FORM_KINDS),
        (
            "storage type",
            names(STORAGE_TYPES),
            catalogue::STORAGE_TYPES,
        ),
    ] {
        let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
        let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
        expected.sort_unstable();
        mine.sort_unstable();
        assert_eq!(mine, expected, "the {what} table is out of step");
    }
}

/// [`value_tables_cover_their_catalogues`], split for clippy's
/// function-length lint: the `value_transfer` vocabulary's own closed
/// word lists. Most are matched by their DSL spelling (`as_str()`:
/// `write_or_preserve`, `tcl.expr`, `bounded_tcl`, …), not the Rust
/// variant name `names` reads — only `evaluate -direct ID`
/// (`NativeEvalId`) keeps the older, Rust-spelled convention, since it
/// predates `-native`.
#[test]
fn value_transfer_tables_cover_their_catalogues() {
    fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
        all.iter().map(catalogue::variant_name).collect()
    }
    for (what, mine, catalogue) in [
        (
            "native evaluator id",
            names(tcl_registry::value_transfer::NativeEvalId::ALL),
            catalogue::NATIVE_EVAL_IDS,
        ),
        (
            "language profile",
            tcl_registry::value_transfer::LanguageProfileId::ALL
                .iter()
                .map(|profile| profile.as_str().to_owned())
                .collect(),
            catalogue::LANGUAGE_PROFILES,
        ),
        (
            "host kind",
            tcl_registry::value_transfer::HostKind::ALL
                .iter()
                .map(|host| host.as_str().to_owned())
                .collect(),
            catalogue::HOST_KINDS,
        ),
        (
            "exactness",
            tcl_registry::value_transfer::Exactness::ALL
                .iter()
                .map(|exactness| exactness.as_str().to_owned())
                .collect(),
            catalogue::EXACTNESS,
        ),
        (
            "context dependency",
            tcl_registry::value_transfer::ContextDependency::WORDS
                .iter()
                .map(|dependency| dependency.as_str().to_owned())
                .collect(),
            catalogue::CONTEXT_DEPENDENCIES,
        ),
        (
            "outcome kind",
            tcl_registry::value_transfer::OutcomeKind::ALL
                .iter()
                .map(|outcome| outcome.as_str().to_owned())
                .collect(),
            catalogue::OUTCOME_KINDS,
        ),
        (
            "declared effect",
            tcl_registry::value_transfer::DeclaredEffect::ALL
                .iter()
                .map(|effect| effect.as_str().to_owned())
                .collect(),
            catalogue::DECLARED_EFFECTS,
        ),
        (
            "evaluate reason",
            tcl_registry::value_transfer::OptionEvaluation::REASONS
                .iter()
                .map(|(word, _)| (*word).to_owned())
                .collect(),
            catalogue::EVALUATE_REASONS,
        ),
    ] {
        let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
        let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
        expected.sort_unstable();
        mine.sort_unstable();
        assert_eq!(mine, expected, "the {what} table is out of step");
    }
}

#[test]
fn a_pack_with_no_speclib_wrapper_loads_nothing_and_says_so() {
    let pack = evaluate_pack("command lonely { arity 1 }");
    assert!(pack.commands.is_empty());
    assert_eq!(pack.notices.len(), 1);
    assert!(pack.notices[0].message.contains("no `speclib`"));
}

#[test]
fn statement_separation_is_ordinary_tcl() {
    // `;` separates, `;#` is a trailing comment, and a bare `#` mid-line is
    // NOT one — it becomes flags on the row, all dropped with a notice.
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command demo { arity 2 # not a comment ; return_type String }\n\
             }",
    );
    let demo = pack.command("demo").expect("demo loads");
    assert_eq!(demo.spec.arity, Arity::exact(2));
    let dropped: Vec<&str> = pack
        .notices
        .iter()
        .map(|notice| notice.message.as_str())
        .collect();
    assert_eq!(
        dropped.len(),
        4,
        "each of `#`, `not`, `a`, `comment` drops: {dropped:?}"
    );
}

#[test]
fn native_cardinality_role_contract_loads_and_tcl_count_hook_is_rejected() {
    let pack = evaluate_pack(
        r"speclib counts 1.0 {
            command copied {
                arg_role_count_resolver -native foreach::arg_role_count_resolver
                arg_role_resolver_roles Body
            }
            command unsupported {
                arg_role_count_resolver {nwords} {return {2 Body}}
                arg_role_resolver_roles Body
            }
        }",
    );
    let selected = pack.command("copied").expect("native contract loads");
    let resolver = selected
        .spec
        .arg_role_count_resolver
        .expect("actual count resolver");
    assert_eq!(resolver(3), vec![(2, ArgRole::Body)]);
    assert_eq!(resolver(5), vec![(4, ArgRole::Body)]);
    assert!(pack.command("unsupported").is_none());
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("Tcl count hooks are unsupported"))
    );
}

#[test]
fn native_scan_count_contracts_load_without_value_hooks() {
    let pack = evaluate_pack(
        r"speclib scans 1.0 {
                command copiedscan {
                    arg_role_count_resolver -native scan::arg_role_count_resolver
                    arg_role_resolver_roles {ScanFormat VarWrite}
                }
                command copiedbinary {
                    arg_role_count_resolver -native binary::scan::arg_role_count_resolver
                    arg_role_resolver_roles {ScanFormat VarWrite}
                }
            }",
    );
    for name in ["copiedscan", "copiedbinary"] {
        let spec = &pack
            .command(name)
            .expect("native cardinality descriptor")
            .spec;
        assert!(spec.arg_role_resolver.is_none());
        assert_eq!(
            spec.arg_role_count_resolver
                .expect("count-only native hook")(4),
            vec![
                (1, ArgRole::ScanFormat),
                (2, ArgRole::VarWrite),
                (3, ArgRole::VarWrite)
            ]
        );
    }
}

#[test]
fn native_structured_layout_loads_and_conflicting_inputs_are_rejected() {
    let pack = evaluate_pack(
        r"speclib layouts 1.0 {
            command copied {
                arg_role_layout_resolver -native regsub::arg_role_layout_resolver
                arg_role_resolver_roles {Pattern FormatString VarWrite}
            }
            command unsupported {
                arg_role_layout_resolver {words options} {return {0 Pattern}}
                arg_role_resolver_roles Pattern
            }
            command conflict {
                arg_role_layout_resolver -native regsub::arg_role_layout_resolver
                arg_role_count_resolver -native foreach::arg_role_count_resolver
                arg_role_resolver_roles {Pattern FormatString VarWrite Body}
            }
        }",
    );
    assert!(
        pack.command("copied")
            .unwrap()
            .spec
            .arg_role_layout_resolver
            .is_some()
    );
    assert!(pack.command("unsupported").is_none());
    assert!(pack.command("conflict").is_none());
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("Tcl layout hooks are unsupported"))
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("mutually exclusive"))
    );
}

#[test]
fn dialect_shorthands_close_over_the_right_versions() {
    let mut log = Log::default();
    assert_eq!(
        parse_dialects("tcl8.6+", 1, &mut log),
        Some(SpecSurface::TCL86_PLUS)
    );
    assert_eq!(
        parse_dialects("all-tcl f5-irules", 1, &mut log),
        Some(SpecSurface::ALL_TCL_AND_IRULES)
    );
    assert_eq!(
        parse_dialects("tcl8.x", 1, &mut log),
        Some(SpecSurface::TCL8X)
    );
    assert!(log.notices.is_empty());
}

/// Jim's rows: a bare `jim` is every Jim release, `jimX.Y+` the row
/// open-ended at that release, as `regsub -command` declares it beside
/// Tcl 9.0 and later.
#[test]
fn jim_dialect_words_name_every_release_or_one_and_later() {
    use tcl_dialect::model::Family;
    const FROM_0_84: &[SpecSurface] = &[SpecSurface::core_in(Family::Jim, &[("0.84", None)])];
    let mut log = Log::default();
    assert_eq!(parse_dialects("jim", 1, &mut log), Some(SpecSurface::JIM));
    assert_eq!(parse_dialects("jim0.84+", 1, &mut log), Some(FROM_0_84));
    let regsub_command = parse_dialects("tcl9.0+ jim0.84+", 1, &mut log).expect("two rows");
    assert_eq!(
        regsub_command,
        [SpecSurface::TCL90_PLUS, FROM_0_84].concat().as_slice()
    );
    assert!(log.notices.is_empty(), "{:?}", log.notices);
    for unknown in ["jim+", "jim0.+", "jimx+", "jim0.84"] {
        let mut log = Log::default();
        assert_eq!(parse_dialects(unknown, 1, &mut log), None, "{unknown}");
        assert_eq!(log.notices.len(), 1, "{unknown}");
    }
}

#[test]
fn an_argument_index_above_the_table_width_drops() {
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command demo { arg 300 -role Body\n arg 2 -role Body }\n\
             }",
    );
    let demo = pack.command("demo").expect("demo loads");
    assert_eq!(demo.spec.arg_roles, &[(2, ArgRole::Body)]);
    assert!(pack.notices.iter().any(|n| n.message.contains("above the")));
}

#[test]
fn resolver_role_capabilities_load_at_both_registry_scopes() {
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n\
             command demo {\n\
               arg_role_resolver {words ctx} { return }\n\
               arg_role_resolver_roles {Expr Body Keyword Body}\n\
               subcommand run {\n\
                 arg_role_resolver {words ctx} { return }\n\
                 arg_role_resolver_roles {VarWrite Body}\n\
               }\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(
        spec.arg_role_resolver_roles,
        &[ArgRole::Expr, ArgRole::Body, ArgRole::Keyword]
    );
    assert_eq!(
        spec.subcommands[0].arg_role_resolver_roles,
        &[ArgRole::VarWrite, ArgRole::Body]
    );
}

#[test]
fn resolver_role_capabilities_fail_closed_for_assistance() {
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n\
             command missing {\n\
               arg_role_resolver {words ctx} { return }\n\
             }\n\
             command empty {\n\
               arg_role_resolver {words ctx} { return }\n\
               arg_role_resolver_roles {}\n\
             }\n\
             command orphan {\n\
               arg_role_resolver_roles Body\n\
             }\n\
             command nested {\n\
               subcommand bad {\n\
                 arg_role_resolver {words ctx} { return }\n\
                 arg_role_resolver_roles {Body Invented}\n\
               }\n\
             }\n\
             }",
    );

    for name in ["missing", "empty", "orphan", "nested"] {
        assert!(
            pack.command(name)
                .expect("the safe degraded spec loads")
                .degraded,
            "{name} must not advertise complete assistance facts"
        );
    }
    assert!(
        pack.command("nested")
            .expect("nested loads")
            .spec
            .subcommands[0]
            .arg_role_resolver_roles
            .is_empty(),
        "one invalid member rejects the incomplete capability set"
    );
    assert!(
        pack.notices
            .iter()
            .all(|notice| notice.class == VocabularyClass::Assistance),
        "{:#?}",
        pack.notices
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("without a non-empty")),
        "{:#?}",
        pack.notices
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("without an `arg_role_resolver`")),
        "{:#?}",
        pack.notices
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("`Invented`")),
        "{:#?}",
        pack.notices
    );
}

#[test]
fn resolver_role_capabilities_require_the_2_1_declaration() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n\
             command demo {\n\
               arg_role_resolver {words ctx} { return }\n\
               arg_role_resolver_roles Body\n\
             }\n\
             }",
    );
    assert!(pack.command("demo").is_some());
    assert!(pack.notices.iter().any(|notice| {
        notice
            .message
            .contains("`arg_role_resolver_roles` is SpecTcl 2.1 vocabulary")
    }));
}

/// The frozen spelling, whole: `object_class NAME ?-superclass {…}?
/// ?-allow-unknown? ?-method-prefix-matching Enabled|Strict?
/// { method … }`, with `method` rows reusing the
/// `subcommand` body grammar.
#[test]
fn an_object_class_carries_its_name_flags_and_method_table() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n\
             command factory {\n\
               arity 1..\n\
               object_class ::probe::Widget -superclass {::probe::Base ::probe::Mixin} \
                 -allow-unknown -method-prefix-matching Enabled {\n\
                 method configure {\n\
                   arity 1..\n\
                   detail   {Reconfigure the widget.}\n\
                   synopsis {$w configure ?-opt value ...?}\n\
                   mutator\n\
                   option -text -takes value\n\
                 }\n\
                 method cget { arity 1 ; detail {Read one option.} ; pure }\n\
               }\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("factory").expect("factory loads").spec;
    let class = spec.object_class.expect("the object_class is read");
    assert_eq!(class.class_name, "::probe::Widget");
    assert_eq!(class.superclasses, &["::probe::Base", "::probe::Mixin"]);
    assert!(class.allow_unknown_methods);
    assert_eq!(class.method_prefix_matching, PrefixMatching::Enabled);

    let names: Vec<&str> = class.instance_methods.iter().map(|m| m.name).collect();
    assert_eq!(names, ["configure", "cget"]);
    let configure = class.instance_method("configure").expect("by name");
    assert_eq!(configure.detail, "Reconfigure the widget.");
    assert_eq!(configure.synopsis, "$w configure ?-opt value ...?");
    assert!(configure.mutator);
    assert_eq!(configure.options.len(), 1);
    assert_eq!(configure.options[0].name, "-text");
    assert!(class.instance_method("cget").expect("by name").pure);

    // The method table is the class's, not the command's: a factory
    // declaring no `subcommand` rows still has none.
    assert!(spec.subcommands.is_empty());
}

/// `object_class NAME` with neither flags nor a block is the documented
/// short form — a class known by name whose methods are not enumerated.
#[test]
fn an_object_class_may_be_a_bare_name() {
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command factory { object_class ::probe::Opaque }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let class = pack
        .command("factory")
        .expect("factory loads")
        .spec
        .object_class
        .expect("the object_class is read");
    assert_eq!(class.class_name, "::probe::Opaque");
    assert!(class.instance_methods.is_empty());
    assert!(class.superclasses.is_empty());
    assert!(!class.allow_unknown_methods);
    assert_eq!(class.method_prefix_matching, PrefixMatching::Strict);
}

/// A nameless class, an unknown flag, and a non-`method` member row are
/// each a degradation with a notice, never a failure.
#[test]
fn a_malformed_object_class_degrades_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command nameless { object_class }\n\
             command odd { object_class ::probe::C -mixin {::X} { subcommand s { arity 1 } } }\n\
             }",
    );
    let said = |needle: &str| {
        pack.notices
            .iter()
            .any(|notice| notice.message.contains(needle))
    };
    assert!(
        pack.command("nameless")
            .expect("still loads")
            .spec
            .object_class
            .is_none()
    );
    assert!(said("`object_class` with no class name dropped"));

    let odd = pack.command("odd").expect("still loads").spec;
    let class = odd.object_class.expect("the named class survives");
    assert_eq!(class.class_name, "::probe::C");
    assert!(class.instance_methods.is_empty());
    assert!(said("unknown flag `-mixin` on `object_class` dropped"));
    assert!(said("unknown property `subcommand` dropped"));
}

/// Every level the registry can gate reads the same three flags, so one
/// pack exercises the whole 1.1 vocabulary at once.
#[test]
fn every_gateable_row_reads_the_three_lifecycle_flags() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             values probe-codes {\n\
               value fast -detail {Quick path.} -min-tcl tcl8.6 -introduced 1.2 \
                 -deprecated 1.6 -retired 2.0\n\
             }\n\
             command demo {\n\
               introduced_version 1.0\n\
               form Default {demo ?-x? word} -introduced 1.1 -retired 2.0\n\
               side_effect FileIo -reads -introduced 1.1\n\
               option_conflict {-a -b} -introduced 1.1 -retired 2.0\n\
               arg 0 -values-from probe-codes\n\
               versioned_arg_value 0 fast -introduced 1.2 -retired 2.0\n\
               subcommand run {\n\
                 arity 1\n\
                 sub_subcommand now -detail {Right away.} -introduced 1.3 -deprecated 1.9\n\
               }\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;

    let form = spec.forms.first().expect("the form row loads");
    assert_eq!(form.lifecycle.introduced, Some("1.1"));
    assert_eq!(form.lifecycle.retired, Some("2.0"));

    let effect = spec
        .side_effects
        .lower_bound()
        .first()
        .expect("the side_effect loads");
    assert_eq!(effect.lifecycle.introduced, Some("1.1"));

    let constraint = spec
        .option_relations
        .first()
        .expect("the option_conflict loads");
    assert_eq!(constraint.lifecycle.introduced, Some("1.1"));
    assert_eq!(constraint.lifecycle.retired, Some("2.0"));

    let value = spec.arg_values[0].1[0];
    assert_eq!(value.min_tcl, Some(TclVersion::V8_6));
    assert_eq!(value.lifecycle.introduced, Some("1.2"));
    assert_eq!(value.lifecycle.deprecated, Some("1.6"));
    assert_eq!(value.lifecycle.retired, Some("2.0"));

    let gate = spec
        .versioned_arg_values
        .first()
        .expect("a command-level versioned_arg_value loads");
    assert_eq!((gate.index, gate.value), (0, "fast"));
    assert_eq!(gate.lifecycle.introduced, Some("1.2"));

    let row = spec.subcommands[0].sub_subcommands[0];
    assert_eq!(row.lifecycle.introduced, Some("1.3"));
    assert_eq!(row.lifecycle.deprecated, Some("1.9"));
}

/// The data form of the option's quick-fix hook, read by the same flag
/// reader as the command-level `deprecation_fix` statement.
#[test]
fn an_option_row_reads_a_deprecation_fix_block() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             command demo {\n\
               option -old -deprecated 1.4 -deprecation-fix {-replace -new \
                 -description {Renamed in 1.4.} -safety SemanticsEquivalent}\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let option = &pack.command("demo").expect("demo loads").spec.options[0];
    assert_eq!(option.lifecycle.deprecated, Some("1.4"));
    match option.lifecycle.deprecation_fix {
        Some(DeprecationFixHook::ReplaceMatchedWord {
            replacement,
            description,
            safety,
        }) => {
            assert_eq!(replacement, "-new");
            assert_eq!(description, "Renamed in 1.4.");
            assert_eq!(safety, DeprecationFixSafety::SemanticsEquivalent);
        }
        other => panic!("expected a matched-word replacement, got {other:?}"),
    }
}

/// An impossible ordering drops the lifecycle and keeps the entity: a pack
/// is never refused for a fact it got wrong.
#[test]
fn an_impossible_lifecycle_is_dropped_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             command demo {\n\
               option -x -introduced 2.0 -retired 1.0\n\
               form Default {demo} -deprecated 2.0 -retired 1.0\n\
             }\n\
             }",
    );
    let spec = pack.command("demo").expect("demo still loads").spec;
    assert_eq!(spec.options[0].name, "-x");
    assert!(spec.options[0].lifecycle.is_unspecified());
    assert!(spec.forms[0].lifecycle.is_unspecified());
    let said = |needle: &str| {
        pack.notices
            .iter()
            .any(|notice| notice.message.contains(needle))
    };
    assert!(
        said("option `-x`: retired release predates the introducing release; lifecycle dropped"),
        "{:?}",
        pack.notices
    );
    assert!(
        said("retired release predates the deprecating release"),
        "{:?}",
        pack.notices
    );
}

/// Containment is notice-only: the declaration stays, and the author is
/// told it claims availability its parent does not have.
#[test]
fn a_child_reaching_outside_its_parent_is_a_notice_not_a_drop() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             command demo {\n\
               introduced_version 1.5\n\
               retired_version 3.0\n\
               option -early -introduced 1.0\n\
               subcommand run { arity 1 ; retired_version 4.0 }\n\
             }\n\
             }",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.options[0].lifecycle.introduced, Some("1.0"));
    assert_eq!(spec.subcommands[0].lifecycle.retired, Some("4.0"));
    let said = |needle: &str| {
        pack.notices
            .iter()
            .any(|notice| notice.message.contains(needle))
    };
    assert!(
        said("option `-early`: introduced 1.0 reaches outside"),
        "{:?}",
        pack.notices
    );
    assert!(
        said("subcommand `run`: retired 4.0 reaches outside"),
        "{:?}",
        pack.notices
    );
}

/// `1`, `1.0`, `1.1`, `1.2`, `2.0`, `2.1` and `2.2` are all known; an
/// unknown *minor* loads with a notice rather than being refused.
#[test]
fn the_speclib_version_word_names_a_vocabulary_this_loader_knows() {
    for known in KNOWN_VOCABULARY_VERSIONS {
        let pack = evaluate_pack(&format!(
            "speclib probe {known} {{\n command demo {{ arity 1 }}\n}}"
        ));
        assert!(pack.notices.is_empty(), "{known}: {:?}", pack.notices);
        assert_eq!(&pack.dsl_version, known);
    }
    // An unknown minor within a supported major keeps loading maximally
    // (§6.1): a minor is only ever additive.
    let pack = evaluate_pack("speclib probe 2.9 {\n command demo { arity 1 }\n}");
    assert!(pack.command("demo").is_some(), "the pack still loads");
    assert_eq!(pack.dsl_version, "2.9");
    assert_eq!(pack.load_error, None);
    assert_eq!(
        pack.notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "pack declares SpecTcl vocabulary 2.9; this loader knows 2.2 — \
                 newer words may be dropped"
        ]
    );
    // A number *below* every vocabulary this loader knows never named a
    // vocabulary at all — the classic slip is writing the library's own
    // release in the slot — so the notice points at `introduced_version`
    // instead of claiming the loader is behind.
    let pack = evaluate_pack("speclib probe 0.15 {\n command demo { arity 1 }\n}");
    assert!(pack.command("demo").is_some(), "the pack still loads");
    assert_eq!(
        pack.notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "`0.15` is not a SpecTcl vocabulary version (this loader knows \
                 2.2); if it is the library's own version, it belongs in \
                 `introduced_version`, not the `speclib` slot"
        ]
    );
}

/// A 1.1-only word under a 1.0 declaration draws one notice per site:
/// this loader reads the word fine (additions never gate), but a
/// genuinely-1.0 loader drops it silently, so the declaration must say
/// 1.1. Declaring 1.1 clears every notice; the option row's lifecycle
/// flags stay 1.0 vocabulary and never trip it.
#[test]
fn v11_words_under_a_10_declaration_draw_a_per_site_notice() {
    let body = "\n command demo {\n                        arity 1\n                        option -x -introduced 1.2\n                        form Default {demo word} -introduced 1.2\n                        side_effect FileIo -writes -deprecated 2.0\n                        versioned_arg_value 0 utf-8 -introduced 1.2\n                    }\n}";

    let pack = evaluate_pack(&format!("speclib probe 1.0 {{{body}"));
    let v11_notices: Vec<&str> = pack
        .notices
        .iter()
        .filter(|n| n.message.contains("SpecTcl 1.1 vocabulary"))
        .map(|n| n.message.as_str())
        .collect();
    // form -introduced, side_effect -deprecated, and the command-scope
    // versioned_arg_value statement; the option row's -introduced is 1.0
    // vocabulary and stays silent.
    assert_eq!(v11_notices.len(), 3, "{:?}", pack.notices);
    assert!(
        v11_notices
            .iter()
            .all(|m| m.contains("declare `speclib probe 1.1`")),
        "{v11_notices:?}"
    );
    assert!(
        !v11_notices.iter().any(|m| m.contains("`-x`")),
        "option-row lifecycle is 1.0 vocabulary: {v11_notices:?}"
    );

    let pack = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
    assert!(
        pack.notices.is_empty(),
        "declaring 1.1 clears it: {:?}",
        pack.notices
    );
}

/// The all-versions contract: the same pack body loads to the identical
/// command surface under every vocabulary this loader knows — a pack is
/// never refused, and no known version reads fewer words than another.
#[test]
fn every_known_vocabulary_loads_the_same_command_surface() {
    let body = "{\n command demo {\n                        arity 1..\n                        option -mode -takes mode -values {a b} -closed\n                        form Default {demo ?-mode mode? word}\n                        hover { summary {Demo.} synopsis {demo word} }\n                    }\n command other { arity 0 }\n}";
    let baseline = evaluate_pack(&format!("speclib probe 1.2 {body}"));
    let baseline_names: Vec<&str> = baseline.commands.iter().map(|c| c.spec.name).collect();
    for known in ["1", "1.0", "1.1"] {
        let pack = evaluate_pack(&format!("speclib probe {known} {body}"));
        let names: Vec<&str> = pack.commands.iter().map(|c| c.spec.name).collect();
        assert_eq!(names, baseline_names, "{known}");
        assert!(pack.notices.is_empty(), "{known}: {:?}", pack.notices);
        let demo = pack.command("demo").expect("demo loads");
        assert_eq!(demo.spec.options.len(), 1, "{known}");
    }
}

#[test]
fn option_external_input_link_round_trips_as_taint_metadata() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command widget {\n \
             arity 1..\n \
             option -textvariable -takes variable -role VarWrite \
                 -also-role VarRead -taints-var-write -variable-scope Global\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("widget").expect("widget loads").spec;
    let option = spec
        .find_option("-textvariable", None, None)
        .expect("linked option loads");
    assert!(option.taints_var_write());
    assert_eq!(option.value_role(), Some(ArgRole::VarWrite));
    assert_eq!(option.value_also_role(), Some(ArgRole::VarRead));
    assert_eq!(option.value_variable_scope(), Some(VariableScope::Global));
}

#[test]
fn option_script_timing_is_loaded_independently_of_body_scope() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command widget {\n \
             option -command -takes script -role Body -body-kind Structural \
                 -script-timing Deferred\n \
             option -body -takes script -role Body -body-kind Structural \
                 -script-timing SameInvocation\n \
             option -remove -takes command-prefix -role CommandPrefix \
                 -script-timing ReferenceOnly\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("widget").expect("widget loads").spec;
    let callback = spec
        .find_option("-command", None, None)
        .expect("callback option loads");
    let immediate = spec
        .find_option("-body", None, None)
        .expect("immediate body option loads");
    let reference = spec
        .find_option("-remove", None, None)
        .expect("reference-only option loads");
    assert_eq!(callback.value_script_timing(), Some(ScriptTiming::Deferred));
    assert_eq!(
        immediate.value_script_timing(),
        Some(ScriptTiming::SameInvocation)
    );
    assert_eq!(
        reference.value_script_timing(),
        Some(ScriptTiming::ReferenceOnly)
    );
    let OptionValue::Takes(callback_arg) = callback.value else {
        panic!("callback takes a value")
    };
    let OptionValue::Takes(immediate_arg) = immediate.value else {
        panic!("body takes a value")
    };
    let OptionValue::Takes(reference_arg) = reference.value else {
        panic!("removal reference takes a value")
    };
    assert_eq!(callback_arg.body_kind, BodyKind::Structural);
    assert_eq!(immediate_arg.body_kind, BodyKind::Structural);
    assert_eq!(reference_arg.role, ArgRole::CommandPrefix);
}

#[test]
fn callback_taint_inputs_are_authorable_but_reject_bookkeeping_markers() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command widget {\n \
             traits DEFERS_BODY\n \
             arity 2\n \
             arg 1 -role Body\n \
             callback_taint_inputs {{1 {%A %K %W}}}\n \
             option -validatecommand -takes script -role Body -body-kind Structural \
                 -script-timing Deferred -callback-taint-inputs {%P %s %S %V}\n \
             }\n}",
    );
    let spec = &pack.command("widget").expect("widget loads").spec;
    assert_eq!(spec.callback_taint_inputs[0].0, 1);
    assert_eq!(
        spec.callback_taint_inputs[0].1,
        &[
            CallbackTaintInput::TK_EVENT_CHAR,
            CallbackTaintInput::TK_EVENT_KEYSYM,
        ]
    );
    let option = spec
        .find_option("-validatecommand", None, None)
        .expect("callback option loads");
    assert_eq!(
        option.value_callback_taint_inputs(),
        &[
            CallbackTaintInput::TK_PROPOSED_VALUE,
            CallbackTaintInput::TK_CURRENT_VALUE,
            CallbackTaintInput::TK_EDIT_TEXT,
        ]
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("%W"))
            && pack
                .notices
                .iter()
                .any(|notice| notice.message.contains("%V")),
        "metadata substitutions must be rejected: {:?}",
        pack.notices
    );
}

#[test]
fn contradictory_option_and_callback_properties_are_not_installed() {
    let pack = evaluate_pack(
        r"speclib probe 1.2 {
 command bad {
   arg 0 -role Value
   callback_taint_inputs {{0 {%A}}}
   option -timing -takes value -role Value -script-timing Deferred
   option -input -takes script -role Body -script-timing SameInvocation \
       -callback-taint-inputs {%P}
   option -scope -takes value -role Value -variable-scope Global
   option -taint -takes variable -role VarRead -taints-var-write
   subcommand child {
     arg 0 -role Body
     callback_taint_inputs {{0 {%S}}}
   }
 }
}",
    );
    let spec = &pack.command("bad").expect("command survives").spec;
    assert!(spec.callback_taint_inputs.is_empty());
    let OptionValue::Takes(timing) = spec.options[0].value else {
        panic!("option takes a value")
    };
    assert_eq!(timing.script_timing, ScriptTiming::SameInvocation);
    let OptionValue::Takes(input) = spec.options[1].value else {
        panic!("option takes a value")
    };
    assert!(input.callback_taint_inputs.is_empty());
    let OptionValue::Takes(scope) = spec.options[2].value else {
        panic!("option takes a value")
    };
    assert_eq!(scope.variable_scope, VariableScope::CurrentFrame);
    let OptionValue::Takes(taint) = spec.options[3].value else {
        panic!("option takes a value")
    };
    assert!(!taint.taints_var_write);
    assert!(spec.subcommands[0].callback_taint_inputs.is_empty());

    for fragment in [
        "non-executable value",
        "not a deferred executable",
        "non-variable value",
        "without a VarWrite role",
        "argument 0, which is not a deferred executable position",
    ] {
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains(fragment)),
            "missing `{fragment}` notice: {:?}",
            pack.notices
        );
    }
}

#[test]
fn tk_geometry_form_and_release_metadata_loads() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command layout {\n \
             tk_geometry Exclusive -container-option -inside -direct-form \
                 -placement-subcommand arrange \
                 -release-subcommands {release unmanage}\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let geometry = pack
        .command("layout")
        .expect("layout loads")
        .spec
        .tk_geometry
        .expect("geometry loads");
    assert_eq!(
        geometry.container_policy,
        tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
    );
    assert_eq!(geometry.container_option, Some("-inside"));
    assert!(geometry.direct_form);
    assert_eq!(geometry.placement_subcommand, Some("arrange"));
    assert_eq!(geometry.release_subcommands, ["release", "unmanage"]);
}

/// The 1.2 words go through the *same* per-site mechanism the 1.1 words
/// do, which is the point of generalising it: a 1.2 word is a defect
/// under a 1.1 declaration exactly as a 1.1 word is under 1.0, and the
/// notice names the version that actually has the word rather than a
/// hard-coded one.
#[test]
fn v12_words_under_an_older_declaration_draw_a_per_site_notice() {
    // Two 1.2 sites: the arity window's lifecycle flag and `tk_geometry`.
    let body = "\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0\n \
             arg 0 -role Body\n \
             tk_geometry Exclusive\n \
             option -x -introduced 1.2\n \
             }\n}";

    // Under 1.1, only the 1.2 words are noticed — the option row's
    // lifecycle flags have been 1.0 vocabulary all along.
    let pack = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
    let messages: Vec<&str> = pack.notices.iter().map(|n| n.message.as_str()).collect();
    assert_eq!(
        messages.len(),
        2,
        "one per 1.2 site, and nothing else: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|m| m.contains("is SpecTcl 1.2 vocabulary")
                && m.contains("declare `speclib probe 1.2`")),
        "{messages:?}"
    );

    // Under 1.0 the same two sites are still the only 1.2 ones, and the
    // older mechanism keeps working beside it.
    let older = evaluate_pack(&format!("speclib probe 1.0 {{{body}"));
    assert_eq!(
        older
            .notices
            .iter()
            .filter(|n| n.message.contains("SpecTcl 1.2 vocabulary"))
            .count(),
        2,
        "{:?}",
        older.notices
    );

    // Declaring 1.2 clears them.
    let declared = evaluate_pack(&format!("speclib probe 1.2 {{{body}"));
    assert!(
        declared.notices.is_empty(),
        "declaring 1.2 clears it: {:?}",
        declared.notices
    );
}

#[test]
fn callback_and_timing_vocabulary_reports_every_pre_12_site() {
    let body = r"
 command demo {
   traits DEFERS_BODY
   arity 2
   arg 1 -role Body
   callback_taint_inputs {{1 {%A %K}}}
   script_timing_resolver {words ctx} { timing 1 Deferred }
   option -validatecommand -takes script -role Body -script-timing Deferred \
       -callback-taint-inputs {%P %s %S}
   subcommand child {
     traits DEFERS_BODY
     arity 2
     arg 1 -role Body
     callback_taint_inputs {{1 {%A}}}
     script_timing_resolver {words ctx} { timing 1 Deferred }
   }
 }
}";

    let older = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
    let vocabulary: Vec<&str> = older
        .notices
        .iter()
        .filter(|notice| notice.message.contains("SpecTcl 1.2 vocabulary"))
        .map(|notice| notice.message.as_str())
        .collect();
    for (word, expected) in [
        ("`callback_taint_inputs`", 2),
        ("`script_timing_resolver`", 2),
        ("`-script-timing`", 1),
        ("`-callback-taint-inputs`", 1),
    ] {
        assert_eq!(
            vocabulary
                .iter()
                .filter(|message| message.contains(word))
                .count(),
            expected,
            "missing per-site notice for {word}: {:?}",
            older.notices
        );
    }
    assert_eq!(
        vocabulary.len(),
        6,
        "only the six 1.2 sites should be reported: {:?}",
        older.notices
    );

    let declared = evaluate_pack(&format!("speclib probe 1.2 {{{body}"));
    assert!(
        declared
            .notices
            .iter()
            .all(|notice| !notice.message.contains("SpecTcl 1.2 vocabulary")),
        "declaring 1.2 clears every vocabulary notice: {:?}",
        declared.notices
    );
}

/// An `arity` row without lifecycle flags is the command's plain arity,
/// exactly as before 1.2; one *with* them is a window, and the two
/// coexist — the plain row stays the fallback for a floor no window
/// covers.
///
/// Consecutive windows are written **closed**: a window with no
/// `-retired` never ends, so "two arguments from 3.0, three from 5.0" is
/// spelled with the first retiring where the second begins. Leaving it
/// open would overlap, which is what
/// `overlapping_arity_windows_draw_a_notice_and_the_later_one_is_dropped`
/// covers.
#[test]
fn a_gated_arity_row_becomes_a_window_and_leaves_the_plain_arity_alone() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0 -retired 5.0\n \
             arity 3 -introduced 5.0\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.arity, Arity::exact(1), "the ungated row is the arity");
    assert_eq!(spec.arity_windows.len(), 2, "{:?}", spec.arity_windows);
    assert_eq!(spec.arity_windows[0].arity, Arity::exact(2));
    assert_eq!(spec.arity_windows[0].lifecycle.introduced, Some("3.0"));
    assert_eq!(spec.arity_windows[1].arity, Arity::exact(3));

    // The selection contract the windows exist for.
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("3.0")).map(|w| w.arity),
        Some(Arity::exact(2))
    );
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("5.0")).map(|w| w.arity),
        Some(Arity::exact(3))
    );
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("1.0")),
        None,
        "below every window, the plain arity is the fallback"
    );
}

#[test]
fn positional_arity_preserves_scope_and_release_selection() {
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n command demo {\n \
             arity 0..1 -positionals\n \
             arity 0..2 -positionals -introduced 3.0\n \
             option -safe\n \
             subcommand child { arity 1.. -positionals -step 2 -also 0 }\n \
             refine Alternate { arity 2 -positionals }\n \
             }\n command ordinary { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").unwrap().spec;
    assert_eq!(spec.arity, Arity::new(0, 1).with_positionals());
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("3.0")).map(|row| row.arity),
        Some(Arity::new(0, 2).with_positionals())
    );
    assert_eq!(
        spec.subcommand("child").unwrap().arity,
        Arity::stepped(1, Arity::UNLIMITED, 2)
            .with_also_exact(0)
            .with_positionals()
    );
    assert_eq!(
        spec.command_forms[0].arity,
        Arity::exact(2).with_positionals()
    );
    assert_eq!(
        pack.command("ordinary").unwrap().spec.arity,
        Arity::exact(1)
    );
    let older = evaluate_pack("speclib probe 2.0 { command demo { arity 0..1 -positionals } }");
    assert_eq!(older.command("demo").unwrap().spec.arity, spec.arity);
    assert!(older.notices.iter().any(|notice| {
        notice.message.contains("-positionals") && notice.message.contains("2.1")
    }));
}

/// Two windows covering one release would make the selected signature
/// depend on declaration order. The pack keeps the first and is told.
#[test]
fn overlapping_arity_windows_draw_a_notice_and_the_later_one_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0\n \
             arity 9 -introduced 4.0\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.arity_windows.len(), 1, "{:?}", spec.arity_windows);
    assert_eq!(spec.arity_windows[0].arity, Arity::exact(2), "the first");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("overlaps") && n.message.contains("dropped")),
        "{:?}",
        pack.notices
    );
}

/// Windows and gated argument rows join the containment pass every other
/// lifecycle already goes through: a row that outlives the command
/// declaring it is a pack defect the author can only find if told.
#[test]
fn a_window_reaching_outside_the_command_lifecycle_is_noticed() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             introduced_version 2.0\n \
             retired_version 4.0\n \
             arity 1\n \
             arity 2 -introduced 5.0\n \
             }\n}",
    );
    let said = |needle: &str| pack.notices.iter().any(|n| n.message.contains(needle));
    assert!(said("arity window"), "{:?}", pack.notices);
}

/// A stamp and its three releases, as the tests read a window.
type StampRow<T> = (
    T,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
);

fn stamp_rows<T: Copy>(windows: &[StampWindow<T>]) -> Vec<StampRow<T>> {
    windows
        .iter()
        .map(|window| {
            (
                window.value,
                window.lifecycle.introduced,
                window.lifecycle.deprecated,
                window.lifecycle.retired,
            )
        })
        .collect()
}

/// A stamp statement that ends in lifecycle flags is one window of the
/// stamp, and the plain statement beside it stays the stamp for every
/// release no window covers (`SpecTcl` 2.2). The same at command and
/// subcommand scope.
#[test]
fn stamp_windows_load_beside_the_plain_stamp() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign\n \
             codegen_hook -native Llength -introduced 9.0\n \
             inline_codegen_hook -native Expr -retired 9.0\n \
             semantic_operation {Intrinsic StringLength} -introduced 9.0 -deprecated 9.1\n \
             subcommand get {\n \
             codegen_hook -native Dict -introduced 9.0\n \
             inline_codegen_hook -native Expr -introduced 8.6 -retired 9.0\n \
             semantic_operation {Intrinsic Concat} -introduced 9.0\n \
             }\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.codegen_hook, Some(CodegenHookId::Lassign));
    assert_eq!(
        stamp_rows(spec.codegen_hook_windows),
        [(CodegenHookId::Llength, Some("9.0"), None, None)]
    );
    assert_eq!(spec.inline_codegen_hook, None);
    assert_eq!(
        stamp_rows(spec.inline_codegen_hook_windows),
        [(InlineCodegenHookId::Expr, None, None, Some("9.0"))]
    );
    assert_eq!(spec.semantic_operation, None);
    assert_eq!(
        stamp_rows(spec.semantic_operation_windows),
        [(
            SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
            Some("9.0"),
            Some("9.1"),
            None
        )]
    );
    let sub = &spec.subcommands[0];
    assert_eq!(sub.codegen_hook, None);
    assert_eq!(
        stamp_rows(sub.codegen_hook_windows),
        [(CodegenHookId::Dict, Some("9.0"), None, None)]
    );
    assert_eq!(
        stamp_rows(sub.inline_codegen_hook_windows),
        [(InlineCodegenHookId::Expr, Some("8.6"), None, Some("9.0"))]
    );
    assert_eq!(
        stamp_rows(sub.semantic_operation_windows),
        [(
            SemanticOperationId::Intrinsic(IntrinsicId::Concat),
            Some("9.0"),
            None,
            None
        )]
    );
}

/// Two windows covering one release make the stamp depend on declaration
/// order. The pack keeps the first and is told.
#[test]
fn overlapping_stamp_windows_draw_a_notice_and_the_later_one_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign -introduced 8.6\n \
             codegen_hook -native Llength -introduced 9.0\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(
        stamp_rows(spec.codegen_hook_windows),
        [(CodegenHookId::Lassign, Some("8.6"), None, None)],
        "the first"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("codegen_hook window")
                && n.message.contains("overlaps")
                && n.message.contains("dropped")),
        "{:?}",
        pack.notices
    );
}

/// A rejected lifecycle comes back UNSPECIFIED, which as a window would
/// cover every release. An arity window degrades to the plain arity, which
/// only gates less; a stamp widened to every release would specialise where
/// its author never meant it to, so the row is dropped.
#[test]
fn a_stamp_window_with_an_impossible_lifecycle_is_dropped_and_not_widened() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign -introduced 9.0 -retired 8.6\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.codegen_hook, None, "not the plain stamp");
    assert!(spec.codegen_hook_windows.is_empty());
    let said = |needle: &str| pack.notices.iter().any(|n| n.message.contains(needle));
    assert!(said("codegen hook window"), "{:?}", pack.notices);
    assert!(said("dropped"), "{:?}", pack.notices);
}

/// A form is already a shape the call's arguments pick out and has no window
/// list, so a gated stamp on one is dropped with a notice — never read as an
/// ungated stamp that applies at every release.
#[test]
fn a_stamp_on_a_form_takes_no_window() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n arity 0..\n \
             refine pair {\n arity 2\n codegen_hook -native Lassign -introduced 9.0\n }\n \
             refine whole {\n arity 1\n codegen_hook -native Llength\n }\n }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    let hook = |name: &str| {
        spec.command_forms
            .iter()
            .find(|form| form.name == name)
            .expect("the form loads")
            .codegen_hook
    };
    assert_eq!(hook("pair"), None, "the gated stamp is not applied");
    assert_eq!(
        hook("whole"),
        Some(CodegenHookId::Llength),
        "an ungated one is"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("takes no lifecycle window")),
        "{:?}",
        pack.notices
    );
}

/// Anything after a stamp's id that is not a lifecycle flag is a notice, not
/// silence: a mistyped flag would otherwise leave a window ungated.
#[test]
fn an_unknown_flag_on_a_stamp_statement_is_noticed() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign -introdced 9.0\n \
             }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("unknown flag") && n.message.contains("codegen_hook")),
        "{:?}",
        pack.notices
    );
}

/// The window flags are 2.2 vocabulary, noticed per site under an older
/// declaration as every newer word is.
#[test]
fn stamp_window_flags_under_an_older_declaration_draw_a_per_site_notice() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n \
             codegen_hook -native Lassign -introduced 9.0\n \
             }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("is SpecTcl 2.2 vocabulary")
                && n.message.contains("declare `speclib probe 2.2`")),
        "{:?}",
        pack.notices
    );
}

/// An impossibly-ordered window lifecycle comes back UNSPECIFIED, which
/// as a window would cover every release and shadow the well-formed ones.
/// The row keeps its shape and loses only the gate — the same degradation
/// every other rejected lifecycle takes.
#[test]
fn an_arity_window_with_an_impossible_lifecycle_falls_back_to_the_plain_arity() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             arity 7 -introduced 5.0 -retired 2.0\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert!(
        spec.arity_windows.is_empty(),
        "a rejected gate must not become an always-on window: {:?}",
        spec.arity_windows
    );
    assert_eq!(spec.arity, Arity::exact(7), "the shape survives");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("arity window")),
        "{:?}",
        pack.notices
    );
}

/// `special_var NAME -kind K -access A -origin O ?-dialects {…}?
/// ?-startup B?` builds a special-variable row as the shipped table
/// states one: the startup binding gates `initially_bound` (or, for a read
/// trace, `lazily_readable`) on the row's own dialects, and a row missing
/// one of its three descriptors is dropped rather than guessed.
#[test]
fn special_var_rows_build_registry_rows_and_an_incomplete_one_is_dropped() {
    use tcl_registry::{SpecialVarKind, StartupBinding, VarAccess, VarOrigin};
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n \
             special_var sim_home -kind Scalar -access ReadOnly -origin Dialect -startup Interpreter\n \
             special_var sim_opts -kind Array -access ReadWrite -origin Environment -dialects {tcl8.6}\n \
             special_var sim_lazy -kind Scalar -access ReadOnly -origin Interpreter -startup ReadTrace\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let [home, opts, lazy] = pack.special_vars.as_slice() else {
        panic!("three rows: {:?}", pack.special_vars);
    };
    assert_eq!(home.spec.name, "sim_home");
    assert_eq!(home.spec.kind, SpecialVarKind::Scalar);
    assert_eq!(home.spec.access, VarAccess::ReadOnly);
    assert_eq!(home.spec.origin, VarOrigin::Dialect);
    assert_eq!(home.spec.startup_binding, StartupBinding::Interpreter);
    let tcl86 = Some(tcl_dialect::model::SurfaceQuery::core(
        tcl_dialect::model::Family::Tcl,
        "8.6",
    ));
    let tcl90 = Some(tcl_dialect::model::SurfaceQuery::core(
        tcl_dialect::model::Family::Tcl,
        "9.0",
    ));
    assert!(home.spec.readable_at_startup_in(tcl86));
    assert!(home.spec.readable_at_startup_in(tcl90));
    // No `-startup`: recognised, never readable before user code; and its
    // `-dialects` gate its existence.
    assert_eq!(opts.spec.kind, SpecialVarKind::Array);
    assert!(!opts.spec.readable_at_startup_in(tcl86));
    assert!(opts.spec.available_in(tcl86));
    assert!(!opts.spec.available_in(tcl90));
    // A read trace is readable at startup, but not bound.
    assert!(lazy.spec.readable_at_startup_in(tcl90));
    assert!(lazy.spec.initially_bound.is_empty());

    let incomplete = evaluate_pack(
        "speclib probe 2.1 {\n \
             special_var sim_home -kind Scalar -origin Dialect\n \
             special_var sim_bad -kind Matrix -access ReadOnly -origin Dialect\n \
             special_var sim_nowhere -kind Scalar -access ReadOnly -origin Dialect -dialects {nosuch}\n \
             command demo { arity 1 }\n}",
    );
    assert!(
        incomplete.special_vars.is_empty(),
        "a row missing a descriptor, or naming dialects this build cannot read, is \
             dropped rather than widened: {:?}",
        incomplete.special_vars
    );
    assert!(
        incomplete
            .notices
            .iter()
            .any(|n| n.message.contains("needs `-kind`, `-access` and `-origin`")),
        "{:?}",
        incomplete.notices
    );
    assert!(
        incomplete
            .notices
            .iter()
            .any(|n| n.message.contains("-kind Matrix")),
        "{:?}",
        incomplete.notices
    );
}

/// `ambient_package NAME VERSION` names a package the pack's dialect
/// provides without a `package require`. Both words are required: a row
/// with no version would floor at nothing, which is the situation the row
/// exists to end.
#[test]
fn ambient_package_rows_load_and_an_incomplete_one_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n \
             ambient_package Tk 8.6\n \
             ambient_package Itcl 4.0\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let named: Vec<(&str, &str)> = pack
        .ambient_packages
        .iter()
        .map(|row| (row.name, row.version))
        .collect();
    assert_eq!(named, vec![("Tk", "8.6"), ("Itcl", "4.0")]);

    let missing = evaluate_pack(
        "speclib probe 1.2 {\n \
             ambient_package Tk\n \
             command demo { arity 1 }\n}",
    );
    assert!(
        missing.ambient_packages.is_empty(),
        "a version-less row is dropped: {:?}",
        missing.ambient_packages
    );
    assert!(
        missing
            .notices
            .iter()
            .any(|n| n.message.contains("needs the version the runtime provides")),
        "{:?}",
        missing.notices
    );

    // 1.2 vocabulary: the same per-site notice every other new word gets.
    let older = evaluate_pack(
        "speclib probe 1.1 {\n \
             ambient_package Tk 8.6\n \
             command demo { arity 1 }\n}",
    );
    assert_eq!(
        older.ambient_packages.len(),
        1,
        "the word still loads — additions never gate"
    );
    assert!(
        older.notices.iter().any(|n| n
            .message
            .contains("`ambient_package` is SpecTcl 1.2 vocabulary")),
        "{:?}",
        older.notices
    );
}

/// The version of an `ambient_package` row is a floor, so a word that is not a
/// package version is no floor: the row is dropped with a notice, and the rows
/// beside it stand.
#[test]
fn an_ambient_package_row_whose_version_is_not_a_version_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n \
             ambient_package Tk junk\n \
             ambient_package Itcl 4.0.x\n \
             ambient_package Tcllib 1.21\n \
             ambient_package Thread 2.8a1\n \
             command demo { arity 1 }\n}",
    );
    let named: Vec<(&str, &str)> = pack
        .ambient_packages
        .iter()
        .map(|row| (row.name, row.version))
        .collect();
    assert_eq!(named, vec![("Tcllib", "1.21"), ("Thread", "2.8a1")]);
    for word in ["junk", "4.0.x"] {
        let said = format!("names `{word}`, which is not a package version");
        assert!(
            pack.notices.iter().any(|n| n.message.contains(&said)),
            "{word}: {:?}",
            pack.notices
        );
    }
}

/// `ambient_package NAME VERSION -dialects {…}`'s scoping flag is
/// refused, and — the point of the test — it takes the row with it.
///
/// Dropping the flag alone would leave the claim *unscoped*, flooring
/// the package in every dialect including the ones the pack just said
/// it is absent from. That is §6.1's fail-closed rule read backwards,
/// so the row fails closed instead, and the notice names the 2.0
/// spelling that says the same thing properly.
#[test]
fn an_ambient_package_row_scoped_with_dialects_fails_closed() {
    for declared in ["1.2", "2.0"] {
        let pack = evaluate_pack(&format!(
            "speclib probe {declared} {{\n \
                 ambient_package Tk 8.6 -dialects {{tcl8.6}}\n \
                 ambient_package Itcl 4.0\n \
                 command demo {{ arity 1 }}\n}}"
        ));
        let named: Vec<&str> = pack.ambient_packages.iter().map(|row| row.name).collect();
        assert_eq!(
            named,
            vec!["Itcl"],
            "the scoped row is dropped whole, the unscoped one is untouched ({declared})"
        );
        let scoping = pack
            .notices
            .iter()
            .find(|n| n.message.contains("-dialects"))
            .unwrap_or_else(|| panic!("a notice names the flag ({declared}): {:?}", pack.notices));
        assert_eq!(
            scoping.class,
            VocabularyClass::Semantic,
            "an availability-narrowing word is semantic ({declared})"
        );
        assert!(
            scoping
                .message
                .contains("environment NAME { ambient Tk 8.6 }"),
            "the notice names the 2.0 spelling ({declared}): {}",
            scoping.message
        );
    }
}

/// `display_name` and `file_extension` are pack-level metadata: names
/// for humans, extensions for editors, and `-dialect` routing that must
/// resolve to a canonical profile — a typo keeps the row and drops only
/// the routing, with a notice.
#[test]
fn display_name_and_file_extension_rows_load() {
    let pack = evaluate_pack(
        "speclib upfdemo 1.1 {\n             display_name {IEEE 1801 UPF}\n             file_extension .UPF -name {Unified Power Format} -dialect synopsys-eda-tcl\n             file_extension pwr -dialect no-such-dialect\n             file_extension upf -name {duplicate}\n             command demo { arity 1 }\n}",
    );
    assert_eq!(pack.display_name.as_deref(), Some("IEEE 1801 UPF"));
    assert_eq!(pack.file_extensions.len(), 2, "{:?}", pack.file_extensions);
    let upf = &pack.file_extensions[0];
    assert_eq!(upf.extension, "upf", "lower-cased, dot stripped");
    assert_eq!(upf.display_name.as_deref(), Some("Unified Power Format"));
    assert_eq!(upf.dialect, Some("synopsys-eda-tcl"));
    let pwr = &pack.file_extensions[1];
    assert_eq!(pwr.extension, "pwr");
    assert_eq!(pwr.dialect, None, "typo'd dialect drops only the routing");
    let messages: Vec<&str> = pack.notices.iter().map(|n| n.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`-dialect no-such-dialect` names no dialect profile")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`file_extension upf` redeclared")),
        "{messages:?}"
    );
}
/// The 2.0 round trip: the same body spelled `dialects` and spelled
/// `available` loads to byte-equal specs.
///
/// This is the whole of §6.1's "a new word plus a translation of the
/// legacy word": if the two spellings could ever diverge, `dialects`
/// would be a second representation rather than a translation, and the
/// upgrade tool's U9 byte-identical registry snapshot could not hold.
#[test]
fn available_and_dialects_load_byte_equal_specs() {
    // (legacy dialect word, 2.0 available row) — the whole U2 table
    // this build can express on the 1.x fields.
    let pairs = [
        ("tcl8.5", "{tcl 8.5}"),
        ("tcl8.6+", "{tcl 8.6-}"),
        ("tcl8.4+", "{tcl 8.4-}"),
        ("all-tcl", "{tcl 8.4-}"),
        ("tcl8.x", "{tcl 8.4-9.0}"),
        ("f5-irules", "{f5-irules}"),
        ("tk", "{package Tk}"),
        ("{tcl8.6 tcl9.0}", "{tcl 8.6} {tcl 9.0}"),
        ("{tcl8.6+ f5-irules}", "{tcl 8.6-} {f5-irules}"),
    ];
    for (legacy, modern) in pairs {
        let old = evaluate_pack(&format!(
            "speclib probe 1.2 {{\n command demo {{\n arity 1\n dialects {legacy}\n \
                 subcommand sub {{\n arity 0\n dialects {legacy}\n }}\n \
                 option -x -dialects {legacy}\n }}\n}}"
        ));
        let new = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n command demo {{\n arity 1\n available {modern}\n \
                 subcommand sub {{\n arity 0\n available {modern}\n }}\n \
                 option -x -available {{{modern}}}\n }}\n}}"
        ));
        assert!(old.notices.is_empty(), "{legacy}: {:?}", old.notices);
        assert!(new.notices.is_empty(), "{modern}: {:?}", new.notices);
        let old_spec = old.command("demo").expect("legacy demo loads").spec;
        let new_spec = new.command("demo").expect("2.0 demo loads").spec;
        assert_eq!(old_spec.surface, new_spec.surface, "{legacy} vs {modern}");
        assert_eq!(
            old_spec.subcommands[0].surface, new_spec.subcommands[0].surface,
            "{legacy} vs {modern}"
        );
        assert_eq!(
            old_spec.options[0].surface, new_spec.options[0].surface,
            "{legacy} vs {modern}"
        );
        assert_eq!(
            format!("{old_spec:?}"),
            format!("{new_spec:?}"),
            "{legacy} vs {modern}"
        );
    }
}

/// `available` is accepted at every scope the legacy word is, and the
/// projection is identical at each.
#[test]
fn available_lands_at_every_dialects_scope() {
    let legacy = evaluate_pack(
        "speclib probe 1.2 {\n default dialects tcl8.6+\n \
             command demo {\n arity 1..\n \
             option -x -dialects tcl8.6+\n \
             form Default {demo word} -dialects tcl8.6+\n \
             side_effect FileIo -writes -dialects tcl8.6+\n \
             option_conflict {-x -y} -dialects tcl8.6+\n \
             subcommand sub {\n arity 0\n dialects tcl8.6+\n \
             sub_subcommand deep -dialects tcl8.6+\n }\n }\n}",
    );
    let modern = evaluate_pack(
        "speclib probe 2.0 {\n default available {tcl 8.6-}\n \
             command demo {\n arity 1..\n \
             option -x -available {tcl 8.6-}\n \
             form Default {demo word} -available {tcl 8.6-}\n \
             side_effect FileIo -writes -available {tcl 8.6-}\n \
             option_conflict {-x -y} -available {tcl 8.6-}\n \
             subcommand sub {\n arity 0\n available {tcl 8.6-}\n \
             sub_subcommand deep -available {tcl 8.6-}\n }\n }\n}",
    );
    assert!(legacy.notices.is_empty(), "{:?}", legacy.notices);
    assert!(modern.notices.is_empty(), "{:?}", modern.notices);
    let legacy_spec = legacy.command("demo").expect("legacy demo loads").spec;
    let modern_spec = modern.command("demo").expect("2.0 demo loads").spec;
    assert_eq!(
        format!("{legacy_spec:?}"),
        format!("{modern_spec:?}"),
        "every scope projects alike"
    );
    assert_eq!(modern_spec.surface, Some(SpecSurface::TCL86_PLUS));
}

/// `available {package NAME}` fills `required_package` at command
/// scope, and must agree with a `required_package` the spec declares.
#[test]
fn available_carries_a_package_requirement_at_command_scope() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {package Tcllib}\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(
        pack.command("demo")
            .expect("demo loads")
            .spec
            .required_package,
        Some("Tcllib")
    );

    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             required_package Snit\n available {package Tcllib}\n }\n}",
    );
    assert_eq!(
        pack.command("demo")
            .expect("demo loads")
            .spec
            .required_package,
        Some("Snit"),
        "the declared requirement wins"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("disagrees with `required_package Snit`")),
        "{:?}",
        pack.notices
    );
}

/// A package-only `available` row gates, it does not narrow.
///
/// `available {package Foo}` is 2.0's spelling of 1.x's bare
/// `required_package Foo`, which never said *where* a command lives. If
/// the row counted towards the surface it would leave an empty
/// disjunction — a command available nowhere, with nothing logged — so
/// the surface stays open and the requirement alone gates it.
#[test]
fn a_package_only_available_row_leaves_the_surface_open() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n ambient_package Foo 1.0\n \
             command demo {\n arity 0\n available {package Foo}\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.surface, None, "the row states no provider surface");
    assert_eq!(spec.required_package, Some("Foo"));

    let context = tcl_registry::model::ingress::static_document_context_for("tcl9.0");
    assert!(
        context.spec_available(spec),
        "an open world admits a command gated on an installable package"
    );

    // A core row alongside it still narrows, and the package still gates.
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 0\n \
             available {tcl 8.6-} {package Foo}\n }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert!(spec.surface.is_some(), "the core row is the surface");
    assert_eq!(spec.required_package, Some("Foo"));
    assert!(
        !context_for("tcl8.4").spec_available(spec),
        "8.4 is below the declared floor"
    );
}

fn context_for(environment: &str) -> &'static tcl_registry::model::ResolvedContext {
    tcl_registry::model::ingress::static_document_context_for(environment)
}

/// Q3: `f5-bigip` leaves the Tcl axis, so it is never an `available`
/// provider — and an unknown provider is reported, not guessed at.
#[test]
fn available_refuses_f5_bigip_and_unknown_providers() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {f5-bigip}\n }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("off the Tcl axis")),
        "{:?}",
        pack.notices
    );
    assert_eq!(
        pack.command("demo").expect("demo still loads").spec.surface,
        None,
        "a dropped row narrows nothing"
    );

    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {klingon 1.0}\n }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("unknown `available` provider `klingon`")),
        "{:?}",
        pack.notices
    );
}

/// `refine` states one invocation form (Q12/D2): the words are the
/// owning scope's own, and every overlay it omits inherits.
#[test]
fn a_refine_block_states_one_invocation_form() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 0..2\n \
             traits {PURE MUTATES_ARGUMENT}\n \
             refine query {\n arity 0\n traits {PURE}\n mutator no\n \
             side_effects none\n }\n \
             refine set {\n arity 2\n selector {put}\n mutator yes\n \
             side_effect InterpState -writes\n }\n }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    let [query, set] = spec.command_forms else {
        panic!("two forms, got {:?}", spec.command_forms);
    };
    assert_eq!(query.name, "query");
    assert_eq!(query.arity, Arity::exact(0));
    assert_eq!(query.traits, Some(Traits::PURE));
    assert_eq!(query.mutator, Some(false));
    assert_eq!(
        query.side_effects,
        Some(Knowledge::Partial(&[] as &[SideEffect]))
    );
    assert_eq!(set.name, "set");
    assert_eq!(set.arity, Arity::exact(2));
    // An unmentioned overlay is inherited, not blanked.
    assert_eq!(set.traits, None);
    assert_eq!(set.mutator, Some(true));
    let selector = set.literal_argument_prefix.expect("a selector");
    assert_eq!(selector.words, ["put"]);
    assert_eq!(selector.prefix_matching, PrefixMatching::Enabled);
    let [effect] = set.side_effects.expect("declared effects").lower_bound() else {
        panic!("one effect");
    };
    assert_eq!(effect.target, SideEffectTarget::InterpState);
    assert!(effect.writes);
}

/// A subcommand refines its own forms, through the same reader — which is
/// the shape Tk's widget methods need: one query arity and one mutation
/// arity under a single method word.
#[test]
fn a_subcommand_refines_its_own_forms() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command widget {\n arity 1..\n \
             subcommand cget {\n arity 0..1\n \
             refine query {\n arity 0\n traits {PURE}\n mutator no\n }\n \
             refine set {\n arity 1\n mutator yes\n }\n }\n }\n}",
    );
    let spec = pack.command("widget").expect("widget loads").spec;
    let sub = spec.subcommand("cget").expect("the cget subcommand");
    assert_eq!(
        sub.subcommand_forms
            .iter()
            .map(|form| form.name)
            .collect::<Vec<_>>(),
        ["query", "set"]
    );
    assert_eq!(sub.subcommand_forms[1].mutator, Some(true));
}

/// A `refine` body takes only the fields a form carries: a fact that
/// belongs on the parent is an unknown property here, not a silent loss.
#[test]
fn a_refine_block_refuses_a_fact_the_form_cannot_hold() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             refine only {\n arity 1\n return_type List\n }\n }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("return_type")),
        "{:?}",
        pack.notices
    );
}

/// A Jim-only window names the Jim family directly (Q13): the retired
/// bitmask has no Jim member, so relying on it alone would gate the
/// command off entirely rather than say what the row meant.
#[test]
fn a_jim_only_window_names_the_jim_family() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {jim 0.78-}\n }\n}",
    );
    let rows = pack
        .command("demo")
        .expect("demo loads")
        .spec
        .surface
        .expect("the row list is stated");
    assert!(
        rows.iter()
            .all(|row| row.provider == SpecProvider::Core(Family::Jim)),
        "{rows:?}"
    );
    assert!(
        surface_admits(rows, Some(&SurfaceQuery::core(Family::Jim, "0.82"))),
        "{rows:?}"
    );
    assert!(
        !surface_admits(rows, Some(&SurfaceQuery::core(Family::Jim, "0.76"))),
        "{rows:?}"
    );
    assert!(
        !surface_admits(rows, Some(&SurfaceQuery::core(Family::Tcl, "8.6"))),
        "{rows:?}"
    );
}

/// A 2.0 word under a 1.x declaration draws the same per-site notice
/// every newer word does.
#[test]
fn v20_words_under_an_older_declaration_draw_a_per_site_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n arity 1\n \
             available {tcl 8.6-}\n }\n}",
    );
    assert!(
        pack.notices.iter().any(|n| {
            n.message.contains("`available` is SpecTcl 2.0 vocabulary")
                && n.message.contains("declare `speclib probe 2.0`")
        }),
        "{:?}",
        pack.notices
    );
}

/// The seven ratified words (§6.2's list) each land on the
/// `CommandSpec` field the design memo's coverage matrix names for it,
/// closing §6.3's `DraftOpaque`-masks-`LoaderGap` blind spot.
#[test]
fn the_ratified_words_reach_their_model_fields() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n\
             command probe::collect {\n\
             \x20  arity 0\n\
             \x20  result_stability Volatile\n\
             \x20  data_collection -native HTTP_COLLECT\n\
             \x20  side_switch_target Server\n\
             \x20  event_handler_priority -default 500 -min 0 -max 1000 -warn-implicit\n\
             \x20  event_requirement_form {append} -only-in {HTTP_REQUEST} {\n\
             \x20      client_side yes\n\
             \x20  }\n\
             }\n\
             command probe::read {\n\
             \x20  arity 1\n\
             \x20  result_stability {ReadsVersionedWorld {CommandBindings NamespaceLookup}}\n\
             \x20  body_scope {\n\
             \x20      name {probe body}\n\
             \x20      include_sibling_definitions\n\
             \x20      allow_unknown_commands no\n\
             \x20      command top {\n\
             \x20          arity 1..2\n\
             \x20          detail {the top row}\n\
             \x20          subcommand set { arity 1 }\n\
             \x20      }\n\
             \x20  }\n\
             \x20  subcommand line {\n\
             \x20      arity 0\n\
             \x20      result_stability ReferentiallyTransparent\n\
             \x20  }\n\
             }\n\
             }\n",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let collect = pack
        .commands
        .iter()
        .find(|command| command.spec.name == "probe::collect")
        .expect("the collect command loads");
    assert_eq!(
        collect.spec.result_stability,
        Some(ResultStability::Volatile)
    );
    assert_eq!(
        collect.spec.data_collection.map(|op| op.action),
        Some(tcl_registry::events::DataCollectionAction::Collect)
    );
    assert_eq!(
        collect.spec.side_switch_target,
        Some(SideSwitchTarget::Server)
    );
    let priority = collect
        .spec
        .event_handler_priority
        .expect("the priority policy loads");
    assert_eq!(priority.default_priority, 500);
    assert_eq!(priority.max_priority, 1000);
    assert!(priority.warn_when_implicit);
    assert_eq!(collect.spec.event_requirement_forms.len(), 1);
    let form = &collect.spec.event_requirement_forms[0];
    assert_eq!(form.argument_prefix, ["append"]);
    assert_eq!(form.only_in, ["HTTP_REQUEST"]);
    assert!(
        form.requires
            .as_ref()
            .is_some_and(|requires| requires.client_side)
    );

    let read = pack
        .commands
        .iter()
        .find(|command| command.spec.name == "probe::read")
        .expect("the read command loads");
    assert_eq!(
        read.spec.result_stability,
        Some(ResultStability::ReadsVersionedWorld(&[
            WorldStateDomain::CommandBindings,
            WorldStateDomain::NamespaceLookup,
        ]))
    );
    let scope = read.spec.body_scope.expect("the body scope loads");
    assert_eq!(scope.name, "probe body");
    assert!(scope.include_sibling_definitions);
    assert!(!scope.allow_unknown_commands);
    let top = scope.command("top").expect("the scoped command loads");
    assert!(top.arity.accepts(2));
    assert_eq!(top.detail, "the top row");
    assert!(top.subcommand("set").is_some());
    assert_eq!(
        read.spec
            .subcommands
            .iter()
            .find(|sub| sub.name == "line")
            .and_then(|sub| sub.result_stability),
        Some(ResultStability::ReferentiallyTransparent)
    );
}

/// A `body_scope` may also name a shipped environment or a pack
/// `descriptor`, exactly as `definition_body` and `case_list` do.
#[test]
fn a_body_scope_resolves_a_shipped_name_and_a_descriptor() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n\
             descriptor body_scope probe-style {\n\
             \x20  name {probe style}\n\
             \x20  command row { arity 1 }\n\
             }\n\
             command probe::defstyle { arity 2; body_scope report-defstyle }\n\
             command probe::style { arity 2; body_scope probe-style }\n\
             }\n",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let named = |name: &str| {
        pack.commands
            .iter()
            .find(|command| command.spec.name == name)
            .and_then(|command| command.spec.body_scope)
            .expect("a body scope")
    };
    assert_eq!(
        named("probe::defstyle").name,
        tcl_registry::scoped::REPORT_DEFSTYLE_ENV.name
    );
    assert!(named("probe::defstyle").is_command("top"));
    assert_eq!(named("probe::style").name, "probe style");
    assert!(named("probe::style").is_command("row"));
}

/// A malformed ratified row is dropped with a notice rather than
/// guessed at: an unreadable world-state domain would *narrow* a
/// dependency set, and a priority outside its own range is not a range.
#[test]
fn a_malformed_ratified_row_is_dropped_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n\
             command probe::bad {\n\
             \x20  arity 0\n\
             \x20  result_stability {ReadsVersionedWorld {NotADomain}}\n\
             \x20  event_handler_priority -default 2000 -max 1000\n\
             \x20  data_collection -native NOT_A_DESCRIPTOR\n\
             }\n\
             }\n",
    );
    let command = pack
        .commands
        .iter()
        .find(|command| command.spec.name == "probe::bad")
        .expect("the command still loads");
    assert_eq!(command.spec.result_stability, None);
    assert_eq!(command.spec.event_handler_priority, None);
    assert_eq!(command.spec.data_collection, None);
    assert_eq!(pack.notices.len(), 3, "{:?}", pack.notices);
}

/// An `environment` block parses, validates, and converts.
#[test]
fn an_environment_block_loads_and_converts_to_a_definition() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment vivado-tcl {\n \
             display_name {Xilinx Vivado}\n core tcl 8.6\n \
             ambient Vivado keyed ToolVersion\n hosted Tk 8.5-\n \
             alias vivado\n editor_identity tcl\n \
             file_extension .XDC -name {Xilinx Design Constraints}\n \
             filename vivado.jou\n signature {create_project}\n \
             policy ambient-plus-require\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(pack.command("demo").is_some(), "the pack's commands load");
    let environment = &pack.environments[0];
    assert_eq!(environment.id, "vivado-tcl");
    assert_eq!(environment.aliases, vec!["vivado".to_owned()]);
    assert_eq!(environment.world_policy, WorldPolicy::AmbientPlusRequire);
    assert_eq!(environment.file_extensions[0].extension.as_ref(), "xdc");

    let definition = environment.to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(definition.id.as_str(), "vivado-tcl");
    assert_eq!(definition.display_name.as_ref(), "Xilinx Vivado");
    assert_eq!(definition.provenance, Provenance::WorkspaceTrusted);
    // The editor's trust state is what decides the workspace's class.
    assert_eq!(
        environment
            .to_definition(PackEnvironmentTier::Workspace(
                tcl_dialect::model::WorkspaceTrust::Untrusted,
            ))
            .provenance,
        Provenance::WorkspaceUntrusted
    );
    assert_eq!(
        definition.core.expect("a core selector").default_release,
        Release::TCL_8_6
    );
    assert_eq!(definition.expected_packages.len(), 2);
    assert!(definition.expected_packages[0].ambient);
    assert!(!definition.expected_packages[1].ambient);
    assert_eq!(definition.server_detection.filenames.len(), 1);
    assert_eq!(definition.server_detection.content_signatures.len(), 1);
    assert_eq!(
        definition
            .editor_identity
            .expect("a contributed identity")
            .as_str(),
        "tcl"
    );
    assert_eq!(
        environment
            .to_definition(PackEnvironmentTier::Bundled)
            .provenance,
        Provenance::BundledPack
    );
}

/// `help_terms` and `version_ceiling` (D17: the two facts the compiled
/// EDA shells carried that the block could not yet say) load and
/// convert; the ceiling resolves against the `core` family whichever
/// order the two rows come in.
#[test]
fn help_terms_and_version_ceiling_rows_load_and_convert() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment vivado-tcl {\n \
             version_ceiling 8.5\n core tcl 8.5\n \
             help_terms {Vivado xilinx}\n help_terms {fpga}\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let environment = &pack.environments[0];
    assert_eq!(environment.version_ceiling, Some(Release::TCL_8_5));
    assert_eq!(environment.help_terms, ["vivado", "xilinx", "fpga"]);
    let definition = environment.to_definition(PackEnvironmentTier::Bundled);
    assert_eq!(
        definition.policy_defaults.version_ceiling,
        Some(Release::TCL_8_5)
    );
    let terms: Vec<&str> = definition.help_terms.iter().map(AsRef::as_ref).collect();
    assert_eq!(terms, ["vivado", "xilinx", "fpga"]);
}

/// `kind` and `short_name` load and convert; a block that states
/// neither is a base release plus packages, named by its display name.
#[test]
fn kind_and_short_name_rows_load_and_convert() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n \
             environment probe-language {\n display_name {Probe Language}\n \
             short_name {Probe}\n kind language\n core tcl 8.6\n }\n \
             environment probe-shell {\n display_name {Probe Shell}\n \
             core tcl 8.6\n }\n \
             environment probe-bare {\n core tcl 8.6\n kind packages\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(pack.environments.len(), 3);

    let language = pack.environments[0].to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(language.kind, EnvironmentKind::Language);
    assert_eq!(language.short_name.as_ref(), "Probe");
    assert_eq!(language.description(), "Probe Language");

    let shell = pack.environments[1].to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(shell.kind, EnvironmentKind::Packages, "packages by default");
    assert_eq!(shell.short_name.as_ref(), "Probe Shell", "the display name");

    let bare = pack.environments[2].to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(bare.kind, EnvironmentKind::Packages);
    assert_eq!(bare.short_name.as_ref(), "probe-bare", "the id");
}

/// An unknown kind word is ignored with a notice: kind is presentation,
/// so the environment still loads and resolves.
#[test]
fn an_unknown_kind_word_is_ignored_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-shell {\n core tcl 8.6\n \
             kind dialect\n }\n}",
    );
    assert_eq!(pack.environments.len(), 1, "the block still loads");
    assert_eq!(pack.environments[0].kind, EnvironmentKind::Packages);
    assert!(
        pack.notices.iter().any(|n| n
            .message
            .contains("`kind dialect` is not an environment kind")),
        "{:?}",
        pack.notices
    );
}

/// An `-extend` block is additive and may not restate the owner's
/// identity rows, `kind` and `short_name` among them.
#[test]
fn an_extend_block_cannot_restate_kind_or_short_name() {
    for row in ["kind language", "short_name {Probe}"] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n environment tcl8.6 -extend {{\n {row}\n }}\n}}"
        ));
        assert!(
            pack.environments.is_empty(),
            "`{row}`: the block is rejected"
        );
    }
}

/// A ceiling off the core's ladder, or with no compiled core to sit on,
/// rejects the block: it is a point on a ladder or it is nothing.
#[test]
fn a_version_ceiling_off_the_ladder_rejects_the_block() {
    for body in ["core tcl 8.5\n version_ceiling 8.7", "version_ceiling 8.5"] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n environment vivado-tcl {{\n {body}\n }}\n \
                 command demo {{ arity 1 }}\n}}"
        ));
        assert!(
            pack.environments.is_empty(),
            "{body}: {:?}",
            pack.environments
        );
        assert!(
            pack.command("demo").is_some(),
            "{body}: the rest still loads"
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("version_ceiling")),
            "{body}: {:?}",
            pack.notices
        );
    }
}

/// §3.3: a compiled canonical name (or alias) is reserved, and a block
/// claiming one is rejected rather than merged.
#[test]
fn an_environment_block_claiming_a_compiled_name_is_rejected() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment tcl8.6 {\n core tcl 8.6\n }\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.environments.is_empty(), "{:?}", pack.environments);
    assert!(pack.command("demo").is_some(), "the rest still loads");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("a compiled environment name")),
        "{:?}",
        pack.notices
    );
}

/// An unknown row in an `environment` block is semantic-class: the
/// block is rejected, and the pack's other content still loads.
#[test]
fn an_unknown_environment_row_rejects_the_block() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             no_such_row yes\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.environments.is_empty());
    assert!(pack.command("demo").is_some(), "the rest still loads");
    assert!(
        pack.notices.iter().any(|n| {
            n.class == VocabularyClass::Presentation
                && n.message.contains("semantic-class vocabulary")
        }),
        "{:?}",
        pack.notices
    );
}

/// An `environment NAME -extend { … }` block parses additively: it may
/// name a compiled environment, carries detection rows and placements,
/// and refuses identity rows.
#[test]
fn an_environment_extend_block_is_additive() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment synopsys-eda-tcl -extend {\n \
             file_extension upf -name {Unified Power Format}\n \
             ambient upf_extras 1.0\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let environment = &pack.environments[0];
    assert!(environment.extends);
    assert_eq!(environment.id, "synopsys-eda-tcl");
    assert_eq!(environment.file_extensions[0].extension.as_ref(), "upf");
    assert_eq!(environment.placements.len(), 1);
    let extension = environment.to_extension(PackEnvironmentTier::Bundled);
    assert_eq!(extension.base, "synopsys-eda-tcl");
    assert_eq!(extension.provenance, Provenance::BundledPack);
    assert_eq!(extension.file_extensions.len(), 1);
    assert_eq!(extension.placements.len(), 1);

    // An identity row inside an extend block rejects the block.
    let rejected = evaluate_pack(
        "speclib probe 2.0 {\n environment synopsys-eda-tcl -extend {\n \
             core tcl 8.6\n }\n command demo { arity 1 }\n}",
    );
    assert!(rejected.environments.is_empty());
    assert!(
        rejected
            .notices
            .iter()
            .any(|n| n.message.contains("identity row")),
        "{:?}",
        rejected.notices
    );
}

/// `provides` declares the pack's package trains and defaults command
/// providers; an explicit `default required_package` still wins.
#[test]
fn provides_defaults_the_command_provider() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n provides upf 1.0 2.1\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(pack.provides.len(), 1);
    assert_eq!(pack.provides[0].name, "upf");
    assert_eq!(pack.provides[0].versions, vec!["1.0", "2.1"]);
    assert_eq!(
        pack.command("demo").expect("demo").spec.required_package,
        Some("upf")
    );

    let explicit = evaluate_pack(
        "speclib probe 2.0 {\n provides upf\n \
             default required_package sdc\n command demo { arity 1 }\n}",
    );
    assert_eq!(
        explicit
            .command("demo")
            .expect("demo")
            .spec
            .required_package,
        Some("sdc"),
        "an explicit default beats the provides fallback"
    );
}

/// `co_provides` parses its predicated relation and carries it as data.
#[test]
fn co_provides_carries_the_predicated_relation() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n provides tk\n \
             co_provides Tk -requires-exact tk -when {without TK_NO_DEPRECATED}\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let relation = &pack.co_provides[0];
    assert_eq!(relation.name, "Tk");
    assert_eq!(relation.requires_exact, Some("tk"));
    assert_eq!(relation.when, Some("without TK_NO_DEPRECATED"));
}

/// `dynamic_surface` / `unknown_members` set the open-surface fact on a
/// command, and the object-class flags set it on a class.
#[test]
fn dynamic_surface_opens_a_commands_member_set() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             dynamic_surface\n }\n command duo {\n arity 1\n \
             unknown_members\n }\n command trio { arity 1 }\n \
             command quad {\n arity 1\n \
             object_class ::probe::tree -dynamic-surface {\n \
             method walk { arity 0 }\n }\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(
        pack.command("demo")
            .expect("demo")
            .spec
            .allow_unknown_subcommands
    );
    assert!(
        pack.command("duo")
            .expect("duo")
            .spec
            .allow_unknown_subcommands
    );
    assert!(
        !pack
            .command("trio")
            .expect("trio")
            .spec
            .allow_unknown_subcommands,
        "saying nothing keeps the closed default"
    );
}

/// `include` splices a fragment's declarations in place — provenance
/// inherited, registrations carrying the included statements — and the
/// determinism guards hold: no context, a cycle, and a path-shaped name
/// each drop the row with a semantic notice.
#[test]
fn include_splices_a_fragment_under_the_determinism_contract() {
    let fragment = "command extra {\n arity 2\n}\n";
    let context = IncludeContext::new(move |name| match name {
        "extra.tclspec-frag" => Ok(fragment.to_owned()),
        "self.tclspec-frag" => Ok("include self.tclspec-frag\n".to_owned()),
        other => Err(format!("no such fragment `{other}`")),
    });
    let context = std::rc::Rc::new(context);
    let including = |source: &str| {
        evaluate_pack_in(
            source,
            &EvalOptions::default(),
            Some(std::rc::Rc::clone(&context)),
        )
    };
    let pack = including(
        "speclib probe 2.0 {\n include extra.tclspec-frag\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(
        pack.command("extra").is_some(),
        "the fragment's command loads"
    );
    assert!(pack.command("demo").is_some());
    assert!(
        pack.registrations.iter().any(|reg| reg.arg(1) == "extra"),
        "the record carries the included statements"
    );
    assert!(
        !pack.registrations.iter().any(|reg| reg.word() == "include"),
        "…and not the include row itself"
    );

    // No context: the row drops with a semantic notice.
    let dropped = evaluate_pack("speclib probe 2.0 {\n include extra.tclspec-frag\n}");
    assert!(
        dropped.notices.iter().any(|n| {
            n.class == VocabularyClass::Semantic && n.message.contains("needs a pack search path")
        }),
        "{:?}",
        dropped.notices
    );

    // A cycle is rejected by content hash.
    let cyclic = including("speclib probe 2.0 {\n include self.tclspec-frag\n}");
    assert!(
        cyclic
            .notices
            .iter()
            .any(|n| n.message.contains("include cycle")),
        "{:?}",
        cyclic.notices
    );

    // A path-shaped name never reaches the resolver.
    let hostile = including("speclib probe 2.0 {\n include ../outside\n}");
    assert!(
        hostile
            .notices
            .iter()
            .any(|n| n.message.contains("not a plain file name")),
        "{:?}",
        hostile.notices
    );

    // An unresolvable name reports the resolver's reason.
    let missing = including("speclib probe 2.0 {\n include nowhere.tclspec-frag\n}");
    assert!(
        missing
            .notices
            .iter()
            .any(|n| n.message.contains("did not resolve")),
        "{:?}",
        missing.notices
    );
}

/// An unknown editor identity keeps the row and drops only the routing
/// An environment selects from the contributed set.
#[test]
fn an_unknown_editor_identity_drops_only_the_routing() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             editor_identity klingon\n }\n}",
    );
    let environment = &pack.environments[0];
    assert_eq!(environment.editor_identity, None);
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("not a contributed editor language id")),
        "{:?}",
        pack.notices
    );
}

/// `selecting_identity` rows carry the contributed language ids that
/// select the environment, and an unknown id is dropped with a notice.
#[test]
fn selecting_identity_rows_carry_contributed_language_ids() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             editor_identity tcl-microchip\n selecting_identity tcl-libero\n \
             selecting_identity klingon\n }\n}",
    );
    let environment = &pack.environments[0];
    let carried: Vec<&str> = environment
        .selecting_identities
        .iter()
        .map(|identity| identity.as_str())
        .collect();
    assert_eq!(carried, ["tcl-libero"]);
    let definition = environment.to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(
        definition.selecting_identities,
        environment.selecting_identities
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("`selecting_identity klingon`")),
        "{:?}",
        pack.notices
    );
}

/// A `dialect` block parses its ladder and its axes against the closed
/// vocabulary.
#[test]
fn a_dialect_block_loads_its_ladder_and_axes() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n dialect picol2 {\n release 2.0\n \
             release 2.1 -build Unknown\n axis expand_syntax off\n \
             axis braced_var first-close\n axis array_index tcl8\n axis numbers tcl90\n \
             axis escapes tcl90\n axis expr_comments hash\n \
             axis bom_skip on\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let dialect = &pack.dialects[0];
    assert_eq!(dialect.name, "picol2");
    assert_eq!(dialect.releases.len(), 2);
    assert_eq!(dialect.releases[1].build, BuildProfileId::Unknown);
    assert_eq!(dialect.axis("expand_syntax"), Some("off"));
    assert_eq!(dialect.axis("nonesuch"), None);
    let grammar = dialect.to_grammar().expect("every value has a backing");
    assert!(!grammar.expand_syntax);
    assert_eq!(grammar.braced_var, BracedVarStyle::FirstClose);
    assert_eq!(grammar.array_index, tcl_dialect::ArrayIndexSyntax::Tcl8);
}

/// An unknown axis, or an unknown value on a known axis, is §6.1's
/// semantic class: the whole block is rejected, the notice names the
/// axis, and the pack's other content still loads.
#[test]
fn an_unknown_axis_or_value_rejects_the_dialect_block() {
    for (body, needle) in [
        (
            "axis no_such_axis on",
            "is not in the closed axis vocabulary",
        ),
        ("axis numbers tcl99", "is not a value of `numbers`"),
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n dialect probe-dialect {{\n {body}\n }}\n \
                 command demo {{ arity 1 }}\n}}"
        ));
        assert!(pack.dialects.is_empty(), "{body}: {:?}", pack.dialects);
        assert!(pack.command("demo").is_some(), "{body}: the rest loads");
        assert!(
            pack.notices.iter().any(|n| n.message.contains(needle)),
            "{body}: {:?}",
            pack.notices
        );
    }
}

/// §2's classification gate: a `dialect` block whose axes reproduce a
/// compiled release is not a dialect at all, and the notice names the
/// environment it should have been.
#[test]
fn a_dialect_block_duplicating_a_compiled_release_is_rejected() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n dialect my-tcl {\n release 1.0\n \
             axis expand_syntax on\n axis braced_var first-close\n \
             axis numbers tcl85\n axis escapes tcl86\n \
             axis array_index tcl8\n axis expr_comments none\n axis bom_skip off\n \
             axis irules_brace_separator off\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.dialects.is_empty(), "{:?}", pack.dialects);
    assert!(
        pack.notices.iter().any(|n| {
            n.message.contains("declares the grammar of tcl 8.6")
                && n.message.contains("environment my-tcl { core tcl 8.6")
        }),
        "{:?}",
        pack.notices
    );
}

/// §6.1's three classes, in the forward direction that is the only one
/// they apply in.
///
/// The downgrade fixture pattern: each body's word is absent from this
/// build's vocabulary, and the assertion is that its absence yields
/// *abstention* — a dropped label, a degraded capability, or an excluded
/// command — never a stronger claim.
#[test]
fn unknown_words_classify_by_compatibility_effect() {
    // Presentation: prose. Warn and drop, as ever.
    let pack = evaluate_pack(
        "speclib probe 2.9 {\n command demo {\n arity 1\n \
             marketing_blurb {Buy now.}\n }\n}",
    );
    let demo = pack.command("demo").expect("the command loads");
    assert!(!demo.degraded, "prose costs the command nothing");
    assert_eq!(pack.notices[0].class, VocabularyClass::Presentation);

    // Assistance: a shape word. The command stays known, marked.
    let pack = evaluate_pack(
        "speclib probe 2.9 {\n command demo {\n arity 1\n \
             arg_cardinality 3\n }\n}",
    );
    let demo = pack.command("demo").expect("the command still loads");
    assert!(demo.degraded, "the affected capability must answer Unknown");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.class == VocabularyClass::Assistance),
        "{:?}",
        pack.notices
    );

    // Semantic: a security word. The command is excluded outright.
    let pack = evaluate_pack(
        "speclib probe 2.9 {\n command demo {\n arity 1\n \
             taint_launders yes\n }\n command safe_neighbour { arity 0 }\n}",
    );
    assert!(
        pack.command("demo").is_none(),
        "a command whose security facts cannot be read is excluded"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.class == VocabularyClass::Semantic
                && n.message.contains("excluded from strong analysis")),
        "{:?}",
        pack.notices
    );
}

#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn combined_variable_receiver_positions_preserve_scope_and_explicit_withdrawal() {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined;
    let pack = evaluate_pack(
        "speclib probe 2.0 {command demo {arity 1; arg 0 -role VarWrite; variable_receivers {0}; refine read {arity 1; variable_receivers {}}; subcommand nested {arity 1; arg 0 -role VarRead; variable_receivers {0}; refine ordinary {arity 1; variable_receivers inherit}}}}",
    );
    let spec = &pack.command("demo").expect("pure naming metadata").spec;
    assert_eq!(spec.variable_receivers, Some(&[(0, Combined)][..]));
    assert_eq!(spec.command_forms[0].variable_receivers, Some(&[][..]));
    assert_eq!(
        spec.subcommands[0].variable_receivers,
        Some(&[(0, Combined)][..])
    );
    assert_eq!(
        spec.subcommands[0].subcommand_forms[0].variable_receivers,
        None
    );
    assert!(spec.successful_handler.is_none());
    assert!(spec.native_compilation.is_none());
    for bad in ["0 0", "256", "RootOnly", "-1"] {
        let rejected = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{variable_receivers {{{bad}}}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(rejected.command("demo").is_none(), "{bad}");
        assert!(rejected.command("neighbour").is_some());
    }
}

#[test]
fn native_successful_handler_contracts_are_explicit_semantic_exclusions() {
    for block in [
        "successful_handler -native Leaf",
        "successful_handler -native ArithmeticSequenceArguments",
        "successful_handler -native EnsembleLeaf",
        "successful_handler -native EnsemblePathLeaf",
        "subcommand capture {successful_handler -native CatchOutputs}",
        "refine getter {arity 0; successful_handler -native Leaf}",
        "refine getter {arity 0; successful_handler -native InitialiseEmptyVariable}",
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{{block}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(
            pack.command("demo").is_none(),
            "native claims cannot be asserted by a pack"
        );
        assert!(pack.command("neighbour").is_some());
        assert!(
            pack.notices.iter().any(|notice| {
                notice.class == VocabularyClass::Semantic
                    && notice.message.contains("successful_handler")
                    && notice.message.contains("excluded from strong analysis")
            }),
            "{:?}",
            pack.notices
        );
    }
}

#[test]
fn native_form_value_overrides_are_explicit_semantic_exclusions() {
    for property in [
        "return_type",
        "arg_types",
        "byte_array_effect",
        "var_elements_effect",
        "safe_on_uninit",
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{refine getter {{arity 0; {property} none}}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(pack.command("demo").is_none());
        assert!(pack.command("neighbour").is_some());
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.class == VocabularyClass::Semantic
                    && notice.message.contains("native form value overrides"))
        );
    }
}

/// The classes apply **only** in §6.1's forward direction. An unknown
/// word in a pack whose vocabulary this build knows in full is an
/// author's typo, and keeps today's warn-and-drop treatment exactly —
/// which is what keeps every 1.x pack in the corpus loading unchanged.
#[test]
fn unknown_words_in_a_known_vocabulary_stay_presentation_class() {
    for declared in ["1.2", "2.0"] {
        let pack = evaluate_pack(&format!(
            "speclib probe {declared} {{\n command demo {{\n arity 1\n \
                 taint_launders yes\n }}\n}}"
        ));
        let demo = pack.command("demo").expect("the command loads");
        assert!(!demo.degraded, "{declared}");
        assert!(
            pack.notices
                .iter()
                .all(|n| n.class == VocabularyClass::Presentation),
            "{declared}: {:?}",
            pack.notices
        );
    }
}

/// [`ArgRows::seal`] is the one projection from the authored row form to
/// the parallel slices the registry stores, so a row with every column
/// set must land in every one of the six vectors. A column added to
/// [`ArgRow`] and not projected fails here rather than vanishing.
#[test]
fn projection_carries_every_row_column() {
    let values = leak_slice(vec![ArgValue {
        value: "alnum",
        ..ArgValue::DEFAULT
    }]);
    let rows = ArgRows {
        rows: vec![ArgRow {
            index: 3,
            role: Some(ArgRole::Body),
            type_hint: Some(ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            }),
            values,
            closed: true,
            presentation: Some(ArgPresentation::InlineScript),
            appends: Some(AppendedArity::Exactly(2)),
        }],
    };

    let sealed = rows.seal();

    assert_eq!(sealed.roles, vec![(3, ArgRole::Body)]);
    assert_eq!(
        sealed.types,
        vec![(
            3,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            }
        )]
    );
    assert_eq!(sealed.values, vec![(3, values)]);
    assert_eq!(sealed.closed, vec![3]);
    assert_eq!(
        sealed.presentation,
        vec![(3, ArgPresentation::InlineScript)]
    );
    assert_eq!(sealed.prefixes, vec![(3, AppendedArity::Exactly(2))]);
}
#[test]
fn original_loader_notices_retain_their_typed_property_and_flag_subjects() {
    // Implementation contract: naming.consumer.typed-spec-pack-notice-actions
    // docs/design/analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md
    let source = "speclib demo 1 {\n command demo::x {\n arty 1; arg 0 -rle Body\n }\n}\n";
    let pack = evaluate_pack(source);
    let subjects: Vec<_> = pack
        .notices
        .iter()
        .filter_map(|notice| notice.subject.as_ref())
        .collect();
    assert!(subjects.iter().any(|subject| subject.word() == "arty"
        && matches!(
            subject.kind(),
            tcl_lsp_core::code_actions::SpecPackNoticeKind::Property
        )));
    assert!(subjects.iter().any(|subject|subject.word()=="-rle" && matches!(subject.kind(),tcl_lsp_core::code_actions::SpecPackNoticeKind::Flag{row} if row=="arg")));
    assert!(
        subjects
            .iter()
            .all(|subject| subject.image().bytes() == source.as_bytes())
    );
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn trace_subject_receiver_metadata_preserves_form_and_rejects_ambiguous_fields() {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::{Combined, TraceSubject};
    let pack = evaluate_pack(
        "speclib probe 2.0 {command demo {arity 2; arg 0 -role VarRead; arg 1 -role VarRead; variable_receivers {0 {1 trace-subject}}; refine nested {arity 2; variable_receivers {{0 trace-subject}}}; subcommand query {arity 1; arg 0 -role VarRead; variable_receivers {{0 trace-subject}}}}}",
    );
    let spec = &pack.command("demo").unwrap().spec;
    assert_eq!(
        spec.variable_receivers,
        Some(&[(0, Combined), (1, TraceSubject)][..])
    );
    assert_eq!(
        spec.command_forms[0].variable_receivers,
        Some(&[(0, TraceSubject)][..])
    );
    assert_eq!(
        spec.subcommands[0].variable_receivers,
        Some(&[(0, TraceSubject)][..])
    );
    assert!(spec.successful_handler.is_none());
    assert!(spec.native_compilation.is_none());
    for bad in [
        "{0 unsupported}",
        "{0 trace-subject} 0",
        "{0 trace-subject extra}",
        "{256 trace-subject}",
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{variable_receivers {{{bad}}}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(pack.command("demo").is_none(), "{bad}");
        assert!(pack.command("neighbour").is_some());
    }
}

#[test]
fn original_pack_include_navigation_and_loader_share_name_and_body_rules() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let source = "\u{feff}speclib p 2.0 {include {shared α}; include ~; include ../outside; command c {include nested}}";
    let config = pack_source_lexer_config(tcl_lexer::SourceSegmentationPurpose::WholeInput);
    let rows = tcl_compiler::parsing::spectcl_source::pack_source_includes(source, config);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let reached = Rc::clone(&calls);
    let context = Rc::new(IncludeContext::new(move |name| {
        reached.borrow_mut().push(name.to_owned());
        Ok(String::new())
    }));
    let _pack = evaluate_pack_in(source, &EvalOptions::default(), Some(context));
    assert_eq!(
        *calls.borrow(),
        rows.iter()
            .map(|row| row.name().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(*calls.borrow(), ["shared α", "~"]);
    assert!(uses_include(source));
    assert!(!uses_include(
        "speclib p 2.0 {include from tcl into jim {a}}"
    ));
    assert!(!uses_include("speclib p 2.0 {command c {include nested}}"));
}

#[test]
fn original_pack_source_facade_separates_file_prologue_from_block_content() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    use tcl_lexer::SourceSegmentationPurpose::{NestedScript, WholeInput};
    let source = "\u{feff}speclib probe {1.1} {\ncommand visible {arity 0}\n\u{feff}command hidden {arity 0}\n}";
    let pack = evaluate_pack(source);
    assert!(pack.command("visible").is_some());
    assert!(pack.command("hidden").is_none());
    let (version, value) = speclib_version_span(source).expect("original file version");
    assert_eq!(&source[version], "1.1");
    assert_eq!(value, "1.1");
    assert_eq!(pack_statements(source)[0].words[0].text, "speclib");
    assert_eq!(
        statements(source, 1, FileBom::Content)[0].words[0].text,
        "\u{feff}speclib"
    );
    let whole = pack_source_lexer_config(WholeInput);
    let body = pack_source_lexer_config(NestedScript);
    assert_eq!(whole.leading_bom, LeadingBom::Skip);
    assert_eq!(body.leading_bom, LeadingBom::Content);
    assert_eq!(body, NestedScript.config(whole));
    assert_eq!(pack_source_lexer_config(WholeInput), whole);
}
