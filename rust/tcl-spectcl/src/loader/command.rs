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

//! Loading one `command`: its accumulator and sealing, the body statements, effect and relation rows, state transitions and `refine` forms.

use super::block_descriptors::{
    case_list_block, check_clause_grammar, definition_body_block, event_requires_block,
    record_clause_grammar_derivations, shipped_case_list, shipped_definition_body,
};
use super::body_rows::validated_callback_taint_input_table;
use super::statement_reader::{Stmt, block};
use super::value_parsers::{
    checked_arity_windows, checked_lifecycle, checked_option_effect_families,
    checked_stamp_windows, list_words,
};
use super::vocabulary_class::VocabularyClass;
use super::{
    ArgRows, HookDecl, HookOwner, HookSource, Log, PackCommand, PackTables, leak_one, leak_slice,
    leak_str, semantics,
};
use tcl_registry::arity::ArityWindow;
use tcl_registry::clause_grammar::ClauseGrammarSpec;
use tcl_registry::definer::{DefinitionBodyGrammar, ManufacturerMethod};
use tcl_registry::events::{EventRequirementForm, EventRequires};
use tcl_registry::forms::CommandForm;
use tcl_registry::hooks::{CodegenHookId, InlineCodegenHookId};
use tcl_registry::hover::{CallbackTaintInput, FormSpec, OptionSpec};
use tcl_registry::native_argument_role_hook::HOOK_PATH_SEPARATOR;
use tcl_registry::option_effect::OptionEffectFamily;
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::{CaseListSpec, CommandSpec, SubCommand};
use tcl_registry::stamp_window::StampWindow;
use tcl_registry::traits::Traits;

mod body_statements;
mod containment;
mod effect_rows;
mod refinements;
mod role_resolvers;
mod world_state;

pub(super) use body_statements::{apply_command_stmt, repeat_row};
use containment::check_command_containment;
pub(super) use effect_rows::{
    StatedKnowledge, completion_value, effects_knowledge, effects_row, option_relation_row,
    side_effect_row, split_knowledge_flags, stated_knowledge, taint_row,
};
pub(super) use refinements::{load_refinement, variable_receivers_value};
pub(super) use role_resolvers::{
    arg_role_capabilities, native_count_role_resolver, native_id, native_layout_role_resolver,
    validate_arg_role_capabilities,
};
use world_state::derive_transitions_from_frame_effect;
pub(super) use world_state::{state_transitions_knowledge, world_effects_value};

// Command loading

