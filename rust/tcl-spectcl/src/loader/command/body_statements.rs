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

//! One statement of a `command` body, with its `repeat` and `form` rows.

use super::super::block_descriptors::{
    IRULES_SOURCE_FLOWS, SIDE_SWITCH_TARGETS, body_scope_value, clause_grammar_value,
    data_collection_row, event_handler_priority_row, event_requirement_form_row, manufacturer_row,
    result_stability_value,
};
use super::super::body_rows::{
    hover_block, option_effect_family_row, option_row, parse_callback_taint_input_table,
};
use super::super::statement_reader::{Stmt, block};
use super::super::subcommand::{load_subcommand, versioned_arg_value_row};
use super::super::value_parsers::{
    ANALYSER_HOOKS, BODY_KINDS, CODEGEN_HOOKS, DEFAULT_FORM_FIRST_WORDS, FORM_KINDS, FORMAT_TYPES,
    INLINE_CODEGEN_HOOKS, LOWERING_HOOKS, RETURN_TYPE_HOOKS, STORAGE_TYPES, TCL_TYPES,
    apply_availability, checked_lifecycle, enum_by_name, lifecycle_flag, list_words, next_text,
    object_class_row, parse_arity, parse_byte_array_effect, parse_byte_array_payload,
    parse_defines_symbol, parse_deprecation_fix, parse_dialects, parse_flag, parse_handle_binding,
    parse_representation_effect, parse_return_elements, parse_semantic_operation,
    parse_setter_constraint, parse_taint, parse_traits, parse_tristate, parse_var_elements_effect,
    parse_var_write_typing, place_stamp, stamp_gate,
};
use super::super::vocabulary_class::VocabularyClass;
use super::super::{
    HookDecl, HookOwner, HookSource, Log, PackTables, abstain_arg_roles, abstain_command_prefixes,
    abstain_const_fold, abstain_const_fold_versioned, abstain_script_timings, accept_clause_shape,
    allow_context, available, leak_one, leak_slice, leak_str, leak_strs, literals_valid,
    no_constraint_reports, semantics, sink_applies,
};
use super::effect_rows::{
    completion_value, effects_row, frame_effect_row, option_relation_row, side_effect_row,
};
use super::refinements::{load_refinement, variable_receivers_value};
use super::role_resolvers::{
    arg_role_capabilities, index_list, native_count_role_resolver, native_id,
    native_layout_role_resolver,
};
use super::world_state::{state_transitions_value, world_effects_value};
use super::{
    CommandAcc, case_list_value, definition_body_value, event_requires_value, hook_source,
    native_fold,
};
use crate::backing::BackingSyntax;
use tcl_registry::ScriptLookupScope;
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::arity::ArityWindow;
use tcl_registry::command_table::CommandTableEffect;
use tcl_registry::hover::{FormKind, FormSpec};
use tcl_registry::pack_hooks::HookFamily;
use tcl_registry::patterns::PatternType;
use tcl_registry::spec::{CommandSpec, OptionPlacement};
use tcl_registry::taint::{TaintNumericCoercion, TaintTransformCondition};

