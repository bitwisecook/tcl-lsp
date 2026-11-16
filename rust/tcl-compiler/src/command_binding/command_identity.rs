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

//! Command identity: the token that survives slot movement and the allocation that created it.

use super::namespace_context::SourceNamespaceKey;
use super::source_origin::{SourceOriginId, SourceOriginKind};
use std::hash::Hash;
use std::sync::Arc;

/// Object identity survives movement of its command-table slot. Imports keep
/// this identity; interpreter aliases perform target-name lookup instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandIdentity {
    /// Actual interpreter token supplied by a live runtime entry snapshot.
    pub runtime: Option<RuntimeCommandTokenIdentity>,
    /// Slot in which this token was originally created.
    pub origin: String,
    /// Source creation site; zero denotes an explicit entry implementation.
    pub declaration: u32,
    /// Allocation identity; source offsets alone do not identify derived code
    /// or a command recreated by another execution of the same source site.
    pub allocation: Option<CommandAllocation>,
}

/// Stable runtime command identity; generations belong to implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeCommandTokenIdentity {
    /// Interpreter owner and non-reused interpreter arena identity.
    pub interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    /// Non-reused command token within the interpreter.
    pub token: u64,
}

pub(super) type CommandToken = CommandIdentity;

/// An allocation instruction in one authored or materialised source instance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandAllocationSite {
    /// Source instance; derived offsets are never authored document offsets.
    pub source: Arc<SourceOriginId>,
    /// Offset within that source instance.
    pub offset: u32,
}

impl CommandAllocationSite {
    /// Compact storage projection for this exact source allocation instruction.
    /// Callers retain the typed site as the equality proof and append their
    /// allocation incarnation and implementation generation independently.
    #[must_use]
    pub fn variable_storage_identity(&self) -> String {
        format!(
            "source:{}:{}",
            SourceOriginId::activation_key(&self.source),
            self.offset
        )
    }
}

/// Bounded incarnation of an allocation instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AllocationIncarnation {
    /// First reached execution.
    First,
    /// Second reached execution, distinct from an earlier captured token.
    Second,
    /// Further executions; no unique runtime token is asserted.
    RepeatedFresh,
}

/// Command allocation, distinct from its current name and implementation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandAllocation {
    /// Instruction allocating the token or implementation.
    pub site: CommandAllocationSite,
    /// Repeated executions must not alias a prior captured runtime identity.
    pub incarnation: AllocationIncarnation,
    /// Presentation of the evaluated output slot, without lookup authority.
    pub command: String,
    /// Exact namespace owning the allocation, independent of its presentation.
    pub namespace: SourceNamespaceKey,
}

impl CommandAllocation {
    /// Correlate a reached implementation with an analysed declaration's bytes
    /// and offset. Loaded files retain their own identity; this value-purpose
    /// match grants no edit, native compiler, frame or object-erasure licence.
    #[must_use]
    pub fn matches_source_declaration(&self, source: &str, offset: u32) -> bool {
        self.matches_source_declaration_image(&tcl_lexer::SourceImage::document(source), offset)
    }

    /// Match the exact source bytes and input channel of a declaration.
    #[must_use]
    pub fn matches_source_declaration_image(
        &self,
        source: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> bool {
        self.site.offset == offset
            && self.incarnation != AllocationIncarnation::RepeatedFresh
            && match self.site.source.kind() {
                SourceOriginKind::Authored(text) => text == source,
                SourceOriginKind::Loaded {
                    source: text,
                    implementation_id,
                    ..
                } => !implementation_id.is_empty() && text == source,
                SourceOriginKind::InheritedPoint { .. }
                | SourceOriginKind::Derived { .. }
                | SourceOriginKind::ConditionalField { .. } => false,
            }
    }
}
