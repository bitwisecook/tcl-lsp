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

//! `refine` blocks: the invocation form a refinement describes, its statements, options and selector.

use super::super::block_descriptors::result_stability_value;
use super::super::body_rows::option_row;
use super::super::statement_reader::{Stmt, block};
use super::super::value_parsers::{
    CODEGEN_HOOKS, LOWERING_HOOKS, apply_availability, form_stamp_is_plain, list_words,
    parse_arity, parse_dialects, parse_flag, parse_representation_effect, parse_semantic_operation,
    parse_traits,
};
use super::super::vocabulary_class::VocabularyClass;
use super::super::{
    ArgRows, Log, PackTables, available, leak_slice, leak_str, leak_strs, semantics,
};
use super::effect_rows::{
    StatedKnowledge, effects_knowledge, effects_row, option_relation_row, side_effect_row,
};
use super::hook_path;
use super::role_resolvers::native_id;
use super::world_state::{form_state_transitions, world_effects_value};
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::forms::{CommandForm, LiteralArgumentPrefix};
use tcl_registry::hover::OptionSpec;
use tcl_registry::knowledge::Knowledge;
use tcl_registry::side_effects::SideEffect;

// `refine` — invocation refinement (2.0, design Q12/D2)

/// The rows a `refine` body accumulates before the form is sealed.
#[derive(Default)]
struct RefineAcc {
    pub(in super::super) args: ArgRows,
    pub(in super::super) options: Vec<OptionSpec>,
    pub(in super::super) option_relations: Vec<tcl_registry::spec::OptionRelation>,
    pub(in super::super) side_effects: Vec<SideEffect>,
    /// `side_effects none` — declaring the parent's effects away is not the
    /// same as saying nothing, so the empty slice needs a spelling of its own.
    silenced_side_effects: bool,
    pub(in super::super) effects: Option<StatedKnowledge>,
    pub(in super::super) declarations: semantics::Declarations,
}

/// Read a `refine NAME { … }` block into the invocation form it describes.
///
/// A form is a **refinement** of the command or subcommand that owns it:
/// every overlay field it does not mention inherits. That makes an omitted
/// row and an empty one different declarations — no `traits` row inherits the
/// parent's traits, while `traits {}` replaces them with none, which is how a
/// read form drops a mutation trait its conservative parent has to carry.
pub(in super::super) fn load_refinement(
    stmt: &Stmt,
    tables: &PackTables,
    parent: &str,
    log: &mut Log,
) -> Option<CommandForm> {
    let name = stmt.word_text(1).to_owned();
    let body = stmt.arg(2)?;
    let line = stmt.line;
    let outer = log.context.clone();
    let path = hook_path(parent, &name);
    log.scoped(format!("{outer} / refine {name}"), |log| {
        let mut form = CommandForm {
            name: leak_str(&name),
            ..CommandForm::DEFAULT
        };
        let mut acc = RefineAcc::default();
        for stmt in block(body) {
            apply_refine_stmt(&mut form, &mut acc, &stmt, tables, &path, log);
        }
        acc.declarations.report_orphan_option_flags(log);
        let args = acc.args.seal();
        if !args.types.is_empty()
            || !args.values.is_empty()
            || !args.closed.is_empty()
            || !args.presentation.is_empty()
            || !args.prefixes.is_empty()
        {
            log.say(
                line,
                "a form refines argument *roles* only; the other `arg` columns \
                 stay on the owning command, where every form shares them",
            );
        }
        form.arg_roles = leak_slice(args.roles);
        form.options = leak_slice(acc.options);
        form.option_relations = leak_slice(acc.option_relations);
        if !acc.side_effects.is_empty() || acc.effects.is_some() {
            form.side_effects = Some(effects_knowledge(acc.side_effects, acc.effects));
        } else if acc.silenced_side_effects {
            form.side_effects = Some(Knowledge::Partial(&[]));
        }
        Some(form)
    })
}

/// Authored combined-argv receiver positions. Absent inherits, `{}`
/// explicitly withdraws, and invalid positions exclude strong semantics.
pub(in super::super) fn variable_receivers_value(
    value: &str,
    line: u32,
    log: &mut Log,
) -> Option<
    &'static [(
        u8,
        tcl_registry::resolved_invocation::VariableReceiverOperandForm,
    )],
