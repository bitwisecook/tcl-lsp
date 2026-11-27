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

//! Taint of original operand reads: reference roots, sink arguments and the positioned `SourceWarningRead` projection.

use super::super::{
    HashMap, SinkCall, SourceWarningRead, TaintLattice, TaintScan, ValueKey, WordExpr, var_taint,
};

pub(in super::super) fn original_scalar_reference_name(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
) -> Option<&str> {
    let (reference, _) = word.sole_variable_substitution()?;
    let variable = tcl_lexer::word_parts::whole_var_ref(reference.as_bytes(), config).ok()??;
    if variable.index.is_some()
        || tcl_syntax::naming::split_element_ref_bytes(variable.name).is_some()
    {
        return None;
    }
    std::str::from_utf8(variable.name)
        .ok()
        .filter(|name| !name.contains('\0'))
}

pub(in super::super) fn reference_root_with_config(
    reference: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<&str> {
    let root =
        tcl_syntax::naming::variable_reference_root_bytes(reference.as_bytes(), config).ok()??;
    std::str::from_utf8(root)
        .ok()
        .filter(|name| !name.contains('\0'))
}

/// Taint of an original operand read. A missing SSA use remains an unknown
/// value only when the retained word identifies that exact variable read.
/// This projection creates no SSA symbol or variable-presence proof.
pub(in super::super) fn operand_read_taint<S: std::hash::BuildHasher>(
    env: &TaintScan<'_, S>,
    name: &str,
    original: Option<&WordExpr>,
) -> Option<TaintLattice> {
    // An unproved dispatch selects no SSA version, so it may read any version
    // of the variable or any withdrawn-world write of it.
    if let Some(writes) = env.withdrawn_writes.filter(|_| original.is_some()) {
        let written = writes
            .get(name)
            .copied()
            .unwrap_or_else(TaintLattice::clean);
        return Some(
            env.taints
                .iter()
                .filter(|((held, _), _)| env.ssa.var_name(*held) == name)
                .fold(written, |taint, (_, held)| taint.join(*held)),
        );
    }
    let versioned = env
        .ssa
        .var_symbol(name)
        .is_some_and(|symbol| env.uses.contains_key(&symbol));
    if let Some(hosted) = env.source_read {
        return hosted.taint(name, original?, env.taints);
    }
    if versioned {
        let value = var_taint(name, env.uses, env.taints, env.ssa);
        if value.is_conservative_global_seed_only() {
            return None;
        }
        return Some(value);
    }
    let (reference, _) = original?.sole_variable_substitution()?;
    (reference_root_with_config(reference, env.source.config) == Some(name))
        .then_some(TaintLattice::tainted())
}

pub(super) fn original_sink_argument<'a>(
    call: &SinkCall<'a>,
    index: usize,
) -> Option<&'a WordExpr> {
    if let Some(effective) = call.effective {
        let effective_index = call.argument_offset.checked_add(index)?.checked_add(1)?;
        match effective.origins.get(effective_index)? {
            crate::registry_invocation::InvocationWordOrigin::Written(_) => {
                effective.words.get(effective_index)
            }
            _ => None,
        }
    } else {
        call.tokens?.words().get(index.checked_add(1)?)
    }
}

pub(in super::super) enum SourceWarningMetadata {
    Authored(Box<crate::registry_invocation::AuthoredSourceTaintInvocation>),
    Hosted(Box<crate::registry_invocation::OriginalConditionalVendorRegistryMetadata>),
    Native(Box<crate::registry_invocation::NativeSourceTaintInvocation>),
    Original(Box<crate::registry_invocation::OriginalSourceTaintReadOperands>),
    WithOriginalReads {
        invocation: Box<SourceWarningMetadata>,
        reads: Box<crate::registry_invocation::OriginalSourceTaintReadOperands>,
    },
}

impl SourceWarningMetadata {
    pub(super) fn original_reads(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceTaintReadOperands> {
        match self {
            Self::Original(reads) | Self::WithOriginalReads { reads, .. } => Some(reads),
            Self::Authored(_) | Self::Hosted(_) | Self::Native(_) => None,
        }
    }

    pub(super) fn invocation_metadata(&self) -> &Self {
        match self {
            Self::WithOriginalReads { invocation, .. } => invocation.invocation_metadata(),
            other => other,
        }
    }
}

/// Supplemental source geometry is independent of the selected invocation.
/// Only Unowned may use that invocation's existing authenticated read domain.
enum OriginalReadTaintProjection {
    Unowned,
    Refused,
    NoRead,
    Value(TaintLattice),
}

impl<'a> SourceWarningRead<'a> {
    fn project_original_read<S: std::hash::BuildHasher>(
        self,
        name: &str,
        original: &WordExpr,
        taints: &HashMap<ValueKey, TaintLattice, S>,
    ) -> OriginalReadTaintProjection {
        let Some(operands) = self.metadata.original_reads() else {
            return OriginalReadTaintProjection::Unowned;
        };
        if !operands.matches_source_point(self.point) {
            return OriginalReadTaintProjection::Refused;
        }
        let reads = match operands.read_sites(original, name) {
            crate::registry_invocation::OriginalSourceReadSites::Unowned => {
                return OriginalReadTaintProjection::Unowned;
            }
            crate::registry_invocation::OriginalSourceReadSites::Unknown => {
                return OriginalReadTaintProjection::Value(TaintLattice::tainted());
            }
            crate::registry_invocation::OriginalSourceReadSites::Known(reads) => reads,
        };
        if reads.is_empty() {
            return OriginalReadTaintProjection::NoRead;
        }
        OriginalReadTaintProjection::Value(reads.into_iter().fold(
            TaintLattice::clean(),
            |taint, (reference, site)| {
                let value = self
                    .point
                    .read_reference(site, reference)
                    .and_then(|read| read.version.map(|version| (read.symbol, version)))
                    .and_then(|key| taints.get(&key).copied())
                    .unwrap_or_else(TaintLattice::tainted);
                taint.join(value)
            },
        ))
    }

