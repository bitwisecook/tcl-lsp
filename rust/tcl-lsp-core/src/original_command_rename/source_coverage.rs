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
//! Complete, finite coverage of one resolved source cut under its actual owners.

use super::{
    ByteCommandSlot, CommandLinkCoverage, Declaration, HashSet, NameBytes, OriginalCommandLookup,
    OriginalCommandRenameRefusal, OriginalConditionalCommandWorld, OriginalDeclarationDocument,
    OriginalDeclarationIdentity, SignatureSourceNameInput, SourceCommandReference, SourceImage,
    authored_result_completion_coverage, coverage_region, destination_matches_with_cut,
    queue_declaration, quiet_store_coverage, reject_command_link_coverage,
    reject_nonliteral_source_procedure_words, reject_readonly_math_references,
    reject_selected_command_link_coverage,
};
use super::{coverage_trace, require_coverage, trace_coverage_document};
use crate::original_source_views::OriginalSourceViewDocument;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::command_binding::OriginalSingleSourceCutSurvey;
use tcl_compiler::segmenter::SegmentedCommand;

pub(super) struct SourceCoverage<'a> {
    cuts: Vec<SourceCut<'a>>,
}
struct SourceCut<'a> {
    prefix: OriginalDeclarationDocument<'a>,
    child: OriginalDeclarationDocument<'a>,
    suffix: OriginalDeclarationDocument<'a>,
    survey: OriginalSingleSourceCutSurvey<'a>,
}

fn unique_view<'a, 'v>(
    views: &'v [OriginalSourceViewDocument<'a>],
    analysis: &AnalysisResult,
) -> Result<&'v OriginalSourceViewDocument<'a>, OriginalCommandRenameRefusal> {
    let mut selected = views
        .iter()
        .filter(|view| std::ptr::eq(view.document.analysis, analysis));
    let first = selected
        .next()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
    if selected.next().is_some() {
        return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
    }
    crate::original_context::CurrentSourceContext::capture(first.document.source, analysis)
        .ok_or(OriginalCommandRenameRefusal::StaleSource)?;
    Ok(first)
}

fn physical_document<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    view: OriginalDeclarationDocument<'_>,
) -> Result<OriginalDeclarationDocument<'a>, OriginalCommandRenameRefusal> {
    let mut selected = documents.iter().filter(|document| document.uri == view.uri);
    let first = *selected
        .next()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
    if selected.next().is_some() {
        return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
    }
    if first.source != view.source {
        return Err(OriginalCommandRenameRefusal::StaleSource);
    }
    Ok(first)
}

fn capture_cut_relationship<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    world: &OriginalConditionalCommandWorld<'a>,
    views: &[OriginalSourceViewDocument<'a>],
) -> Result<SourceCut<'a>, OriginalCommandRenameRefusal> {
    let prefix = documents
        .iter()
        .find(|document| document.uri == world.uri)
        .copied()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
    let prefix_view = unique_view(views, prefix.analysis)?;
    let candidates = views
        .iter()
        .map(|view| view.document.analysis)
        .collect::<Vec<_>>();
    let survey = world
        .outcome
        .single_source_cut_survey(prefix.analysis, &candidates)
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
    let child_view = unique_view(views, survey.child_analysis())?;
    let suffix_view = unique_view(views, survey.suffix_analysis())?;
    physical_document(documents, child_view.document)?;
    physical_document(documents, prefix_view.document)?;
    physical_document(documents, suffix_view.document)?;
    if prefix_view.document.uri != world.uri
        || prefix_view.region.is_some()
        || !prefix_view.ancestors.is_empty()
        || child_view.region.is_some()
        || child_view.ancestors.len() != 1
        || suffix_view.ancestors.len() != 2
        || suffix_view.region != Some(survey.region())
        || child_view.document.uri == prefix.uri
        || suffix_view.document.uri != prefix.uri
        || !child_view.ancestors.contains(prefix_view.publication)
        || !suffix_view.ancestors.contains(prefix_view.publication)
        || !suffix_view.ancestors.contains(child_view.publication)
        || prefix_view.publication == child_view.publication
        || prefix_view.publication == suffix_view.publication
        || child_view.publication == suffix_view.publication
    {
        return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
    }
    Ok(SourceCut {
        prefix,
        child: child_view.document,
        suffix: suffix_view.document,
        survey,
    })
}

