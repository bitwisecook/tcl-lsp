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

//! Block-valued descriptors: case lists, effect and state-transition blocks, the ratified rows and scoped bodies.

use super::*;
use tcl_registry::result_stability::ResultStability;
use tcl_registry::side_effects::SideSwitchTarget;
use tcl_registry::state_transition::StateTransitionArgumentShape;
use tcl_registry::state_transition::StateTransitionCommit;
use tcl_registry::state_transition::StateTransitionComposition;
use tcl_registry::state_transition::StateTransitionDomain;
use tcl_registry::state_transition::StateTransitionOperandLayout;
use tcl_registry::world_effect::WorldEffectDescriptor;
use tcl_registry::world_effect::WorldEffectWriteSource;
use tcl_registry::world_effect::WorldStateDomain;

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
    let spec_source = include_str!("../../../../tcl-registry/src/spec/case_list.rs");
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
    let effects = spec.world_effects.stated().unwrap();
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
        spec.world_effects
            .stated()
            .expect("a descriptor")
            .composition,
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

/// Issue #1712: a `world_effects` row's trailing flags state how completely
/// its descriptor is known. Tcl 8.4–9.1 and the DSL's own vocabulary are
/// the same here, so the probe names no release.
#[test]
fn world_effects_rows_state_their_completeness_issue_1712() {
    use tcl_dialect::model::{Family, SurfaceQuery};
    use tcl_registry::knowledge::KnowledgeLevel;
    let load = |row: &str| {
        let pack = evaluate_pack(&format!("speclib probe 2.2 {{ command demo {{ {row} }} }}"));
        (
            pack.command("demo").unwrap().spec.world_effects,
            pack.notices,
        )
    };
    let at = |release| SurfaceQuery::core(Family::Tcl, release);

    let (absent, _) = load("arity 0");
    assert!(absent.is_unknown(), "no row states nothing");

    let (stated, _) = load("world_effects none");
    assert_eq!(stated.level(), KnowledgeLevel::Partial);
    assert!(
        stated
            .stated()
            .is_some_and(WorldEffectDescriptor::is_effect_free)
    );

    let (complete, notices) = load("world_effects none -complete {8.4 9.1}");
    assert!(notices.is_empty(), "{notices:?}");
    for release in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
        assert!(
            complete.complete_at(Some(&at(release))).is_some(),
            "{release}"
        );
    }

    let (narrow, _) = load("world_effects none -complete {9.0 9.0}");
    assert!(narrow.complete_at(Some(&at("9.0"))).is_some());
    assert!(narrow.complete_at(Some(&at("8.6"))).is_none());
    assert!(
        narrow
            .complete_at(Some(&SurfaceQuery::any_release(Family::Tcl)))
            .is_none()
    );

    let (unreadable, notices) = load("world_effects none -complete {9.1 8.4}");
    assert_eq!(unreadable.level(), KnowledgeLevel::Partial);
    assert!(
        notices
            .iter()
            .any(|notice| notice.message.contains("`world_effects -complete` takes")),
        "{notices:?}"
    );
}
