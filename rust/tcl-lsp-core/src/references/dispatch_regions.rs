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

//! The nested script regions a dispatch scan visits, and the census of them per command.

use crate::original_invocation::schema_script_arguments;
use tcl_compiler::analyser::AnalysisResult;

/// Strip a single layer of `{`/`}` delimiters from `span`'s source text, if
/// present.  The analyser's body / member spans are inclusive of the braces,
/// but the segmenter treats a leading `{` as a braced-literal opener and
/// refuses to descend into it, so every re-scanned body needs the braces
/// stripped first.  Out-of-bounds / empty input is returned unchanged — the
/// caller is expected to bounds-check before segmenting.
pub(crate) fn strip_outer_braces(source: &str, span: tcl_lexer::Span) -> (usize, usize) {
    let mut start = span.start() as usize;
    let mut end = span.end() as usize;
    if start >= source.len() || end > source.len() || start > end {
        return (start, end);
    }
    if source.as_bytes().get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    (start, end)
}

/// Recursion bound for [`nested_dispatch_regions`] — mirrors the analyser's
/// own `MAX_BODY_DEPTH` (`tcl_compiler::analyser::commands`): a guard against
/// a stack overflow on pathologically nested / generated / minified Tcl, not
/// a limit any hand-written script should ever approach.
pub(crate) const MAX_DISPATCH_SCAN_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

pub(super) struct RetainedDispatchContext {
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
}

pub(super) fn retained_dispatch_context(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<RetainedDispatchContext> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    Some(RetainedDispatchContext {
        dialect: current.profile(),
        config: current.config(),
        context: current.context(),
    })
}

/// Validated full source context shared by source scans and rename hazards.
/// This does not select an execution frame, target identity or edit permission.
pub(crate) fn dispatch_source_context(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<(
    tcl_lexer::LexerConfig,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
)> {
    let selected = retained_dispatch_context(source, analysis)?;
    Some((selected.config, selected.context))
}

/// Active lexical command substitutions and potential same-frame script bodies
/// at the unchanged original call. The shared source schema retains moved and
/// captured operands, availability, script timing and known command barriers.
/// Conditional source regions grant neither runtime dispatch nor edit authority.
pub(crate) fn nested_dispatch_regions(
    source: &str,
    analysis: &AnalysisResult,
    _dialect: &'static tcl_dialect::DialectProfile,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let Some(RetainedDispatchContext {
        config, context, ..
    }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let mut regions = cmd
        .argv
        .iter()
        .flat_map(|token| {
            crate::executable_regions::command_substitution_regions(source, config, *token)
        })
        .collect::<Vec<_>>();
    if let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, cmd,
    ) && words.with_source_schema(&context, |schema| {
        schema.semantics.body_kind == tcl_registry::BodyKind::Plain
    }) == Some(true)
    {
        regions.extend(words.source_script_bodies_for(&context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation)
            .into_iter().map(|body| {
                let span = body.content_span();
                (span.start() as usize, span.end() as usize)
            }));
    }
    regions
}

/// Potential structural bodies and genuinely braced lambda bodies selected by
/// the original source schema. Object scans drop variable receivers across this
/// boundary; command receivers still require their own positioned identity proof.
/// These regions describe source topology, never an entered runtime frame.
pub(crate) fn frame_shifted_dispatch_regions(
    source: &str,
    analysis: &AnalysisResult,
    _dialect: &'static tcl_dialect::DialectProfile,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    // Source structure identifies possible shifted bodies, never an entered frame.
    let Some(RetainedDispatchContext { context, .. }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, cmd,
    ) else {
        return Vec::new();
    };
    let mut regions = Vec::new();
    if words.with_source_schema(&context, |schema| {
        schema.semantics.body_kind == tcl_registry::BodyKind::Structural
    }) == Some(true)
    {
        regions.extend(words.source_script_bodies_for(&context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation)
            .into_iter().map(|body| {
                let span = body.content_span();
                (span.start() as usize, span.end() as usize)
            }));
    }
    let executable = words
        .with_source_schema(&context, schema_script_arguments)
        .flatten()
        .unwrap_or_default();
    for (index, role) in words.roles().unwrap_or_default() {
        if *role != tcl_registry::ArgRole::LambdaLiteral || !executable.contains(index) {
            continue;
        }
        let Some(word) = words
            .operands()
            .get(*index)
            .and_then(Option::as_ref)
            .and_then(|operand| operand.word())
        else {
            continue;
        };
        if let Some(body) = tcl_compiler::lambda_literal::split_original_lambda_literal(word)
            .and_then(|fields| fields.braced_body())
        {
            regions.push((body.start() as usize, body.end() as usize));
        }
    }
    regions
}

