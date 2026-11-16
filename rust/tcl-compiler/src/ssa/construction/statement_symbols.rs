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

//! The symbols a statement's source words, assignments and native operands retain while it is renamed.

use super::super::defs::defs_of_in_context;
use super::super::source_view::original_written_source_variable_name;
use super::super::{Symbol, UseClass};
use super::phi_placement::authored_scalar_source_variable;
use super::{RenameWalk, SsaVariableIdentity};
use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::var_resolve::{ResolveContext, VariableCellKey, canonical_variable_key};
use crate::variable_bindings::PointResolveContexts;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

fn definition_source_name(
    stmt: &Statement,
    name: &str,
    after: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    match stmt {
        Statement::AssignConst { name_braced, .. }
        | Statement::AssignValue { name_braced, .. }
        | Statement::AssignExpr { name_braced, .. }
        | Statement::Incr { name_braced, .. } => crate::var_resolve::canonical_binding_value_key(
            &crate::var_resolve::resolve_target_access(
                name,
                *name_braced,
                after,
                registry,
                tcl_registry::TraceOperation::Write,
            ),
        ),
        _ => crate::var_resolve::canonical_literal_variable_key(name, after, registry),
    }
}

/// The existing definition producer owns its operand-binding phase. Source
/// name correspondence cannot resolve that definition again after its effects.
fn original_structural_definition_key_at(
    statement: &Statement,
    block: BlockId,
    index: usize,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    if !matches!(
        statement,
        Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
    ) {
        return None;
    }
    let places = crate::place_bridge::def_places_at(statement, block, index, points, registry);
    let mut keys = places
        .iter()
        .map(crate::var_resolve::canonical_binding_value_key);
    let key = keys.next()??;
    keys.all(|other| other.as_ref() == Some(&key))
        .then_some(key)
}

