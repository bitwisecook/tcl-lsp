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

//! Which code-generation hook, semantic operation and native lowering a command or subcommand selects under one dialect.

use super::CommandSpec;
use super::subcommand::SubCommand;
use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::stamp_window::StampSelection;
use tcl_dialect::model::SurfaceQuery;

impl CommandSpec {
    /// The native lowering shape this command's invocations take, defaulting
    /// to the generic argv invocation when no descriptor is stamped. The plain
    /// field alone: [`Self::native_lowering_at`] reads the windows too.
    #[must_use]
    pub const fn native_lowering(&self) -> crate::native_lowering::NativeLowering {
        match self.native_lowering {
            Some(shape) => shape,
            None => crate::native_lowering::NativeLowering::Generic,
        }
    }

    /// What this command says about its codegen hook at the point `query` asks
    /// about: the first [`Self::codegen_hook_windows`] window covering the
    /// primary release, the unversioned [`Self::codegen_hook`] where none does,
    /// and a decline where the point does not settle which.
    #[must_use]
    pub fn codegen_hook_selection(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<CodegenHookId> {
        StampSelection::of(self.codegen_hook, self.codegen_hook_windows, query)
    }

    /// [`Self::codegen_hook_selection`] as the hook to act on, if any.
    #[must_use]
    pub fn codegen_hook_at(&self, query: Option<&SurfaceQuery<'_>>) -> Option<CodegenHookId> {
        self.codegen_hook_selection(query).stamp()
    }

    /// [`Self::codegen_hook_selection`] for the inline codegen hook.
    #[must_use]
    pub fn inline_codegen_hook_selection(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<InlineCodegenHookId> {
        StampSelection::of(
            self.inline_codegen_hook,
            self.inline_codegen_hook_windows,
            query,
        )
    }

    /// [`Self::inline_codegen_hook_selection`] as the hook to act on, if any.
    #[must_use]
    pub fn inline_codegen_hook_at(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> Option<InlineCodegenHookId> {
        self.inline_codegen_hook_selection(query).stamp()
    }

    /// [`Self::codegen_hook_selection`] for the semantic operation.
    #[must_use]
    pub fn semantic_operation_selection(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<crate::semantic_operation::SemanticOperationId> {
        StampSelection::of(
            self.semantic_operation,
            self.semantic_operation_windows,
            query,
        )
    }

    /// The native lowering shape this command's invocations take at the point
    /// `query` asks about, the generic argv invocation where it states none or
    /// the point does not settle which. [`Self::native_lowering`] is the
    /// unversioned field alone.
    #[must_use]
    pub fn native_lowering_at(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> crate::native_lowering::NativeLowering {
        StampSelection::of(self.native_lowering, self.native_lowering_windows, query)
            .stamp()
            .unwrap_or(crate::native_lowering::NativeLowering::Generic)
    }

    /// Every `(semantic operation, codegen hook, inline codegen hook)` this
    /// command's own descriptors could resolve to at some release: the
    /// unversioned stamps and each window's, in every combination. What a
    /// surface-blind question asks of a command that may be stamped
    /// differently at different releases.
    pub(crate) fn descriptor_combinations(
        &self,
    ) -> impl Iterator<
        Item = (
            Option<crate::semantic_operation::SemanticOperationId>,
            Option<CodegenHookId>,
            Option<InlineCodegenHookId>,
        ),
    > + '_ {
        descriptor_combinations(
            (self.semantic_operation, self.semantic_operation_windows),
            (self.codegen_hook, self.codegen_hook_windows),
            (self.inline_codegen_hook, self.inline_codegen_hook_windows),
        )
    }

    /// All target-neutral intrinsic identities declared anywhere in this
    /// command's command, subcommand, or invocation-form descriptors, plain or
    /// in a stamp window.
    ///
    /// The result is deduplicated and ordered by [`crate::IntrinsicId`], so a
    /// runtime can attach every semantic identity to one live implementation
    /// without knowing the command's subcommand layout.
    #[must_use]
    pub fn intrinsic_ids(&self) -> Vec<crate::IntrinsicId> {
        let mut ids = std::collections::BTreeSet::new();
        let mut add = |semantic, lowering, codegen, inline_codegen| {
            if let Some(id) =
                crate::IntrinsicId::from_descriptors(semantic, lowering, codegen, inline_codegen)
            {
                ids.insert(id);
            }
        };
        for (semantic, codegen, inline_codegen) in self.descriptor_combinations() {
            add(semantic, self.lowering_hook, codegen, inline_codegen);
        }
        for form in self.command_forms {
            add(
                form.semantic_operation,
                form.lowering_hook,
                form.codegen_hook,
                None,
            );
        }
        for subcommand in self.subcommands {
            for (semantic, codegen, inline_codegen) in subcommand.descriptor_combinations() {
                add(semantic, subcommand.lowering_hook, codegen, inline_codegen);
            }
            for form in subcommand.subcommand_forms {
                add(
                    form.semantic_operation,
                    form.lowering_hook,
                    form.codegen_hook,
                    None,
                );
            }
        }
        ids.into_iter().collect()
    }
}

/// The product of three levels' candidate stamps. See
/// [`CommandSpec::descriptor_combinations`].
pub(super) fn descriptor_combinations<'a>(
    semantic: (
        Option<crate::semantic_operation::SemanticOperationId>,
        &'a [crate::stamp_window::StampWindow<crate::semantic_operation::SemanticOperationId>],
    ),
    codegen: (
        Option<CodegenHookId>,
        &'a [crate::stamp_window::StampWindow<CodegenHookId>],
    ),
    inline_codegen: (
        Option<InlineCodegenHookId>,
        &'a [crate::stamp_window::StampWindow<InlineCodegenHookId>],
    ),
) -> impl Iterator<
    Item = (
        Option<crate::semantic_operation::SemanticOperationId>,
        Option<CodegenHookId>,
        Option<InlineCodegenHookId>,
    ),
> + 'a {
    use crate::stamp_window::candidates;
    candidates(semantic.0, semantic.1).flat_map(move |semantic| {
        candidates(codegen.0, codegen.1).flat_map(move |codegen| {
            candidates(inline_codegen.0, inline_codegen.1)
                .map(move |inline_codegen| (semantic, codegen, inline_codegen))
        })
    })
}

impl SubCommand {
    /// The subcommand's own descriptor combinations; see
    /// [`CommandSpec::descriptor_combinations`].
    pub(crate) fn descriptor_combinations(
        &self,
    ) -> impl Iterator<
        Item = (
            Option<crate::semantic_operation::SemanticOperationId>,
            Option<CodegenHookId>,
            Option<InlineCodegenHookId>,
        ),
    > + '_ {
        descriptor_combinations(
            (self.semantic_operation, self.semantic_operation_windows),
            (self.codegen_hook, self.codegen_hook_windows),
            (self.inline_codegen_hook, self.inline_codegen_hook_windows),
        )
    }

    /// What this subcommand says about its codegen hook at the point `query`
    /// asks about; see [`CommandSpec::codegen_hook_selection`]. A subcommand
    /// that states nothing there inherits its command's answer, and one that
    /// declines does not.
    #[must_use]
    pub fn codegen_hook_selection(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<CodegenHookId> {
        StampSelection::of(self.codegen_hook, self.codegen_hook_windows, query)
    }

    /// [`Self::codegen_hook_selection`] for the inline codegen hook.
    #[must_use]
    pub fn inline_codegen_hook_selection(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<InlineCodegenHookId> {
        StampSelection::of(
            self.inline_codegen_hook,
            self.inline_codegen_hook_windows,
            query,
        )
    }

    /// [`Self::codegen_hook_selection`] for the semantic operation.
    #[must_use]
    pub fn semantic_operation_selection(
        &self,
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<crate::semantic_operation::SemanticOperationId> {
        StampSelection::of(
            self.semantic_operation,
            self.semantic_operation_windows,
            query,
        )
    }
}
