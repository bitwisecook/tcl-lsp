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

//! `world_effects` and `state_transitions`: resolvers, widening, coverage and the derivation from a frame effect.

use super::super::statement_reader::Stmt;
use super::super::value_parsers::{by_name, enum_by_name, next_text};
use super::super::vocabulary_class::VocabularyClass;
use super::super::{
    HookDecl, HookOwner, HookSource, Log, PackTables, abstain_state_transitions, leak_slice,
};
use super::effect_rows::{split_knowledge_flags, stated_knowledge};
use super::{hook_source, native_fold, resolve_block};
use std::fmt;
use tcl_registry::knowledge::Knowledge;
use tcl_registry::pack_hooks::HookFamily;
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_registry::state_transition::{
    StateTransitionArgumentShape, StateTransitionCommit, StateTransitionComposition,
    StateTransitionDescriptor, StateTransitionDomain, StateTransitionOperandLayout,
    StateTransitionResolver, StateTransitionWideningRule,
};
use tcl_registry::world_effect::{
    TransitionEffectCoverage, WorldEffectDescriptor, WorldEffectWriteSource, WorldStateDomain,
};

/// `world_effects none | NAME | { … } ?-partial? ?-complete WINDOW?…`.
///
/// The descriptor alone is a lower bound; `-complete {FROM UNTIL}` closes it
/// for the windows evidenced from the implementation (issue #1712).
pub(in super::super) fn world_effects_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Knowledge<WorldEffectDescriptor> {
    let (stated, value) = split_knowledge_flags(stmt, log);
    stated_knowledge(world_effects_descriptor(&value, tables, log), stated)
}