> {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm;
    if value == "inherit" {
        return None;
    }
    let Ok(positions) = tcl_syntax::list::split_list(value) else {
        log.say_classified(line, VocabularyClass::Semantic,
            "variable_receivers requires a complete Tcl list; invalid naming metadata is excluded from strong analysis");
        return Some(&[]);
    };
    let mut receivers = Vec::new();
    for word in positions {
        let Ok(fields) = tcl_syntax::list::split_list(&word) else {
            log.say_classified(line, VocabularyClass::Semantic,
                "variable_receivers requires complete receiver fields; invalid naming metadata is excluded from strong analysis");
            return Some(&[]);
        };
        let (index, form) = match fields.as_slice() {
            [index] => (index, VariableReceiverOperandForm::Combined),
            [index, form] if form == "combined" => (index, VariableReceiverOperandForm::Combined),
            [index, form] if form == "trace-subject" => {
                (index, VariableReceiverOperandForm::TraceSubject)
            }
            _ => {
                log.say_classified(line, VocabularyClass::Semantic,
                    "variable_receivers requires an argv index and optional combined or trace-subject form; invalid naming metadata is excluded from strong analysis");
                return Some(&[]);
            }
        };
        let Ok(index) = index.parse::<u8>() else {
            log.say_classified(line, VocabularyClass::Semantic,
                "variable_receivers requires unique byte-sized argv positions; invalid naming metadata is excluded from strong analysis");
            return Some(&[]);
        };
        if receivers.iter().any(|(previous, _)| *previous == index) {
            log.say_classified(line, VocabularyClass::Semantic,
                "variable_receivers repeats an argv position; ambiguous naming metadata is excluded from strong analysis");
            return Some(&[]);
        }
        receivers.push((index, form));
    }
    Some(leak_slice(receivers))
}

/// One statement of a `refine` body.
///
/// The words are the owning scope's own words, read by the owning scope's own
/// readers — a form states the same facts a command does, about one call shape
/// instead of all of them. Only the fields a [`CommandForm`] carries are legal
/// here; anything else is an unknown property, so a fact that belongs on the
/// parent cannot be quietly lost inside a form.
fn apply_refine_stmt(
    form: &mut CommandForm,
    acc: &mut RefineAcc,
    stmt: &Stmt,
    tables: &PackTables,
    path: &str,
    log: &mut Log,
) {
    let key = stmt.word_text(0).to_owned();
    let value = stmt.word_text(1).to_owned();
    // A form's hook bodies are not bound (they bind at command and
    // subcommand scope), so its `evaluate -implementation` is reported and
    // dropped, and no hook is recorded here.
    let scope = semantics::Scope {
        path,
        binds_bodies: false,
    };
    match key.as_str() {
        "variable_receivers" => {
            form.variable_receivers = variable_receivers_value(&value, stmt.line, log);
        }
        "successful_handler" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                "native successful_handler contracts require converged implementation, physical storage and observer proofs; this pack cannot supply them and is excluded from strong analysis");
        }
        "return_type"
        | "arg_types"
        | "byte_array_effect"
        | "var_elements_effect"
        | "safe_on_uninit" => {
            log.say_classified(stmt.line, VocabularyClass::Semantic,
                format!("`{key}`: native form value overrides are excluded from strong analysis until the refinement loader and renderer preserve explicit inheritance withdrawal"));
        }
        // Value transfers (vocabulary 2.2).
        "semantics" | "evaluate" | "facts" => {
            acc.declarations.read(stmt, &scope, log);
            form.semantics = acc.declarations.declaration(&scope).0;
        }
        "arity" => match parse_arity(stmt, log) {
            (arity, None) => form.arity = arity,
            (arity, Some(_)) => {
                form.arity = arity;
                log.say(
                    stmt.line,
                    "a form's `arity` takes no lifecycle window — gate the whole \
                     form with `available` instead",
                );
            }
        },
        "selector" => form.literal_argument_prefix = selector_row(stmt, log),
        "arg" => acc.args.apply(stmt, tables, log),
        "option" => apply_refine_option(form, acc, stmt, tables, &scope, log),
        "option_conflict" => acc.option_relations.push(option_relation_row(
            stmt,
            tcl_registry::RelationKind::MutuallyExclusive,
            log,
        )),
        "option_requires" | "option_requires_one_of" | "option_forbids" => {
            let kind = match key.as_str() {
                "option_requires" => tcl_registry::RelationKind::Requires,
                "option_requires_one_of" => tcl_registry::RelationKind::RequiresOneOf,
                _ => tcl_registry::RelationKind::Forbids,
            };
            acc.option_relations
                .push(option_relation_row(stmt, kind, log));
        }
        "dialects" => form.surface = parse_dialects(&value, stmt.line, log),
        "available" => {
            let availability = available::from_statement(stmt, 1, log);
            apply_availability(&mut form.surface, availability, "form", stmt.line, log);
        }
        "traits" => form.traits = Some(parse_traits(&value, stmt.line, log)),
        "mutator" => form.mutator = Some(parse_flag(stmt.tail())),
        "side_effect" => {
            if let Some(effect) = side_effect_row(stmt, log) {
                acc.side_effects.push(effect);
            }
        }
        "effects" => acc.effects = effects_row(stmt, log),
        "side_effects" => {
            if value == "none" {
                acc.silenced_side_effects = true;
            } else {
                log.say(
                    stmt.line,
                    "`side_effects` takes only `none`; write each effect as its \
                     own `side_effect` row",
                );
            }
        }
        "semantic_operation" => form_semantic_operation(form, stmt, &value, log),
        "result_stability" => {
            let stability = result_stability_value(stmt, log);
            if !stability.is_unknown() {
                form.result_stability = stability;
            }
        }
        "representation_effect" => {
            form.representation_effect = parse_representation_effect(&value, stmt.line, log);
        }
        "world_effects" => form.world_effects = world_effects_value(stmt, tables, log),
        "state_transitions" => {
            form.state_transitions = form_state_transitions(stmt, tables, path, log);
        }
        "lowering_hook" => {
            form.lowering_hook = native_id(stmt, LOWERING_HOOKS, "lowering hook", log);
        }
        "codegen_hook" => form_codegen_hook(form, stmt, log),
        _ => log.unknown_property(stmt),
    }
}

