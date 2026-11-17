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

//! Loading one `subcommand` or `method`: its accumulator and sealing, the body statements and `sub_subcommand` rows.

use super::block_descriptors::{
    check_clause_grammar, clause_grammar_value, record_clause_grammar_derivations,
    result_stability_row,
};
use super::body_rows::{
    hover_block, option_effect_family_row, option_row, parse_callback_taint_input_table,
    validated_callback_taint_input_table,
};
use super::command::{
    StatedKnowledge, arg_role_capabilities, effects_knowledge, effects_row, hook_path, hook_source,
    load_refinement, native_count_role_resolver, native_fold, native_id,
    native_layout_role_resolver, option_relation_row, repeat_row, side_effect_row,
    state_transitions_value, validate_arg_role_capabilities, validate_role_resolver_inputs,
    variable_receivers_value, world_effects_value,
};
use super::statement_reader::{Stmt, Word, block};
use super::value_parsers::{
    ANALYSER_HOOKS, BODY_KINDS, CODEGEN_HOOKS, FORMAT_TYPES, INLINE_CODEGEN_HOOKS, LOWERING_HOOKS,
    STORAGE_TYPES, TCL_TYPES, apply_availability, checked_arity_windows, checked_lifecycle,
    checked_option_effect_families, checked_stamp_windows, completion_row, enum_by_name,
    lifecycle_flag, list_words, next_text, parse_arity, parse_byte_array_effect,
    parse_deprecation_fix, parse_dialects, parse_flag, parse_representation_effect,
    parse_return_elements, parse_semantic_operation, parse_taint, parse_traits,
    parse_var_elements_effect, parse_var_write_typing, place_stamp, stamp_gate,
};
use super::vocabulary_class::VocabularyClass;
use super::{
    ArgRows, HookDecl, HookOwner, HookSource, Log, PackTables, abstain_arg_roles,
    abstain_command_prefixes, abstain_const_fold, abstain_const_fold_versioned,
    abstain_script_timings, available, leak_slice, leak_str, leak_strs, literals_valid,
    no_constraint_reports, semantics,
};
use tcl_registry::ScriptLookupScope;
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::arity::ArityWindow;
use tcl_registry::command_table::CommandTableEffect;
use tcl_registry::forms::CommandForm;
use tcl_registry::hooks::{CodegenHookId, InlineCodegenHookId};
use tcl_registry::hover::{CallbackTaintInput, OptionSpec};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::option_effect::OptionEffectFamily;
use tcl_registry::pack_hooks::HookFamily;
use tcl_registry::patterns::PatternType;
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::{OptionPlacement, SubCommand, SubSubCommand};
use tcl_registry::stamp_window::StampWindow;
use tcl_registry::taint::TaintTransformCondition;
use tcl_registry::traits::Traits;

// Subcommands