#[allow(clippy::too_many_lines)]
pub(in super::super) fn apply_command_stmt(
    spec: &mut CommandSpec,
    acc: &mut CommandAcc,
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) {
    let key = stmt.word_text(0).to_owned();
    let value = stmt.word_text(1).to_owned();
    match key.as_str() {
        // Identity and availability.
        "dialects" => spec.surface = parse_dialects(&value, stmt.line, log),
        "available" => {
            log.v20(stmt.line, "available");
            let availability = available::from_statement(stmt, 1, log);
            if availability.surface().is_some() {
                spec.surface = availability.surface();
            }
            match (availability.required_package, spec.required_package) {
                (Some(named), None) => spec.required_package = Some(named),
                (Some(named), Some(existing)) if named != existing => log.say(
                    stmt.line,
                    format!(
                        "`available {{package {named}}}` disagrees with `required_package \
                         {existing}`; the declared `required_package` is kept"
                    ),
                ),
                _ => {}
            }
        }
        "source_index_bounds" => {
            spec.source_index_bounds = enum_by_name(
                tcl_registry::SourceIndexBounds::ALL,
                &value,
                "source index bounds",
                stmt.line,
                log,
            );
        }
        "traits" => spec.traits |= parse_traits(&value, stmt.line, log),
        "arity" => match parse_arity(stmt, log) {
            (arity, None) => spec.arity = arity,
            (arity, Some(lifecycle)) => acc.arity_windows.push(ArityWindow { lifecycle, arity }),
        },
        "required_package" => spec.required_package = Some(leak_str(&value)),
        "tk_geometry" => {
            log.v12(stmt.line, "tk_geometry");
            let policy = match value.as_str() {
                "Exclusive" => tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive,
                "Independent" => tcl_registry::tk_geometry::TkGeometryContainerPolicy::Independent,
                other => {
                    log.say(
                        stmt.line,
                        format!("unknown Tk geometry container policy `{other}`"),
                    );
                    return;
                }
            };
            let mut container_option = None;
            let mut direct_form = false;
            let mut placement_subcommand = None;
            let mut release_subcommands: &'static [&'static str] = &[];
            let mut index = 2;
            while index < stmt.words.len() {
                let rejected_flag_word = stmt.words[index].clone();
                match stmt.word_text(index) {
                    "-container-option" => {
                        index += 1;
                        if index < stmt.words.len() {
                            container_option = Some(leak_str(stmt.word_text(index)));
                        } else {
                            log.say(stmt.line, "`-container-option` needs an option name");
                        }
                    }
                    "-direct-form" => direct_form = true,
                    "-placement-subcommand" => {
                        index += 1;
                        if index < stmt.words.len() {
                            placement_subcommand = Some(leak_str(stmt.word_text(index)));
                        } else {
                            log.say(stmt.line, "`-placement-subcommand` needs a subcommand name");
                        }
                    }
                    "-release-subcommands" => {
                        index += 1;
                        if index < stmt.words.len() {
                            release_subcommands = leak_strs(&list_words(stmt.word_text(index)));
                        } else {
                            log.say(stmt.line, "`-release-subcommands` needs a list");
                        }
                    }
                    _ => log.unknown_flag("tk_geometry", stmt.line, &rejected_flag_word),
                }
                index += 1;
            }
            spec.tk_geometry = Some(tcl_registry::tk_geometry::TkGeometryManagerSpec {
                container_policy: policy,
                container_option,
                direct_form,
                placement_subcommand,
                release_subcommands,
            });
        }
        "tcllib_package" => spec.tcllib_package = Some(leak_str(&value)),
        "implementation_namespace" => spec.implementation_namespace = Some(leak_str(&value)),
        "introduced_version" => spec.lifecycle.introduced = Some(leak_str(&value)),
        "deprecated_version" => spec.lifecycle.deprecated = Some(leak_str(&value)),
        "retired_version" => spec.lifecycle.retired = Some(leak_str(&value)),
        "warn_missing_import" => spec.warn_missing_import = parse_flag(stmt.tail()),
        "is_namespace_exported" => spec.is_namespace_exported = parse_flag(stmt.tail()),
        "unsafe_command" => spec.unsafe_command = parse_flag(stmt.tail()),
        "excluded_events" => spec.excluded_events = leak_strs(&list_words(&value)),
        "safe_on_uninit" => spec.safe_on_uninit = parse_dialects(&value, stmt.line, log),
        "deprecated_replacement" => spec.deprecated_replacement = Some(leak_str(&value)),
        "source_deprecation_advice" => {
            spec.source_deprecation_advice = enum_by_name(
                &tcl_registry::deprecation::SourceDeprecationAdvice::ALL,
                &value,
                "source deprecation advice",
                stmt.line,
                log,
            );
        }
        "deprecated_replacement_drop_in" => {
            spec.deprecated_replacement_drop_in = parse_flag(stmt.tail());
        }
        "alias_of" => spec.alias_of = Some(leak_str(&value)),
        // How the command's behaviour reaches the runtime. A declaration that
        // does not read is dropped, which claims nothing: the default is
        // `none`, "nothing executes it".
        "runtime_backing" => {
            let words: Vec<String> = stmt.tail().iter().map(|word| word.text.clone()).collect();
            match BackingSyntax::parse(&words) {
                Ok(syntax) => spec.runtime_backing = syntax.leak(),
                Err(why) => log.say(
                    stmt.line,
                    format!("unreadable `runtime_backing` dropped: {why}"),
                ),
            }
        }
        "xc_translatable" => {
            spec.xc_translatable = parse_tristate(&value);
            if spec.xc_translatable.is_none() {
                log.say(stmt.line, "`xc_translatable` requires yes or no");
            }
        }

        // Shape.
        "arg" => acc.args.apply(stmt, tables, log),
        "arg_values_accept_prefix" => spec.arg_values_accept_prefix = parse_flag(stmt.tail()),
        "repeat" => {
            if let Some(layout) = repeat_row(stmt, log) {
                acc.repeats.push(layout);
            }
        }
        "option_prefix_words" => {
            spec.option_prefix_words = value.parse().unwrap_or(0);
        }
        "reserved_trailing_words" => {
            spec.reserved_trailing_words = value.parse().unwrap_or(0);
        }
        "assigns_variable_at" => spec.assigns_variable_at = value.parse().ok(),
        "creates_instance_at" => spec.creates_instance_at = value.parse().ok(),
        "defines_command_at" => spec.defines_command_at = value.parse().ok(),
        "body_arg_implicit_args" => spec.body_arg_implicit_args = value.parse().unwrap_or(0),
        "variable_receivers" => {
            spec.variable_receivers = variable_receivers_value(&value, stmt.line, log);
        }
        "successful_handler" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native successful_handler contracts require converged implementation, physical storage and observer proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "native_compilation" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native compilation contracts require selected interpreter compiler and source-entry proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "procedure_definition" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native procedure_definition contracts require implementation and storage proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "body_execution" => {
            // The studio's explicit native gap must not become a dropped
            // execution contract when a provider authors it directly.
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native body_execution contracts require live implementation and hook proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "body_kind" => {
            if let Some(kind) = enum_by_name(BODY_KINDS, &value, "body kind", stmt.line, log) {
                spec.body_kind = kind;
            }
        }
        "script_lookup_scope" => {
            log.v20(stmt.line, "script_lookup_scope");
            spec.script_lookup_scope = enum_by_name(
                &ScriptLookupScope::ALL,
                &value,
                "executable lookup frame",
                stmt.line,
                log,
            );
        }
        "allow_unknown_subcommands" => {
            spec.allow_unknown_subcommands = parse_flag(stmt.tail());
        }
        // §6.2's honesty escape hatch: a provider whose member
        // set is runtime-extensible declares so instead of pretending
        // closure. Two ratified spellings, one fact — on a command the
        // fact is the existing open-subcommand-table flag; the
        // object-class form is the `-dynamic-surface` flag on
        // `object_class`.
        "dynamic_surface" | "unknown_members" => {
            log.v20(stmt.line, stmt.word_text(0));
            spec.allow_unknown_subcommands = parse_flag(stmt.tail());
        }
        "prefix_matching" => {
            const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];
            if let Some(mode) = enum_by_name(MATCHING, &value, "prefix matching", stmt.line, log) {
                spec.prefix_matching = mode;
            }
        }
        // E-R14: where the relation checker looks for this command's options.
        "option_placement" => {
            const PLACEMENTS: &[OptionPlacement] =
                &[OptionPlacement::Leading, OptionPlacement::Anywhere];
            log.v20(stmt.line, "option_placement");
            if let Some(placement) =
                enum_by_name(PLACEMENTS, &value, "option placement", stmt.line, log)
            {
                spec.option_placement = placement;
            }
        }
        "self_receiver_words" => spec.self_receiver_words = leak_strs(&list_words(&value)),
        "arg_role_count_resolver" => {
            spec.arg_role_count_resolver = native_count_role_resolver(stmt, log);
        }
        "arg_role_layout_resolver" => {
            spec.arg_role_layout_resolver = native_layout_role_resolver(stmt, log);
        }
        "arg_role_resolver_roles" => {
            log.v21(stmt.line, "arg_role_resolver_roles");
            spec.arg_role_resolver_roles = arg_role_capabilities(&value, stmt.line, log);
        }

        // Types.
        "return_type" => {
            spec.return_type = enum_by_name(TCL_TYPES, &value, "return type", stmt.line, log);
        }
        "completion" => spec.completion = completion_value(stmt, log),
        "var_write_typing" => {
            if let Some(typing) = parse_var_write_typing(&value, stmt.line, log) {
                spec.var_write_typing = typing;
            }
        }
        "inferred_storage_type" => {
            spec.inferred_storage_type =
                enum_by_name(STORAGE_TYPES, &value, "storage type", stmt.line, log);
        }
        "byte_array_effect" => {
            if let Some(effect) = parse_byte_array_effect(&value, stmt.line, log) {
                spec.byte_array_effect = effect;
            }
        }
        "byte_array_payload" => {
            spec.byte_array_payload = parse_byte_array_payload(stmt, log);
        }
        "pattern_type" => {
            const PATTERNS: &[PatternType] = &[PatternType::Glob, PatternType::Regex];
            spec.pattern_type = enum_by_name(PATTERNS, &value, "pattern type", stmt.line, log);
        }
        "format_string_type" => {
            spec.format_string_type =
                enum_by_name(FORMAT_TYPES, &value, "format string type", stmt.line, log);
        }
        "return_type_hook" => {
            spec.return_type_hook = native_id(stmt, RETURN_TYPE_HOOKS, "return-type hook", log);
        }
        "return_elements" => {
            spec.return_elements = parse_return_elements(&value, stmt.line, log);
        }
        "var_elements_effect" => {
            spec.var_elements_effect = parse_var_elements_effect(&value, stmt.line, log);
        }
        "representation_effect" => {
            spec.representation_effect = parse_representation_effect(&value, stmt.line, log);
        }
        "default_form_first_word" => {
            spec.default_form_first_word = enum_by_name(
                DEFAULT_FORM_FIRST_WORDS,
                &value,
                "default-form first word",
                stmt.line,
                log,
            );
        }
        "defines_symbol" => spec.defines_symbol = parse_defines_symbol(stmt, log),
        "deprecation_fix" => {
            spec.lifecycle.deprecation_fix = parse_deprecation_fix(stmt, log);
        }
        "setter_constraint" => {
            if let Some(constraint) = parse_setter_constraint(stmt, log) {
                acc.setter_constraints.push(constraint);
            }
        }

        // Documentation.
        "hover" => {
            if let Some(word) = stmt.arg(1) {
                let (hover, examples) = hover_block(&block(word), log);
                spec.hover = Some(hover);
                if !examples.is_empty() {
                    spec.documentation_examples = Box::leak(examples.into_boxed_slice());
                }
            }
        }
        "form" => acc.forms.push(form_row(stmt, log)),
        "refine" => {
            log.v20(stmt.line, "refine");
            if let Some(form) = load_refinement(stmt, tables, spec.name, log) {
                acc.refinements.push(form);
            }
        }

        // Value transfers (vocabulary 2.2).
        "semantics" | "evaluate" | "facts" => {
            let scope = semantics::Scope {
                path: spec.name,
                binds_bodies: true,
            };
            acc.declarations.read(stmt, &scope, log);
            let (declaration, body) = acc.declarations.declaration(&scope);
            spec.semantics = declaration;
            semantics::rebind(&mut acc.hooks, &HookOwner::Command, body);
        }

        // Effects.
        "side_effect" => {
            if let Some(effect) = side_effect_row(stmt, log) {
                acc.side_effects.push(effect);
            }
        }
        "effects" => acc.effects = effects_row(stmt, log),
        "command_table_effect" => {
            const EFFECTS: &[CommandTableEffect] = &[
                CommandTableEffect::DefinesProcedure,
                CommandTableEffect::RenamesCommands,
                CommandTableEffect::CreatesAliases,
                CommandTableEffect::CreatesCallerAliases,
            ];
            spec.command_table_effect =
                enum_by_name(EFFECTS, &value, "command-table effect", stmt.line, log);
        }
        "frame_effect" => spec.frame_effect = frame_effect_row(stmt, log),
        "world_effects" => spec.world_effects = world_effects_value(stmt, tables, log),
        "state_transitions" => {
            let scope = log.command.clone();
            spec.state_transitions =
                state_transitions_value(stmt, tables, &scope, log).map(|(descriptor, source)| {
                    if let Some((source, line)) = source {
                        acc.hooks.push(HookDecl {
                            owner: HookOwner::Command,
                            field: HookFamily::StateTransitionResolver.field(),
                            family: HookFamily::StateTransitionResolver,
                            source,
                            line,
                            row: None,
                        });
                    }
                    descriptor
                });
        }
        // The ratified words (design §6.2, §6.3's blind spot).
        "result_stability" => {
            let stability = result_stability_value(stmt, log);
            if !stability.is_unknown() {
                spec.result_stability = stability;
            }
        }
        "irules_source_flow" => {
            spec.irules_source_flow = enum_by_name(
                IRULES_SOURCE_FLOWS,
                &value,
                "iRules source flow grammar",
                stmt.line,
                log,
            );
        }
        "side_switch_target" => {
            spec.side_switch_target = enum_by_name(
                SIDE_SWITCH_TARGETS,
                &value,
                "side-switch target",
                stmt.line,
                log,
            );
        }
        "event_handler_priority" => {
            if let Some(policy) = event_handler_priority_row(stmt, log) {
                spec.event_handler_priority = Some(policy);
            }
        }
        "data_collection" => {
            if let Some(operation) = data_collection_row(stmt, log) {
                spec.data_collection = Some(operation);
            }
        }
        "event_requirement_form" => {
            acc.event_requirement_forms
                .push(event_requirement_form_row(stmt, log));
        }
        "body_scope" => {
            if let Some(env) = body_scope_value(stmt, tables, &mut acc.hooks, log) {
                spec.body_scope = Some(env);
            }
        }

        // Taint.
        "taint_source" => spec.taint_source = Some(parse_taint(&value, stmt.line, log)),
        "taint_transform" => spec.taint_transform = Some(parse_taint(&value, stmt.line, log)),
        "taint_transform_when" => {
            spec.taint_transform_when = enum_by_name(
                TaintTransformCondition::ALL.as_slice(),
                &value,
                "taint transform condition",
                stmt.line,
                log,
            );
            // A condition names the proof that earns the colour, so an
            // unreadable one has to take the colour with it. Dropping only the
            // condition would leave the transform unconditional — a typo would
            // widen a sanitiser to every call of the command.
            if spec.taint_transform_when.is_none() {
                spec.taint_transform = None;
            }
        }
        "taint_double_encode_colour" => {
            spec.taint_double_encode_colour = Some(parse_taint(&value, stmt.line, log));
        }
        "taint_sink_safe_colour" => {
            spec.taint_sink_safe_colour = Some(parse_taint(&value, stmt.line, log));
        }
        "taint_output_sink" => spec.taint_output_sink = Some(leak_str(&value)),
        "taint_log_sink" => spec.taint_log_sink = Some(leak_str(&value)),
        "taint_output_sink_subcommands" => {
            spec.taint_output_sink_subcommands = leak_strs(&list_words(&value));
        }
        "taint_interp_eval_subcommands" => {
            spec.taint_interp_eval_subcommands = leak_strs(&list_words(&value));
        }
        "taint_network_sink_args" => {
            spec.taint_network_sink_args = Some(leak_slice(index_list(&value)));
        }
        "taint_code_sink_args" => {
            spec.taint_code_sink_args = Some(leak_slice(index_list(&value)));
        }
        "taint_numeric_coercion" => {
            spec.taint_numeric_coercion = enum_by_name(
                TaintNumericCoercion::ALL.as_slice(),
                &value,
                "taint numeric coercion",
                stmt.line,
                log,
            );
        }
        "callback_taint_inputs" => {
            log.v12(stmt.line, "callback_taint_inputs");
            acc.callback_taint_inputs = parse_callback_taint_input_table(&value, stmt.line, log);
        }
        "channel_configuration" => {
            spec.channel_configuration = enum_by_name(
                tcl_registry::ChannelConfigurationSpec::ALL,
                &value,
                "channel configuration",
                stmt.line,
                log,
            );
        }
        "credential_options" => spec.credential_options = leak_strs(&list_words(&value)),
        "sensitive_headers" => spec.sensitive_headers = leak_strs(&list_words(&value)),

        // iRules.
        "event_requires" => spec.event_requires = event_requires_value(stmt, tables, log),

        // Options.
        "option" => {
            let (row, evaluation) = semantics::option_flags(stmt, log);
            let (option, hook) = option_row(&row, tables, log);
            if let Some(evaluation) = evaluation {
                acc.declarations
                    .decline_option(&option, evaluation, stmt.line, log);
                let scope = semantics::Scope {
                    path: spec.name,
                    binds_bodies: true,
                };
                let (declaration, body) = acc.declarations.declaration(&scope);
                spec.semantics = declaration;
                semantics::rebind(&mut acc.hooks, &HookOwner::Command, body);
            }
            if let Some((source, option_name)) = hook {
                acc.hooks.push(HookDecl {
                    owner: HookOwner::Option {
                        subcommand: None,
                        option: option_name,
                    },
                    field: "options.arity_hook",
                    family: HookFamily::OptionArity,
                    source,
                    line: stmt.line,
                    row: None,
                });
            }
            acc.options.push(option);
        }
        // The families an option row's own `-effect`/`-family` cite by name.
        "option_effect_family" => {
            if let Some(family) = option_effect_family_row(stmt, log) {
                acc.option_effect_families.push(family);
            }
        }
        // The four E-R14 option-relation statements, one shared row parser.
        // `option_conflict` is the 1.x spelling and keeps its exact shape; the
        // other three are 2.0 vocabulary.
        "option_conflict" => {
            acc.option_relations.push(option_relation_row(
                stmt,
                tcl_registry::RelationKind::MutuallyExclusive,
                log,
            ));
        }
        "option_requires" | "option_requires_one_of" | "option_forbids" => {
            log.v20(stmt.line, &key);
            let kind = match key.as_str() {
                "option_requires" => tcl_registry::RelationKind::Requires,
                "option_requires_one_of" => tcl_registry::RelationKind::RequiresOneOf,
                _ => tcl_registry::RelationKind::Forbids,
            };
            acc.option_relations
                .push(option_relation_row(stmt, kind, log));
        }
        // The command-level mirror of the subcommand row, sharing its parser:
        // a literal value of one argument gated on the package's own axis.
        // The statement is 1.1 vocabulary at THIS scope (it was
        // subcommand-only in 1.0).
        "versioned_arg_value" => {
            log.v11(stmt.line, "versioned_arg_value");
            if let Some(gate) = versioned_arg_value_row(stmt, log) {
                acc.versioned_arg_values.push(gate);
            }
        }

        // Descriptors.
        "case_list" => spec.case_list = case_list_value(stmt, tables, log),
        "definition_body" => spec.definition_body = definition_body_value(stmt, tables, log),
        "manufacturer" => acc.manufacturers.push(manufacturer_row(stmt, log)),
        "clause_grammar" => {
            acc.clause_grammar = clause_grammar_value(stmt, log);
            acc.clause_grammar_line = stmt.line;
        }
        "binds_handle" => {
            spec.binds_handle = parse_handle_binding(&value, stmt.line, log).map(leak_one);
        }
        "object_class" => spec.object_class = object_class_row(stmt, tables, log).map(leak_one),
        "oo_context_fact" => {
            const FACTS: &[tcl_registry::spec::OoContextFact] =
                &[tcl_registry::spec::OoContextFact::DefiningClass];
            if let Some(fact) =
                enum_by_name(FACTS, stmt.word_text(2), "oo context fact", stmt.line, log)
            {
                acc.oo_context_facts.push((leak_str(&value), fact));
            }
        }

        // Subcommands.
        "subcommand" => {
            if let Some(sub) = load_subcommand(stmt, tables, &mut acc.hooks, "subcommand", log) {
                acc.subcommands.push(sub);
            }
        }

        // Named engine hooks.
        "lowering_hook" => {
            spec.lowering_hook = native_id(stmt, LOWERING_HOOKS, "lowering hook", log);
            if spec.lowering_hook.is_some() {
                log.say(
                    stmt.line,
                    "names a lowering hook: this changes how the compiler translates \
                     the command, not just what the editor knows about it",
                );
            }
        }
        // A codegen-axis stamp is read as written; whether it survives is
        // the stamp rejection rule's call, made on the merged command at its
        // provenance (`crate::stamps`), which reports a refusal on this row.
        "codegen_hook" => {
            let gate = stamp_gate(stmt, 3, "codegen hook", log);
            let id = native_id(stmt, CODEGEN_HOOKS, "codegen hook", log);
            place_stamp(
                id,
                gate,
                &mut spec.codegen_hook,
                &mut acc.codegen_hook_windows,
            );
        }
        "inline_codegen_hook" => {
            let gate = stamp_gate(stmt, 3, "inline codegen hook", log);
            let id = native_id(stmt, INLINE_CODEGEN_HOOKS, "inline codegen hook", log);
            place_stamp(
                id,
                gate,
                &mut spec.inline_codegen_hook,
                &mut acc.inline_codegen_hook_windows,
            );
        }
        "analyser_hook" => {
            spec.analyser_hook = native_id(stmt, ANALYSER_HOOKS, "analyser hook", log);
        }
        "semantic_operation" => {
            let gate = stamp_gate(stmt, 2, "semantic operation", log);
            let operation = parse_semantic_operation(&value, stmt.line, log);
            place_stamp(
                operation,
                gate,
                &mut spec.semantic_operation,
                &mut acc.semantic_operation_windows,
            );
        }

        // Tcl-body hooks.
        "arg_role_resolver"
        | "command_prefix_resolver"
        | "script_timing_resolver"
        | "const_fold"
        | "const_fold_versioned"
        | "taint_sink_gate"
        | "context_gate"
        | "literal_argument_validator"
        | "constraints"
        | "clause_shape_check" => {
            if key == "script_timing_resolver" {
                log.v12(stmt.line, "script_timing_resolver");
            }
            if key == "constraints" {
                log.v20(stmt.line, "constraints");
            }
            let Some(source) = hook_source(stmt) else {
                log.say(stmt.line, format!("unreadable `{key}` hook dropped"));
                return;
            };
            let (field, family) = match key.as_str() {
                "arg_role_resolver" => {
                    spec.arg_role_resolver = Some(abstain_arg_roles);
                    ("arg_role_resolver", HookFamily::ArgRoleResolver)
                }
                "command_prefix_resolver" => {
                    spec.command_prefix_resolver = Some(abstain_command_prefixes);
                    ("command_prefix_resolver", HookFamily::CommandPrefixResolver)
                }
                "script_timing_resolver" => {
                    spec.script_timing_resolver = Some(abstain_script_timings);
                    ("script_timing_resolver", HookFamily::ScriptTimingResolver)
                }
                "const_fold" => {
                    spec.const_fold = Some(match &source {
                        HookSource::Native { id } => native_fold(
                            &log.command.clone(),
                            "const_fold",
                            id,
                            tcl_registry::pack_hooks::CONST_FOLD_NATIVE,
                            stmt.line,
                            log,
                        )
                        .unwrap_or(abstain_const_fold),
                        _ => abstain_const_fold,
                    });
                    ("const_fold", HookFamily::ConstFold)
                }
                "const_fold_versioned" => {
                    spec.const_fold_versioned = Some(match &source {
                        HookSource::Native { id } => native_fold(
                            &log.command.clone(),
                            "const_fold_versioned",
                            id,
                            tcl_registry::pack_hooks::CONST_FOLD_VERSIONED_NATIVE,
                            stmt.line,
                            log,
                        )
                        .unwrap_or(abstain_const_fold_versioned),
                        _ => abstain_const_fold_versioned,
                    });
                    ("const_fold_versioned", HookFamily::ConstFoldVersioned)
                }
                "taint_sink_gate" => {
                    spec.taint_sink_gate = Some(sink_applies);
                    ("taint_sink_gate", HookFamily::TaintSinkGate)
                }
                "context_gate" => {
                    spec.context_gate = Some(allow_context);
                    ("context_gate", HookFamily::ContextGate)
                }
                "literal_argument_validator" => {
                    spec.literal_argument_validator = Some(literals_valid);
                    (
                        "literal_argument_validator",
                        HookFamily::LiteralArgumentValidator,
                    )
                }
                // E-R14's escape hatch. The pre-binding placeholder reports
                // nothing, so a pack whose host never installs answers exactly
                // as a pack with no hook: silence.
                "constraints" => {
                    spec.constraints = Some(no_constraint_reports);
                    ("constraints", HookFamily::Constraints)
                }
                _ => {
                    spec.clause_shape_check = Some(accept_clause_shape);
                    ("clause_shape_check", HookFamily::ClauseShapeCheck)
                }
            };
            acc.hooks.push(HookDecl {
                owner: HookOwner::Command,
                field,
                family,
                source,
                line: stmt.line,
                row: None,
            });
        }

        _ => log.unknown_property(stmt),
    }
}

