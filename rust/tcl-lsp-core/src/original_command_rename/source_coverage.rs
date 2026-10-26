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
    ByteCommandSlot, CommandLinkCoverage, Declaration, HashSet, NameBytes,
    OriginalCommandRenameRefusal, OriginalConditionalCommandWorld, OriginalDeclarationDocument,
    OriginalDeclarationIdentity, SignatureSourceNameInput, SourceCommandReference, SourceImage,
    authored_result_completion_coverage, coverage_region, destination_matches, queue_declaration,
    quiet_store_coverage, reject_command_link_coverage, reject_incomplete_source_procedure,
    reject_readonly_math_references, reject_selected_command_link_coverage,
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

fn capture_cut<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
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
    let child_physical = physical_document(documents, child_view.document)?;
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
        || (identity.uri() == child_physical.uri
            && !std::ptr::eq(child_physical.analysis, child_view.document.analysis))
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
            document,
            commands: walk.segment_nested(source, offset),
            purpose,
            start,
            end,
            depth,
        })
    }
    fn key(&self) -> (usize, usize, Purpose, usize, usize) {
        (
            std::ptr::from_ref(self.document.analysis).addr(),
            self.document
                .analysis
                .original_source_region()
                .map_or(0, |region| std::ptr::from_ref(region).addr()),
            self.purpose,
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
        let tokens = walk.tokens(document.source, command);
        let binding = require_coverage(tokens.source_binding.as_ref(), offset, "cut.binding")?;
        let input = require_coverage(
            binding.original_head_name_input(&tokens),
            offset,
            "cut.input",
        )?;
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
        let reference = binding.original_command_reference(&tokens);
        if let Some(reference) = &reference {
            if !reference.matches_original_invocation_site(lookup.site()) {
                coverage_trace(offset, "cut.reference-site");
                return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
            }
            if context.declaration.reference_matches(reference, true) {
                if !context.declaration.reference_matches(reference, false) {
                    return Err(OriginalCommandRenameRefusal::UneditableLinks);
                }
                destination_matches(
                    context.documents,
                    context.conditional,
                    &lookup,
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
            &self.survey,
            task,
            walk,
            command,
            &CommandSubject {
                words: &words,
                input: &input,
                reference: reference.as_ref(),
            },
            output,
        )
    }
}
struct CommandSubject<'a> {
    words: &'a [tcl_lexer::NativeWord],
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
    output.nested(
        task,
        walk,
        vec![(body.start() as usize, body.end() as usize, task.depth + 1)],
        Purpose::ProcedureBody,
    )?;
    Ok(true)
}

fn collect_command_regions<'a>(
    survey: &OriginalSingleSourceCutSurvey<'_>,
    task: &Task<'a>,
    walk: &crate::refactor::FrameWalk<'a>,
    command: &SegmentedCommand,
    subject: &CommandSubject<'_>,
    output: &mut Output<'a>,
) -> Result<(), OriginalCommandRenameRefusal> {
    if queue_cut_declaration(survey, task, walk, command, output)? {
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
    if let Some(selected) = walk.structure(document.source, command) {
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
        reject_incomplete_source_procedure(
            document,
            subject.reference,
            subject.words,
            subject.input,
            offset,
        )?;
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

        fn logical_namespace(source: &'static str, child_source: &'static str) -> Self {
            let profile = tcl_dialect::DialectProfile::plain_tcl();
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            assert!(input.has_logical_source_name_context());
            let prefix = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            let source_offset = u32::try_from(source.find("source child").unwrap()).unwrap();
            let load = tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &prefix, source_offset,
            ).unwrap();
            let point = load.source_point_entry(&prefix).unwrap();
            let child = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse_with_source_point_entry(child_source, profile.name, &point)
                .unwrap();
            let regions = load
                .conditional_continuation(&prefix, &child)
                .unwrap()
                .source_regions(&prefix, &child)
                .unwrap();
            let region = regions.regions()[0].clone();
            let suffix = Analyser::new()
                .with_resolved_input(input)
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
        }
        let prefix_walk = crate::refactor::FrameWalk::new(fixture.source, &fixture.prefix).unwrap();
        let prefix_commands = prefix_walk.segment_document(fixture.source);
        assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
            fixture.source, &fixture.prefix, &prefix_commands[0], survey,
        ).is_none());
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

    fn assert_native_namespace_child_entry_refused(source: &str, child_source: &str) {
        let prefix = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &prefix, offset,
            )
            .unwrap();
        let point = load.source_point_entry(&prefix).unwrap();
        assert!(
            !prefix
                .resolved_input
                .as_ref()
                .unwrap()
                .has_logical_source_name_context()
        );
        assert!(
            Analyser::new()
                .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
                .is_none()
        );
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
                assert_native_namespace_child_entry_refused(source, child_source);
                Fixture::logical_namespace(source, child_source)
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
}