#[derive(Default)]
pub(super) struct SubAcc {
    pub(super) args: ArgRows,
    pub(super) arity_windows: Vec<ArityWindow>,
    pub(super) codegen_hook_windows: Vec<StampWindow<CodegenHookId>>,
    pub(super) inline_codegen_hook_windows: Vec<StampWindow<InlineCodegenHookId>>,
    pub(super) semantic_operation_windows: Vec<StampWindow<SemanticOperationId>>,
    pub(super) options: Vec<OptionSpec>,
    pub(super) refinements: Vec<CommandForm>,
    pub(super) side_effects: Vec<SideEffect>,
    pub(super) effects: Option<StatedKnowledge>,
    pub(super) sub_subcommands: Vec<SubSubCommand>,
    pub(super) repeats: Vec<tcl_registry::repeated::RepeatedArgLayout>,
    pub(super) option_relations: Vec<tcl_registry::spec::OptionRelation>,
    pub(super) option_effect_families: Vec<OptionEffectFamily>,
    pub(super) versioned_arg_values: Vec<tcl_registry::spec::VersionedArgValue>,
    pub(super) callback_taint_inputs: Vec<(u8, &'static [CallbackTaintInput])>,
    pub(super) declarations: semantics::Declarations,
}

/// Read a `subcommand NAME { … }` body, or the `method NAME { … }` row of an
/// `object_class` block — one grammar, `kind` naming which statement word
/// spelled it so notices say `method walk` rather than `subcommand walk`.
pub(super) fn load_subcommand(
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    kind: &str,
    log: &mut Log,
) -> Option<SubCommand> {
    let name = stmt.word_text(1).to_owned();
    let body = stmt.arg(2)?;
    subcommand_from_parts(&name, kind, stmt.line, log, |sub, acc, log| {
        for stmt in block(body) {
            apply_subcommand_stmt(sub, acc, &stmt, tables, hooks, &name, log);
        }
    })
}

/// Everything [`load_subcommand`] does past reading the statement's own
/// words, shared with the evaluation loader exactly as
/// [`command_from_parts`] is.
pub(super) fn subcommand_from_parts(
    name: &str,
    kind: &str,
    line: u32,
    log: &mut Log,
    fill: impl FnOnce(&mut SubCommand, &mut SubAcc, &mut Log),
) -> Option<SubCommand> {
    let outer = log.context.clone();
    log.scoped(format!("{outer} / {kind} {name}"), |log| {
        let mut sub = SubCommand {
            name: leak_str(name),
            ..SubCommand::DEFAULT
        };
        let mut acc = SubAcc::default();
        fill(&mut sub, &mut acc, log);
        acc.declarations.report_orphan_option_flags(log);
        if let Some(grammar) = sub.clause_grammar {
            check_clause_grammar(grammar, line, log);
        }
        validate_role_resolver_inputs(
            [
                sub.arg_role_resolver.is_some(),
                sub.arg_role_count_resolver.is_some(),
                sub.arg_role_layout_resolver.is_some(),
            ],
            line,
            log,
        );
        validate_arg_role_capabilities(
            sub.has_dynamic_argument_roles(),
            sub.clause_grammar.is_some(),
            sub.arg_role_resolver_roles,
            kind,
            line,
            log,
        );
        let args = acc.args.seal();
        let callback_taint_inputs = validated_callback_taint_input_table(
            acc.callback_taint_inputs,
            &args.roles,
            sub.arg_role_resolver.is_some()
                || sub.arg_role_count_resolver.is_some()
                || sub.arg_role_layout_resolver.is_some()
                || sub.command_prefix_resolver.is_some(),
            sub.script_timing_resolver.is_some() || sub.traits.contains(Traits::DEFERS_BODY),
            &format!("{kind} `{name}`"),
            line,
            log,
        );
        sub.arg_roles = leak_slice(args.roles);
        sub.arg_types = leak_slice(args.types);
        sub.arg_values = leak_slice(args.values);
        sub.closed_value_args = leak_slice(args.closed);
        sub.arg_presentation = leak_slice(args.presentation);
        sub.command_prefixes = leak_slice(args.prefixes);
        sub.callback_taint_inputs = leak_slice(callback_taint_inputs);
        sub.arity_windows = checked_arity_windows(acc.arity_windows, kind, line, log);
        sub.codegen_hook_windows =
            checked_stamp_windows(acc.codegen_hook_windows, kind, "codegen_hook", line, log);
        sub.inline_codegen_hook_windows = checked_stamp_windows(
            acc.inline_codegen_hook_windows,
            kind,
            "inline_codegen_hook",
            line,
            log,
        );
        sub.semantic_operation_windows = checked_stamp_windows(
            acc.semantic_operation_windows,
            kind,
            "semantic_operation",
            line,
            log,
        );
        sub.options = checked_option_effect_families(
            acc.options,
            &acc.option_effect_families,
            kind,
            line,
            log,
        );
        sub.subcommand_forms = leak_slice(acc.refinements);
        sub.side_effects = effects_knowledge(acc.side_effects, acc.effects);
        sub.sub_subcommands = leak_slice(acc.sub_subcommands);
        sub.repeated_args = leak_slice(acc.repeats);
        sub.option_relations = leak_slice(acc.option_relations);
        sub.option_effect_families = leak_slice(acc.option_effect_families);
        sub.versioned_arg_values = leak_slice(acc.versioned_arg_values);
        sub.lifecycle = checked_lifecycle(sub.lifecycle, &format!("{kind} `{name}`"), line, log);
        Some(sub)
    })
}

#[allow(clippy::too_many_lines)]
pub(super) fn apply_subcommand_stmt(
    sub: &mut SubCommand,
    acc: &mut SubAcc,
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    owner: &str,
    log: &mut Log,
) {
    let key = stmt.word_text(0).to_owned();
    let value = stmt.word_text(1).to_owned();
    match key.as_str() {
        "traits" => sub.traits |= parse_traits(&value, stmt.line, log),
        "arity" => match parse_arity(stmt, log) {
            (arity, None) => sub.arity = arity,
            (arity, Some(lifecycle)) => acc.arity_windows.push(ArityWindow { lifecycle, arity }),
        },
        // `dict for`'s shape: a subcommand's own clause grammar, walked over
        // the words after the subcommand word.
        "clause_grammar" => {
            sub.clause_grammar = clause_grammar_value(stmt, log);
            if sub.clause_grammar.is_some() {
                record_clause_grammar_derivations(
                    hooks,
                    &HookOwner::Subcommand(owner.to_owned()),
                    stmt.line,
                );
            }
        }
        "detail" => sub.detail = leak_str(&value),
        "synopsis" => sub.synopsis = leak_str(&value),
        "hover" => {
            if let Some(word) = stmt.arg(1) {
                let (hover, examples) = hover_block(&block(word), log);
                if !examples.is_empty() {
                    log.say(
                        stmt.line,
                        "a structured `example` belongs to the command's own `hover`; \
                         a subcommand's examples stay hover text",
                    );
                }
                sub.hover = Some(hover);
            }
        }
        "arg" => acc.args.apply(stmt, tables, log),
        "repeat" => {
            if let Some(layout) = repeat_row(stmt, log) {
                acc.repeats.push(layout);
            }
        }
        "return_type" => {
            sub.return_type = enum_by_name(TCL_TYPES, &value, "return type", stmt.line, log);
        }
        "completion" => sub.completion = completion_row(&value, stmt.line, log),
        "var_write_typing" => {
            if let Some(typing) = parse_var_write_typing(&value, stmt.line, log) {
                sub.var_write_typing = typing;
            }
        }
        "pure" => sub.pure = parse_flag(stmt.tail()),
        "mutator" => sub.mutator = parse_flag(stmt.tail()),
        "destructive" => sub.destructive = parse_flag(stmt.tail()),
        "returns_path" => sub.returns_path = parse_flag(stmt.tail()),
        "source_index_bounds" => {
            sub.source_index_bounds = enum_by_name(
                tcl_registry::SourceIndexBounds::ALL,
                &value,
                "source index bounds",
                stmt.line,
                log,
            );
        }
        "source_path_operation" => {
            sub.source_path_operation = enum_by_name(
                tcl_registry::SourcePathOperation::ALL,
                &value,
                "source path operation",
                stmt.line,
                log,
            );
        }
        "source_configuration_reference" => {
            sub.source_configuration_reference = enum_by_name(
                tcl_registry::SourceConfigurationReferenceLayout::ALL,
                &value,
                "source configuration reference",
                stmt.line,
                log,
            );
        }
        "is_unescape" => sub.is_unescape = parse_flag(stmt.tail()),
        "loop_list_header" => sub.loop_list_header = parse_flag(stmt.tail()),
        "creates_scope_alias" => sub.creates_scope_alias = parse_flag(stmt.tail()),
        "arg_values_accept_prefix" => sub.arg_values_accept_prefix = parse_flag(stmt.tail()),
        "cfg_rewrite_name" => sub.cfg_rewrite_name = Some(leak_str(&value)),
        "min_abbrev" => sub.min_abbrev = value.parse().ok(),
        "max_leading_option_words" => sub.max_leading_option_words = value.parse().ok(),
        "defines_command_at" => sub.defines_command_at = value.parse().ok(),
        "body_arg_implicit_args" => sub.body_arg_implicit_args = value.parse().unwrap_or(0),
        "channel_configuration" => {
            sub.channel_configuration = enum_by_name(
                tcl_registry::ChannelConfigurationSpec::ALL,
                &value,
                "channel configuration",
                stmt.line,
                log,
            );
        }
        "credential_arg" => sub.credential_arg = value.parse().ok(),
        "sensitive_headers" => sub.sensitive_headers = leak_strs(&list_words(&value)),
        "arg_role_count_resolver" => {
            sub.arg_role_count_resolver = native_count_role_resolver(stmt, log);
        }
        "arg_role_layout_resolver" => {
            sub.arg_role_layout_resolver = native_layout_role_resolver(stmt, log);
        }
        "arg_role_resolver_roles" => {
            log.v21(stmt.line, "arg_role_resolver_roles");
            sub.arg_role_resolver_roles = arg_role_capabilities(&value, stmt.line, log);
        }
        "dialects" => sub.surface = parse_dialects(&value, stmt.line, log),
        "available" => {
            log.v20(stmt.line, "available");
            let availability = available::from_statement(stmt, 1, log);
            apply_availability(&mut sub.surface, availability, "subcommand", stmt.line, log);
        }
        "safe_on_uninit" => sub.safe_on_uninit = parse_dialects(&value, stmt.line, log),
        "introduced_version" => sub.lifecycle.introduced = Some(leak_str(&value)),
        "deprecated_version" => sub.lifecycle.deprecated = Some(leak_str(&value)),
        "retired_version" => sub.lifecycle.retired = Some(leak_str(&value)),
        "taint_output_sink" => sub.taint_output_sink = Some(leak_str(&value)),
        "taint_transform" => sub.taint_transform = Some(parse_taint(&value, stmt.line, log)),
        "taint_transform_when" => {
            sub.taint_transform_when = enum_by_name(
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
            if sub.taint_transform_when.is_none() {
                sub.taint_transform = None;
            }
        }
        "taint_double_encode_colour" => {
            sub.taint_double_encode_colour = Some(parse_taint(&value, stmt.line, log));
        }
        "variable_receivers" => {
            sub.variable_receivers = variable_receivers_value(&value, stmt.line, log);
        }
        "successful_handler" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native successful_handler contracts require converged implementation, physical storage and observer proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "native_compilation" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native compilation contracts require selected interpreter compiler and source-entry proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "body_execution" => {
            // The studio's explicit native gap must not become a dropped
            // execution contract when a provider authors it directly.
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native body_execution contracts require live implementation and hook proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "body_kind" => {
            if let Some(kind) = enum_by_name(BODY_KINDS, &value, "body kind", stmt.line, log) {
                sub.body_kind = kind;
            }
        }
        "script_lookup_scope" => {
            log.v20(stmt.line, "script_lookup_scope");
            sub.script_lookup_scope = enum_by_name(
                &ScriptLookupScope::ALL,
                &value,
                "executable lookup frame",
                stmt.line,
                log,
            );
        }
        "byte_array_effect" => {
            if let Some(effect) = parse_byte_array_effect(&value, stmt.line, log) {
                sub.byte_array_effect = effect;
            }
        }
        "format_string_type" => {
            sub.format_string_type =
                enum_by_name(FORMAT_TYPES, &value, "format string type", stmt.line, log);
        }
        "return_elements" => {
            sub.return_elements = parse_return_elements(&value, stmt.line, log);
        }
        "var_elements_effect" => {
            sub.var_elements_effect = parse_var_elements_effect(&value, stmt.line, log);
        }
        "representation_effect" => {
            sub.representation_effect = parse_representation_effect(&value, stmt.line, log);
        }
        "deprecation_fix" => {
            sub.lifecycle.deprecation_fix = parse_deprecation_fix(stmt, log);
        }
        "inferred_storage_type" => {
            sub.inferred_storage_type =
                enum_by_name(STORAGE_TYPES, &value, "storage type", stmt.line, log);
        }
        "prefix_matching" => {
            const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];
            if let Some(mode) = enum_by_name(MATCHING, &value, "prefix matching", stmt.line, log) {
                sub.prefix_matching = mode;
            }
        }
        "option_prefix_words" => {
            sub.option_prefix_words = value.parse().unwrap_or(0);
        }
        "reserved_trailing_words" => {
            sub.reserved_trailing_words = value.parse().unwrap_or(0);
        }
        "option_placement" => {
            const PLACEMENTS: &[OptionPlacement] =
                &[OptionPlacement::Leading, OptionPlacement::Anywhere];
            log.v20(stmt.line, "option_placement");
            if let Some(placement) =
                enum_by_name(PLACEMENTS, &value, "option placement", stmt.line, log)
            {
                sub.option_placement = placement;
            }
        }
        "pattern_type" => {
            const PATTERNS: &[PatternType] = &[PatternType::Glob, PatternType::Regex];
            sub.pattern_type = enum_by_name(PATTERNS, &value, "pattern type", stmt.line, log);
        }
        "option" => {
            let (row, evaluation) = semantics::option_flags(stmt, log);
            let (option, hook) = option_row(&row, tables, log);
            if let Some(evaluation) = evaluation {
                acc.declarations
                    .decline_option(&option, evaluation, stmt.line, log);
                declare_subcommand_semantics(sub, acc, hooks, owner, log);
            }
            if let Some((source, option_name)) = hook {
                hooks.push(HookDecl {
                    owner: HookOwner::Option {
                        subcommand: Some(owner.to_owned()),
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
        "option_effect_family" => {
            if let Some(family) = option_effect_family_row(stmt, log) {
                acc.option_effect_families.push(family);
            }
        }
        "refine" => {
            log.v20(stmt.line, "refine");
            let parent = hook_path(&log.command, owner);
            if let Some(form) = load_refinement(stmt, tables, &parent, log) {
                acc.refinements.push(form);
            }
        }
        // Value transfers (vocabulary 2.2).
        "semantics" | "evaluate" | "facts" => {
            let path = hook_path(&log.command, owner);
            let scope = semantics::Scope {
                path: &path,
                binds_bodies: true,
            };
            acc.declarations.read(stmt, &scope, log);
            declare_subcommand_semantics(sub, acc, hooks, owner, log);
        }
        "option_conflict" => acc.option_relations.push(option_relation_row(
            stmt,
            tcl_registry::RelationKind::MutuallyExclusive,
            log,
        )),
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
        // The ratified word `SubCommand` carries in its own right: an
        // ensemble arm's result contract is not its command's.
        "result_stability" => {
            if let Some(stability) = result_stability_row(stmt, log) {
                sub.result_stability = Some(stability);
            }
        }
        "side_effect" => {
            if let Some(effect) = side_effect_row(stmt, log) {
                acc.side_effects.push(effect);
            }
        }
        "effects" => acc.effects = effects_row(stmt, log),
        "sub_subcommand" => acc
            .sub_subcommands
            .push(sub_subcommand_row(stmt, tables, log)),
        "versioned_arg_value" => {
            if let Some(gate) = versioned_arg_value_row(stmt, log) {
                acc.versioned_arg_values.push(gate);
            }
        }
        "command_table_effect" => {
            const EFFECTS: &[CommandTableEffect] = &[
                CommandTableEffect::DefinesProcedure,
                CommandTableEffect::RenamesCommands,
                CommandTableEffect::CreatesAliases,
                CommandTableEffect::CreatesCallerAliases,
            ];
            sub.command_table_effect =
                enum_by_name(EFFECTS, &value, "command-table effect", stmt.line, log);
        }
        "lowering_hook" => {
            sub.lowering_hook = native_id(stmt, LOWERING_HOOKS, "lowering hook", log);
        }
        "codegen_hook" => {
            let gate = stamp_gate(stmt, 3, "codegen hook", log);
            let id = native_id(stmt, CODEGEN_HOOKS, "codegen hook", log);
            place_stamp(
                id,
                gate,
                &mut sub.codegen_hook,
                &mut acc.codegen_hook_windows,
            );
        }
        "inline_codegen_hook" => {
            let gate = stamp_gate(stmt, 3, "inline codegen hook", log);
            let id = native_id(stmt, INLINE_CODEGEN_HOOKS, "inline codegen hook", log);
            place_stamp(
                id,
                gate,
                &mut sub.inline_codegen_hook,
                &mut acc.inline_codegen_hook_windows,
            );
        }
        "analyser_hook" => {
            sub.analyser_hook = native_id(stmt, ANALYSER_HOOKS, "analyser hook", log);
        }
        "semantic_operation" => {
            let gate = stamp_gate(stmt, 2, "semantic operation", log);
            let operation = parse_semantic_operation(&value, stmt.line, log);
            place_stamp(
                operation,
                gate,
                &mut sub.semantic_operation,
                &mut acc.semantic_operation_windows,
            );
        }
        "callback_taint_inputs" => {
            log.v12(stmt.line, "callback_taint_inputs");
            acc.callback_taint_inputs = parse_callback_taint_input_table(&value, stmt.line, log);
        }
        "world_effects" => sub.world_effects = world_effects_value(stmt, tables, log),
        "state_transitions" => {
            let scope = hook_path(&log.command, owner);
            sub.state_transitions =
                state_transitions_value(stmt, tables, &scope, log).map(|(descriptor, source)| {
                    if let Some((source, line)) = source {
                        hooks.push(HookDecl {
                            owner: HookOwner::Subcommand(owner.to_owned()),
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
        "arg_role_resolver"
        | "command_prefix_resolver"
        | "script_timing_resolver"
        | "const_fold"
        | "const_fold_versioned"
        | "constraints"
        | "literal_argument_validator" => {
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
                    sub.arg_role_resolver = Some(abstain_arg_roles);
                    ("arg_role_resolver", HookFamily::ArgRoleResolver)
                }
                "command_prefix_resolver" => {
                    sub.command_prefix_resolver = Some(abstain_command_prefixes);
                    ("command_prefix_resolver", HookFamily::CommandPrefixResolver)
                }
                "script_timing_resolver" => {
                    sub.script_timing_resolver = Some(abstain_script_timings);
                    ("script_timing_resolver", HookFamily::ScriptTimingResolver)
                }
                "const_fold" => {
                    let scope = hook_path(&log.command, owner);
                    sub.const_fold = Some(match &source {
                        HookSource::Native { id } => native_fold(
                            &scope,
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
                    let scope = hook_path(&log.command, owner);
                    sub.const_fold_versioned = Some(match &source {
                        HookSource::Native { id } => native_fold(
                            &scope,
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
                "constraints" => {
                    sub.constraints = Some(no_constraint_reports);
                    ("constraints", HookFamily::Constraints)
                }
                _ => {
                    sub.literal_argument_validator = Some(literals_valid);
                    (
                        "literal_argument_validator",
                        HookFamily::LiteralArgumentValidator,
                    )
                }
            };
            hooks.push(HookDecl {
                owner: HookOwner::Subcommand(owner.to_owned()),
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

/// Seal a subcommand's `semantics` / `evaluate` statements into its
/// declaration and its `evaluate` hook, after each statement that changes
/// them, so the last statement read is what the subcommand declares.
fn declare_subcommand_semantics(
    sub: &mut SubCommand,
    acc: &SubAcc,
    hooks: &mut Vec<HookDecl>,
    owner: &str,
    log: &Log,
) {
    let path = hook_path(&log.command, owner);
    let scope = semantics::Scope {
        path: &path,
        binds_bodies: true,
    };
    let (declaration, body) = acc.declarations.declaration(&scope);
    sub.semantics = declaration;
    semantics::rebind(hooks, &HookOwner::Subcommand(owner.to_owned()), body);
}

/// `sub_subcommand NAME ?flags? ?{ option … }?`.
///
/// The trailing block is optional and holds `option` rows: a second-level
/// operation whose option table genuinely differs from its siblings'
/// (`namespace ensemble create` has `-command`, `configure` has `-namespace`)
/// declares its own, and a consumer that can see the dispatch word prefers it
/// over the owning subcommand's.
///
/// **No block and an empty block mean different things.** No block leaves
/// `options` at `None` — this operation says nothing, so the owning
/// subcommand's table applies. An empty block `{}` sets `Some(&[])`: this
/// operation takes no options at all, and the parent's table must not leak
/// into it (`namespace ensemble exists`, whose C arm takes `cmdname` and
/// nothing else).
///
/// The block is recognised by being a **braced** word in a position where a
/// flag name was expected, which no flag value can be: every flag here is
/// `-name value`, so a value is only ever read through `next_text`.
fn sub_subcommand_row(stmt: &Stmt, tables: &PackTables, log: &mut Log) -> SubSubCommand {
    let mut row = SubSubCommand {
        name: leak_str(stmt.word_text(1)),
        ..SubSubCommand::DEFAULT
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-native_compilation" => {
                let _ = next_text(words, &mut i);
                log.say_classified(stmt.line, VocabularyClass::Semantic,
                    "nested native compilation contracts require selected interpreter and actual worker identity proofs; this pack cannot supply them and is excluded from strong analysis");
            }
            "-option_prefix_words" => {
                row.option_prefix_words = next_text(words, &mut i).parse().unwrap_or(0);
            }
            "-detail" => row.detail = leak_str(&next_text(words, &mut i)),
            "-synopsis" => row.synopsis = leak_str(&next_text(words, &mut i)),
            "-dialects" => {
                let text = next_text(words, &mut i);
                row.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut row.surface,
                    availability,
                    "sub_subcommand",
                    stmt.line,
                    log,
                );
            }
            other if !other.starts_with('-') && words[i].braced => {
                log.v12(stmt.line, "an option block on `sub_subcommand`");
                // `Some`, even when the block is empty — that is the whole
                // point of writing one.
                row.options = Some(leak_slice(sub_subcommand_options(
                    &words[i], row.name, tables, log,
                )));
            }
            other => {
                if lifecycle_flag(&mut row.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag("sub_subcommand", stmt.line, &rejected_flag_word);
                }
            }
        }
        i += 1;
    }
    row.lifecycle = checked_lifecycle(
        row.lifecycle,
        &format!("sub_subcommand `{}`", row.name),
        stmt.line,
        log,
    );
    row
}

/// The `option` rows inside a `sub_subcommand NAME { … }` body.
///
/// An `-arity-hook` here is reported rather than dropped in silence: the hook
/// binder walks the command and its subcommands only, so a hook declared at
/// the second level would never be bound, and a pack that wrote one would get
/// a silently arity-hookless option. The option itself survives; only the hook
/// is refused.
fn sub_subcommand_options(
    body: &Word,
    owner: &str,
    tables: &PackTables,
    log: &mut Log,
) -> Vec<OptionSpec> {
    let mut options = Vec::new();
    for stmt in block(body) {
        if stmt.word_text(0) != "option" {
            log.say(
                stmt.line,
                format!(
                    "a `sub_subcommand` body holds `option` rows only; `{}` dropped",
                    stmt.word_text(0)
                ),
            );
            continue;
        }
        let (option, hook) = option_row(&stmt, tables, log);
        if hook.is_some() {
            log.say(
                stmt.line,
                format!(
                    "`-arity-hook` on `{}`'s option `{}` is not bindable at the \
                     second subcommand level; the option is kept, the hook is not",
                    owner, option.name
                ),
            );
        }
        options.push(option);
    }
    options
}

pub(super) fn versioned_arg_value_row(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::spec::VersionedArgValue> {
    let index = stmt.word_text(1).parse().ok()?;
    let mut gate = tcl_registry::spec::VersionedArgValue {
        index,
        value: leak_str(stmt.word_text(2)),
        lifecycle: Lifecycle::UNSPECIFIED,
    };
    let words = &stmt.words;
    let mut i = 3;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        if !lifecycle_flag(&mut gate.lifecycle, &flag, words, &mut i) {
            log.unknown_flag("versioned_arg_value", stmt.line, &rejected_flag_word);
        }
        i += 1;
    }
    gate.lifecycle = checked_lifecycle(
        gate.lifecycle,
        &format!("versioned_arg_value `{}`", gate.value),
        stmt.line,
        log,
    );
    Some(gate)
}
