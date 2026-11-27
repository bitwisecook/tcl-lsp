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

//! The original arguments of a procedure declaration: formals, bindings and operands.

use super::OriginalOperandSource;
use super::procedure::original_procedure_source_descriptor;
use crate::analyser::AnalysisResult;
use tcl_registry::{CommandRegistry, InvocationArguments};

/// Readonly source argument topology of the actual retained procedure
/// implementation. Formal bindings describe argv ordinals only; they do not
/// install a frame, read a cell, prepare a native call or permit an edit.
pub struct OriginalProcedureArguments<'a> {
    pub(super) declaration: &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
        crate::analyser::ProcDef,
    >,
    pub(super) formals: crate::signature_scan::formal_parameters::SignatureSourceFormalParameters,
    pub(super) bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    pub(super) arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    pub(super) operands: Vec<Option<OriginalOperandSource>>,
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) registry: tcl_registry::RegistrySemanticKey,
}

impl<'a> OriginalProcedureArguments<'a> {
    /// Canonical immutable declaration selected by the implementation allocation.
    #[must_use]
    pub const fn declaration(
        &self,
    ) -> &'a crate::signature_scan::original_name::SourceDeclarationMetadata<crate::analyser::ProcDef>
    {
        self.declaration
    }
    /// Actual original `ParamList` and its independently selected source grammar.
    #[must_use]
    pub const fn formals(
        &self,
    ) -> &crate::signature_scan::formal_parameters::SignatureSourceFormalParameters {
        &self.formals
    }
    /// Required/default/rest/caller-link syntax at this exact effective argc.
    #[must_use]
    pub fn bindings(&self) -> &[tcl_syntax::formal_params::FormalByteArgumentBinding] {
        &self.bindings
    }
    /// Effective post-head values, with captured prefix slots preserved.
    #[must_use]
    pub fn arguments(&self) -> &[crate::registry_invocation::EffectiveInvocationWord] {
        &self.arguments
    }
    /// Source anchors of those exact effective values; captured operands have none.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Exact immutable complete source/channel and lexer configuration.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.image == *image && self.config == config && self.formals.matches_source(image, config)
    }
    /// Registry generation used for the independently selected formal grammar.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

/// Match an actual original procedure reference to its same-document immutable
/// declaration and exact effective argv. Alias prefixes and expansion children
/// retain their own origins; unknown lookup, foreign formals, ambiguous records,
/// unavailable child extents and changed source cannot acquire another anchor.
#[must_use]
pub fn original_procedure_arguments<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Option<OriginalProcedureArguments<'a>> {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    let descriptor = original_procedure_source_descriptor(source, analysis, segment)?;
    let words = descriptor
        .arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect::<Vec<_>>();
    let count = InvocationArguments::structured(&words).exact_argv_len()?;
    let bindings = descriptor.formals.bindings(count).ok()?;
    Some(OriginalProcedureArguments {
        declaration: descriptor.declaration,
        formals: descriptor.formals,
        bindings,
        arguments: descriptor.arguments,
        operands: descriptor.operands,
        image: descriptor.image,
        config: descriptor.config,
        registry: descriptor.registry,
    })
}
