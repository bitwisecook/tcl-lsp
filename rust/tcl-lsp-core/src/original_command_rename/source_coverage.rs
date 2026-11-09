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
    if queue_declaration(walk, document, command, task.depth, &mut nested)? {
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
mod tests {
    use super::*;
    use crate::original_command_rename::{
        original_command_rename_edits_with_conditional_worlds,
        original_command_rename_edits_with_source_views,
    };
    use crate::original_declaration::{OriginalDeclarationRole, OriginalSourceViewPublication};
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::command_binding::{
        OriginalSourceConditionalOutcome, OriginalSourceConditionalRegion,
    };

    struct Fixture {
        source: &'static str,
        child_source: &'static str,
        prefix: AnalysisResult,
        child: AnalysisResult,
        ordinary_child: AnalysisResult,
        suffix: AnalysisResult,
        region: OriginalSourceConditionalRegion,
        region_count: usize,
        outcome: OriginalSourceConditionalOutcome,
        peer: AnalysisResult,
    }
    const PEER: &str = "proc host_old {} {return VALUE}; host_old";
    impl Fixture {
        fn new(source: &'static str, child_source: &'static str) -> Self {
            let prefix = Analyser::new().analyse(source, "tcl8.6");
            let source_offset = u32::try_from(source.find("source child").unwrap()).unwrap();
            let load = tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &prefix, source_offset,
            ).unwrap();
            let point = load.source_point_entry(&prefix).unwrap();
            let child = Analyser::new()
                .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
                .unwrap();
            let regions = load
                .conditional_continuation(&prefix, &child)
                .unwrap()
                .source_regions(&prefix, &child)
                .unwrap();
            let region = regions.regions()[0].clone();
            let suffix = Analyser::new()
                .analyse_source_region(&region, &prefix, &child)
                .unwrap();
            Self {
                source,
                child_source,
                ordinary_child: Analyser::new().analyse(child_source, "tcl8.6"),
                outcome: regions.normal_outcome().unwrap(),
                region_count: regions.regions().len(),
                region,
                prefix,
                child,
                suffix,
                peer: Analyser::new().analyse(PEER, "tcl8.6"),
            }
        }

        fn with_suffix_body_layout(
            &self,
            check: impl FnOnce(
                &tcl_compiler::registry_invocation::OriginalSourceCutBodyLayout<'_>,
                &tcl_compiler::ir::CommandTokens,
            ),
        ) {
            self.with_body_layout(None, check);
        }

        fn with_body_layout(
            &self,
            child_command: Option<usize>,
            check: impl FnOnce(
                &tcl_compiler::registry_invocation::OriginalSourceCutBodyLayout<'_>,
                &tcl_compiler::ir::CommandTokens,
            ),
        ) {
            let survey = self
                .outcome
                .single_source_cut_survey(&self.prefix, &[&self.child, &self.suffix])
                .unwrap();
            let (source, analysis) = if child_command.is_some() {
                (self.child_source, &self.child)
            } else {
                (self.source, &self.suffix)
            };
            let walk = crate::refactor::FrameWalk::new(source, analysis).unwrap();
            let commands = if child_command.is_some() {
                walk.segment_document(source)
            } else {
                Vec::new()
            };
            let command =
                child_command.map_or(&survey.suffix_commands()[0], |index| &commands[index]);
            let declaration = tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, analysis, command, &survey,
            ).expect("actual cut-owned declaration");
            let span = declaration.body().content_span();
            let layout = declaration.into_body_layout(&survey).unwrap();
            let commands = walk.segment_nested(&source[span.as_range()], span.start());
            let mut tokens = walk.tokens(source, &commands[0]);
            layout
                .stamp_original_tokens(source, analysis, &mut tokens)
                .unwrap();
            check(&layout, &tokens);
        }

        fn documents(&self) -> Vec<OriginalDeclarationDocument<'_>> {
            vec![
                OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source: self.source,
                    analysis: &self.prefix,
                },
                OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: self.child_source,
                    analysis: &self.ordinary_child,
                },
                OriginalDeclarationDocument {
                    uri: "file:///peer.tcl",
                    source: PEER,
                    analysis: &self.peer,
                },
            ]
        }
        fn identity(&self) -> OriginalDeclarationIdentity {
            crate::original_declaration::declarations("file:///peer.tcl", PEER, &self.peer)
                .unwrap()
                .into_iter()
                .find(|row| row.role() == OriginalDeclarationRole::Procedure)
                .unwrap()
        }
        fn with_views<T>(&self, run: impl FnOnce(&[OriginalSourceViewDocument<'_>]) -> T) -> T {
            let root = OriginalSourceViewPublication::retain(1_u8);
            let child = OriginalSourceViewPublication::retain(2_u8);
            let suffix = OriginalSourceViewPublication::retain(3_u8);
            let child_ancestors = [root.clone()];
            let suffix_ancestors = [child.clone(), root.clone()];
            run(&[
                OriginalSourceViewDocument {
                    publication: &root,
                    document: OriginalDeclarationDocument {
                        uri: "file:///main.tcl",
                        source: self.source,
                        analysis: &self.prefix,
                    },
                    region: None,
                    ancestors: &[],
                },
                OriginalSourceViewDocument {
                    publication: &child,
                    document: OriginalDeclarationDocument {
                        uri: "file:///child.tcl",
                        source: self.child_source,
                        analysis: &self.child,
                    },
                    region: None,
                    ancestors: &child_ancestors,
                },
                OriginalSourceViewDocument {
                    publication: &suffix,
                    document: OriginalDeclarationDocument {
                        uri: "file:///main.tcl",
                        source: self.source,
                        analysis: &self.suffix,
                    },
                    region: Some(&self.region),
                    ancestors: &suffix_ancestors,
                },
            ])
        }
    }

    fn assert_malformed_source_cut_body_refused() {
        let fixture = Fixture::new("source child.tcl; set done OK", "proc helper {} {[}");
        let documents = fixture.documents();
        let identity = fixture.identity();
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            let proof = SourceCoverage::capture(&documents, &identity, &worlds, views).unwrap();
            let walk = crate::refactor::FrameWalk::new(fixture.child_source, &fixture.child).unwrap();
            let commands = walk.segment_document(fixture.child_source);
            assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                fixture.child_source, &fixture.child, &commands[0], &proof.cuts[0].survey,
            ).is_none());
            assert!(original_command_rename_edits_with_source_views(
                &documents, &identity, "host_new", &worlds, views,
            ).is_err());
        });
    }
    fn assert_source_cut_body_layout(
        source: &str,
        analysis: &AnalysisResult,
        ordinary: &AnalysisResult,
        command: &SegmentedCommand,
        survey: &OriginalSingleSourceCutSurvey<'_>,
        receipt: tcl_compiler::registry_invocation::OriginalSourceCutProcedureDeclaration<'_>,
    ) {
        let content = receipt.body().content_span();
        let layout = receipt
            .into_body_layout(survey)
            .expect("same cut conditional body owner");
        let walk = crate::refactor::FrameWalk::new(source, analysis).unwrap();
        let commands = walk.segment_nested(&source[content.as_range()], content.start());
        let mut stronger = walk.tokens(source, &commands[0]);
        let retained = stronger.source_binding.as_mut().unwrap();
        retained.may_use_live_dispatch = true;
        retained.evaluated_argument_values = vec![Some("retained argument fact".to_owned())].into();
        let retained = stronger.source_binding.clone();
        assert!(
            layout
                .stamp_original_tokens(source, analysis, &mut stronger)
                .is_some()
        );
        assert_eq!(stronger.source_binding, retained);
        let mut tokens = walk.tokens(source, &commands[0]);
        let before = tokens.source_binding.as_ref().unwrap().clone();
        let had_original_head = before.original_head_name_input(&tokens).is_some();
        assert!(
            layout
                .stamp_original_tokens(source, analysis, &mut tokens)
                .is_some()
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        if had_original_head {
            assert_eq!(
                binding, &before,
                "an existing original head keeps its entire binding"
            );
        } else {
            assert!(binding.unknown);
            assert_eq!(
                binding.targets,
                [] as [tcl_compiler::command_binding::SourceCommandTarget; 0]
            );
            assert!(!binding.unobserved_native_dispatch());
        }
        let input = binding
            .original_head_name_input(&tokens)
            .expect("same complete body vector, no entered invocation");
        assert!(binding.original_command_lookup(&tokens, &input).is_some());
        let mut outside = walk.tokens(source, command);
        assert!(
            layout
                .stamp_original_tokens(source, analysis, &mut outside)
                .is_none()
        );
        let mut stale = tokens.clone();
        assert!(
            layout
                .stamp_original_tokens(&format!("{source}\n# changed"), analysis, &mut stale)
                .is_none()
        );
        let mut foreign = tokens.clone();
        assert!(
            layout
                .stamp_original_tokens(source, ordinary, &mut foreign)
                .is_none()
        );
        let equal_foreign = analysis.clone();
        let mut foreign = tokens;
        assert!(
            layout
                .stamp_original_tokens(source, &equal_foreign, &mut foreign)
                .is_none()
        );
    }

    fn assert_source_cut_declaration_geometry(
        fixture: &Fixture,
        survey: &OriginalSingleSourceCutSurvey<'_>,
    ) {
        assert_malformed_source_cut_body_refused();
        let child_walk =
            crate::refactor::FrameWalk::new(fixture.child_source, &fixture.child).unwrap();
        let child_commands = child_walk.segment_document(fixture.child_source);
        for (source, analysis, command, ordinary) in [
            (
                fixture.child_source,
                &fixture.child,
                &child_commands[0],
                &fixture.ordinary_child,
            ),
            (
                fixture.source,
                &fixture.suffix,
                &survey.suffix_commands()[0],
                &fixture.prefix,
            ),
        ] {
            let receipt = tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, analysis, command, survey,
            ).expect("same actual source-cut declaration");
            assert!(receipt.is_for_analysis(analysis));
            assert!(!receipt.is_for_analysis(ordinary));
            assert_eq!(
                receipt.invocation().written_argument_roles(),
                vec![
                    (0, tcl_registry::ArgRole::Name),
                    (1, tcl_registry::ArgRole::ParamList),
                    (2, tcl_registry::ArgRole::Body),
                ]
            );
            assert_eq!(
                &source[receipt.body().content_span().as_range()],
                "return VALUE"
            );
            let walk = crate::refactor::FrameWalk::new(source, analysis).unwrap();
            assert!(
                walk.procedure_declaration_for_rename_coverage(source, command)
                    .is_none()
            );
            assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, ordinary, command, survey,
            ).is_none());
            assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                &format!("# foreign whole source\n{source}"), analysis, command, survey,
            ).is_none());
            let mut foreign = analysis.clone();
            foreign.body_lexer_config.as_mut().unwrap().strict_quoting =
                !analysis.body_lexer_config.unwrap().strict_quoting;
            assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, &foreign, command, survey,
            ).is_none());
            assert_source_cut_body_layout(source, analysis, ordinary, command, survey, receipt);
        }
        let prefix_walk = crate::refactor::FrameWalk::new(fixture.source, &fixture.prefix).unwrap();
        let prefix_commands = prefix_walk.segment_document(fixture.source);
        assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
            fixture.source, &fixture.prefix, &prefix_commands[0], survey,
        ).is_none());
    }

    fn cut_rename_edits(fixture: &Fixture) -> Result<usize, OriginalCommandRenameRefusal> {
        let documents = fixture.documents();
        let identity = fixture.identity();
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            original_command_rename_edits_with_source_views(
                &documents, &identity, "host_new", &worlds, views,
            )
            .map(|edits| edits.len())
        })
    }

    /// tclsh 8.6/9.0: each option form changes the completion code or level,
    /// `return a b` reads `a b` as an option, and `uplevel`/`eval` move the
    /// return into another script; none is modelled for a cut procedure body.
    #[test]
    fn single_cut_body_return_option_forms_stay_refused() {
        let child = "proc helper {} {return VALUE}";
        for body in [
            "return -code error E",
            "return -code break",
            "return -code continue",
            "return -code return R",
            "return -code 7 N",
            "return -level 2 L",
            "return -options {-code error} O",
            "return -errorcode X E",
            "return a b",
            "uplevel 1 {return U}",
            "eval {return EV}",
        ] {
            let source: &'static str =
                Box::leak(format!("source child.tcl; proc run {{}} {{{body}}}").into_boxed_str());
            assert_eq!(
                cut_rename_edits(&Fixture::new(source, child)),
                Err(OriginalCommandRenameRefusal::IncompleteCoverage),
                "{body}"
            );
        }
    }

    /// tclsh 8.6/9.0: a top-level `return` in the sourced file ends `source`
    /// with its value and the issuing script continues; that propagation into
    /// the issuing point is unmodelled, so no rename world is issued from it.
    #[test]
    fn single_cut_child_top_level_return_stays_refused() {
        let source = "source child.tcl; proc run {} {return VALUE}";
        let child_source = "proc helper {} {return VALUE}\nreturn done";
        let prefix = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let outcome =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &prefix, offset,
            )
            .and_then(|load| {
                let point = load.source_point_entry(&prefix)?;
                let child = Analyser::new().analyse_with_source_point_entry(
                    child_source,
                    "tcl8.6",
                    &point,
                )?;
                load.conditional_continuation(&prefix, &child)?
                    .source_regions(&prefix, &child)?
                    .normal_outcome()
            });
        if outcome.is_some() {
            assert_eq!(
                cut_rename_edits(&Fixture::new(source, child_source)),
                Err(OriginalCommandRenameRefusal::IncompleteCoverage)
            );
        }
    }

    #[test]
    fn single_cut_source_coverage_surveys_actual_child_and_issued_suffix() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {return VALUE}",
        );
        let documents = fixture.documents();
        let identity = fixture.identity();
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        assert!(
            original_command_rename_edits_with_conditional_worlds(
                &documents, &identity, "host_new", &worlds
            )
            .is_err()
        );
        fixture.with_views(|views| {
            let proof = SourceCoverage::capture(&documents, &identity, &worlds, views).unwrap();
            assert_eq!(proof.cuts.len(), 1);
            assert_eq!(proof.cuts[0].survey.prefix_commands().len(), 1);
            assert_eq!(proof.cuts[0].survey.suffix_commands().len(), 1);
            assert_source_cut_declaration_geometry(&fixture, &proof.cuts[0].survey);
            let edits = original_command_rename_edits_with_source_views(
                &documents, &identity, "host_new", &worlds, views,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{error:?}: {:?}",
                    super::super::take_missing_rename_world_trace()
                )
            });
            assert_eq!(edits.len(), 2);
            assert!(
                edits
                    .iter()
                    .all(|edit| edit.uri == "file:///peer.tcl" && edit.new_text == "host_new")
            );
            assert!(!std::ptr::eq(
                proof.cuts[0].child.analysis,
                documents[1].analysis
            ));
            assert!(std::ptr::eq(documents[2].analysis, &raw const fixture.peer));
        });
    }
    fn assert_foreign_source_membership_refused(
        fixture: &Fixture,
        documents: &[OriginalDeclarationDocument<'_>],
        identity: &OriginalDeclarationIdentity,
        worlds: &[OriginalConditionalCommandWorld<'_>],
        views: &[OriginalSourceViewDocument<'_>],
    ) {
        let duplicate = [documents[0], documents[1], documents[1], documents[2]];
        assert!(SourceCoverage::capture(&duplicate, identity, worlds, views).is_err());
        let foreign = Analyser::new().analyse(fixture.source, "tcl8.6");
        let stale = [
            OriginalDeclarationDocument {
                analysis: &foreign,
                ..documents[0]
            },
            documents[1],
            documents[2],
        ];
        assert!(SourceCoverage::capture(&stale, identity, worlds, views).is_err());
        let foreign_input = Analyser::new().analyse(fixture.source, "tcl9.0");
        let wrong_config = [
            OriginalDeclarationDocument {
                analysis: &foreign_input,
                ..documents[0]
            },
            documents[1],
            documents[2],
        ];
        assert!(SourceCoverage::capture(&wrong_config, identity, worlds, views).is_err());
        let changed = [
            documents[0],
            OriginalDeclarationDocument {
                source: "changed",
                ..documents[1]
            },
            documents[2],
        ];
        assert!(SourceCoverage::capture(&changed, identity, worlds, views).is_err());
    }
    #[test]
    fn source_coverage_refuses_missing_child_reference_and_foreign_membership() {
        let fixture = Fixture::new(
            "proc target {} {return VALUE}; source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {target}",
        );
        let documents = fixture.documents();
        let identity = crate::original_declaration::declarations(
            "file:///main.tcl",
            fixture.source,
            &fixture.prefix,
        )
        .unwrap()
        .into_iter()
        .find(|row| row.role() == OriginalDeclarationRole::Procedure)
        .unwrap();
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(SourceCoverage::capture(&documents, &identity, &worlds, views).is_ok());
            if let Ok(edits) = original_command_rename_edits_with_source_views(
                &documents,
                &identity,
                "target_new",
                &worlds,
                views,
            ) {
                assert!(
                    edits.iter().any(
                        |edit| edit.uri == "file:///child.tcl" && edit.new_text == "target_new"
                    )
                );
            }
            for omitted in [0, 1, 2] {
                let retained = views
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != omitted)
                    .map(|(_, view)| OriginalSourceViewDocument {
                        publication: view.publication,
                        document: view.document,
                        region: view.region,
                        ancestors: view.ancestors,
                    })
                    .collect::<Vec<_>>();
                assert!(
                    SourceCoverage::capture(&documents, &identity, &worlds, &retained).is_err()
                );
                assert!(
                    original_command_rename_edits_with_source_views(
                        &documents,
                        &identity,
                        "target_new",
                        &worlds,
                        &retained
                    )
                    .is_err()
                );
            }
            let without_child = [documents[0], documents[2]];
            assert!(SourceCoverage::capture(&without_child, &identity, &worlds, views).is_err());
            assert!(
                original_command_rename_edits_with_source_views(
                    &without_child,
                    &identity,
                    "target_new",
                    &worlds,
                    views
                )
                .is_err()
            );
            assert_foreign_source_membership_refused(
                &fixture, &documents, &identity, &worlds, views,
            );
        });
    }
    #[test]
    fn source_coverage_keeps_selected_owner_and_opaque_body_refusals() {
        for child_source in [
            "proc helper {} {return VALUE}",
            "proc helper {} {puts opaque}",
        ] {
            let fixture =
                Fixture::new("source child.tcl; proc run {} {return VALUE}", child_source);
            let documents = fixture.documents();
            let identity = fixture.identity();
            let worlds = [OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &fixture.outcome,
            }];
            let child_identity = crate::original_declaration::declarations(
                "file:///child.tcl",
                child_source,
                &fixture.ordinary_child,
            )
            .unwrap()
            .into_iter()
            .find(|row| row.role() == OriginalDeclarationRole::Procedure)
            .unwrap();
            fixture.with_views(|views| {
                assert!(
                    SourceCoverage::capture(&documents, &child_identity, &worlds, views).is_err()
                );
                if child_source.contains("puts") {
                    assert!(
                        original_command_rename_edits_with_source_views(
                            &documents, &identity, "host_new", &worlds, views
                        )
                        .is_err()
                    );
                }
            });
        }
    }

    fn assert_logical_source_rename_issuer_unavailable(namespace_source: &str, child_source: &str) {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        for source in [
            namespace_source,
            "source child.tcl; proc run {} {return VALUE}",
        ] {
            let parent = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            assert_eq!(parent.resolved_input.as_ref(), Some(&input));
            let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
            let load = tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &parent, offset,
            ).expect("actual Logical project-source load card");
            assert!(load.matches_source(&SourceImage::document(source), &input));
            let point = load
                .source_point_entry(&parent)
                .expect("actual Logical source point");
            let child = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse_with_source_point_entry(child_source, profile.name, &point)
                .expect("genuine Logical child under its actual point");
            assert!(point.matches_parent_analysis(&parent));
            assert!(point.matches_child_analysis(&child));
            assert!(
                child
                    .resolved_input
                    .as_ref()
                    .unwrap()
                    .has_logical_source_name_context()
            );
            assert!(child.original_completed_command_world().is_none());
            assert!(
                load.conditional_continuation(&parent, &child).is_none(),
                "current Logical source issuer has no completed continuation"
            );
        }
    }

    fn native_namespace_fixture(source: &'static str, child_source: &'static str) -> Fixture {
        let fixture = Fixture::new(source, child_source);
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source,
                &fixture.prefix,
                offset,
            )
            .unwrap();
        let point = load.source_point_entry(&fixture.prefix).unwrap();
        assert!(point.matches_parent_analysis(&fixture.prefix));
        assert!(point.matches_child_analysis(&fixture.child));
        assert!(fixture.child.original_completed_command_world().is_none());
        assert!(
            !fixture
                .child
                .resolved_input
                .as_ref()
                .unwrap()
                .has_logical_source_name_context()
        );
        fixture
    }

    #[test]
    fn source_coverage_never_omits_extra_cuts_or_namespace_frames() {
        for (source, child_source) in [
            (
                "source child.tcl; proc run {} {return VALUE}",
                "proc helper {} {source hidden.tcl}",
            ),
            (
                "namespace eval :: {source child.tcl}; proc run {} {return VALUE}",
                "proc helper {} {return VALUE}",
            ),
        ] {
            let fixture = if source.starts_with("namespace") {
                assert_logical_source_rename_issuer_unavailable(source, child_source);
                native_namespace_fixture(source, child_source)
            } else {
                Fixture::new(source, child_source)
            };
            if source.starts_with("namespace") {
                assert!(fixture.region_count > 1);
            }
            let documents = fixture.documents();
            let identity = fixture.identity();
            let worlds = [OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &fixture.outcome,
            }];
            fixture.with_views(|views| {
                assert!(SourceCoverage::capture(&documents, &identity, &worlds, views).is_err());
                assert!(
                    original_command_rename_edits_with_source_views(
                        &documents, &identity, "host_new", &worlds, views
                    )
                    .is_err()
                );
            });
        }
    }

    fn inherited_child_identity(fixture: &Fixture) -> OriginalDeclarationIdentity {
        let std::ops::ControlFlow::Break(Some(identity)) = super::super::select_in_documents(
            &[OriginalDeclarationDocument {
                uri: "file:///child.tcl",
                source: fixture.child_source,
                analysis: &fixture.ordinary_child,
            }],
            "file:///child.tcl",
            5,
        ) else {
            panic!("actual physical child declaration")
        };
        identity
    }

    #[test]
    fn inherited_selection_keeps_both_physical_and_child_declaration_guards() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {return VALUE}; proc caller {} {helper}",
        );
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(
                original_command_rename_edits_with_source_views(
                    &documents,
                    &identity,
                    "updated_helper",
                    &worlds,
                    views,
                )
                .is_err()
            );
            let edits = super::super::original_command_rename_edits_with_selected_source_view(
                &documents,
                &identity,
                "updated_helper",
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .expect("genuine inherited declaration plus independent physical edit agreement");
            assert_eq!(edits.len(), 2);
            assert!(
                edits.iter().all(
                    |edit| edit.uri == "file:///child.tcl" && edit.new_text == "updated_helper"
                )
            );
            assert_eq!(
                super::super::original_command_prepare_span_in_with_selected_source_view(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .unwrap(),
                identity.span()
            );
            let reference = u32::try_from(fixture.child_source.rfind("helper").unwrap()).unwrap();
            assert!(
                super::super::original_command_rename_edits_with_selected_source_view(
                    &documents,
                    &identity,
                    "updated_helper",
                    "file:///child.tcl",
                    reference,
                    &worlds,
                    views,
                )
                .is_err(),
                "reference-origin selection cannot retarget an inherited declaration"
            );
        });
    }

    #[test]
    fn cut_body_result_coverage_keeps_inherited_purpose_and_selects_registered_result() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return ROOT}",
            "proc helper {} {return VALUE}",
        );
        fixture.with_body_layout(Some(0), |layout, tokens| {
            let before = tokens.source_binding.clone();
            assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
                fixture.child_source, &fixture.child, tokens,
            ).is_none());
            let coverage = layout.result_completion_coverage(fixture.child_source, &fixture.child, tokens)
                .expect("same captured child row and actual registered result contract");
            assert!(coverage.is_for_analysis(&fixture.child));
            assert_eq!(coverage.invocation().written_argument_roles(), [(0, tcl_registry::ArgRole::Result)]);
            assert_eq!(coverage.invocation().facts.native_result,
                Some(tcl_registry::native_result::NativeResultContract::ReturnResult));
            assert_eq!(tokens.source_binding, before);
            assert!(!tokens.source_binding.as_ref().unwrap().unobserved_native_dispatch());
            assert!(fixture.child.original_completed_command_world().is_none());
            assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
                fixture.child_source, &fixture.child, tokens,
            ).is_none());
            let foreign = fixture.child.clone();
            assert!(layout.result_completion_coverage(fixture.child_source, &foreign, tokens).is_none());
            assert!(layout.result_completion_coverage(&format!("{}\n", fixture.child_source), &fixture.child, tokens).is_none());
        });
    }

    #[test]
    fn cut_body_result_coverage_refuses_options_dynamic_argv_and_replaced_handlers() {
        for body in [
            "return -code error E",
            "return -level 2 X",
            "return $unknown",
            "return [set x VALUE]",
            "return {*}$args",
        ] {
            let child = Box::leak(format!("proc helper {{}} {{{body}}}").into_boxed_str());
            let fixture = Fixture::new("source child.tcl; proc run {} {return ROOT}", child);
            fixture.with_body_layout(Some(0), |layout, tokens| {
                assert!(
                    layout
                        .result_completion_coverage(fixture.child_source, &fixture.child, tokens)
                        .is_none(),
                    "{body}"
                );
            });
        }
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return ROOT}",
            "proc return args {error custom}; proc helper {} {return VALUE}",
        );
        fixture.with_body_layout(Some(1), |layout, tokens| {
            assert!(
                layout
                    .result_completion_coverage(fixture.child_source, &fixture.child, tokens)
                    .is_none()
            );
        });
    }

    #[test]
    fn cut_body_reference_is_exact_and_leaves_ordinary_dispatch_unavailable() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {helper}",
            "proc helper {} {return VALUE}",
        );
        fixture.with_suffix_body_layout(|layout, tokens| {
            let binding = tokens.source_binding.as_ref().unwrap();
            let before = binding.clone();
            assert!(binding.original_command_reference(tokens).is_none());
            let reference = layout
                .source_command_reference(fixture.source, &fixture.suffix, tokens)
                .expect("same captured row, complete static argv and actual child allocation");
            assert!(reference.is_for_analysis(&fixture.suffix));
            assert!(crate::original_declaration::reference_matches_declaration(
                reference.source_reference(),
                fixture
                    .child
                    .original_procedure_declarations()
                    .next()
                    .unwrap(),
                false,
            ));
            assert_eq!(tokens.source_binding.as_ref(), Some(&before));
            assert!(binding.original_command_reference(tokens).is_none());
            assert!(fixture.child.original_completed_command_world().is_none());
            let foreign = fixture.suffix.clone();
            assert!(
                layout
                    .source_command_reference(fixture.source, &foreign, tokens)
                    .is_none()
            );
            assert!(
                layout
                    .source_command_reference(
                        &format!("{}\n", fixture.source),
                        &fixture.suffix,
                        tokens
                    )
                    .is_none()
            );
            let mut uncertain = tokens.clone();
            assert!(!uncertain.source_binding.as_ref().unwrap().may_be_absent);
            uncertain.source_binding.as_mut().unwrap().may_be_absent = true;
            assert_ne!(uncertain.source_binding.as_ref(), Some(&before));
            assert!(
                layout
                    .source_command_reference(fixture.source, &fixture.suffix, &uncertain)
                    .is_none()
            );
            assert!(uncertain.source_binding.as_ref().unwrap().may_be_absent);
        });
        let foreign = Fixture::new(fixture.source, fixture.child_source);
        fixture.with_suffix_body_layout(|layout, _| {
            foreign.with_suffix_body_layout(|_, tokens| {
                assert!(
                    layout
                        .source_command_reference(fixture.source, &fixture.suffix, tokens)
                        .is_none()
                );
            });
        });
    }

    #[test]
    fn cut_body_reference_refuses_dynamic_arguments_under_a_real_layout() {
        for body in ["helper $unknown", "helper {*}$args", "helper [set value X]"] {
            let source =
                Box::leak(format!("source child.tcl; proc run {{}} {{{body}}}").into_boxed_str());
            let fixture = Fixture::new(source, "proc helper {} {return VALUE}");
            fixture.with_suffix_body_layout(|layout, tokens| {
                assert!(
                    layout
                        .source_command_reference(fixture.source, &fixture.suffix, tokens)
                        .is_none(),
                    "{body}"
                );
            });
        }
    }

    #[test]
    fn genuine_single_cut_inherited_declaration_renames_both_physical_files() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {helper}",
            "proc helper {} {return VALUE}",
        );
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            let edits = super::super::original_command_rename_edits_with_selected_source_view(
                &documents,
                &identity,
                "updated_helper",
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .expect("actual whole-root Normal source cut, declaration and complete call coverage");
            assert_eq!(edits.len(), 2);
            assert_eq!(
                edits
                    .iter()
                    .filter(|edit| edit.uri == "file:///child.tcl")
                    .count(),
                1
            );
            assert_eq!(
                edits
                    .iter()
                    .filter(|edit| edit.uri == "file:///main.tcl")
                    .count(),
                1
            );
            assert!(edits.iter().all(|edit| edit.new_text == "updated_helper"));
        });
    }

    #[test]
    fn inherited_cut_keeps_root_child_and_registry_collisions() {
        for (source, child_source, new_tail, child_occupied) in [
            (
                "source child.tcl; proc run {} {helper}; proc updated_helper {} {return ROOT}",
                "proc helper {} {return VALUE}",
                "updated_helper",
                false,
            ),
            (
                "source child.tcl; proc run {} {helper}",
                "proc helper {} {return VALUE}; proc updated_helper {} {return CHILD}",
                "updated_helper",
                true,
            ),
            (
                "source child.tcl; proc run {} {helper}",
                "proc helper {} {return VALUE}",
                "puts",
                true,
            ),
        ] {
            let fixture = Fixture::new(source, child_source);
            let documents = fixture.documents();
            let identity = inherited_child_identity(&fixture);
            let worlds = [OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &fixture.outcome,
            }];
            fixture.with_views(|views| {
                let selected = OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .unwrap()
                .expect("genuine cut and unchanged selected allocation");
                let declaration =
                    super::super::rename_declaration(&fixture.child, selected.identity()).unwrap();
                let policy = declaration.name().policy();
                let destination = super::super::renamed_slot(
                    declaration.name().slot(),
                    policy,
                    new_tail.as_bytes(),
                )
                .unwrap();
                assert_eq!(
                    selected
                        .publication()
                        .command_slot_occupied(&destination, policy),
                    Some(child_occupied),
                );
                assert_eq!(
                    super::super::occupied_with_cut(
                        selected.documents(),
                        &worlds,
                        Some(selected.publication()),
                        &destination,
                        policy,
                    ),
                    Ok(true),
                );
                assert_eq!(
                    super::super::original_command_rename_edits_with_selected_source_view(
                        &documents,
                        &identity,
                        new_tail,
                        "file:///child.tcl",
                        5,
                        &worlds,
                        views,
                    ),
                    Err(OriginalCommandRenameRefusal::Collision),
                );
            });
        }
    }

    #[test]
    fn inherited_header_refuses_uncovered_namespace_eval_in_physical_body() {
        let fixture = Fixture::new(
            "namespace eval ::ns {}; proc ::ns::helper {} {return PREFIX}; source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {return CHILD}; proc caller {} {namespace eval ::ns {helper}}",
        );
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            let selected = OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .unwrap()
            .expect("actual inherited declaration header");
            assert_eq!(
                super::super::original_command_rename_edits(
                    &[selected.physical()],
                    &identity,
                    "updated_helper",
                ),
                Err(OriginalCommandRenameRefusal::IncompleteCoverage),
                "the standalone namespace eval has no complete source coverage",
            );
            assert_eq!(
                super::super::original_command_rename_edits_with_selected_source_view(
                    &documents,
                    &identity,
                    "updated_helper",
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                ),
                Err(OriginalCommandRenameRefusal::IncompleteCoverage),
            );
        });
    }

    #[test]
    fn selected_physical_agreement_refuses_omitted_real_reference_edit() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {return VALUE}; proc caller {} {helper}",
        );
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            let selected = OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .unwrap()
            .expect("actual inherited declaration header");
            let physical = super::super::original_command_rename_edits(
                &[selected.physical()],
                &identity,
                "updated_helper",
            )
            .expect("complete actual physical plan");
            let coverage =
                SourceCoverage::capture(selected.documents(), selected.identity(), &worlds, views)
                    .expect("actual complete cut coverage");
            let mut inherited = super::super::original_command_rename_units_with_source_coverage(
                selected.documents(),
                selected.identity(),
                b"updated_helper",
                &worlds,
                Some(&coverage),
                Some(selected.publication()),
            )
            .expect("complete actual inherited plan");
            assert_eq!(physical.len(), 2);
            assert_eq!(
                super::super::require_selected_physical_edits(
                    &physical,
                    &inherited,
                    "file:///child.tcl"
                ),
                Ok(())
            );
            // Deliberately omit one actual issued edit to test the agreement guard;
            // this does not assert that the actual owners diverged.
            inherited.pop().expect("actual issued inherited edit");
            assert_eq!(
                super::super::require_selected_physical_edits(
                    &physical,
                    &inherited,
                    "file:///child.tcl"
                ),
                Err(OriginalCommandRenameRefusal::IncompleteCoverage)
            );
        });
    }

    #[test]
    fn inherited_cut_reference_coverage_keeps_dynamic_and_unowned_calls_refused() {
        for body in [
            "helper $dynamic",
            "helper {*}$args",
            "helper [set x V]",
            "unknown_helper",
        ] {
            let source =
                Box::leak(format!("source child.tcl; proc run {{}} {{{body}}}").into_boxed_str());
            let fixture = Fixture::new(source, "proc helper {} {return VALUE}");
            let documents = fixture.documents();
            let identity = inherited_child_identity(&fixture);
            let worlds = [OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &fixture.outcome,
            }];
            fixture.with_views(|views| {
                assert!(
                    OriginalInheritedDeclarationSelection::capture(
                        &documents,
                        &identity,
                        "file:///child.tcl",
                        5,
                        &worlds,
                        views,
                    )
                    .unwrap()
                    .is_some(),
                    "genuine closed publication before reference coverage: {body}",
                );
                assert_eq!(
                    super::super::original_command_rename_edits_with_selected_source_view(
                        &documents,
                        &identity,
                        "updated_helper",
                        "file:///child.tcl",
                        5,
                        &worlds,
                        views,
                    ),
                    Err(OriginalCommandRenameRefusal::IncompleteCoverage),
                    "{body}",
                );
            });
        }
    }

    #[test]
    fn inherited_selection_refuses_foreign_and_duplicate_owners() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {return VALUE}",
        );
        let other = Fixture::new(fixture.source, fixture.child_source);
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .unwrap()
                .is_some(),
                "non-vacuous genuine source relationship"
            );
            let foreign = [OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &other.outcome,
            }];
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &foreign,
                    views,
                )
                .is_err()
            );
            let duplicate = [
                OriginalConditionalCommandWorld {
                    uri: "file:///main.tcl",
                    outcome: &fixture.outcome,
                },
                OriginalConditionalCommandWorld {
                    uri: "file:///main.tcl",
                    outcome: &fixture.outcome,
                },
            ];
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &duplicate,
                    views,
                )
                .is_err()
            );
            let mut duplicate_documents = documents.clone();
            duplicate_documents.push(documents[1]);
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &duplicate_documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .is_err()
            );
        });
    }

    #[test]
    fn inherited_selection_refuses_stale_source_input_and_ancestry() {
        let fixture = Fixture::new(
            "source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {return VALUE}",
        );
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .unwrap()
                .is_some()
            );
            let changed = format!("{} ", fixture.child_source);
            let mut stale = documents.clone();
            stale[1].source = &changed;
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &stale,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .is_err()
            );
            let changed_input = Analyser::new().analyse(fixture.child_source, "tcl9.0");
            let mut stale_input = documents.clone();
            stale_input[1].analysis = &changed_input;
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &stale_input,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .is_err()
            );
            let missing_ancestry = views
                .iter()
                .map(|view| OriginalSourceViewDocument {
                    publication: view.publication,
                    document: view.document,
                    region: view.region,
                    ancestors: &[],
                })
                .collect::<Vec<_>>();
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    &missing_ancestry,
                )
                .is_err()
            );
        });
    }

    #[test]
    fn inherited_selection_never_promotes_a_native_namespace_entry_or_logical_cut() {
        let source = "namespace eval ::N {source child.tcl}; proc run {} {return VALUE}";
        let child_source = "proc helper {} {return VALUE}";
        let fixture = Fixture::new(source, child_source);
        assert!(fixture.region_count > 1);

        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .is_err()
            );
            assert!(
                super::super::original_command_rename_edits_with_selected_source_view(
                    &documents,
                    &identity,
                    "updated_helper",
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .is_err()
            );
        });
        assert_logical_source_rename_issuer_unavailable(source, child_source);
    }
}