    pub(super) fn script_arguments(self, call: &SinkCall<'_>) -> Option<&'a [usize]> {
        let operands = self.metadata.original_reads()?;
        operands.matches_source_point(self.point).then_some(())?;
        operands.script_arguments(call.command, call.effective?)
    }

    pub(in super::super) fn references(
        self,
        name: &str,
        original: &WordExpr,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        if let Some(operands) = self.metadata.original_reads() {
            if !operands.matches_source_point(self.point) {
                return false;
            }
            match operands.read_sites(original, name) {
                crate::registry_invocation::OriginalSourceReadSites::Unowned => {}
                crate::registry_invocation::OriginalSourceReadSites::Unknown => return true,
                crate::registry_invocation::OriginalSourceReadSites::Known(reads) => {
                    return !reads.is_empty();
                }
            }
        }
        if !matches!(
            self.metadata.invocation_metadata(),
            SourceWarningMetadata::Native(_)
                | SourceWarningMetadata::Hosted(_)
                | SourceWarningMetadata::Authored(_)
        ) {
            return false;
        }
        let config = if let SourceWarningMetadata::Authored(selected) =
            self.metadata.invocation_metadata()
        {
            if !selected.matches_point(self.point) || !selected.owns_operand(original) {
                return false;
            }
            selected.config()
        } else {
            config
        };
        original
            .sole_variable_substitution()
            .is_some_and(|(reference, _)| {
                reference_root_with_config(reference, config) == Some(name)
            })
    }

    pub(in super::super) fn taint<S: std::hash::BuildHasher>(
        self,
        name: &str,
        original: &WordExpr,
        taints: &HashMap<ValueKey, TaintLattice, S>,
    ) -> Option<TaintLattice> {
        match self.project_original_read(name, original, taints) {
            OriginalReadTaintProjection::Value(value) => return Some(value),
            OriginalReadTaintProjection::Refused | OriginalReadTaintProjection::NoRead => {
                return None;
            }
            OriginalReadTaintProjection::Unowned => {}
        }
        let (reference, site) = original.sole_variable_substitution()?;
        let (config, owns_word) = match self.metadata.invocation_metadata() {
            SourceWarningMetadata::Authored(selected) => (
                selected.config(),
                selected.matches_point(self.point) && selected.owns_operand(original),
            ),
            SourceWarningMetadata::Hosted(metadata) => (
                metadata.shape().original_words().first()?.config(),
                metadata
                    .shape()
                    .original_words()
                    .iter()
                    .any(|word| word.span() == original.source().span),
            ),
            SourceWarningMetadata::Native(selected) => (
                selected.words.original_argument_word(0)?.config(),
                (0..selected.words.arguments().len()).any(|argument| {
                    selected
                        .words
                        .original_argument_word(argument)
                        .is_some_and(|word| word.span() == original.source().span)
                }),
            ),
            // An absent supplemental domain can fall through only to the
            // unchanged authenticated Native/hosted invocation owner.
            SourceWarningMetadata::Original(_)
            | SourceWarningMetadata::WithOriginalReads { .. } => return None,
        };
        if reference_root_with_config(reference, config) != Some(name) || !owns_word {
            return None;
        }
        if let SourceWarningMetadata::Authored(selected) = self.metadata.invocation_metadata()
            && !selected.uses_symbolic_body_reads()
        {
            let key = self
                .point
                .read_reference(site, reference)
                .and_then(|read| read.version.map(|version| (read.symbol, version)));
            return selected
                .literal_read_coverage(self.point, key)
                .map(|complete| {
                    if complete {
                        TaintLattice::clean()
                    } else {
                        TaintLattice::tainted()
                    }
                });
        }
        // Only the exact positioned read receipt can select represented
        // contents. Missing cells or versions retain an unknown MAY value;
        // no SSA symbol, presence, effect or Normal receipt is created here.
        let read = self.point.read_reference(site, reference);
        let key = match self.metadata.invocation_metadata() {
            SourceWarningMetadata::Authored(_)
            | SourceWarningMetadata::Hosted(_)
            | SourceWarningMetadata::Original(_) => {
                read.and_then(|read| read.version.map(|version| (read.symbol, version)))
            }
            SourceWarningMetadata::Native(_) => {
                self.point.read_occurrence_reference(site, reference)
            }
            SourceWarningMetadata::WithOriginalReads { .. } => return None,
        };
        let value = key.and_then(|key| taints.get(&key).copied());
        Some(value.unwrap_or_else(TaintLattice::tainted))
    }
}