/// Complete potential-source traversal with its actual per-command positive
/// child projections. Completeness belongs to the checked source walker;
/// neither an empty projection nor a missing command can issue a census.
pub(super) struct RetainedDispatchRegions {
    pub(super) command: tcl_compiler::segmenter::SegmentedCommand,
    regions: Vec<(usize, usize)>,
}

pub(crate) struct DispatchRegionCensus {
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    pub(super) commands: std::collections::BTreeMap<(u32, u32), Vec<RetainedDispatchRegions>>,
}

impl DispatchRegionCensus {
    pub(crate) fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let input = analysis.resolved_input.as_ref()?.clone();
        let mut commands = std::collections::BTreeMap::<_, Vec<_>>::new();
        let mut regions = Vec::new();
        let coverage =
            crate::executable_regions::visit_checked_analysis_executable_commands_with_regions(
                source,
                analysis,
                &mut |command, _, _| {
                    commands
                        .entry((command.span.start(), command.span.end()))
                        .or_default()
                        .push(RetainedDispatchRegions {
                            command: command.clone(),
                            regions: Vec::new(),
                        });
                    false
                },
                &mut regions,
            );
        if coverage != crate::executable_regions::PotentialSourceVisit::Complete {
            return None;
        }
        regions.sort_unstable_by_key(|(span, depth)| (span.start(), span.end(), *depth));
        for bucket in commands.values_mut() {
            for retained in bucket {
                let first = regions
                    .partition_point(|(span, _)| span.start() < retained.command.span.start());
                let last = regions
                    .partition_point(|(span, _)| span.start() <= retained.command.span.end());
                retained.regions = regions[first..last]
                    .iter()
                    .filter_map(|(span, _)| {
                        let command = &retained.command;
                        (command.span.start() <= span.start()
                            && span.end() <= command.span.end()
                            && *span != command.span)
                            .then_some((span.start() as usize, span.end() as usize))
                    })
                    .collect();
                retained.regions.sort_unstable();
                retained.regions.dedup();
            }
        }
        Some(Self {
            image: analysis
                .retained_command_realm()?
                .original_source_image()?
                .clone(),
            input,
            commands,
        })
    }

    /// Exact retained source/input/command membership, including genuine empty
    /// children. A source mismatch or absent command remains unavailable.
    pub(crate) fn regions_for(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        command: &tcl_compiler::segmenter::SegmentedCommand,
    ) -> Option<&[(usize, usize)]> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image.bytes() != source.as_bytes()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || current.config() != self.input.lexer_config()
        {
            return None;
        }
        let mut selected = self
            .commands
            .get(&(command.span.start(), command.span.end()))?
            .iter()
            .filter(|retained| retained.command.all_tokens == command.all_tokens);
        let selected_regions = &selected.next()?.regions;
        if selected.any(|other| other.regions != *selected_regions) {
            return None;
        }
        Some(selected_regions)
    }
}

/// Every nested script region a **rename-safety** scan must also visit from
/// one segmented command: the union of the same-frame set
/// ([`nested_dispatch_regions`]) and the frame-shifted set
/// ([`frame_shifted_dispatch_regions`]).
///
/// The reference scan keeps the two apart because a `$var` receiver's bare
/// name stops naming the enclosing frame's variable across a frame shift.
/// The safety gate has no such distinction to make: it is asking "is there a
/// dispatch anywhere in this document that this rename cannot account for",
/// and a hazard written inside a `namespace eval` / `uplevel` / `apply` body
/// is every bit as unrewritable as one written beside it.
pub(crate) fn dispatch_scan_regions(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let mut regions = nested_dispatch_regions(source, analysis, dialect, cmd);
    regions.extend(frame_shifted_dispatch_regions(
        source, analysis, dialect, cmd,
    ));
    regions
}
