// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One original-word projection for a compiler-selected private handler.

use super::{Arc, SourceExecutionContext};
use crate::registry_invocation::EffectiveInvocationWord;

pub(super) struct ProjectedNamedArguments {
    pub(super) words: Vec<crate::ir::WordExpr>,
    pub(super) effective: Vec<EffectiveInvocationWord>,
    arguments: Option<Vec<EffectiveInvocationWord>>,
    values: Option<Vec<Option<Arc<super::native_result::EvaluatedSourceValue>>>>,
    representations: Option<Vec<Option<super::source_representation::FrozenSourceRepresentation>>>,
    objects: Option<Vec<Option<Arc<super::SourceObjectInstanceProof>>>>,
    prefixes: Option<Vec<Option<Arc<super::SourceCapturedMethodPrefix>>>>,
    reads: Option<Vec<Option<Arc<super::argument_reads::FrozenSourceArgumentRead>>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectionError {
    MissingHead,
    ConsumedWordsUnavailable,
    SidecarMisaligned,
    EffectiveWordsMisaligned,
}

fn project<T: Clone>(words: &[T], consumed: usize) -> Result<Vec<T>, ProjectionError> {
    let first = words.first().ok_or(ProjectionError::MissingHead)?;
    let start = consumed
        .checked_add(1)
        .ok_or(ProjectionError::ConsumedWordsUnavailable)?;
    let tail = words
        .get(start..)
        .ok_or(ProjectionError::ConsumedWordsUnavailable)?;
    Ok(std::iter::once(first.clone())
        .chain(tail.iter().cloned())
        .collect())
}

fn project_optional<T: Clone>(
    words: Option<&[T]>,
    count: usize,
    consumed: usize,
) -> Result<Option<Vec<T>>, ProjectionError> {
    match words {
        Some(words) if words.len() == count => Ok(Some(project(words, consumed)?)),
        Some(_) => Err(ProjectionError::SidecarMisaligned),
        None => Ok(None),
    }
}

impl ProjectedNamedArguments {
    /// Retain original sites and frozen receipts together. Selection consumes
    /// written words; known expansion is flattened only after that projection.
    #[inline(never)]
    pub(super) fn capture(
        words: &[crate::ir::WordExpr],
        effective: &[EffectiveInvocationWord],
        context: &SourceExecutionContext<'_>,
        consumed: usize,
    ) -> Result<Box<Self>, ProjectionError> {
        let count = words.len();
        let arguments = project_optional(context.written_arguments, count, consumed)?;
        let effective = match &arguments {
            Some(arguments) => arguments
                .iter()
                .flat_map(super::frozen_arguments::runtime_words)
                .collect(),
            None if effective.len() == count => project(effective, consumed)?,
            None => return Err(ProjectionError::EffectiveWordsMisaligned),
        };
        Ok(Box::new(Self {
            words: project(words, consumed)?,
            effective,
            arguments,
            values: project_optional(context.written_values, count, consumed)?,
            representations: project_optional(context.written_representations, count, consumed)?,
            objects: project_optional(context.written_objects, count, consumed)?,
            prefixes: project_optional(context.written_method_prefixes, count, consumed)?,
            reads: project_optional(context.written_variable_reads, count, consumed)?,
        }))
    }

    pub(super) fn context<'a>(
        &'a self,
        original: &SourceExecutionContext<'a>,
    ) -> Box<SourceExecutionContext<'a>> {
        Box::new(SourceExecutionContext {
            selected_compilation: Some(
                &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
            ),
            written_arguments: self.arguments.as_deref(),
            written_values: self.values.as_deref(),
            written_representations: self.representations.as_deref(),
            written_objects: self.objects.as_deref(),
            written_method_prefixes: self.prefixes.as_deref(),
            written_variable_reads: self.reads.as_deref(),
            ..*original
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_projection_distinguishes_absence_alignment_and_rejection() {
        assert_eq!(project_optional::<u8>(None, 3, 1), Ok(None));
        assert_eq!(
            project_optional(Some(&[1, 2, 3][..]), 3, 1),
            Ok(Some(vec![1, 3]))
        );
        assert_eq!(
            project_optional(Some(&[1, 2][..]), 3, 1),
            Err(ProjectionError::SidecarMisaligned)
        );
        assert_eq!(project(&[1, 2][..], 1), Ok(vec![1]));
        assert_eq!(
            project(&[1, 2][..], 2),
            Err(ProjectionError::ConsumedWordsUnavailable)
        );
        assert_eq!(project::<u8>(&[], 0), Err(ProjectionError::MissingHead));
    }
}