/// `world_effects none | NAME | { … }`.
///
/// Composition is supported; typed effect rows remain reference-only. An
/// incomplete block cannot issue a closed empty effect descriptor.
fn world_effects_descriptor(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<WorldEffectDescriptor> {
    let word = stmt.arg(1)?;
    if !word.braced && word.text == "none" {
        return Some(WorldEffectDescriptor::EMPTY);
    }
    let stmts = resolve_block(stmt, "world_effects", tables, log)?;
    let mut descriptor = WorldEffectDescriptor::EMPTY;
    let mut complete = true;
    for stmt in &stmts {
        match stmt.word_text(0) {
            "composition" => {
                const COMPOSITIONS: &[tcl_registry::world_effect::WorldEffectComposition] = &[
                    tcl_registry::world_effect::WorldEffectComposition::Replace,
                    tcl_registry::world_effect::WorldEffectComposition::Extend,
                ];
                if let Some(composition) = enum_by_name(
                    COMPOSITIONS,
                    stmt.word_text(1),
                    "world-effect composition",
                    stmt.line,
                    log,
                ) {
                    descriptor.composition = composition;
                } else {
                    complete = false;
                    log.say_classified(
                        stmt.line,
                        VocabularyClass::Semantic,
                        "invalid semantic composition excludes this command from strong analysis",
                    );
                }
            }
            // The remaining rows (`access`, `callback`, `resolver`,
            // `dynamic_fallback`) describe typed effect facts the DSL cannot
            // yet construct. Their omission withdraws the complete semantic
            // descriptor rather than claiming an empty footprint.
            other => {
                complete = false;
                log.say_classified(stmt.line, VocabularyClass::Semantic,
                    format!("`world_effects` row `{other}` is not loadable; the incomplete semantic descriptor excludes this command from strong analysis"));
            }
        }
    }
    complete.then_some(descriptor)
}

/// `state_transitions NAME | { … }`.
/// `state_transitions NAME|{ … }` — a [`StateTransitionDescriptor`], and,
/// when its `resolver` row names one, where the resolver comes from: a body
/// the hook host binds, a shipped native by `SCOPE::state_transitions.resolver`
/// id, or `from-frame-effect`, which the owning command derives once its
/// `frame_effect` is read ([`derive_transitions_from_frame_effect`]).
///
/// The rows are the descriptor's own fields: `composition Extend|Replace`,
/// `argument_shape Independent|Positional`, `resolver none |
/// from-frame-effect | -native ID | ?-inputs {…}? {words ctx} {BODY}`,
/// `widen -operands L -domains {D …}` and `covers SOURCE -domains {D …}`
/// (both repeatable), and `commit OnOkOnly|MayCommitBeforeAbruptCompletion`.
/// A resolver body emits `alias LOCAL TARGET ?-level LEVEL?` and
/// `namespace-variable NAME` and nothing else, so a pack states variable-cell
/// alias facts only (`docs/design/compiler/registry-consumer-contracts.md`
/// § *The two hook bodies that remain*). A row this build cannot read
/// withdraws the complete descriptor and excludes strong analysis. The hook
/// source comes back with its `resolver` row's line.
/// A `state_transitions` row with its trailing `-partial` / `-complete
/// WINDOW` flags: the descriptor's completeness, and where its resolver is.
pub(in super::super) fn state_transitions_knowledge(
    stmt: &Stmt,
    tables: &PackTables,
    scope: &str,
    log: &mut Log,
) -> (
    Knowledge<StateTransitionDescriptor>,
    Option<(HookSource, u32)>,
) {
    let (stated, value) = split_knowledge_flags(stmt, log);
    let (descriptor, source) = state_transitions_value(&value, tables, scope, log)
        .map_or((None, None), |(descriptor, source)| {
            (Some(descriptor), source)
        });
    (stated_knowledge(descriptor, stated), source)
}

fn state_transitions_value(
    stmt: &Stmt,
    tables: &PackTables,
    scope: &str,
    log: &mut Log,
) -> Option<(StateTransitionDescriptor, Option<(HookSource, u32)>)> {
    const COMPOSITIONS: &[StateTransitionComposition] = &[
        StateTransitionComposition::Replace,
        StateTransitionComposition::Extend,
    ];
    const ARGUMENT_SHAPES: &[StateTransitionArgumentShape] = &[
        StateTransitionArgumentShape::Independent,
        StateTransitionArgumentShape::Positional,
    ];
    const COMMITS: &[StateTransitionCommit] = &[
        StateTransitionCommit::OnOkOnly,
        StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    ];
    let stmts = resolve_block(stmt, "state_transitions", tables, log)?;
    let mut descriptor = StateTransitionDescriptor::EMPTY;
    let mut resolver = None;
    let mut widening = Vec::new();
    let mut coverage = Vec::new();
    let mut complete = true;
    for stmt in &stmts {
        let notices_before = log.notices.len();
        let word = stmt.word_text(1);
        match stmt.word_text(0) {
            "composition" => {
                if let Some(composition) = enum_by_name(
                    COMPOSITIONS,
                    word,
                    "state-transition composition",
                    stmt.line,
                    log,
                ) {
                    descriptor.composition = composition;
                } else {
                    complete = false;
                    log.say_classified(
                        stmt.line,
                        VocabularyClass::Semantic,
                        "invalid semantic composition excludes this command from strong analysis",
                    );
                }
            }
            "argument_shape" => {
                if let Some(shape) = enum_by_name(
                    ARGUMENT_SHAPES,
                    word,
                    "state-transition argument shape",
                    stmt.line,
                    log,
                ) {
                    descriptor.argument_shape = shape;
                }
            }
            "commit" => {
                if let Some(commit) =
                    enum_by_name(COMMITS, word, "state-transition commit", stmt.line, log)
                {
                    descriptor.commit = commit;
                }
            }
            "resolver" => {
                let source;
                (descriptor.resolver, source) = transition_resolver_row(stmt, scope, log);
                resolver = source.map(|source| (source, stmt.line));
            }
            "widen" => widening.extend(widening_row(stmt, log)),
            "covers" => coverage.extend(coverage_row(stmt, log)),
            other => log.say(
                stmt.line,
                format!("unknown `state_transitions` row `{other}` dropped"),
            ),
        }
        if log.notices.len() != notices_before {
            complete = false;
            log.say_classified(
                stmt.line,
                VocabularyClass::Semantic,
                "incomplete state-transition row excludes this command from strong analysis",
            );
        }
    }
    descriptor.dynamic_widening = leak_slice(widening);
    descriptor.effect_coverage = leak_slice(coverage);
    complete.then_some((descriptor, resolver))
}

/// A `resolver` row: the resolver the descriptor carries now, and the hook
/// source to record — `none` records nothing, a body carries the abstaining
/// placeholder until the host binds it, `-native ID` the shipped resolver the
/// id names, and `from-frame-effect` nothing until the command is sealed.
fn transition_resolver_row(
    stmt: &Stmt,
    scope: &str,
    log: &mut Log,
) -> (Option<StateTransitionResolver>, Option<HookSource>) {
    let Some(source) = hook_source(stmt) else {
        log.say(stmt.line, "unreadable `state_transitions` resolver dropped");
        return (None, None);
    };
    match &source {
        HookSource::Derived { keyword } if keyword == "none" => (None, None),
        HookSource::Derived { keyword } if keyword == FROM_FRAME_EFFECT => (None, Some(source)),
        HookSource::Derived { keyword } => {
            log.say(
                stmt.line,
                format!("unknown `state_transitions` resolver derivation `{keyword}` dropped"),
            );
            (None, None)
        }
        HookSource::Native { id } => (
            native_fold(
                scope,
                HookFamily::StateTransitionResolver.field(),
                id,
                |table| table.state_transition_resolver,
                stmt.line,
                log,
            ),
            Some(source),
        ),
        HookSource::Body { .. } => (Some(abstain_state_transitions), Some(source)),
    }
}

/// The derivation keyword a `state_transitions` resolver may name.
const FROM_FRAME_EFFECT: &str = "from-frame-effect";

/// `widen -operands L -domains {D …}` — the argument positions whose computed
/// words widen the named domains. `L` is `EveryArgument`, `{Indices N …}` or
/// `{Strided FIRST STRIDE}`.
fn widening_row(stmt: &Stmt, log: &mut Log) -> Option<StateTransitionWideningRule> {
    let mut operands = None;
    let mut domains = None;
    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-operands" => operands = operand_layout(&next_text(words, &mut i), stmt.line, log),
            "-domains" => {
                domains = named_list(
                    &next_text(words, &mut i),
                    StateTransitionDomain::ALL,
                    "state-transition domain",
                    stmt.line,
                    log,
                );
            }
            _ => log.unknown_flag("widen", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    let (Some(operands), Some(domains)) = (operands, domains) else {
        log.say(
            stmt.line,
            "a `widen` row needs `-operands` and `-domains`; row dropped",
        );
        return None;
    };
    Some(StateTransitionWideningRule { operands, domains })
}

fn operand_layout(text: &str, line: u32, log: &mut Log) -> Option<StateTransitionOperandLayout> {
    let Ok(words) = crate::loader::value_parsers::strict_pack_list(text) else {
        log.say(line, format!("unreadable operand layout `{text}` dropped"));
        return None;
    };
    let numbers: Option<Vec<u8>> = words.iter().skip(1).map(|word| word.parse().ok()).collect();
    let layout = match (words.first().map(AsRef::as_ref), numbers.as_deref()) {
        (Some("EveryArgument"), Some([])) => Some(StateTransitionOperandLayout::EveryArgument),
        (Some("Indices"), Some(indices)) if !indices.is_empty() => Some(
            StateTransitionOperandLayout::Indices(leak_slice(indices.to_vec())),
        ),
        (Some("Strided"), Some(&[first, stride])) => {
            Some(StateTransitionOperandLayout::Strided { first, stride })
        }
        _ => None,
    };
    if layout.is_none() {
        log.say(line, format!("unreadable operand layout `{text}` dropped"));
    }
    layout
}

/// `covers SOURCE -domains {D …}` — the world-effect writes the transitions
/// are authoritative for. `SOURCE` is `LegacyCommandTable`, `LegacyFrame`,
/// `DeclaredWorldEffect` or `{LegacySideEffect TARGET}`; a domain is a
/// `WorldStateDomain` name or `{LegacyExternal TARGET}`.
fn coverage_row(stmt: &Stmt, log: &mut Log) -> Option<TransitionEffectCoverage> {
    let source = write_source(stmt.word_text(1));
    let mut domains = None;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-domains" => {
                let text = next_text(words, &mut i);
                domains = crate::loader::value_parsers::strict_pack_list(&text)
                    .ok()
                    .and_then(|elements| {
                        elements
                            .iter()
                            .map(|element| world_state_domain(element))
                            .collect::<Option<Vec<_>>>()
                    })
                    .map(leak_slice);
                if domains.is_none() {
                    log.say(
                        stmt.line,
                        format!("unreadable world-state domain in `{text}`"),
                    );
                }
            }
            _ => log.unknown_flag("covers", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    let (Some(source), Some(domains)) = (source, domains) else {
        log.say(
            stmt.line,
            "a `covers` row needs a write source and `-domains`; row dropped",
        );
        return None;
    };
    Some(TransitionEffectCoverage { source, domains })
}

fn write_source(text: &str) -> Option<WorldEffectWriteSource> {
    let words = crate::loader::value_parsers::strict_pack_list(text).ok()?;
    match words.as_slice() {
        [name] if name == "LegacyCommandTable" => Some(WorldEffectWriteSource::LegacyCommandTable),
        [name] if name == "LegacyFrame" => Some(WorldEffectWriteSource::LegacyFrame),
        [name] if name == "DeclaredWorldEffect" => {
            Some(WorldEffectWriteSource::DeclaredWorldEffect)
        }
        [kind, target] if kind == "LegacySideEffect" => {
            by_name(SideEffectTarget::ALL, target).map(WorldEffectWriteSource::LegacySideEffect)
        }
        _ => None,
    }
}

fn world_state_domain(text: &str) -> Option<WorldStateDomain> {
    const DOMAINS: &[WorldStateDomain] = &[
        WorldStateDomain::InterpreterTopology,
        WorldStateDomain::CommandBindings,
        WorldStateDomain::NamespaceLookup,
        WorldStateDomain::NamespaceUnknown,
        WorldStateDomain::ExecutionTraces,
        WorldStateDomain::VariableTraces,
        WorldStateDomain::CommandTraces,
        WorldStateDomain::OoDispatch,
        WorldStateDomain::InterpreterPolicy,
        WorldStateDomain::PackageState,
        WorldStateDomain::HostCapabilities,
        WorldStateDomain::ThreadSharedStore,
        WorldStateDomain::VariableStore,
    ];
    let words = crate::loader::value_parsers::strict_pack_list(text).ok()?;
    match words.as_slice() {
        [name] => by_name(DOMAINS, name),
        [kind, target] if kind == "LegacyExternal" => {
            by_name(SideEffectTarget::ALL, target).map(WorldStateDomain::LegacyExternal)
        }
        _ => None,
    }
}

/// A braced list of catalogue names, every one read or the list dropped with
/// a notice — a partial list would state a narrower fact than the author
/// wrote.
fn named_list<T: Copy + fmt::Debug>(
    text: &str,
    all: &[T],
    what: &str,
    line: u32,
    log: &mut Log,
) -> Option<&'static [T]> {
    let read = crate::loader::value_parsers::strict_pack_list(text)
        .ok()
        .and_then(|names| {
            names
                .iter()
                .map(|name| by_name(all, name))
                .collect::<Option<Vec<_>>>()
        });
    match read {
        Some(read) if !read.is_empty() => Some(leak_slice(read)),
        _ => {
            log.say(line, format!("unreadable {what} list `{text}` dropped"));
            None
        }
    }
}

/// A form's `state_transitions` block. A form binds no hook body and has no
/// frame effect of its own, so its resolver may be `none` or `-native ID`;
/// a body or a derivation is a semantic exclusion and the descriptor keeps
/// no resolver.
pub(super) fn form_state_transitions(
    stmt: &Stmt,
    tables: &PackTables,
    path: &str,
    log: &mut Log,
) -> Knowledge<StateTransitionDescriptor> {
    let (mut knowledge, source) = state_transitions_knowledge(stmt, tables, path, log);
    let Some(descriptor) = knowledge.as_mut() else {
        return knowledge;
    };
    if matches!(
        source,
        Some((HookSource::Body { .. } | HookSource::Derived { .. }, _))
    ) {
        log.say_classified(
            stmt.line,
            VocabularyClass::Semantic,
            "a form's `state_transitions` resolver is not bound: only `none` and `-native ID` \
             read at form scope; the descriptor keeps no resolver and cannot supply strong analysis",
        );
        descriptor.resolver = None;
    }
    knowledge
}

/// Install the resolver each `resolver from-frame-effect` of this command —
/// its own, or a subcommand's — derives from the command's `frame_effect`:
/// the alias pairs an `AliasPairs` layout states, located by its level-word
/// policy ([`tcl_registry::state_transition::alias_pairs_resolver`], which
/// carries the README's two abstentions). Run at the seal, because the
/// `frame_effect` may follow the block that names it. A command with no
/// `frame_effect`, or another layout, has nothing to derive; the descriptor
/// keeps no resolver.
pub(super) fn derive_transitions_from_frame_effect(
    spec: &mut CommandSpec,
    subcommands: &mut [SubCommand],
    hooks: &[HookDecl],
    line: u32,
    log: &mut Log,
) {
    let derived = spec
        .frame_effect
        .filter(|frame| frame.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs)
        .map(|frame| tcl_registry::state_transition::alias_pairs_resolver(frame.level_word));
    for hook in hooks.iter().filter(|hook| {
        hook.family == HookFamily::StateTransitionResolver
            && matches!(&hook.source, HookSource::Derived { keyword } if keyword == FROM_FRAME_EFFECT)
    }) {
        // Nothing to derive is "a notice, and no resolver" (the README's
        // `from-frame-effect` rule): the command still loads, degraded, so
        // the transition answers `Unknown` rather than the command vanishing.
        if derived.is_none() {
            log.say_classified(
                line,
                VocabularyClass::Assistance,
                "`resolver from-frame-effect` needs the command's `frame_effect -layout \
                 AliasPairs`; the descriptor keeps no resolver and cannot supply strong analysis",
            );
        }
        let descriptor = match &hook.owner {
            HookOwner::Command => spec.state_transitions.as_mut(),
            HookOwner::Subcommand(name) => subcommands
                .iter_mut()
                .find(|sub| sub.name == name)
                .and_then(|sub| sub.state_transitions.as_mut()),
            HookOwner::Option { .. } => None,
        };
        if let Some(descriptor) = descriptor {
            descriptor.resolver = derived;
            descriptor.logical_source_resolver = spec.frame_effect
                .filter(|frame| frame.layout == tcl_registry::FrameArgLayout::AliasPairs)
                .map(|frame| tcl_registry::state_transition::LogicalSourceStateTransitionResolver::FrameAliasPairs {
                    level_word: frame.level_word,
                    resolver: tcl_registry::state_transition::logical_source_alias_pairs_resolver(frame.level_word),
                });
        }
    }
}
