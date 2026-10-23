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

//! Exact repeated-read contents evidence for source rewrite schedules.

use crate::ir::{CommandTokens, SourceSite};
use crate::var_resolve::{ContentsOrigin, resolve_substitution_access};
use tcl_registry::{CommandRegistry, InvocationDialect, TraceOperation};

/// Two original reads select the same live, unobserved stored value.
/// This proves read continuity only. Consumers must independently prove the
/// intervening operations and the omitted read's representation effects.
pub(crate) struct RepeatedReadContentsWitness {
    value: String,
    dialect: InvocationDialect,
}

impl RepeatedReadContentsWitness {
    /// Require exact retained reads and closed singleton physical contexts.
    pub(crate) fn prove(
        tokens: &CommandTokens,
        first: &SourceSite,
        second: &SourceSite,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        if first == second
            || first.provenance != crate::ir::Provenance::Source
            || second.provenance != crate::ir::Provenance::Source
        {
            return None;
        }
        let first = tokens.variable_access_for_site(first)?;
        let second = tokens.variable_access_for_site(second)?;
        if first.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed
            || second.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            return None;
        }
        let [before] = first.context_alternatives() else {
            return None;
        };
        let [after] = second.context_alternatives() else {
            return None;
        };
        let place = resolve_substitution_access(
            &first.original_spelling,
            before,
            registry,
            TraceOperation::Read,
        );
        let other = resolve_substitution_access(
            &second.original_spelling,
            after,
            registry,
            TraceOperation::Read,
        );
        if place != other
            || before.binding_identity != crate::var_resolve::BindingIdentity::Bound
            || after.binding_identity != crate::var_resolve::BindingIdentity::Bound
            || place.dynamic
            || place.observed
            || place
                .cell
                .as_ref()
                .is_none_or(|cell| cell.generation == crate::place::CellGeneration::Unknown)
            || !matches!(
                before.read_contents_origin(&place, registry),
                ContentsOrigin::WrittenAt(_)
            )
            || before.store_would_error(&place)
            || after.store_would_error(&other)
            || before.contents_presence(&place) != crate::var_resolve::ContentsPresence::Defined
            || after.contents_presence(&other) != crate::var_resolve::ContentsPresence::Defined
            || before.read_contents_origin(&place, registry)
                != after.read_contents_origin(&other, registry)
            || before.invocation_dialect != after.invocation_dialect
            || before.literal_contents_at(&place, registry)
                != after.literal_contents_at(&other, registry)
        {
            return None;
        }
        Some(Self {
            value: before.literal_contents_at(&place, registry)?.to_owned(),
            dialect: before.invocation_dialect?,
        })
    }

    /// Exact stored bytes; these do not imply a freshly created value object.
    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    /// Actual read policy, independently selected from the editing catalogue.
    pub(crate) const fn dialect(&self) -> InvocationDialect {
        self.dialect
    }
}