fn capture_cut<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
    world: &OriginalConditionalCommandWorld<'a>,
    views: &[OriginalSourceViewDocument<'a>],
) -> Result<SourceCut<'a>, OriginalCommandRenameRefusal> {
    let cut = capture_cut_relationship(documents, world, views)?;
    let physical = physical_document(documents, cut.child)?;
    if identity.uri() == physical.uri && !std::ptr::eq(physical.analysis, cut.child.analysis) {
        return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
    }
    Ok(cut)
}

/// Selection from one authentic inherited interpretation, separately from its
/// physical document's independently checked declaration. The host must retain
/// the same published source-view revision throughout selection and planning.
pub(super) struct OriginalInheritedDeclarationSelection<'a> {
    physical: OriginalDeclarationDocument<'a>,
    documents: Vec<OriginalDeclarationDocument<'a>>,
    identity: OriginalDeclarationIdentity,
    publication: tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>,
}

impl<'a> OriginalInheritedDeclarationSelection<'a> {
    pub(super) fn capture(
        documents: &[OriginalDeclarationDocument<'a>],
        physical_identity: &OriginalDeclarationIdentity,
        cursor_uri: &str,
        cursor: u32,
        conditional: &[OriginalConditionalCommandWorld<'a>],
        views: &[OriginalSourceViewDocument<'a>],
    ) -> Result<Option<Self>, OriginalCommandRenameRefusal> {
        if conditional.len() > 4096 || views.len() > 4096 {
            return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
        }
        let mut physical = documents
            .iter()
            .filter(|doc| doc.uri == physical_identity.uri());
        let physical_owner = *physical
            .next()
            .ok_or(OriginalCommandRenameRefusal::AmbiguousOwner)?;
        if physical.next().is_some() {
            return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
        }
        if !physical_identity.is_current(
            physical_owner.uri,
            physical_owner.source,
            physical_owner.analysis,
        ) {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
        let mut child = None;
        for world in conditional {
            let cut = capture_cut_relationship(documents, world, views)?;
            if cut.child.uri != physical_owner.uri {
                continue;
            }
            if child.replace(cut).is_some() {
                return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
            }
        }
        let Some(cut) = child else {
            return Ok(None);
        };
        let inherited = cut.child;
        if std::ptr::eq(inherited.analysis, physical_owner.analysis) {
            return Ok(None);
        }
        let identity = Self::select_inherited_header(
            physical_owner,
            physical_identity,
            inherited,
            cursor_uri,
            cursor,
        )?;
        let Declaration::Procedure(declaration) =
            super::rename_declaration(inherited.analysis, &identity)?
        else {
            return Err(OriginalCommandRenameRefusal::UnsupportedRole);
        };
        let publication = cut
            .survey
            .closed_child_procedure_publication(declaration)
            .ok_or(OriginalCommandRenameRefusal::UnknownWorld)?;
        let interpreted = documents
            .iter()
            .map(|document| {
                if document.uri == inherited.uri {
                    inherited
                } else {
                    *document
                }
            })
            .collect();
        Ok(Some(Self {
            physical: physical_owner,
            documents: interpreted,
            identity,
            publication,
        }))
    }

    fn select_inherited_header(
        physical_owner: OriginalDeclarationDocument<'a>,
        physical_identity: &OriginalDeclarationIdentity,
        inherited: OriginalDeclarationDocument<'a>,
        cursor_uri: &str,
        cursor: u32,
    ) -> Result<OriginalDeclarationIdentity, OriginalCommandRenameRefusal> {
        if cursor_uri != physical_identity.uri()
            || cursor < physical_identity.span().start()
            || cursor >= physical_identity.span().end()
        {
            return Err(OriginalCommandRenameRefusal::UneditableInput);
        }
        if inherited.analysis.resolved_input != physical_owner.analysis.resolved_input
            || inherited.analysis.body_lexer_config != physical_owner.analysis.body_lexer_config
            || inherited.analysis.allows_lexical_declaration_advice()
        {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
        crate::original_context::CurrentSourceContext::capture(
            inherited.source,
            inherited.analysis,
        )
        .ok_or(OriginalCommandRenameRefusal::StaleSource)?;
        let std::ops::ControlFlow::Break(Some(identity)) =
            super::select_in_documents(&[inherited], cursor_uri, cursor)
        else {
            return Err(OriginalCommandRenameRefusal::UneditableInput);
        };
        if identity.role() != physical_identity.role()
            || identity.span() != physical_identity.span()
            || identity
                .input()
                .map(|input| (input.bytes(), input.policy()))
                != physical_identity
                    .input()
                    .map(|input| (input.bytes(), input.policy()))
        {
            return Err(OriginalCommandRenameRefusal::ChangedPublication);
        }
        let physical_declaration =
            super::rename_declaration(physical_owner.analysis, physical_identity)?;
        let inherited_declaration = super::rename_declaration(inherited.analysis, &identity)?;
        if physical_declaration.name().slot() != inherited_declaration.name().slot() {
            return Err(OriginalCommandRenameRefusal::UnknownDestination);
        }
        let site = identity
            .site()
            .ok_or(OriginalCommandRenameRefusal::StaleSource)?;
        inherited
            .analysis
            .original_namespace_scope_at(site.offset)
            .and_then(tcl_compiler::signature_scan::scope::SignatureNamespaceScope::context)
            .ok_or(OriginalCommandRenameRefusal::UnknownDestination)?;
        Ok(identity)
    }

    pub(super) fn publication(
        &self,
    ) -> &tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a> {
        &self.publication
    }

    pub(super) const fn physical(&self) -> OriginalDeclarationDocument<'a> {
        self.physical
    }
    pub(super) fn documents(&self) -> &[OriginalDeclarationDocument<'a>] {
        &self.documents
    }
    pub(super) const fn identity(&self) -> &OriginalDeclarationIdentity {
        &self.identity
    }
}

impl<'a> SourceCoverage<'a> {
    pub(super) fn capture(
        documents: &[OriginalDeclarationDocument<'a>],
        identity: &OriginalDeclarationIdentity,
        conditional: &[OriginalConditionalCommandWorld<'a>],
        views: &[OriginalSourceViewDocument<'a>],
    ) -> Result<Self, OriginalCommandRenameRefusal> {
        if conditional.len() > 4096 || views.len() > 4096 {
            return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
        }
        let mut participants = HashSet::new();
        let mut cuts = Vec::with_capacity(conditional.len());
        for world in conditional {
            let cut = capture_cut(documents, identity, world, views)?;
            if !participants.insert(cut.prefix.uri) || !participants.insert(cut.child.uri) {
                return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
            }
            cuts.push(cut);
        }
        Ok(Self { cuts })
    }
    pub(super) fn requests(
        &self,
        document: OriginalDeclarationDocument<'a>,
        context: &RequestContext<'_, 'a>,
    ) -> Option<Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalCommandRenameRefusal>>
    {
        let cut = self
            .cuts
            .iter()
            .find(|cut| cut.prefix.uri == document.uri || cut.child.uri == document.uri)?;
        Some(cut.requests(document, context))
    }
}

pub(super) struct RequestContext<'b, 'a> {
    pub documents: &'b [OriginalDeclarationDocument<'a>],
    pub conditional: &'b [OriginalConditionalCommandWorld<'a>],
    pub owner: OriginalDeclarationDocument<'a>,
    pub declaration: &'b Declaration<'a>,
    pub destination: &'b ByteCommandSlot,
    pub tail: &'b [u8],
    pub selected_cut:
        Option<&'b tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Purpose {
    Prefix,
    ChildWholeInput,
    IssuedSuffix,
    ProcedureBody,
    ResultWord,
    SameFrame,
    ShiftedFrame,
}
struct Task<'a> {
    body_layout:
        Option<std::sync::Arc<tcl_compiler::registry_invocation::OriginalSourceCutBodyLayout<'a>>>,
    document: OriginalDeclarationDocument<'a>,
    commands: Vec<SegmentedCommand>,
    purpose: Purpose,
    start: usize,
    end: usize,
    depth: u32,
}
impl<'a> Task<'a> {
    fn issued(
        document: OriginalDeclarationDocument<'a>,
        commands: &[SegmentedCommand],
        purpose: Purpose,
    ) -> Self {
        Self {
            body_layout: None,
            document,
            commands: commands.to_vec(),
            purpose,
            start: 0,
            end: document.source.len(),
            depth: 0,
        }
    }
    fn nested(
        document: OriginalDeclarationDocument<'a>,
        walk: &crate::refactor::FrameWalk<'a>,
        (start, end, depth): (usize, usize, u32),
        purpose: Purpose,
    ) -> Result<Self, OriginalCommandRenameRefusal> {
        let source = coverage_region(document.source, start, end)?;
        let offset = u32::try_from(start).map_err(|_| OriginalCommandRenameRefusal::StaleSource)?;
        Ok(Self {
            body_layout: None,
            document,
            commands: walk.segment_nested(source, offset),
            purpose,
            start,
            end,
            depth,
        })
    }
    fn tokens(
        &self,
        walk: &crate::refactor::FrameWalk<'a>,
        command: &SegmentedCommand,
    ) -> Result<tcl_compiler::ir::CommandTokens, OriginalCommandRenameRefusal> {
        let mut tokens = walk.tokens(self.document.source, command);
        if let Some(layout) = &self.body_layout {
            let offset = command
                .argv
                .first()
                .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?
                .span
                .start();
            require_coverage(
                layout.stamp_original_tokens(
                    self.document.source,
                    self.document.analysis,
                    &mut tokens,
                ),
                offset,
                "cut.body-layout",
            )?;
        }
        Ok(tokens)
    }

    fn key(&self) -> (usize, usize, Purpose, usize, usize, usize) {
        (
            std::ptr::from_ref(self.document.analysis).addr(),
            self.document
                .analysis
                .original_source_region()
                .map_or(0, |region| std::ptr::from_ref(region).addr()),
            self.purpose,
            self.body_layout
                .as_ref()
                .map_or(0, |layout| std::sync::Arc::as_ptr(layout).addr()),
            self.start,
            self.end,
        )
    }
}
struct Output<'a> {
    requests: Vec<(SignatureSourceNameInput, NameBytes)>,
    pending: Vec<Task<'a>>,
}

impl<'a> SourceCut<'a> {
    fn requests(
        &self,
        document: OriginalDeclarationDocument<'a>,
        context: &RequestContext<'_, 'a>,
    ) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalCommandRenameRefusal> {
        let pending = if document.uri == self.prefix.uri {
            vec![
                Task::issued(
                    self.suffix,
                    self.survey.suffix_commands(),
                    Purpose::IssuedSuffix,
                ),
                Task::issued(self.prefix, self.survey.prefix_commands(), Purpose::Prefix),
            ]
        } else {
            let walk = crate::refactor::FrameWalk::new(self.child.source, self.child.analysis)
                .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
            vec![Task::issued(
                self.child,
                &walk.segment_document(self.child.source),
                Purpose::ChildWholeInput,
            )]
        };
        let mut output = Output {
            requests: Vec::new(),
            pending,
        };
        let mut visited = HashSet::new();
        while let Some(task) = output.pending.pop() {
            if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(task.depth) {
                return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
            }
            if !visited.insert(task.key()) {
                continue;
            }
            trace_coverage_document(&task.document);
            reject_readonly_math_references(task.document, &context.owner, context.declaration)
                .inspect_err(|_| coverage_trace(0, "cut.math"))?;
            let walk = require_coverage(
                crate::refactor::FrameWalk::new(task.document.source, task.document.analysis),
                0,
                "cut.frame",
            )?;
            for command in &task.commands {
                self.command_requests(&task, &walk, command, context, &mut output)?;
            }
        }
        Ok(output.requests)
    }
}

impl<'a> SourceCut<'a> {
    fn record_reference(
        reference: Option<&SourceCommandReference>,
        lookup: &OriginalCommandLookup,
        input: &SignatureSourceNameInput,
        context: &RequestContext<'_, 'a>,
        output: &mut Output<'a>,
    ) -> Result<(), OriginalCommandRenameRefusal> {
        let offset = lookup.site().offset;
        if let Some(reference) = reference {
            if !reference.matches_original_invocation_site(lookup.site()) {
                coverage_trace(offset, "cut.reference-site");
                return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
            }
            if context.declaration.reference_matches(reference, true) {
                if !context.declaration.reference_matches(reference, false) {
                    return Err(OriginalCommandRenameRefusal::UneditableLinks);
                }
                destination_matches_with_cut(
                    context.documents,
                    context.conditional,
                    context.selected_cut,
                    lookup,
                    context.destination,
                    context.tail,
                )?;
                let wanted = input
                    .policy()
                    .recipe()
                    .replace_command_tail(input.bytes(), context.tail)
                    .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
                output
                    .requests
                    .push((input.clone(), NameBytes::from(wanted)));
            }
        }
        Ok(())
    }

    fn command_requests(
        &self,
        task: &Task<'a>,
        walk: &crate::refactor::FrameWalk<'a>,
        command: &SegmentedCommand,
        context: &RequestContext<'_, 'a>,
        output: &mut Output<'a>,
    ) -> Result<(), OriginalCommandRenameRefusal> {
        let document = task.document;
        let offset = command
            .argv
            .first()
            .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?
            .span
            .start();
        let words = require_coverage(
            walk.native_words(document.source, command),
            offset,
            "cut.words",
        )?;
        let tokens = task.tokens(walk, command)?;
        let binding = require_coverage(tokens.source_binding.as_ref(), offset, "cut.binding")?;
        let (input, decline_stage) = binding.original_head_name_input_with_decline_stage(&tokens);
        let input = require_coverage(input, offset, decline_stage)?;
        let lookup = require_coverage(
            binding.original_command_lookup(&tokens, &input),
            offset,
            "cut.lookup",
        )?;
        if lookup.name_input() != &input
            || lookup.site().source.source_image() != &SourceImage::document(document.source)
        {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
        let ordinary = binding.original_command_reference(&tokens);
        let conditional = if ordinary.is_none() {
            task.body_layout.as_ref().and_then(|layout| {
                layout.source_command_reference(document.source, document.analysis, &tokens)
            })
        } else {
            None
        };
        if conditional
            .as_ref()
            .is_some_and(|reference| !reference.is_for_analysis(document.analysis))
        {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
        let reference = ordinary.as_ref().or_else(|| {
            conditional.as_ref().map(
                tcl_compiler::registry_invocation::OriginalSourceCutBodyReference::source_reference,
            )
        });
        Self::record_reference(reference, &lookup, &input, context, output)?;
        let source_offset = self
            .survey
            .prefix_commands()
            .last()
            .and_then(|command| command.argv.first())
            .map(|word| word.span.start())
            .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
        if task.purpose == Purpose::Prefix && offset == source_offset {
            let receipt = require_coverage(
                tcl_compiler::registry_invocation::original_conditional_source_load_for_coverage(
                    document.source,
                    document.analysis,
                    &tokens,
                    &self.survey,
                ),
                offset,
                "cut.load-receipt",
            )?;
            reject_command_link_coverage(CommandLinkCoverage::SourceLoad {
                receipt: &receipt,
                analysis: document.analysis,
            })
            .inspect_err(|_| coverage_trace(offset, "cut.source-link"))?;
            return Ok(());
        }
        collect_command_regions(
            self,
            task,
            walk,
            command,
            &CommandSubject {
                words: &words,
                tokens: &tokens,
                input: &input,
                reference,
            },
            output,
        )
    }
}
struct CommandSubject<'a> {
    words: &'a [tcl_lexer::NativeWord],
    tokens: &'a tcl_compiler::ir::CommandTokens,
    input: &'a SignatureSourceNameInput,
    reference: Option<&'a SourceCommandReference>,
}

fn queue_cut_declaration<'a>(
    survey: &OriginalSingleSourceCutSurvey<'_>,
    task: &Task<'a>,
    walk: &crate::refactor::FrameWalk<'a>,
    command: &SegmentedCommand,
    output: &mut Output<'a>,
) -> Result<bool, OriginalCommandRenameRefusal> {
    if !matches!(
        task.purpose,
        Purpose::ChildWholeInput | Purpose::IssuedSuffix
    ) {
        return Ok(false);
    }
    let Some(receipt) =
        tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
            task.document.source,
            task.document.analysis,
            command,
            survey,
        )
    else {
        return Ok(false);
    };
    if !receipt.is_for_analysis(task.document.analysis) {
        return Err(OriginalCommandRenameRefusal::StaleSource);
    }
    let offset = command
        .argv
        .first()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?
        .span
        .start();
    reject_selected_command_link_coverage(receipt.invocation(), offset)?;
    let body = receipt.body().content_span();
    let layout = require_coverage(receipt.into_body_layout(survey), offset, "cut.body-world")?;
    let mut nested = Task::nested(
        task.document,
        walk,
        (body.start() as usize, body.end() as usize, task.depth + 1),
        Purpose::ProcedureBody,
    )?;
    nested.body_layout = Some(std::sync::Arc::new(layout));
    output.pending.push(nested);
    Ok(true)
}

impl SourceCut<'_> {
    fn reject_unowned_source_procedure(
        &self,
        subject: &CommandSubject<'_>,
        offset: u32,
    ) -> Result<(), OriginalCommandRenameRefusal> {
        let reference = require_coverage(
            subject.reference,
            offset,
            "cut.unselected-registry-or-source-reference",
        )?;
        if !reference.is_direct_definition()
            || ![self.prefix, self.child, self.suffix].iter().any(|owner| {
                owner
                    .analysis
                    .original_procedure_declarations()
                    .any(|declaration| {
                        crate::original_declaration::reference_matches_declaration(
                            reference,
                            declaration,
                            false,
                        )
                    })
            })
        {
            coverage_trace(offset, "cut.direct-procedure-owner");
            return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
        }
        // Every declaration body in these exact cut owners is separately
        // visited; no external source body or executable argv is inferred.
        reject_nonliteral_source_procedure_words(subject.words, subject.input)
    }
}

fn collect_command_regions<'a>(
    cut: &SourceCut<'_>,
    task: &Task<'a>,
    walk: &crate::refactor::FrameWalk<'a>,
    command: &SegmentedCommand,
    subject: &CommandSubject<'_>,
    output: &mut Output<'a>,
) -> Result<(), OriginalCommandRenameRefusal> {
    if queue_cut_declaration(&cut.survey, task, walk, command, output)? {
        return Ok(());
    }
    let document = task.document;
    let offset = command
        .argv
        .first()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?
        .span
        .start();
    let mut nested = Vec::new();
    if queue_declaration(walk, document, command, None, task.depth, &mut nested)? {
        return output.nested(task, walk, nested, Purpose::ProcedureBody);
    }
    if quiet_store_coverage(walk, document, command, offset)? {
        return Ok(());
    }
    if let Some(coverage) = task.body_layout.as_ref().and_then(|layout| {
        layout.result_completion_coverage(document.source, document.analysis, subject.tokens)
    }) {
        if !coverage.is_for_analysis(document.analysis) {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
        reject_selected_command_link_coverage(coverage.invocation(), offset)?;
        return Ok(());
    }
    if authored_result_completion_coverage(
        walk,
        document,
        command,
        offset,
        task.depth,
        &mut nested,
    )? {
        return output.nested(task, walk, nested, Purpose::ResultWord);
    }
    if let Some(selected) = walk.structure_of_tokens(subject.tokens) {
        reject_selected_command_link_coverage(&selected, offset)?;
        output.nested(
            task,
            walk,
            walk.same_frame_regions(document.source, command)
                .into_iter()
                .map(|(a, b)| (a, b, task.depth + 1))
                .collect(),
            Purpose::SameFrame,
        )?;
        output.nested(
            task,
            walk,
            walk.frame_shifted_regions(document.source, command)
                .into_iter()
                .map(|(a, b)| (a, b, task.depth + 1))
                .collect(),
            Purpose::ShiftedFrame,
        )?;
        if !walk.complete() {
            return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
        }
    } else {
        cut.reject_unowned_source_procedure(subject, offset)?;
    }
    Ok(())
}
impl<'a> Output<'a> {
    fn nested(
        &mut self,
        parent: &Task<'a>,
        walk: &crate::refactor::FrameWalk<'a>,
        regions: Vec<(usize, usize, u32)>,
        purpose: Purpose,
    ) -> Result<(), OriginalCommandRenameRefusal> {
        for span in regions {
            self.pending
                .push(Task::nested(parent.document, walk, span, purpose)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
