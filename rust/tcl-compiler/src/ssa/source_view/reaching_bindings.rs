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

//! What reaches or survives at the selected point: reaching bindings, external mutability, the contents a store leaves and literal read coverage.

use super::super::construction::represented_contents_origins;
use super::super::{SsaReadReference, Symbol, ValueKey, Version};
use super::SsaSourceView;
use crate::var_resolve::{ContentsOrigin, VariableCellKey, canonical_variable_key};
use std::collections::{BTreeSet, HashMap, HashSet};
use tcl_registry::CommandRegistry;

impl<'a> SsaSourceView<'a> {
    /// Readonly literal source fact, not a represented cell or raw SSA value.
    /// Missing graph custody refuses; incomplete source coverage stays unknown.
    pub(crate) fn original_script_literal_read_coverage(
        self,
        key: Option<ValueKey>,
    ) -> Option<bool> {
        let (block, index) = self.point?;
        let body = self.ssa.blocks.get(&block)?;
        let held = self
            .ssa
            .source_topology
            .get(&block)?
            .standalone_statements
            .as_ref()?;
        if body.statements.len() != held.len()
            || body
                .statements
                .iter()
                .zip(held.iter())
                .any(|(current, original)| &current.statement != original)
        {
            return None;
        }
        let Some((symbol, version)) = key else {
            return Some(false);
        };
        let Some(read) = self.ssa.original_script_points.get(&(block, index)) else {
            return Some(false);
        };
        let current = body.statements.get(index)?;
        if read.symbol != symbol
            || read.used != Some(version)
            || read.access.is_definition()
            || current.uses.get(&symbol) != read.used.as_ref()
        {
            return None;
        }
        for prior in (0..index).rev() {
            let Some(write) = self.ssa.original_script_points.get(&(block, prior)) else {
                return Some(false);
            };
            // Only closed, quiet, entirely literal Set source steps survive.
            // Any read/callback/unknown/array/multi-name/synthetic step stops.
            if write.access != crate::registry_invocation::AuthoredSourceVariableAccess::Write {
                return Some(false);
            }
            let statement = body.statements.get(prior)?;
            if write.defined.is_none()
                || statement.defs.get(&write.symbol) != write.defined.as_ref()
                || statement.may_defs.contains(&write.symbol)
            {
                return None;
            }
            if write.symbol == symbol {
                return Some(write.defined == Some(version));
            }
            // Distinct interned source identities have different counted
            // namespace slots, under each step's proved alias/trace closure.
        }
        Some(false)
    }

    /// Whether explicit external mutation dependencies cover this physical cell.
    /// Canonical storage keys and names resolved in the retained environment
    /// at this exact point can match; diagnostic display labels cannot. The
    /// sparse operand inventory is not evidence that an alias is absent.
    /// Missing point environments or unresolved dependencies retain uncertainty.
    #[must_use]
    pub fn externally_mutable_by(
        self,
        symbol: Symbol,
        names: &HashSet<String>,
        dynamic_trace: bool,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        self.externally_mutable_given(
            symbol,
            names,
            dynamic_trace,
            self.ssa.has_opaque_native_accesses(),
            registry,
        )
    }

    /// [`Self::externally_mutable_by`] with whether the function's running
    /// code has opaque native accesses already decided by the caller.
    #[must_use]
    pub fn externally_mutable_given(
        self,
        symbol: Symbol,
        names: &HashSet<String>,
        dynamic_trace: bool,
        opaque_native_accesses: bool,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        let key = self.ssa.cell_key(symbol)?;
        if opaque_native_accesses
            || dynamic_trace
            || key.is_native_error_storage(tcl_registry::InvocationDialect::of_profile(
                registry
                    .profile()
                    .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
            ))
            || key
                .authored_spelling()
                .is_some_and(|name| names.contains(name))
        {
            return Some(true);
        }
        if names.is_empty() {
            return Some(false);
        }
        self.source_names()?;
        let (block, index) = self.point?;
        let points = self.ssa.point_contexts.as_ref()?;
        let context = if index == usize::MAX {
            points.before_terminator(block)
        } else {
            points.before_statement(block, index)
        };
        let mut unknown = false;
        for name in names {
            if let Some(&target) = self.ssa.cell_to_symbol.get(&VariableCellKey::from(name)) {
                if target == symbol {
                    return Some(true);
                }
                continue;
            }
            if let Some(bound) = canonical_variable_key(name, context, registry) {
                if &bound == key {
                    return Some(true);
                }
            } else {
                unknown = true;
            }
        }
        (!unknown).then_some(false)
    }