/// Everything a command body accumulates before the spec is sealed.
#[derive(Default)]
pub(super) struct CommandAcc {
    pub(super) args: ArgRows,
    pub(super) arity_windows: Vec<ArityWindow>,
    pub(super) codegen_hook_windows: Vec<StampWindow<CodegenHookId>>,
    pub(super) inline_codegen_hook_windows: Vec<StampWindow<InlineCodegenHookId>>,
    pub(super) semantic_operation_windows: Vec<StampWindow<SemanticOperationId>>,
    pub(super) options: Vec<OptionSpec>,
    pub(super) forms: Vec<FormSpec>,
    pub(super) refinements: Vec<CommandForm>,
    pub(super) side_effects: Vec<SideEffect>,
    pub(super) effects: Option<StatedKnowledge>,
    pub(super) subcommands: Vec<SubCommand>,
    pub(super) manufacturers: Vec<ManufacturerMethod>,
    pub(super) repeats: Vec<tcl_registry::repeated::RepeatedArgLayout>,
    pub(super) option_relations: Vec<tcl_registry::spec::OptionRelation>,
    pub(super) option_effect_families: Vec<OptionEffectFamily>,
    pub(super) versioned_arg_values: Vec<tcl_registry::spec::VersionedArgValue>,
    setter_constraints: Vec<tcl_registry::taint::SetterConstraint>,
    oo_context_facts: Vec<(&'static str, tcl_registry::spec::OoContextFact)>,
    pub(super) callback_taint_inputs: Vec<(u8, &'static [CallbackTaintInput])>,
    pub(super) event_requirement_forms: Vec<EventRequirementForm>,
    pub(super) hooks: Vec<HookDecl>,
    pub(super) clause_grammar: Option<&'static ClauseGrammarSpec>,
    /// The line of the `clause_grammar` row `clause_grammar` came from.
    clause_grammar_line: u32,
    pub(super) declarations: semantics::Declarations,
}

pub(super) fn validate_role_resolver_inputs(inputs: [bool; 3], line: u32, log: &mut Log) {
    if inputs.into_iter().filter(|present| *present).count() > 1 {
        log.say_classified(
            line,
            VocabularyClass::Semantic,
            "argument role value/count/layout resolvers are mutually exclusive",
        );
    }
}

/// Build one command: defaults, the body (delivered by `fill` from the
/// evaluation loader's staged nodes), sealing, containment, and §6.1
/// degradation.
///
/// Reads no statement of its own, so a command registered by a `foreach` and
/// one written out literally are built by exactly the same code from exactly
/// the same parts.
// Loading is a single tolerant transaction: defaults, hooks, projected rows,
// lifecycle containment, and degradation notices must seal together.
#[allow(clippy::too_many_lines)]
pub(super) fn command_from_parts(
    name: &str,
    overrides_shipped: bool,
    line: u32,
    tables: &PackTables,
    log: &mut Log,
    fill: impl FnOnce(&mut CommandSpec, &mut CommandAcc, &mut Log),
) -> Option<PackCommand> {
    log.begin_spec();
    log.scoped(format!("command {name}"), |log| {
        let mut spec = CommandSpec {
            name: leak_str(name),
            surface: tables.defaults.surface,
            required_package: tables
                .defaults
                .required_package
                .or(tables.defaults.provides_package),
            tcllib_package: tables.defaults.tcllib_package,
            ..CommandSpec::DEFAULT
        };
        spec.lifecycle.introduced = tables.defaults.introduced_version;
        spec.lifecycle.deprecated = tables.defaults.deprecated_version;
        spec.lifecycle.retired = tables.defaults.retired_version;
        if let Some(value) = tables.defaults.warn_missing_import {
            spec.warn_missing_import = value;
        }
        if let Some(value) = tables.defaults.is_namespace_exported {
            spec.is_namespace_exported = value;
        }

        let mut acc = CommandAcc::default();
        let outer_command = std::mem::replace(&mut log.command, name.to_owned());
        fill(&mut spec, &mut acc, log);
        log.command = outer_command;
        acc.declarations.report_orphan_option_flags(log);

        // A `clause_grammar` derives BOTH hook behaviours: the registry walks
        // the descriptor the spec carries, so no placeholder is installed and
        // the derivation is recorded for what it is.
        if let Some(grammar) = acc.clause_grammar {
            spec.clause_grammar = Some(grammar);
            record_clause_grammar_derivations(
                &mut acc.hooks,
                &HookOwner::Command,
                acc.clause_grammar_line,
            );
            check_clause_grammar(grammar, line, log);
        }
        derive_transitions_from_frame_effect(
            &mut spec,
            &mut acc.subcommands,
            &acc.hooks,
            line,
            log,
        );

        validate_role_resolver_inputs(
            [
                spec.arg_role_resolver.is_some(),
                spec.arg_role_count_resolver.is_some(),
                spec.arg_role_layout_resolver.is_some(),
            ],
            line,
            log,
        );
        validate_arg_role_capabilities(
            spec.has_dynamic_argument_roles(),
            spec.clause_grammar.is_some(),
            spec.arg_role_resolver_roles,
            "command",
            line,
            log,
        );

        let args = acc.args.seal();
        let callback_taint_inputs = validated_callback_taint_input_table(
            acc.callback_taint_inputs,
            &args.roles,
            spec.arg_role_resolver.is_some()
                || spec.arg_role_count_resolver.is_some()
                || spec.arg_role_layout_resolver.is_some()
                || spec.command_prefix_resolver.is_some(),
            spec.script_timing_resolver.is_some() || spec.traits.contains(Traits::DEFERS_BODY),
            &format!("command `{}`", spec.name),
            line,
            log,
        );
        spec.arg_roles = leak_slice(args.roles);
        spec.arg_types = leak_slice(args.types);
        spec.arg_values = leak_slice(args.values);
        spec.closed_value_args = leak_slice(args.closed);
        spec.arg_presentation = leak_slice(args.presentation);
        spec.arg_domains = leak_slice(args.domains);
        spec.command_prefixes = leak_slice(args.prefixes);
        spec.callback_taint_inputs = leak_slice(callback_taint_inputs);
        spec.arity_windows = checked_arity_windows(acc.arity_windows, "command", line, log);
        spec.codegen_hook_windows = checked_stamp_windows(
            acc.codegen_hook_windows,
            "command",
            "codegen_hook",
            line,
            log,
        );
        spec.inline_codegen_hook_windows = checked_stamp_windows(
            acc.inline_codegen_hook_windows,
            "command",
            "inline_codegen_hook",
            line,
            log,
        );
        spec.semantic_operation_windows = checked_stamp_windows(
            acc.semantic_operation_windows,
            "command",
            "semantic_operation",
            line,
            log,
        );
        spec.options = checked_option_effect_families(
            acc.options,
            &acc.option_effect_families,
            "command",
            line,
            log,
        );
        spec.command_forms = leak_slice(acc.refinements);
        spec.forms = leak_slice(acc.forms);
        spec.side_effects = effects_knowledge(acc.side_effects, acc.effects);
        spec.subcommands = leak_slice(acc.subcommands);
        spec.manufacturer_methods = leak_slice(acc.manufacturers);
        spec.repeated_args = leak_slice(acc.repeats);
        spec.option_relations = leak_slice(acc.option_relations);
        spec.option_effect_families = leak_slice(acc.option_effect_families);
        spec.versioned_arg_values = leak_slice(acc.versioned_arg_values);
        spec.setter_constraints = leak_slice(acc.setter_constraints);
        spec.oo_context_facts = leak_slice(acc.oo_context_facts);
        spec.event_requirement_forms = leak_slice(acc.event_requirement_forms);

        spec.lifecycle = checked_lifecycle(
            spec.lifecycle,
            &format!("command `{}`", spec.name),
            line,
            log,
        );
        check_command_containment(&spec, line, log);

        // §6.1's fail-closed rule. A semantic-class word this build cannot
        // read means the command's security / control-flow / binding facts
        // are incomplete, and analysing it anyway would publish confident
        // answers *because* the field was ignored — so the whole spec is
        // excluded. The weaker class keeps the command and marks it, so the
        // affected capability answers `Unknown` instead.
        if log.semantic_unknown {
            log.say_classified(
                line,
                VocabularyClass::Semantic,
                format!(
                    "`command {name}` is excluded from strong analysis: it uses \
                     semantic-class vocabulary this build does not speak"
                ),
            );
            return None;
        }

        for hook in &mut acc.hooks {
            hook.row = log.row_subject(hook.line);
        }
        Some(PackCommand {
            spec: leak_one(spec),
            overrides_shipped,
            hooks: acc.hooks,
            clause_grammar: acc.clause_grammar,
            degraded: log.assistance_unknown,
            line,
            row: log.row_subject(line),
            file: std::path::PathBuf::new(),
            // Set for every command once the whole evaluation is known
            // (`evaluate_pack_in`), which is the only place the bytes are.
            content_hash: 0,
            dependency_tier: None,
            reference_text: None,
        })
    })
}

/// Read a hook property statement: a body, a `-native` id, or a derivation
/// keyword. Returns the source, or `None` when the statement is malformed.
pub(super) fn hook_source(stmt: &Stmt) -> Option<HookSource> {
    // An optional leading `-inputs {…}` declares what the body reads, and is
    // the only flag a hook statement takes; everything after it is the shape
    // below.
    let (inputs, rest) = if stmt.words.len() >= 2 && stmt.word_text(1) == "-inputs" {
        let declared = list_words(stmt.word_text(2));
        let declared: Vec<&str> = declared.iter().map(String::as_str).collect();
        (HookInputs::parse(&declared), 2)
    } else {
        (HookInputs::unrestricted(), 0)
    };
    match stmt.words.len() - rest {
        2 if rest == 0 => Some(HookSource::Derived {
            keyword: stmt.word_text(1).to_owned(),
        }),
        3 if stmt.word_text(rest + 1) == "-native" => Some(HookSource::Native {
            id: stmt.word_text(rest + 2).to_owned(),
        }),
        3 => Some(HookSource::Body {
            params: list_words(stmt.word_text(rest + 1)),
            body: stmt.word_text(rest + 2).to_owned(),
            inputs,
        }),
        _ => None,
    }
}

/// One step further down a pack hook's identity path.
#[must_use]
pub fn hook_path(owner: &str, member: &str) -> String {
    format!("{owner}{HOOK_PATH_SEPARATOR}{member}")
}

/// A `const_fold -native ID` or `const_fold_versioned -native ID` resolved
/// against the shipped folders' table (`tcl_registry::pack_hooks`'s
/// `CONST_FOLD_NATIVE` / `CONST_FOLD_VERSIONED_NATIVE`), keyed by
/// `SCOPE::FIELD` (`string::range::const_fold`): the named folder, or `None`
/// with a load notice for an id that is not this scope's full id or that
/// names nothing this build ships, and the abstaining placeholder stays. The
/// other hook families' tables hold nothing a build ships yet, so their
/// `-native` statements install nothing and are not looked up.
pub(super) fn native_fold<T: Copy>(
    scope: &str,
    field: &str,
    id: &str,
    table: &[(&'static str, T)],
    line: u32,
    log: &mut Log,
) -> Option<T> {
    let full = hook_path(scope, field);
    if id != full {
        log.say(
            line,
            format!(
                "`{field} -native {id}` is not this scope's id; spell it `{full}` — the \
                 statement installs nothing"
            ),
        );
        return None;
    }
    let found = table
        .iter()
        .find(|(key, _)| *key == full)
        .map(|(_, value)| *value);
    if found.is_none() {
        log.say(
            line,
            format!(
                "`{field} -native {id}` names nothing this build ships; the statement installs \
                 nothing"
            ),
        );
    }
    found
}

/// The block-or-name resolution every block statement shares.
pub(super) fn resolve_block<'a>(
    stmt: &'a Stmt,
    key: &str,
    tables: &'a PackTables,
    log: &mut Log,
) -> Option<Vec<Stmt>> {
    let word = stmt.arg(1)?;
    if word.braced {
        return Some(block(word));
    }
    let Some(stmts) = tables.descriptor(key, &word.text) else {
        log.say(
            stmt.line,
            format!("no `descriptor {key} {}` in this pack", word.text),
        );
        return None;
    };
    Some(stmts.to_vec())
}

fn event_requires_value(stmt: &Stmt, tables: &PackTables, log: &mut Log) -> Option<EventRequires> {
    resolve_block(stmt, "event_requires", tables, log)
        .map(|stmts| event_requires_block(&stmts, log))
}

fn case_list_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<&'static CaseListSpec> {
    let word = stmt.arg(1)?;
    if !word.braced
        && let Some(shipped) = shipped_case_list(&word.text)
    {
        return Some(shipped);
    }
    resolve_block(stmt, "case_list", tables, log)
        .map(|stmts| leak_one(case_list_block(&stmts, log)))
}

fn definition_body_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<&'static DefinitionBodyGrammar> {
    let word = stmt.arg(1)?;
    if !word.braced
        && let Some(shipped) = shipped_definition_body(&word.text)
    {
        return Some(shipped);
    }
    resolve_block(stmt, "definition_body", tables, log)
        .map(|stmts| leak_one(definition_body_block(&stmts, log)))
}