pub(in super::super) fn repeat_row(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::repeated::RepeatedArgLayout> {
    let role = enum_by_name(
        ArgRole::ALL,
        stmt.word_text(1),
        "argument role",
        stmt.line,
        log,
    )?;
    let mut layout = tcl_registry::repeated::RepeatedArgLayout {
        role,
        start: 0,
        stride: 1,
        exclude_trailing: 0,
        optional_leading_word: false,
        conditional_binding: false,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-from" => layout.start = next_text(words, &mut i).parse().unwrap_or(0),
            "-stride" => layout.stride = next_text(words, &mut i).parse().unwrap_or(1),
            "-exclude-trailing" => {
                layout.exclude_trailing = next_text(words, &mut i).parse().unwrap_or(0);
            }
            "-optional-leading" => layout.optional_leading_word = true,
            "-conditional" => layout.conditional_binding = true,
            _ => log.unknown_flag("repeat", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    Some(layout)
}

fn form_row(stmt: &Stmt, log: &mut Log) -> FormSpec {
    let kind = enum_by_name(FORM_KINDS, stmt.word_text(1), "form kind", stmt.line, log)
        .unwrap_or(FormKind::Default);
    let mut form = FormSpec {
        kind,
        synopsis: leak_str(stmt.word_text(2)),
        ..FormSpec::DEFAULT
    };
    let words = &stmt.words;
    let mut i = 3;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-dialects" => {
                let text = next_text(words, &mut i);
                form.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut form.surface,
                    availability,
                    "command form",
                    stmt.line,
                    log,
                );
            }
            other => {
                if lifecycle_flag(&mut form.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag("form", stmt.line, &rejected_flag_word);
                }
            }
        }
        i += 1;
    }
    form.lifecycle = checked_lifecycle(
        form.lifecycle,
        &format!("form `{}`", form.synopsis),
        stmt.line,
        log,
    );
    form
}