fn apply_refine_option(
    form: &mut CommandForm,
    acc: &mut RefineAcc,
    stmt: &Stmt,
    tables: &PackTables,
    scope: &semantics::Scope<'_>,
    log: &mut Log,
) {
    let (row, evaluation) = semantics::option_flags(stmt, log);
    let (option, hook) = option_row(&row, tables, log);
    if let Some(evaluation) = evaluation {
        acc.declarations
            .decline_option(&option, evaluation, stmt.line, log);
        form.semantics = acc.declarations.declaration(scope).0;
    }
    if hook.is_some() {
        log.say(
            stmt.line,
            "a form option's Tcl hook is dropped: hooks bind at command scope",
        );
    }
    acc.options.push(option);
}

/// A form's `semantic_operation` row, which is the plain stamp or nothing.
fn form_semantic_operation(form: &mut CommandForm, stmt: &Stmt, value: &str, log: &mut Log) {
    if form_stamp_is_plain(stmt, 2, "semantic operation", log) {
        form.semantic_operation = parse_semantic_operation(value, stmt.line, log);
    }
}

/// A form's `codegen_hook` row, which is the plain stamp or nothing.
fn form_codegen_hook(form: &mut CommandForm, stmt: &Stmt, log: &mut Log) {
    if form_stamp_is_plain(stmt, 3, "codegen hook", log) {
        form.codegen_hook = native_id(stmt, CODEGEN_HOOKS, "codegen hook", log);
    }
}

/// `selector {WORD …} ?-exact?` — the literal words that pick this form out of
/// its siblings.
///
/// Tcl resolves an unambiguous abbreviation of a literal selector word, so
/// unique-prefix matching is the default and `-exact` is the opt-out for a
/// surface that does not.
fn selector_row(stmt: &Stmt, log: &mut Log) -> Option<LiteralArgumentPrefix> {
    let words = list_words(stmt.word_text(1));
    if words.is_empty() {
        log.say(stmt.line, "`selector` needs at least one literal word");
        return None;
    }
    let mut prefix_matching = PrefixMatching::Enabled;
    for flag in stmt.words.iter().skip(2) {
        match flag.text.as_str() {
            "-exact" => prefix_matching = PrefixMatching::Strict,
            _ => log.unknown_flag("selector", stmt.line, flag),
        }
    }
    Some(LiteralArgumentPrefix {
        words: leak_strs(&words),
        prefix_matching,
    })
}