    /// Whether this statement's represented value survives its successful
    /// store callbacks in the same physical cell. This proves only its normal
    /// successor contents, and grants no future alias, lifetime or purity proof.
    #[must_use]
    pub fn normal_store_contents_preserved(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(key) = self.ssa.cell_key(symbol) else {
            return false;
        };
        let Some((block, index)) = self.point else {
            return false;
        };
        let Some(statement) = self
            .ssa
            .blocks
            .get(&block)
            .and_then(|body| body.statements.get(index))
        else {
            return false;
        };
        if !statement.defs.contains_key(&symbol) {
            return false;
        }
        let Some(points) = self.ssa.point_contexts.as_ref() else {
            return false;
        };
        if points.context_before(block, index).is_none() {
            return false;
        }
        let Some(tokens) = self.source_tokens() else {
            return false;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let captured = binding.normal_variable_continuation();
        let after = captured.unwrap_or_else(|| points.after_statement(block, index));
        crate::place_bridge::def_places_at(&statement.statement, block, index, points, registry)
            .iter()
            .any(|place| {
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
                    eprintln!("ORIGINAL_STORE_CONTENTS site={} captured={} cell_matches={} dynamic={} presence={:?} generation_current={} source_matches={} offset_matches={}",
                        site.offset, captured.is_some(),
                        crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(key),
                        place.dynamic, if captured.is_some() { after.captured_contents_presence(place) } else { after.contents_presence(place) },
                        place.cell.as_ref().is_some_and(|cell| cell.generation != crate::place::CellGeneration::Unknown
                            && after.generations.get(&crate::var_resolve::cell_key(place)).copied().unwrap_or_default() == cell.generation),
                        after.contents_have_source(place, &site.source),
                        after.contents_origin(place) == crate::var_resolve::ContentsOrigin::WrittenAt(site.offset));
                }
                crate::var_resolve::canonical_binding_value_key(place).as_ref()
                    == Some(key)
                    && !place.dynamic
                    && if captured.is_some() {
                        after.captured_contents_presence(place)
                    } else {
                        after.contents_presence(place)
                    } == crate::var_resolve::ContentsPresence::Defined
                    && place.cell.as_ref().is_some_and(|cell| {
                        !matches!(cell.generation, crate::place::CellGeneration::Unknown)
                            && after
                                .generations
                                .get(&crate::var_resolve::cell_key(place))
                                .copied()
                                .unwrap_or(crate::place::CellGeneration::Incoming)
                                == cell.generation
                    })
                    && after.contents_have_source(place, &site.source)
                    && after.contents_origin(place)
                        == crate::var_resolve::ContentsOrigin::WrittenAt(site.offset)
            })
    }

    /// Conditional contents of this exact represented Logical scalar setter.
    /// The actual source continuation owns the literal, cell and write site;
    /// this grants no Native store, completed read, Normal or erasure receipt.
    pub(crate) fn conditional_logical_scalar_store_contents(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> Option<String> {
        let key = self.ssa.cell_key(symbol)?;
        let (block, index) = self.point?;
        let statement = self.ssa.blocks.get(&block)?.statements.get(index)?;
        if statement.defs.len() != 1 || !statement.defs.contains_key(&symbol) {
            return None;
        }
        let points = self.ssa.point_contexts.as_ref()?;
        let store = points.conditional_logical_scalar_store_at(block, index, registry)?;
        let place = store.definition_place(registry)?;
        (crate::var_resolve::canonical_binding_value_key(&place).as_ref() == Some(key))
            .then(|| store.value().map(str::to_owned))
            .flatten()
    }

    /// Resolve contents reaching this boundary without inventing a lexical read.
    /// The name is a lookup input; cell identity, lifetime, observers and represented
    /// store provenance determine whether its reaching version is usable.
    #[must_use]
    pub fn reaching_binding(
        self,
        name: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadReference> {
        self.reaching_binding_with_origins(
            name,
            registry,
            &represented_contents_origins(&self.ssa.blocks, &self.ssa.value_clobbers),
        )
    }

    fn reaching_binding_with_origins(
        self,
        name: &str,
        registry: &CommandRegistry,
        origins: &HashMap<(Symbol, Version), ContentsOrigin>,
    ) -> Option<SsaReadReference> {
        let (block_id, index) = self.point?;
        let block = self.ssa.blocks.get(&block_id)?;
        let points = self.ssa.point_contexts.as_ref()?;
        if index != usize::MAX {
            block.statements.get(index)?;
        }
        let context = points.context_before(block_id, index)?;
        let place = crate::var_resolve::resolve_literal_access(
            name,
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let canonical = crate::var_resolve::canonical_binding_value_key(&place)?;
        let symbol = self.ssa.cell_to_symbol.get(&canonical).copied()?;
        let candidate = if index == usize::MAX {
            block.exit_versions.get(&symbol).copied().unwrap_or(0)
        } else {
            block.statements[..index]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, statement)| {
                    self.ssa
                        .value_clobbers
                        .get(&block_id)
                        .and_then(|markers| markers.get(&index))
                        .and_then(|versions| versions.get(&symbol))
                        .map(|&(_, fresh)| fresh)
                        .or_else(|| statement.defs.get(&symbol).copied())
                })
                .or_else(|| block.entry_versions.get(&symbol).copied())
                .unwrap_or(0)
        };
        let origin = context.read_contents_origin(&place, registry);
        let represented = if candidate == 0 {
            Some(ContentsOrigin::Incoming)
        } else {
            origins.get(&(symbol, candidate)).cloned()
        };
        let version = (!place.observed
            && context.contents_presence(&place) == crate::var_resolve::ContentsPresence::Defined
            && !context.store_would_error(&place)
            && origin != ContentsOrigin::Unknown
            && represented.as_ref() == Some(&origin))
        .then_some(candidate);
        Some(SsaReadReference { symbol, version })
    }

    /// Candidate names from retained source operands, resolved afresh at this boundary.
    /// This inventory grants no whole-function binding or contents consensus.
    pub fn reaching_bindings(
        self,
        registry: &CommandRegistry,
    ) -> impl Iterator<Item = (&'a str, SsaReadReference)> {
        let names: BTreeSet<_> = self
            .ssa
            .point_symbols
            .values()
            .flat_map(|names| names.keys().map(String::as_str))
            .collect();
        let origins = represented_contents_origins(&self.ssa.blocks, &self.ssa.value_clobbers);
        names.into_iter().filter_map(move |name| {
            self.reaching_binding_with_origins(name, registry, &origins)
                .map(|binding| (name, binding))
        })
    }
}