impl RenameWalk<'_> {
    pub(super) fn preserve_unknown_read_liveness(
        &self,
        raw_classified: &[(String, UseClass)],
        context: &ResolveContext,
        registry: &CommandRegistry,
        unknown_arguments: bool,
        classified: &mut Vec<(VariableCellKey, UseClass)>,
    ) {
        if unknown_arguments
            || raw_classified
                .iter()
                .any(|(name, _)| canonical_variable_key(name, context, registry).is_none())
        {
            // An unresolved read may consume any reaching current-frame value.
            // Preserve liveness without claiming a definite uninitialised read.
            let mut visible: Vec<_> = self
                .stacks
                .keys()
                .filter_map(|identity| match identity {
                    SsaVariableIdentity::Location(key) => Some(key.clone()),
                    SsaVariableIdentity::Source(_) => None,
                })
                .collect();
            visible.sort_by_cached_key(|key| format!("{key:?}"));
            for name in visible {
                if !classified.iter().any(|(bound, _)| bound == &name) {
                    classified.push((name, UseClass::Quoted));
                }
            }
        }
    }

    pub(super) fn retain_statement_symbols(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        raw_classified: &[(String, UseClass)],
        captured: &[(String, VariableCellKey)],
        captured_names: &HashSet<String>,
        registry: &CommandRegistry,
    ) {
        let point_context = self.points.before_statement(point.0, point.1).clone();
        let context = &point_context;
        let mut source_symbols = HashMap::new();
        let authored = authored_scalar_source_variable(&self.points, point.0, point.1, registry);
        for (name, _) in raw_classified {
            if let Some(bound) = canonical_variable_key(name, context, registry) {
                source_symbols.insert(name.clone(), self.interner.intern(&bound));
            } else if authored.as_ref().is_none_or(|(_, held, _)| held != name) {
                self.unknown_source_names.insert(name.clone());
            }
        }
        for name in captured_names {
            source_symbols.remove(name);
        }
        let mut ambiguous = HashSet::new();
        for (name, bound) in captured {
            let symbol = self.interner.intern(bound);
            if source_symbols
                .insert(name.clone(), symbol)
                .is_some_and(|prior| prior != symbol)
            {
                ambiguous.insert(name.clone());
            }
        }
        for name in &ambiguous {
            source_symbols.remove(name);
            self.unknown_source_names.insert(name.clone());
        }
        if let Some(store) = self
            .points
            .conditional_logical_scalar_store_at(point.0, point.1, registry)
            && let Some(place) = store.definition_place(registry)
            && let Some(bound) = crate::var_resolve::canonical_binding_value_key(&place)
        {
            let name = store.source_name().to_owned();
            let symbol = self.interner.intern(&bound);
            if source_symbols
                .insert(name.clone(), symbol)
                .is_some_and(|previous| previous != symbol)
            {
                source_symbols.remove(&name);
                self.unknown_source_names.insert(name);
            }
        } else if context.execution_name_policy.is_none()
            && self.invocation_context(registry).standalone
        {
            for name in defs_of_in_context(stmt, Some(self.invocation_context(registry))) {
                let after = self.points.after_statement(point.0, point.1);
                let bound = definition_source_name(stmt, &name, after, registry);
                if let Some(bound) = bound {
                    source_symbols
                        .entry(name)
                        .or_insert_with(|| self.interner.intern(&bound));
                } else {
                    self.unknown_source_names.insert(name);
                }
            }
        }
        self.retain_source_word_symbols(stmt, point, registry, &mut source_symbols);
        self.retain_assignment_symbols(stmt, point, registry, &mut source_symbols);
        self.retain_native_operand_symbols(point, context, registry, &mut source_symbols);
        self.point_symbols
            .insert((point.0, point.1), source_symbols);
    }

    fn retain_source_word_symbols(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        registry: &CommandRegistry,
        source_symbols: &mut HashMap<String, Symbol>,
    ) {
        if let Some(input) = self
            .metadata
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input)
            && let Some(tokens) = self.points.source_tokens_at(point.0, point.1)
            && let Some(binding) = tokens.source_binding.as_ref()
        {
            let words = if binding.has_original_logical_source_advice() {
                binding
                    .original_logical_source_input_for_name_roles(tokens, registry)
                    .filter(|held| *held == input || &held.for_nested_source() == input)
                    .and_then(|held| binding.original_logical_source_words(tokens, held, registry))
            } else if binding.has_original_native_source_command() {
                binding
                    .original_native_source_input_for_name_roles(tokens, registry)
                    .filter(|held| *held == input || &held.for_nested_source() == input)
                    .and_then(|held| binding.original_native_source_words(tokens, held, registry))
            } else {
                None
            };
            if let Some(words) = words {
                for (argument, role) in words.written_argument_roles() {
                    if role != tcl_registry::ArgRole::VarWrite {
                        continue;
                    }
                    let Some(word) = tokens.words().get(argument + 1) else {
                        continue;
                    };
                    let Some(receiver_span) = binding.invocation_site().and_then(|site| {
                        word.original_written_span(site.source.source_image().bytes())
                    }) else {
                        continue;
                    };
                    let Some(name) =
                        original_written_source_variable_name(&words, argument, receiver_span)
                    else {
                        continue;
                    };
                    if !defs_of_in_context(stmt, Some(self.invocation_context(registry)))
                        .iter()
                        .any(|defined| defined == name)
                    {
                        continue;
                    }
                    let after = self.points.after_statement(point.0, point.1);
                    if let Some(bound) = definition_source_name(stmt, name, after, registry) {
                        let symbol = self.interner.intern(&bound);
                        if source_symbols
                            .insert(name.to_owned(), symbol)
                            .is_some_and(|old| old != symbol)
                        {
                            source_symbols.remove(name);
                            self.unknown_source_names.insert(name.to_owned());
                        }
                    }
                }
            }
        }
    }

    fn retain_assignment_symbols(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        registry: &CommandRegistry,
        source_symbols: &mut HashMap<String, Symbol>,
    ) {
        if let Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } = stmt
            && let Some(tokens) = self.points.source_tokens_at(point.0, point.1)
        {
            let original_name = if let Some(metadata) = self.metadata
                && metadata.source_analysis_input().is_some_and(
                    crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
                )
                && tokens.source_binding.as_ref().is_some_and(|binding| {
                    !binding.has_original_logical_source_advice()
                        && !binding.has_original_native_source_command()
                }) {
                crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                    registry, metadata, tokens, None,
                )
                .is_some_and(|invocation| {
                    invocation.written_roles().iter().any(|&(argument, role)| {
                        role == tcl_registry::ArgRole::VarWrite
                            && invocation
                                .written_variable_name_operand(argument)
                                .as_deref()
                                == Some(name.as_str())
                    })
                })
            } else {
                crate::registry_invocation::original_assignment_name_operand(registry, tokens, name)
                    .is_some()
            };
            if original_name
                && let Some(bound) = original_structural_definition_key_at(
                    stmt,
                    point.0,
                    point.1,
                    &self.points,
                    registry,
                )
            {
                let symbol = self.interner.intern(&bound);
                if source_symbols
                    .insert(name.clone(), symbol)
                    .is_some_and(|old| old != symbol)
                {
                    source_symbols.remove(name);
                    self.unknown_source_names.insert(name.clone());
                }
            }
        }
    }

    pub(in super::super) fn retain_native_operand_symbols(
        &mut self,
        point: (BlockId, usize),
        context: &ResolveContext,
        registry: &CommandRegistry,
        source_symbols: &mut HashMap<String, Symbol>,
    ) {
        if let Some(invocation) =
            self.points
                .source_tokens_at(point.0, point.1)
                .and_then(|tokens| {
                    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                        registry,
                        self.metadata,
                        tokens,
                    )
                })
        {
            for (argument, _) in invocation.variable_roles() {
                let Some(name) = invocation.argument_literal(argument) else {
                    continue;
                };
                let Some(target) = invocation.variable_operand_place(argument, context, registry)
                else {
                    continue;
                };
                if let Some(bound) = crate::var_resolve::canonical_binding_value_key(&target) {
                    let symbol = self.interner.intern(&bound);
                    if source_symbols
                        .insert(name.clone(), symbol)
                        .is_some_and(|old| old != symbol)
                    {
                        source_symbols.remove(&name);
                        self.unknown_source_names.insert(name.clone());
                    }
                }
            }
        }
    }
}
