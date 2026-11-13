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

//! Atomic command-name edits from current original owners and byte geometry.

mod linked_authoring;
mod source_coverage;
pub(crate) use linked_authoring::OriginalLinkedSourceSpelling;

use std::collections::HashSet;

use tcl_compiler::analyser::{ClassDef, ProcDef};
use tcl_compiler::command_binding::{OriginalCommandLookup, SourceCommandReference};
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_compiler::signature_scan::scope::{SignatureSourceCommand, SignatureSourceNameInput};
use tcl_core_types::{ByteCommandSlot, NameBytes};
use tcl_lexer::{SourceImage, Span};
use tcl_syntax::naming::NamePolicyProtocol;

use crate::original_declaration::{
    OriginalDeclarationDocument, OriginalDeclarationIdentity, OriginalDeclarationRole,
};
use crate::rename::WorkspaceTextEdit;

/// An independent obligation missing from an atomic original command edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalCommandRenameRefusal {
    /// A declaration or consumer no longer has its full original source/configuration.
    StaleSource,
    /// Equal source bytes do not establish unique document ownership.
    AmbiguousOwner,
    /// This role lacks a command publication edit protocol.
    UnsupportedRole,
    /// The proposed Unicode API tail has no exact selected native component.
    InvalidTail,
    /// A partial/abrupt source world cannot prove collision coverage.
    UnknownWorld,
    /// The selected original command has been moved, replaced or removed.
    ChangedPublication,
    /// Another command occupies the proposed publication or earlier lookup slot.
    Collision,
    /// A required table holder or alternative lookup is unknown.
    UnknownDestination,
    /// The source contains unclosed executable regions or command selections.
    IncompleteCoverage,
    /// Name-link, observer or deferred-prefix edit obligations are not closed.
    UneditableLinks,
    /// A selected producer lacks an independently editable static container.
    UneditableInput,
    /// Expression function references have no separate writable identifier owner.
    ReadonlyMathReferences,
}

impl OriginalCommandRenameRefusal {
    /// Stable code for host adapters; no diagnostic text supplies identity.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StaleSource => "stale-original-source",
            Self::AmbiguousOwner => "ambiguous-original-owner",
            Self::UnsupportedRole => "unsupported-command-edit-role",
            Self::InvalidTail => "invalid-original-command-tail",
            Self::UnknownWorld => "unknown-completed-command-world",
            Self::ChangedPublication => "changed-original-publication",
            Self::Collision => "original-command-collision",
            Self::UnknownDestination => "unknown-command-destination",
            Self::IncompleteCoverage => "incomplete-original-command-coverage",
            Self::UneditableLinks => "unclosed-original-name-link-edits",
            Self::UneditableInput => "uneditable-original-command-input",
            Self::ReadonlyMathReferences => "readonly-expression-function-references",
        }
    }

    /// The missing obligation; this supplies no replacement lookup or edit grant.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::StaleSource => {
                "the command edit no longer matches every complete original source and lexer configuration"
            }
            Self::AmbiguousOwner => {
                "the command edit has duplicate or ambiguous declaring document owners"
            }
            Self::UnsupportedRole => {
                "this declaration has no separately selected command publication edit protocol"
            }
            Self::InvalidTail => {
                "the proposed command tail does not preserve its selected native naming purpose"
            }
            Self::UnknownWorld => {
                "a completed original command world is unavailable for collision coverage"
            }
            Self::ChangedPublication => {
                "the selected declaration no longer occupies its original command publication"
            }
            Self::Collision => {
                "the proposed command name collides with a retained occupied command slot"
            }
            Self::UnknownDestination => {
                "the proposed command lookup has an unknown holder or alternative"
            }
            Self::IncompleteCoverage => {
                "the complete source contains an executable region or command selection without original coverage"
            }
            Self::UneditableLinks => {
                "name links, observers or callback prefixes lack complete separately authorised edit coverage"
            }
            Self::UneditableInput => {
                "a selected command name lacks a complete original static source edit container"
            }
            Self::ReadonlyMathReferences => {
                "expression identifier references have no separately authorised edit owner"
            }
        }
    }
}

enum Declaration<'a> {
    Procedure(&'a SourceDeclarationMetadata<ProcDef>),
    Class(&'a SourceDeclarationMetadata<ClassDef>),
}

impl Declaration<'_> {
    fn name(&self) -> &SignatureSourceCommand {
        match self {
            Self::Procedure(row) => row.name(),
            Self::Class(row) => row.name(),
        }
    }

    fn site(&self) -> &tcl_compiler::command_binding::CommandAllocationSite {
        match self {
            Self::Procedure(row) => row.declaration_site(),
            Self::Class(row) => row.declaration_site(),
        }
    }

    fn reference_matches(&self, reference: &SourceCommandReference, links: bool) -> bool {
        match self {
            Self::Procedure(row) => {
                crate::original_declaration::reference_matches_declaration(reference, row, links)
            }
            Self::Class(row) => {
                crate::original_declaration::reference_matches_declaration(reference, row, links)
            }
        }
    }
}

fn renamed_slot(
    slot: &ByteCommandSlot,
    policy: NamePolicyProtocol,
    tail: &[u8],
) -> Result<ByteCommandSlot, OriginalCommandRenameRefusal> {
    Ok(ByteCommandSlot {
        namespace: slot.namespace.clone(),
        simple: NameBytes::from(
            policy
                .recipe()
                .replace_command_tail(slot.simple.as_bytes(), tail)
                .ok_or(OriginalCommandRenameRefusal::InvalidTail)?,
        ),
    })
}

// WORK-only bounded diagnostic state, consumed synchronously by the server.
thread_local! {
    static MISSING_RENAME_WORLD: std::cell::RefCell<Option<String>> = const {
        std::cell::RefCell::new(None)
    };
}

/// Consume the last reached missing-world producer diagnostic, without querying it.
#[must_use]
pub fn take_missing_rename_world_trace() -> Option<String> {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        MISSING_RENAME_WORLD.with(|trace| trace.borrow_mut().take())
    } else {
        None
    }
}

// WORK-only identity observer: the original producer getter above runs once.
fn trace_missing_rename_world(
    document: &OriginalDeclarationDocument<'_>,
    ordinal: usize,
    count: usize,
) {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        let detail = format!(
            "rename.missing-world ordinal={ordinal} count={count} uri={} analysis_owner={:p} source_bytes={} resolved_input={} unavailable={} body_config={:?}",
            document.uri,
            document.analysis,
            document.source.len(),
            document.analysis.resolved_input.is_some(),
            document.analysis.analysis_context_unavailable.is_some(),
            document.analysis.body_lexer_config,
        );
        MISSING_RENAME_WORLD.with(|trace| *trace.borrow_mut() = Some(detail));
    }
    if *ENABLED.get_or_init(|| std::env::var("TCL_LSP_TRACE_COARSE_SOURCE").as_deref() == Ok("1")) {
        eprintln!(
            "RENAME_WORLD declined ordinal={ordinal} count={count} uri={} source_bytes={} resolved_input={} unavailable={} body_config={:?}",
            document.uri,
            document.source.len(),
            document.analysis.resolved_input.is_some(),
            document.analysis.analysis_context_unavailable.is_some(),
            document.analysis.body_lexer_config,
        );
    }
}

/// A kernel-owned whole-root conditional outcome. The host must independently
/// retain and recheck every physical prefix/child/region publication it used.
pub struct OriginalConditionalCommandWorld<'a> {
    /// Current physical root URI owned and rechecked by the host.
    pub uri: &'a str,
    /// Exact compiler-issued whole-root continuation, independently joined below.
    pub outcome: &'a tcl_compiler::command_binding::OriginalSourceConditionalOutcome,
}

enum RenameCollisionWorld<'a, 'borrow> {
    Ordinary(tcl_compiler::command_binding::OriginalCompletedCommandWorld),
    ClosedCut(&'borrow tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>),
}

impl RenameCollisionWorld<'_, '_> {
    fn source_image(&self) -> &SourceImage {
        match self {
            Self::Ordinary(world) => world.source_image(),
            Self::ClosedCut(cut) => cut.source_image(),
        }
    }
    fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        match self {
            Self::Ordinary(world) => world.lexer_config(),
            Self::ClosedCut(cut) => cut.lexer_config(),
        }
    }
    fn command_slot_occupied(
        &self,
        slot: &ByteCommandSlot,
        policy: NamePolicyProtocol,
    ) -> Option<bool> {
        match self {
            Self::Ordinary(world) => world.command_slot_occupied(slot, policy),
            Self::ClosedCut(cut) => cut.command_slot_occupied(slot, policy),
        }
    }
}

fn collision_world<'a, 'borrow>(
    document: &OriginalDeclarationDocument<'a>,
    conditional: &[OriginalConditionalCommandWorld<'a>],
    selected_cut: Option<
        &'borrow tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>,
    >,
) -> Option<RenameCollisionWorld<'a, 'borrow>> {
    if let Some(world) = document.analysis.original_completed_command_world() {
        return Some(RenameCollisionWorld::Ordinary(world.clone()));
    }
    if let Some(cut) = selected_cut.filter(|cut| cut.is_for_analysis(document.analysis)) {
        return Some(RenameCollisionWorld::ClosedCut(cut));
    }
    let mut selected = conditional.iter().filter(|entry| entry.uri == document.uri);
    let entry = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    entry
        .outcome
        .original_completed_command_world(document.analysis)
        .map(RenameCollisionWorld::Ordinary)
}

fn occupied_with_cut<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    conditional: &[OriginalConditionalCommandWorld<'a>],
    selected_cut: Option<&tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>>,
    slot: &ByteCommandSlot,
    policy: NamePolicyProtocol,
) -> Result<bool, OriginalCommandRenameRefusal> {
    let mut present = false;
    for (ordinal, document) in documents.iter().enumerate() {
        let world = collision_world(document, conditional, selected_cut).ok_or_else(|| {
            trace_missing_rename_world(document, ordinal, documents.len());
            OriginalCommandRenameRefusal::UnknownWorld
        })?;
        present |= world
            .command_slot_occupied(slot, policy)
            .ok_or(OriginalCommandRenameRefusal::UnknownDestination)?;
    }
    Ok(present)
}

fn destination_matches(
    documents: &[OriginalDeclarationDocument<'_>],
    conditional: &[OriginalConditionalCommandWorld<'_>],
    lookup: &OriginalCommandLookup,
    destination: &ByteCommandSlot,
    tail: &[u8],
) -> Result<(), OriginalCommandRenameRefusal> {
    destination_matches_with_cut(documents, conditional, None, lookup, destination, tail)
}

fn destination_matches_with_cut<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    conditional: &[OriginalConditionalCommandWorld<'a>],
    selected_cut: Option<&tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>>,
    lookup: &OriginalCommandLookup,
    destination: &ByteCommandSlot,
    tail: &[u8],
) -> Result<(), OriginalCommandRenameRefusal> {
    if lookup.candidates().is_empty() {
        return Err(OriginalCommandRenameRefusal::UnknownDestination);
    }
    for path in lookup.candidates() {
        let mut reached = false;
        for old in path {
            let candidate = renamed_slot(old, lookup.policy(), tail)?;
            if &candidate == destination {
                reached = true;
                break;
            }
            if occupied_with_cut(
                documents,
                conditional,
                selected_cut,
                &candidate,
                lookup.policy(),
            )? {
                return Err(OriginalCommandRenameRefusal::Collision);
            }
        }
        if !reached {
            return Err(OriginalCommandRenameRefusal::UnknownDestination);
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum CommandLinkCoverage<'borrow, 'analysis> {
    Registry(&'borrow tcl_compiler::registry_invocation::ResolvedStatementInvocation),
    SourceLoad {
        receipt: &'borrow tcl_compiler::registry_invocation::OriginalConditionalSourceLoadCoverage<
            'analysis,
        >,
        analysis: &'borrow tcl_compiler::analyser::AnalysisResult,
    },
    QuietScalarStore {
        receipt: &'borrow tcl_compiler::registry_invocation::OriginalQuietScalarStoreSourceCoverage<
            'analysis,
        >,
        analysis: &'borrow tcl_compiler::analyser::AnalysisResult,
    },
}

pub(crate) fn reject_link_obligations(
    selected: &tcl_compiler::registry_invocation::ResolvedStatementInvocation,
) -> Result<(), OriginalCommandRenameRefusal> {
    reject_command_link_coverage(CommandLinkCoverage::Registry(selected))
}

fn reject_command_link_coverage(
    coverage: CommandLinkCoverage<'_, '_>,
) -> Result<(), OriginalCommandRenameRefusal> {
    use tcl_registry::{CommandBindingTransition, NamespaceTransition, StateTransition};
    let selected = match coverage {
        CommandLinkCoverage::Registry(selected) => selected,
        CommandLinkCoverage::SourceLoad { receipt, analysis } => {
            if !receipt.is_for_analysis(analysis) {
                return Err(OriginalCommandRenameRefusal::StaleSource);
            }
            receipt.invocation()
        }
        CommandLinkCoverage::QuietScalarStore { receipt, analysis } => {
            if !receipt.is_for_analysis(analysis) {
                return Err(OriginalCommandRenameRefusal::StaleSource);
            }
            receipt.invocation()
        }
    };
    if selected.facts.traits.intersects(
        tcl_registry::Traits::REFLECTS_COMMAND_NAMES
            | tcl_registry::Traits::CURRENT_FRAME_INTROSPECTION
            | tcl_registry::Traits::INVOKES_USER_PROC,
    ) {
        return Err(OriginalCommandRenameRefusal::UneditableLinks);
    }
    if !selected.facts.arg_roles_complete {
        return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
    }
    let transition_facts = match selected.facts.state_transitions.declared() {
        Some(transitions) => transitions.facts(),
        None => match coverage {
            // The completed command world closes over the same generic
            // invocation, so its link inventory is the same empty one.
            CommandLinkCoverage::Registry(_)
                if tcl_compiler::command_binding::undeclared_transitions_leave_command_world_closed(
                    &selected.facts,
                ) =>
            {
                &[]
            }
            CommandLinkCoverage::Registry(_) | CommandLinkCoverage::SourceLoad { .. } => {
                return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
            }
            // The exact original quiet-write proof closes only this wildcard
            // identity-transition obligation. Registry facts remain unknown;
            // any declared transitions above retain every original check.
            CommandLinkCoverage::QuietScalarStore { .. } => &[],
        },
    };
    for fact in transition_facts {
        match &fact.transition {
            StateTransition::CommandBinding(CommandBindingTransition::Define { .. })
            | StateTransition::Namespace(NamespaceTransition::Ensure { .. })
            | StateTransition::ObjectDispatch(_)
            | StateTransition::VariableCellAlias(_) => {}
            StateTransition::CommandBinding(_)
            | StateTransition::Namespace(_)
            | StateTransition::Trace(_) => {
                return Err(OriginalCommandRenameRefusal::UneditableLinks);
            }
            StateTransition::Package(
                tcl_registry::model::binding::PackageTransition::SourceLoad { path },
            ) if matches!(coverage, CommandLinkCoverage::SourceLoad { receipt, .. }
                    if path.argument_index() == Some(receipt.path_argument())) => {}
            StateTransition::Interpreter(_)
            | StateTransition::Package(_)
            | StateTransition::Widen(_) => {
                return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
            }
        }
    }
    if selected.written_argument_roles().iter().any(|(_, role)| {
        matches!(
            role,
            tcl_registry::ArgRole::CommandPrefix
                | tcl_registry::ArgRole::CommandName
                | tcl_registry::ArgRole::CommandNameProbe
        )
    }) {
        return Err(OriginalCommandRenameRefusal::UneditableLinks);
    }
    Ok(())
}

fn trace_coverage_document(document: &OriginalDeclarationDocument<'_>) {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        MISSING_RENAME_WORLD.with(|trace| {
            *trace.borrow_mut() = Some(format!(
                "rename.coverage-document source_bytes={} analysis_owner={:p}",
                document.source.len(),
                document.analysis,
            ));
        });
    }
}

pub(crate) fn structure_coverage_trace_enabled() -> bool {
    matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1"))
        && MISSING_RENAME_WORLD.with(|trace| {
            trace
                .borrow()
                .as_deref()
                .is_some_and(|value| value.starts_with("rename.coverage-document "))
        })
}

pub(crate) fn trace_structure_coverage(offset: u32, metadata: bool, rows: &[(&'static str, bool)]) {
    if !structure_coverage_trace_enabled() {
        return;
    }
    MISSING_RENAME_WORLD.with(|trace| {
        let mut trace = trace.borrow_mut();
        let Some(value) = trace.as_mut() else { return; };
        let context = value.split(" structure_offset=").next().unwrap_or(value)
            .split(" offset=").next().unwrap_or(value).to_owned();
        let stages = rows.iter().map(|(stage, passed)| format!("{stage}:{passed}"))
            .collect::<Vec<_>>().join(",");
        *value = format!("{context} structure_offset={offset} structure_metadata={metadata} structure_steps={stages}");
    });
}

fn coverage_trace(offset: u32, stage: &'static str) {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        MISSING_RENAME_WORLD.with(|trace| {
            let mut trace = trace.borrow_mut();
            let context = trace
                .as_deref()
                .filter(|value| value.starts_with("rename.coverage-document "))
                .map_or("rename.coverage", |value| {
                    value.split(" offset=").next().unwrap_or(value)
                });
            *trace = Some(format!("{context} offset={offset} stage={stage}"));
        });
    }
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_RENAME").is_some() {
        eprintln!("ORIGINAL_COMMAND_RENAME offset={offset} stage={stage}");
    }
    #[cfg(not(test))]
    let _ = (offset, stage);
}

fn require_coverage<T>(
    value: Option<T>,
    offset: u32,
    stage: &'static str,
) -> Result<T, OriginalCommandRenameRefusal> {
    value.ok_or_else(|| {
        coverage_trace(offset, stage);
        OriginalCommandRenameRefusal::IncompleteCoverage
    })
}

fn coverage_region(
    source: &str,
    start: usize,
    end: usize,
) -> Result<&str, OriginalCommandRenameRefusal> {
    source
        .get(start..end)
        .ok_or(OriginalCommandRenameRefusal::StaleSource)
}

fn quiet_store_coverage(
    walk: &crate::refactor::FrameWalk<'_>,
    document: OriginalDeclarationDocument<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    offset: u32,
) -> Result<bool, OriginalCommandRenameRefusal> {
    let Some(receipt) = walk.quiet_store_for_rename_coverage(document.source, command) else {
        return Ok(false);
    };
    let result = reject_command_link_coverage(CommandLinkCoverage::QuietScalarStore {
        receipt: &receipt,
        analysis: document.analysis,
    });
    result.inspect_err(|_| {
        coverage_trace(offset, "registry-name-link-or-effect-coverage");
    })?;
    Ok(true)
}

fn authored_result_completion_coverage(
    walk: &crate::refactor::FrameWalk<'_>,
    document: OriginalDeclarationDocument<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    offset: u32,
    depth: u32,
    pending: &mut Vec<(usize, usize, u32)>,
) -> Result<bool, OriginalCommandRenameRefusal> {
    let Some(receipt) = walk.result_completion_for_rename_coverage(document.source, command) else {
        return Ok(false);
    };
    if !receipt.is_for_analysis(document.analysis) {
        return Err(OriginalCommandRenameRefusal::StaleSource);
    }
    // All generic trait/name-role/known and unknown transition refusals remain.
    reject_selected_command_link_coverage(receipt.invocation(), offset)?;
    pending.extend(
        receipt
            .argument_regions()
            .iter()
            .map(|span| (span.start() as usize, span.end() as usize, depth + 1)),
    );
    Ok(true)
}

fn queue_source_coverage(
    walk: &crate::refactor::FrameWalk<'_>,
    document: OriginalDeclarationDocument<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    depth: u32,
    pending: &mut Vec<(usize, usize, u32)>,
) -> Result<bool, OriginalCommandRenameRefusal> {
    let offset = command
        .argv
        .first()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?
        .span
        .start();
    Ok(queue_declaration(walk, document, command, depth, pending)?
        || quiet_store_coverage(walk, document, command, offset)?
        || authored_result_completion_coverage(walk, document, command, offset, depth, pending)?)
}

fn queue_declaration(
    walk: &crate::refactor::FrameWalk<'_>,
    document: OriginalDeclarationDocument<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    depth: u32,
    pending: &mut Vec<(usize, usize, u32)>,
) -> Result<bool, OriginalCommandRenameRefusal> {
    let Some((selected, body)) =
        walk.procedure_declaration_for_rename_coverage(document.source, command)
    else {
        return Ok(false);
    };
    let command_offset = command
        .argv
        .first()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?
        .span
        .start();
    reject_selected_command_link_coverage(&selected, command_offset)?;
    pending.push((body.start() as usize, body.end() as usize, depth + 1));
    Ok(true)
}

fn document_requests(
    document: OriginalDeclarationDocument<'_>,
    documents: &[OriginalDeclarationDocument<'_>],
    conditional: &[OriginalConditionalCommandWorld<'_>],
    declaration: &Declaration<'_>,
    destination: &ByteCommandSlot,
    tail: &[u8],
) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalCommandRenameRefusal> {
    trace_coverage_document(&document);
    let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
    let mut requests = Vec::new();
    let mut pending = vec![(0_usize, document.source.len(), 0_u32)];
    let mut visited = HashSet::new();
    while let Some((start, end, depth)) = pending.pop() {
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth)
            || start > end
            || end > document.source.len()
        {
            return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
        }
        if !visited.insert((start, end)) {
            continue;
        }
        let region = coverage_region(document.source, start, end)?;
        let offset = u32::try_from(start).map_err(|_| OriginalCommandRenameRefusal::StaleSource)?;
        for command in if depth == 0 {
            walk.segment_document(region)
        } else {
            walk.segment_nested(region, offset)
        } {
            let command_offset = command
                .argv
                .first()
                .map_or(offset, |word| word.span.start());
            let words = require_coverage(
                walk.native_words(document.source, &command),
                command_offset,
                "native-words",
            )?;
            let tokens = walk.tokens(document.source, &command);
            let binding = require_coverage(
                tokens.source_binding.as_ref(),
                command_offset,
                "source-binding",
            )?;
            let input = require_coverage(
                binding.original_head_name_input(&tokens),
                command_offset,
                "original-head-input",
            )?;
            let lookup = require_coverage(
                binding.original_command_lookup(&tokens, &input),
                command_offset,
                "original-lookup",
            )?;
            let image = SourceImage::document(document.source);
            if lookup.name_input() != &input || lookup.site().source.source_image() != &image {
                return Err(OriginalCommandRenameRefusal::StaleSource);
            }
            let reference = binding.original_command_reference(&tokens);
            if let Some(reference) = &reference {
                if !reference.matches_original_invocation_site(lookup.site()) {
                    return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
                }
                if declaration.reference_matches(reference, true) {
                    if !declaration.reference_matches(reference, false) {
                        return Err(OriginalCommandRenameRefusal::UneditableLinks);
                    }
                    destination_matches(documents, conditional, &lookup, destination, tail)?;
                    let wanted = input
                        .policy()
                        .recipe()
                        .replace_command_tail(input.bytes(), tail)
                        .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
                    requests.push((input.clone(), NameBytes::from(wanted)));
                }
            }
            if queue_source_coverage(&walk, document, &command, depth, &mut pending)? {
                continue;
            }
            if let Some(selected) = walk.structure_for_rename_coverage(document.source, &command) {
                reject_selected_command_link_coverage(&selected, command_offset)?;
                for (a, b) in walk
                    .same_frame_regions(document.source, &command)
                    .into_iter()
                    .chain(walk.frame_shifted_regions(document.source, &command))
                {
                    pending.push((a, b, depth + 1));
                }
                if !walk.complete() {
                    coverage_trace(command_offset, "executable-regions");
                    return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
                }
            } else {
                reject_incomplete_source_procedure(
                    document,
                    ensemble_dispatch_reference(document, reference.as_ref(), command_offset),
                    &words,
                    &input,
                    command_offset,
                )?;
            }
        }
    }
    Ok(requests)
}

/// Plan a complete Proc/Class command rename over independently current source
/// owners. Declaration and direct call names use their own static containers;
/// native bytes, actual call-point allocations and all lookup alternatives
/// remain independent of reporting maps. Duplicate owners, unclosed regions,
/// name links, readonly math identifiers and unknown collisions refuse the
/// whole edit. This issues editor replacements, never an execution, insertion,
/// allocation, Normal completion or arbitrary reflection-equivalence receipt.
pub fn original_command_rename_edits(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    new_tail: &str,
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    original_command_rename_edits_with_conditional_worlds(documents, identity, new_tail, &[])
}

/// Preserve all ordinary edit guards while admitting independently current,
/// fully completed conditional source worlds solely for collision coverage.
pub fn original_command_rename_edits_with_conditional_worlds(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    new_tail: &str,
    conditional: &[OriginalConditionalCommandWorld<'_>],
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    if !documents
        .iter()
        .any(|document| document.uri == identity.uri())
    {
        return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
    }
    let policy = identity
        .input()
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?
        .policy();
    let tail = tcl_syntax::backslash::native_source_literal_bytes(
        new_tail.as_bytes(),
        SourceImage::document(new_tail).channel(),
        policy.string_protocol(),
    )
    .map_err(|_| OriginalCommandRenameRefusal::InvalidTail)?;
    original_command_rename_units_with_conditional_worlds(documents, identity, &tail, conditional)
}

/// Plan edits with complete typed physical/interpretation coverage for finite
/// source cuts. Every ordinary edit/publication guard remains independent.
pub fn original_command_rename_edits_with_source_views<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
    new_tail: &str,
    conditional: &[OriginalConditionalCommandWorld<'a>],
    views: &[crate::original_source_views::OriginalSourceViewDocument<'a>],
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    let coverage =
        source_coverage::SourceCoverage::capture(documents, identity, conditional, views)?;
    let policy = identity
        .input()
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?
        .policy();
    let tail = tcl_syntax::backslash::native_source_literal_bytes(
        new_tail.as_bytes(),
        SourceImage::document(new_tail).channel(),
        policy.string_protocol(),
    )
    .map_err(|_| OriginalCommandRenameRefusal::InvalidTail)?;
    original_command_rename_units_with_source_coverage(
        documents,
        identity,
        &tail,
        conditional,
        Some(&coverage),
        None,
    )
}

/// Plan under a unique inherited declaration interpretation from a genuine
/// single source cut. The physical declaration retains its independent ordinary
/// rename guards; conditional outcomes supply relationship/collision facts only.
/// The host must hold the exact source-view revision and publication leases.
///
/// # Errors
/// Returns the unchanged ordinary guards or a missing/ambiguous inherited join.
pub fn original_command_rename_edits_with_selected_source_view<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
    new_tail: &str,
    cursor_uri: &str,
    cursor: u32,
    conditional: &[OriginalConditionalCommandWorld<'a>],
    views: &[crate::original_source_views::OriginalSourceViewDocument<'a>],
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    let Some(selected) = source_coverage::OriginalInheritedDeclarationSelection::capture(
        documents,
        identity,
        cursor_uri,
        cursor,
        conditional,
        views,
    )?
    else {
        return original_command_rename_edits_with_source_views(
            documents,
            identity,
            new_tail,
            conditional,
            views,
        );
    };
    // Validate the physical interpretation independently; it is never rewritten
    // into an inherited owner or supplied with a conditional completed world.
    let physical = original_command_rename_edits(&[selected.physical()], identity, new_tail)?;
    let coverage = source_coverage::SourceCoverage::capture(
        selected.documents(),
        selected.identity(),
        conditional,
        views,
    )?;
    let policy = selected
        .identity()
        .input()
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?
        .policy();
    let tail = tcl_syntax::backslash::native_source_literal_bytes(
        new_tail.as_bytes(),
        SourceImage::document(new_tail).channel(),
        policy.string_protocol(),
    )
    .map_err(|_| OriginalCommandRenameRefusal::InvalidTail)?;
    let inherited = original_command_rename_units_with_source_coverage(
        selected.documents(),
        selected.identity(),
        &tail,
        conditional,
        Some(&coverage),
        Some(selected.publication()),
    )?;
    require_selected_physical_edits(&physical, &inherited, identity.uri())?;
    Ok(inherited)
}

/// Prepare the original declaration header under the same unique inherited
/// relationship and complete planning guards used for its subsequent rename.
/// The host must hold the exact source-view revision and publication leases.
///
/// # Errors
/// Refuses cursor, source, namespace, identity, coverage and world mismatches.
pub fn original_command_prepare_span_in_with_selected_source_view<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
    cursor_uri: &str,
    cursor: u32,
    conditional: &[OriginalConditionalCommandWorld<'a>],
    views: &[crate::original_source_views::OriginalSourceViewDocument<'a>],
) -> Result<tcl_lexer::Span, OriginalCommandRenameRefusal> {
    let Some(selected) = source_coverage::OriginalInheritedDeclarationSelection::capture(
        documents,
        identity,
        cursor_uri,
        cursor,
        conditional,
        views,
    )?
    else {
        return original_command_prepare_span_in_with_conditional_worlds(
            documents,
            identity,
            cursor_uri,
            cursor,
            conditional,
        );
    };
    if cursor_uri != identity.uri() {
        return Err(OriginalCommandRenameRefusal::UneditableInput);
    }
    let span = original_command_prepare_span(selected.physical(), identity, cursor)?;
    let input = selected
        .identity()
        .input()
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
    let extent = input
        .policy()
        .recipe()
        .command_tail_extent(input.bytes())
        .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
    let coverage = source_coverage::SourceCoverage::capture(
        selected.documents(),
        selected.identity(),
        conditional,
        views,
    )?;
    let physical = original_command_rename_units_with_conditional_worlds(
        &[selected.physical()],
        identity,
        &input.bytes()[extent.clone()],
        &[],
    )?;
    let inherited = original_command_rename_units_with_source_coverage(
        selected.documents(),
        selected.identity(),
        &input.bytes()[extent],
        conditional,
        Some(&coverage),
        Some(selected.publication()),
    )?;
    require_selected_physical_edits(&physical, &inherited, identity.uri())?;
    Ok(span)
}

fn require_selected_physical_edits(
    physical: &[WorkspaceTextEdit],
    inherited: &[WorkspaceTextEdit],
    selected_uri: &str,
) -> Result<(), OriginalCommandRenameRefusal> {
    if !inherited
        .iter()
        .filter(|edit| edit.uri == selected_uri)
        .eq(physical.iter())
    {
        return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
    }
    Ok(())
}

fn rename_declaration<'a>(
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    identity: &OriginalDeclarationIdentity,
) -> Result<Declaration<'a>, OriginalCommandRenameRefusal> {
    Ok(match identity.role() {
        OriginalDeclarationRole::Procedure => Declaration::Procedure(
            identity
                .procedure_metadata(analysis)
                .ok_or(OriginalCommandRenameRefusal::StaleSource)?,
        ),
        OriginalDeclarationRole::Class => Declaration::Class(
            identity
                .class_metadata(analysis)
                .ok_or(OriginalCommandRenameRefusal::StaleSource)?,
        ),
        _ => return Err(OriginalCommandRenameRefusal::UnsupportedRole),
    })
}

/// Ensemble words naming the renamed command: a `-map` target head, a
/// `-subcommands` entry and a dispatch word under `-subcommands` all spell the
/// command, while a `-map` key is arbitrary and stays. A word of the same tail
/// without an agreed receipt cannot be proved either way, so it refuses.
fn ensemble_word_requests(
    document: OriginalDeclarationDocument<'_>,
    declaration: &Declaration<'_>,
    tail: &[u8],
) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalCommandRenameRefusal> {
    let simple = declaration.name().slot().simple.as_bytes();
    let mut requests = Vec::new();
    for invocation in &document.analysis.command_invocations {
        if !invocation.matches_by_receipt()
            || invocation.ensemble_dispatch
                == Some(tcl_compiler::signature_scan::types::EnsembleSubcommandProvenance::Map)
        {
            continue;
        }
        let Some(reference) = &invocation.resolved_command_reference else {
            if invocation
                .resolved_qualified_name
                .as_deref()
                .is_some_and(|name| tcl_syntax::naming::key_tail(name).as_bytes() == simple)
            {
                return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
            }
            continue;
        };
        if !declaration.reference_matches(reference, true) {
            continue;
        }
        if reference
            .linked_definition()
            .is_some_and(|linked| Some(linked) != reference.definition())
        {
            return Err(OriginalCommandRenameRefusal::UneditableLinks);
        }
        let input = invocation
            .original_name_input
            .as_ref()
            .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
        let wanted = input
            .policy()
            .recipe()
            .replace_command_tail(input.bytes(), tail)
            .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
        requests.push((input.clone(), NameBytes::from(wanted)));
    }
    Ok(requests)
}

fn rename_document_edits<'a>(
    document: OriginalDeclarationDocument<'a>,
    context: &source_coverage::RequestContext<'_, 'a>,
    original: &SignatureSourceNameInput,
    replacement: &[u8],
    coverage: Option<&source_coverage::SourceCoverage<'a>>,
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    reject_readonly_math_references(document, &context.owner, context.declaration)?;
    let mut requests = match coverage.and_then(|coverage| coverage.requests(document, context)) {
        Some(requests) => requests?,
        None => document_requests(
            document,
            context.documents,
            context.conditional,
            context.declaration,
            context.destination,
            context.tail,
        )?,
    };
    requests.extend(ensemble_word_requests(
        document,
        context.declaration,
        context.tail,
    )?);
    if document.uri == context.owner.uri {
        requests.push((original.clone(), NameBytes::from(replacement.to_vec())));
    }
    let image = SourceImage::document(document.source);
    let source_edits = crate::original_name_edit::original_name_input_edits(&image, &requests)
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
    Ok(source_edits
        .into_iter()
        .map(|edit| WorkspaceTextEdit {
            uri: document.uri.to_owned(),
            span: edit.span(),
            new_text: edit.text().to_owned(),
        })
        .collect())
}

fn original_command_rename_units_with_conditional_worlds(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    tail: &[u8],
    conditional: &[OriginalConditionalCommandWorld<'_>],
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    original_command_rename_units_with_source_coverage(
        documents,
        identity,
        tail,
        conditional,
        None,
        None,
    )
}

fn selected_publication<'a>(
    owner: OriginalDeclarationDocument<'a>,
    declaration: &Declaration<'_>,
    selected_cut: Option<&tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>>,
) -> Result<tcl_compiler::command_binding::OriginalCommandPublication, OriginalCommandRenameRefusal>
{
    let policy = declaration.name().policy();
    match selected_cut.filter(|cut| cut.is_for_analysis(owner.analysis)) {
        Some(cut)
            if cut.publication().slot() == declaration.name().slot()
                && cut.publication().policy() == policy =>
        {
            Ok(cut.publication().clone())
        }
        Some(_) => Err(OriginalCommandRenameRefusal::ChangedPublication),
        None => Ok(owner
            .analysis
            .original_completed_command_world()
            .ok_or(OriginalCommandRenameRefusal::UnknownWorld)?
            .declaration_at(declaration.name().slot(), policy)
            .ok_or(OriginalCommandRenameRefusal::ChangedPublication)?
            .clone()),
    }
}

fn original_command_rename_units_with_source_coverage<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
    tail: &[u8],
    conditional: &[OriginalConditionalCommandWorld<'a>],
    coverage: Option<&source_coverage::SourceCoverage<'a>>,
    selected_cut: Option<&tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>>,
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    let mut uris = HashSet::new();
    validate_rename_document_owners(documents, conditional, selected_cut, &mut uris)?;
    let owner = documents
        .iter()
        .find(|document| document.uri == identity.uri())
        .ok_or(OriginalCommandRenameRefusal::AmbiguousOwner)?;
    if !identity.is_current(owner.uri, owner.source, owner.analysis) {
        return Err(OriginalCommandRenameRefusal::StaleSource);
    }
    let declaration = rename_declaration(owner.analysis, identity)?;
    let owners = declaration_owner_count(documents, &declaration);
    if owners != 1 {
        return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
    }
    let original = identity
        .input()
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
    let policy = declaration.name().policy();
    if policy.recipe().command_tail_extent(tail) != Some(0..tail.len()) {
        return Err(OriginalCommandRenameRefusal::InvalidTail);
    }
    let destination = renamed_slot(declaration.name().slot(), policy, tail)?;
    let publication = selected_publication(*owner, &declaration, selected_cut)?;
    if Some(publication.declaration_site()) != identity.site() {
        return Err(OriginalCommandRenameRefusal::ChangedPublication);
    }
    if destination != *declaration.name().slot()
        && occupied_with_cut(documents, conditional, selected_cut, &destination, policy)?
    {
        return Err(OriginalCommandRenameRefusal::Collision);
    }
    let replacement = policy
        .recipe()
        .replace_command_tail(original.bytes(), tail)
        .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
    let scope = owner
        .analysis
        .original_namespace_scope_at(
            identity
                .site()
                .ok_or(OriginalCommandRenameRefusal::StaleSource)?
                .offset,
        )
        .ok_or(OriginalCommandRenameRefusal::UnknownDestination)?;
    let actual = match &declaration {
        Declaration::Procedure(_) => policy.recipe().command_publication_slot(
            scope
                .context()
                .ok_or(OriginalCommandRenameRefusal::UnknownDestination)?,
            &replacement,
        ),
        Declaration::Class(_) => policy.recipe().oo_object_publication_slot(
            scope
                .context()
                .ok_or(OriginalCommandRenameRefusal::UnknownDestination)?,
            &replacement,
        ),
    }
    .map_err(|_| OriginalCommandRenameRefusal::InvalidTail)?;
    if actual != destination {
        return Err(OriginalCommandRenameRefusal::InvalidTail);
    }
    let context = source_coverage::RequestContext {
        documents,
        conditional,
        owner: *owner,
        declaration: &declaration,
        destination: &destination,
        tail,
        selected_cut,
    };
    let mut edits = Vec::new();
    for &document in documents {
        edits.extend(rename_document_edits(
            document,
            &context,
            original,
            &replacement,
            coverage,
        )?);
    }
    validate_workspace_edit_order(&mut edits)?;
    Ok(edits)
}

/// Select a current Proc/Class declaration from its genuine local cursor
/// owner or an actual positioned command reference in another document.
/// This readonly selection supplies no rename, lookup or publication grant;
/// the atomic planner must independently close every edit obligation.
pub fn select_in_documents(
    documents: &[OriginalDeclarationDocument<'_>],
    cursor_uri: &str,
    cursor: u32,
) -> std::ops::ControlFlow<Option<OriginalDeclarationIdentity>> {
    use std::ops::ControlFlow;
    let mut uris = HashSet::new();
    for document in documents {
        let Some(config) = document.analysis.body_lexer_config else {
            return ControlFlow::Break(None);
        };
        if !uris.insert(document.uri)
            || !document
                .analysis
                .matches_original_source_image(&SourceImage::document(document.source), config)
        {
            return ControlFlow::Break(None);
        }
    }
    let Some(document) = documents.iter().find(|document| document.uri == cursor_uri) else {
        return ControlFlow::Break(None);
    };
    if document.analysis.allows_lexical_declaration_advice() {
        return ControlFlow::Continue(());
    }
    match crate::original_declaration::select_at_offset(
        cursor_uri,
        document.source,
        document.analysis,
        cursor,
    ) {
        ControlFlow::Break(Some(identity)) => {
            return ControlFlow::Break(
                matches!(
                    identity.role(),
                    OriginalDeclarationRole::Procedure | OriginalDeclarationRole::Class,
                )
                .then_some(identity),
            );
        }
        ControlFlow::Break(None) | ControlFlow::Continue(()) => {}
    }
    let Some(invocation) = document
        .analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() <= cursor && cursor < invocation.range.end())
    else {
        return ControlFlow::Break(None);
    };
    let (Some(input), Some(lookup), Some(reference)) = (
        invocation.original_name_input.as_ref(),
        invocation.original_lookup.as_ref(),
        invocation.resolved_command_reference.as_ref(),
    ) else {
        return ControlFlow::Break(None);
    };
    if input != lookup.name_input()
        || lookup.site().source.source_image() != &SourceImage::document(document.source)
        || !reference.matches_original_invocation_site(lookup.site())
    {
        return ControlFlow::Break(None);
    }
    let mut matching = Vec::new();
    for owner in documents {
        for row in owner.analysis.original_procedure_declarations() {
            if crate::original_declaration::reference_matches_declaration(reference, row, true)
                && let Some(identity) = OriginalDeclarationIdentity::for_procedure(
                    owner.uri,
                    owner.source,
                    owner.analysis,
                    row,
                )
            {
                matching.push(identity);
            }
        }
        for row in owner.analysis.original_class_declarations() {
            if crate::original_declaration::reference_matches_declaration(reference, row, true)
                && let Some(identity) = OriginalDeclarationIdentity::for_class(
                    owner.uri,
                    owner.source,
                    owner.analysis,
                    row,
                )
            {
                matching.push(identity);
            }
        }
    }
    ControlFlow::Break(match matching.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    })
}

/// Validate the same atomic command-edit obligations before offering a cursor
/// range. Only a genuine complete static word can supply this component span;
/// expression identifiers and readonly produced names remain unavailable.
///
/// # Errors
/// Returns the same missing ownership, collision or coverage obligation as the
/// edit planner, or an unavailable original cursor component.
pub fn original_command_prepare_span(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
    cursor: u32,
) -> Result<Span, OriginalCommandRenameRefusal> {
    original_command_prepare_span_in(&[document], identity, document.uri, cursor)
}

/// Prepare a command component over the same complete independent owners as
/// the edit planner. The cursor and declaration owners remain separate; equal
/// source bytes or URI duplicates do not establish cross-document identity.
///
/// # Errors
/// Returns the planner's missing ownership, coverage or editable-input
/// obligation. Actual source references are matched against the declaring
/// owner's metadata, never a same-spelled cursor-owner declaration.
pub fn original_command_prepare_span_in(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    cursor_uri: &str,
    cursor: u32,
) -> Result<Span, OriginalCommandRenameRefusal> {
    original_command_prepare_span_in_with_conditional_worlds(
        documents,
        identity,
        cursor_uri,
        cursor,
        &[],
    )
}

/// Prepare against the same independently completed conditional collision
/// worlds as the edit planner; cursor/declaration/edit guards remain unchanged.
pub fn original_command_prepare_span_in_with_conditional_worlds(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    cursor_uri: &str,
    cursor: u32,
    conditional: &[OriginalConditionalCommandWorld<'_>],
) -> Result<Span, OriginalCommandRenameRefusal> {
    let owner = documents
        .iter()
        .find(|document| document.uri == identity.uri())
        .ok_or(OriginalCommandRenameRefusal::AmbiguousOwner)?;
    let document = *documents
        .iter()
        .find(|document| document.uri == cursor_uri)
        .ok_or(OriginalCommandRenameRefusal::AmbiguousOwner)?;
    let input = if cursor_uri == identity.uri()
        && identity.span().start() <= cursor
        && cursor < identity.span().end()
    {
        identity
            .input()
            .ok_or(OriginalCommandRenameRefusal::UneditableInput)?
            .clone()
    } else {
        let mut inputs = document
            .analysis
            .command_invocations
            .iter()
            .filter(|invocation| {
                invocation.range.start() <= cursor && cursor < invocation.range.end()
            })
            .filter_map(|invocation| {
                let input = invocation.original_name_input.as_ref()?;
                let lookup = invocation.original_lookup.as_ref()?;
                let reference = invocation.resolved_command_reference.as_ref()?;
                if input != lookup.name_input()
                    || !reference.matches_original_invocation_site(lookup.site())
                {
                    return None;
                }
                let matches = match identity.role() {
                    OriginalDeclarationRole::Procedure => identity
                        .procedure_metadata(owner.analysis)
                        .is_some_and(|row| {
                            crate::original_declaration::reference_matches_declaration(
                                reference, row, false,
                            )
                        }),
                    OriginalDeclarationRole::Class => {
                        identity.class_metadata(owner.analysis).is_some_and(|row| {
                            crate::original_declaration::reference_matches_declaration(
                                reference, row, false,
                            )
                        })
                    }
                    _ => false,
                };
                matches.then(|| input.clone())
            });
        let first = inputs
            .next()
            .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
        if inputs.any(|other| other != first) {
            return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
        }
        first
    };
    if input.original_word_key().is_none() {
        return Err(OriginalCommandRenameRefusal::UneditableInput);
    }
    let native = input
        .policy()
        .recipe()
        .command_tail_extent(input.bytes())
        .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
    let tail = &input.bytes()[native.clone()];
    original_command_rename_units_with_conditional_worlds(documents, identity, tail, conditional)?;
    crate::original_name_edit::original_static_name_value_span(&input, native)
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)
}

/// A literal ensemble's call runs the source procedure its subcommand word
/// selects, so that dispatch receipt stands for the call's own reference.
fn ensemble_dispatch_reference<'a>(
    document: OriginalDeclarationDocument<'a>,
    reference: Option<&'a SourceCommandReference>,
    command_offset: u32,
) -> Option<&'a SourceCommandReference> {
    if reference.is_some_and(SourceCommandReference::is_direct_definition) {
        return reference;
    }
    document
        .analysis
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.lookup
                == tcl_compiler::signature_scan::types::SignatureCommandLookup::EnsembleDispatch {
                    invocation_offset: command_offset,
                }
        })
        .map_or(reference, |dispatch| {
            dispatch.resolved_command_reference.as_ref()
        })
}

fn reject_incomplete_source_procedure(
    document: OriginalDeclarationDocument<'_>,
    reference: Option<&SourceCommandReference>,
    words: &[tcl_lexer::NativeWord],
    input: &SignatureSourceNameInput,
    command_offset: u32,
) -> Result<(), OriginalCommandRenameRefusal> {
    // A genuine direct source procedure with static argv has no
    // Registry-defined executable argument regions. Its own body
    // is visited at the separately selected declaration worker.
    let reference = require_coverage(
        reference,
        command_offset,
        "unselected-registry-or-source-reference",
    )?;
    if !reference.is_direct_definition()
        || !document
            .analysis
            .original_procedure_declarations()
            .any(|row| {
                crate::original_declaration::reference_matches_declaration(reference, row, false)
            })
    {
        coverage_trace(command_offset, "direct-procedure-owner");
        return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
    }
    reject_nonliteral_source_procedure_words(words, input)
}

fn reject_nonliteral_source_procedure_words(
    words: &[tcl_lexer::NativeWord],
    input: &SignatureSourceNameInput,
) -> Result<(), OriginalCommandRenameRefusal> {
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        words,
        input.policy().string_protocol(),
    )
    .map_err(|_| OriginalCommandRenameRefusal::IncompleteCoverage)?;
    if words
        .iter()
        .enumerate()
        .any(|(ordinal, word)| word.group().expand || captured.literal(ordinal).is_none())
    {
        return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
    }
    Ok(())
}

fn validate_rename_document_owners<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    conditional: &[OriginalConditionalCommandWorld<'a>],
    selected_cut: Option<&tcl_compiler::command_binding::OriginalClosedSourceCutPublication<'a>>,
    uris: &mut HashSet<&'a str>,
) -> Result<(), OriginalCommandRenameRefusal> {
    for (ordinal, document) in documents.iter().enumerate() {
        if !uris.insert(document.uri) {
            return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
        }
        let image = SourceImage::document(document.source);
        let config = document
            .analysis
            .body_lexer_config
            .ok_or(OriginalCommandRenameRefusal::StaleSource)?;
        if !document
            .analysis
            .matches_original_source_image(&image, config)
        {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
        let world = collision_world(document, conditional, selected_cut).ok_or_else(|| {
            trace_missing_rename_world(document, ordinal, documents.len());
            OriginalCommandRenameRefusal::UnknownWorld
        })?;
        if world.source_image() != &image || world.lexer_config() != config {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
    }
    Ok(())
}

fn declaration_owner_count(
    documents: &[OriginalDeclarationDocument<'_>],
    declaration: &Declaration<'_>,
) -> usize {
    documents
        .iter()
        .flat_map(|document| {
            document
                .analysis
                .original_procedure_declarations()
                .map(|row| (row.declaration_site(), row.name()))
                .chain(
                    document
                        .analysis
                        .original_class_declarations()
                        .map(|row| (row.declaration_site(), row.name())),
                )
        })
        .filter(|(site, name)| *site == declaration.site() && *name == declaration.name())
        .count()
}

fn reject_readonly_math_references(
    document: OriginalDeclarationDocument<'_>,
    owner: &OriginalDeclarationDocument<'_>,
    declaration: &Declaration<'_>,
) -> Result<(), OriginalCommandRenameRefusal> {
    if let Declaration::Procedure(row) = declaration {
        match crate::math_function_symbol::procedure_references_in(
            document.source,
            document.analysis,
            owner.source,
            owner.analysis,
            row,
            true,
        ) {
            Some(rows) if rows.is_empty() => {}
            Some(_) | None => return Err(OriginalCommandRenameRefusal::ReadonlyMathReferences),
        }
    }
    Ok(())
}

/// A literal ensemble creation whose every subcommand names one command
/// holds those names only in the words [`ensemble_word_requests`] edits or
/// refuses for; its own `-command` word is a definition like `proc`'s.
fn literal_ensemble_names_are_covered(
    selected: &tcl_compiler::registry_invocation::ResolvedStatementInvocation,
) -> bool {
    use tcl_registry::{EnsembleDispatchTarget, NamespaceTransition, StateTransition};
    let Some(transitions) = selected.facts.state_transitions.declared() else {
        return false;
    };
    let [fact] = transitions.facts() else {
        return false;
    };
    let StateTransition::Namespace(NamespaceTransition::Ensemble {
        created: Some(_),
        dispatch: Some(dispatch),
        ..
    }) = &fact.transition
    else {
        return false;
    };
    selected.facts.arg_roles_complete
        && dispatch.entries.iter().all(|(_, target)| match target {
            EnsembleDispatchTarget::Qualified(words) => words.len() == 1,
            EnsembleDispatchTarget::Relative(_) => true,
        })
        && !selected.facts.traits.intersects(
            tcl_registry::Traits::CURRENT_FRAME_INTROSPECTION
                | tcl_registry::Traits::INVOKES_USER_PROC,
        )
        && !selected.written_argument_roles().iter().any(|(_, role)| {
            matches!(
                role,
                tcl_registry::ArgRole::CommandPrefix
                    | tcl_registry::ArgRole::CommandName
                    | tcl_registry::ArgRole::CommandNameProbe
            )
        })
}

fn reject_selected_command_link_coverage(
    selected: &tcl_compiler::registry_invocation::ResolvedStatementInvocation,
    command_offset: u32,
) -> Result<(), OriginalCommandRenameRefusal> {
    if literal_ensemble_names_are_covered(selected) {
        return Ok(());
    }
    reject_link_obligations(selected).inspect_err(|refusal| {
        coverage_trace(command_offset, "registry-name-link-or-effect-coverage");
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_RENAME").is_some() {
            eprintln!("ORIGINAL_COMMAND_RENAME_FACTS offset={command_offset} command={} operation={:?} traits={:?} roles_complete={} roles={:?} transitions={:?} effects={:?} refusal={refusal:?}",
                selected.facts.canonical_command, selected.facts.operation,
                selected.facts.traits, selected.facts.arg_roles_complete,
                selected.written_argument_roles(), selected.facts.state_transitions,
                selected.facts.effects);
        }
        // Read only by the test-build trace above.
        let _ = refusal;
    })
}

fn validate_workspace_edit_order(
    edits: &mut [WorkspaceTextEdit],
) -> Result<(), OriginalCommandRenameRefusal> {
    edits.sort_by(|a, b| {
        (&a.uri, a.span.start(), a.span.end()).cmp(&(&b.uri, b.span.start(), b.span.end()))
    });
    for pair in edits.windows(2) {
        if pair[0].uri == pair[1].uri && pair[0].span.end() > pair[1].span.start() {
            return Err(OriginalCommandRenameRefusal::UneditableInput);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn apply(source: &str, edits: &[WorkspaceTextEdit]) -> String {
        let mut result = source.to_owned();
        for edit in edits.iter().rev() {
            result.replace_range(edit.span.as_range(), &edit.new_text);
        }
        result
    }

    #[test]
    fn authored_declaration_queue_uses_actual_frame_walk_and_refuses_foreign_source() {
        let source = "proc outer {} {proc inner {} {return VALUE}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
        let start = source.find("proc inner").unwrap();
        let end = source.rfind('}').unwrap();
        let offset = u32::try_from(start).unwrap();
        let command = walk
            .segment_nested(&source[start..end], offset)
            .into_iter()
            .next()
            .unwrap();
        let document = OriginalDeclarationDocument {
            uri: "file:///authored.tcl",
            source,
            analysis: &analysis,
        };
        let mut pending = Vec::new();
        assert!(walk.structure(source, &command).is_none());
        assert!(queue_declaration(&walk, document, &command, 1, &mut pending).unwrap());
        assert_eq!(pending.len(), 1);
        let (body_start, body_end, depth) = pending[0];
        assert_eq!(depth, 2);
        assert_eq!(&source[body_start..body_end], "return VALUE");
        let body = walk.segment_nested(
            &source[body_start..body_end],
            u32::try_from(body_start).unwrap(),
        );
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].texts[0], "return");
        let changed = source.replace("VALUE", "OTHER");
        assert!(crate::refactor::FrameWalk::new(&changed, &analysis).is_none());
        let foreign = Analyser::new().analyse("proc other {} {return VALUE}", "tcl8.6");
        assert!(crate::refactor::FrameWalk::new(source, &foreign).is_none());
        let mut wrong_input = analysis.clone();
        wrong_input.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
        assert!(
            crate::refactor::FrameWalk::new(source, &wrong_input).is_none_or(|other| other
                .procedure_declaration_for_rename_coverage(source, &command)
                .is_none())
        );
    }

    fn result_completion_command(
        source: &str,
        walk: &crate::refactor::FrameWalk<'_>,
    ) -> tcl_compiler::segmenter::SegmentedCommand {
        let start = source.rfind("return ").unwrap();
        let end = source.rfind('}').unwrap();
        let commands = walk.segment_nested(&source[start..end], u32::try_from(start).unwrap());
        assert_eq!(commands.len(), 1);
        commands.into_iter().next().unwrap()
    }

    #[test]
    fn authored_result_completion_coverage_retains_nested_call_without_normal_grant() {
        let source = "proc host_old {value} { return $value }\nset host_marker \"🧪 café\"; proc host_caller {} {return [host_old 7]}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
        let command = result_completion_command(source, &walk);
        let tokens = walk.tokens(source, &command);
        assert!(walk.structure(source, &command).is_none());
        assert!(
            !tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_completes_normally(&tokens)
        );
        let receipt = walk
            .result_completion_for_rename_coverage(source, &command)
            .expect("same current authored result grammar and original argument closure");
        assert!(receipt.is_for_analysis(&analysis));
        assert_eq!(
            receipt.invocation().effective.origins,
            [
                tcl_compiler::registry_invocation::InvocationWordOrigin::ResolvedHead,
                tcl_compiler::registry_invocation::InvocationWordOrigin::Written(1),
            ]
        );
        assert_eq!(receipt.argument_regions().len(), 1);
        let span = receipt.argument_regions()[0];
        assert_eq!(&source[span.as_range()], "host_old 7");
        let document = OriginalDeclarationDocument {
            uri: "file:///return.tcl",
            source,
            analysis: &analysis,
        };
        let mut pending = Vec::new();
        assert!(
            authored_result_completion_coverage(
                &walk,
                document,
                &command,
                command.argv[0].span.start(),
                1,
                &mut pending,
            )
            .unwrap()
        );
        assert_eq!(
            pending,
            vec![(span.start() as usize, span.end() as usize, 2)]
        );
        let nested = walk.segment_nested(&source[span.as_range()], span.start());
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].texts, ["host_old", "7"]);
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        assert!(!receipt.is_for_analysis(&foreign));
        assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
            source, &foreign, &tokens,
        ).is_none());
        let mut stale = analysis.clone();
        stale.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
        assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
            source, &stale, &tokens,
        ).is_none());
        assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
            "proc host_caller {} {return OTHER}", &analysis, &tokens,
        ).is_none());
    }

    #[test]
    fn authored_result_completion_coverage_refuses_unknown_shadowed_error_and_observer_inputs() {
        for source in [
            "proc host_caller {} {return [unknown_operation]}",
            "proc return args {puts OBS}; proc host_caller {} {return VALUE}",
            "proc host_caller {} {return -code error BAD}",
            "proc observer args {puts OBS}; proc host_caller {} {set value VALUE; trace add variable value read observer; return $value}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
            let command = result_completion_command(source, &walk);
            assert!(
                walk.result_completion_for_rename_coverage(source, &command)
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn authored_result_completion_never_skips_nested_name_write_or_opaque_channel() {
        for nested in ["rename host_old moved", "puts VALUE"] {
            let source = format!(
                "proc host_old {{value}} {{return $value}}\nproc host_caller {{}} {{return [{nested}]}}\n"
            );
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            let walk = crate::refactor::FrameWalk::new(&source, &analysis).unwrap();
            let command = result_completion_command(&source, &walk);
            if let Some(receipt) = walk.result_completion_for_rename_coverage(&source, &command) {
                assert_eq!(receipt.argument_regions().len(), 1);
                assert_eq!(&source[receipt.argument_regions()[0].as_range()], nested);
                // A conditional Return card never changes the generic link guard.
                let expected = reject_link_obligations(receipt.invocation());
                let document = OriginalDeclarationDocument {
                    uri: "file:///nested.tcl",
                    source: &source,
                    analysis: &analysis,
                };
                let mut pending = Vec::new();
                let result = authored_result_completion_coverage(
                    &walk,
                    document,
                    &command,
                    command.argv[0].span.start(),
                    0,
                    &mut pending,
                );
                match expected {
                    Ok(()) => {
                        assert_eq!(result, Ok(true));
                        assert_eq!(pending.len(), 1);
                    }
                    Err(refusal) => {
                        assert_eq!(result, Err(refusal));
                        assert_eq!(pending, [] as [(usize, usize, u32); 0]);
                    }
                }
            }
            let identity = OriginalDeclarationIdentity::for_procedure(
                "file:///nested.tcl",
                &source,
                &analysis,
                analysis.original_procedure_declarations().next().unwrap(),
            )
            .unwrap();
            assert!(
                original_command_rename_edits(
                    &[OriginalDeclarationDocument {
                        uri: "file:///nested.tcl",
                        source: &source,
                        analysis: &analysis
                    }],
                    &identity,
                    "changed",
                )
                .is_err(),
                "nested operation must not disappear from coverage: {nested}"
            );
        }
    }

    #[test]
    fn quiet_store_coverage_requires_actual_points_and_refuses_traced_foreign_source() {
        let source = "proc host_old {value} { return $value }\nset host_marker \"🧪 café\"; proc host_caller {} {return [host_old 7]}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
        let command = walk.segment_document(source).into_iter().nth(1).unwrap();
        let tokens = walk.tokens(source, &command);
        let selected = walk
            .quiet_store_for_rename_coverage(source, &command)
            .expect("actual same-owner quiet store and complete handler");
        assert_eq!(
            selected.invocation().written_argument_roles(),
            vec![(0, tcl_registry::ArgRole::VarWrite)]
        );
        assert!(matches!(
            selected.invocation().facts.state_transitions,
            tcl_registry::StateTransitionKnowledge::UnknownInvocation
        ));
        // A plain store is the generic invocation the completed world closes
        // over, so its registry facts carry the same empty link inventory.
        assert!(reject_link_obligations(selected.invocation()).is_ok());
        let document = OriginalDeclarationDocument {
            uri: "file:///quiet.tcl",
            source,
            analysis: &analysis,
        };
        assert!(quiet_store_coverage(&walk, document, &command, 40).unwrap());
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        assert!(matches!(
            reject_command_link_coverage(CommandLinkCoverage::QuietScalarStore {
                receipt: &selected,
                analysis: &foreign,
            }),
            Err(OriginalCommandRenameRefusal::StaleSource)
        ));
        assert!(
            tcl_compiler::registry_invocation::original_authored_quiet_store_for_source_coverage(
                source, &foreign, &tokens
            )
            .is_none()
        );
        assert!(
            tcl_compiler::registry_invocation::original_authored_quiet_store_for_source_coverage(
                "set host_marker OTHER",
                &analysis,
                &tokens
            )
            .is_none()
        );
        let mut wrong_input = analysis.clone();
        wrong_input.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
        assert!(
            tcl_compiler::registry_invocation::original_authored_quiet_store_for_source_coverage(
                source,
                &wrong_input,
                &tokens
            )
            .is_none()
        );
        for refused_source in [
            "proc set args {return VALUE}; set host_marker VALUE",
            "proc observer args {puts OBS}; trace add variable host_marker write observer; set host_marker VALUE",
            "unknown_op; set host_marker VALUE",
            "puts VALUE",
        ] {
            let refused = Analyser::new().analyse(refused_source, "tcl8.6");
            let other = crate::refactor::FrameWalk::new(refused_source, &refused).unwrap();
            let last = other.segment_document(refused_source).pop().unwrap();
            assert!(
                other
                    .quiet_store_for_rename_coverage(refused_source, &last)
                    .is_none(),
                "{refused_source}"
            );
        }
    }

    #[test]
    fn conditional_collision_world_requires_exact_prefix_and_unique_publication() {
        let source = "source child.tcl; proc unrelated {} {return VALUE}";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &parent, 0,
            )
            .unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry("proc helper {} {return VALUE}", "tcl8.6", &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&parent, &child)
            .unwrap()
            .source_regions(&parent, &child)
            .unwrap();
        let outcome = regions.normal_outcome().unwrap();
        let probe_source = "proc host_old {} {return VALUE}";
        let probe = Analyser::new().analyse(probe_source, "tcl8.6");
        let world = probe.original_completed_command_world().unwrap();
        let publication = world.declarations().next().unwrap();
        let slot = publication.slot();
        let policy = publication.policy();
        let document = OriginalDeclarationDocument {
            uri: "file:///main.tcl",
            source,
            analysis: &parent,
        };
        assert_eq!(
            occupied_with_cut(&[document], &[], None, slot, policy),
            Err(OriginalCommandRenameRefusal::UnknownWorld)
        );
        let entry = OriginalConditionalCommandWorld {
            uri: document.uri,
            outcome: &outcome,
        };
        assert_eq!(
            occupied_with_cut(&[document], &[entry], None, slot, policy),
            Ok(false)
        );
        let wrong = OriginalConditionalCommandWorld {
            uri: "file:///other.tcl",
            outcome: &outcome,
        };
        assert_eq!(
            occupied_with_cut(&[document], &[wrong], None, slot, policy),
            Err(OriginalCommandRenameRefusal::UnknownWorld)
        );
        let duplicate = [
            OriginalConditionalCommandWorld {
                uri: document.uri,
                outcome: &outcome,
            },
            OriginalConditionalCommandWorld {
                uri: document.uri,
                outcome: &outcome,
            },
        ];
        assert_eq!(
            occupied_with_cut(&[document], &duplicate, None, slot, policy),
            Err(OriginalCommandRenameRefusal::UnknownWorld)
        );
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        let foreign_document = OriginalDeclarationDocument {
            analysis: &foreign,
            ..document
        };
        let entry = OriginalConditionalCommandWorld {
            uri: document.uri,
            outcome: &outcome,
        };
        assert_eq!(
            occupied_with_cut(&[foreign_document], &[entry], None, slot, policy),
            Err(OriginalCommandRenameRefusal::UnknownWorld)
        );
    }

    #[test]
    fn original_command_rename_keeps_opaque_words_and_ignores_reporting_maps() {
        // Implementation contract: naming.source.original-jim-text-evaluation
        // docs/design/analysis/name-resolution-proofs/original-jim-text-evaluation.md
        // Implementation contract: naming.editor.original-command-rename-plans
        // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            for name in ["helper", r"p\uD800"] {
                let source =
                    format!("proc {name} {{argument}} {{return $argument}}\n{name} value\n");
                let mut analysis = Analyser::new().analyse(&source, dialect);
                analysis.all_procs.clear();
                analysis.global_scope.procs.clear();
                analysis.all_classes.clear();
                let row = analysis.original_procedure_declarations().next().unwrap();
                let identity = OriginalDeclarationIdentity::for_procedure(
                    "file:///owner.tcl",
                    &source,
                    &analysis,
                    row,
                )
                .unwrap();
                let documents = [OriginalDeclarationDocument {
                    uri: "file:///owner.tcl",
                    source: &source,
                    analysis: &analysis,
                }];
                let edits = original_command_rename_edits(&documents, &identity, "changed")
                    .unwrap_or_else(|refusal| panic!("{dialect}: {name}: {refusal:?}"));
                assert_eq!(edits.len(), 2, "{dialect}: {name}");
                assert_eq!(
                    apply(&source, &edits),
                    "proc changed {argument} {return $argument}\nchanged value\n"
                );
                assert!(analysis.all_procs.is_empty());
            }
        }
    }

    #[test]
    fn original_command_rename_declines_stale_duplicate_collision_and_readonly_links() {
        // Implementation contract: naming.editor.original-command-rename-plans
        // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
        let source = "proc helper {argument} {return $argument}\nhelper value\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let identity = OriginalDeclarationIdentity::for_procedure(
            "file:///owner.tcl",
            source,
            &analysis,
            analysis.original_procedure_declarations().next().unwrap(),
        )
        .unwrap();
        let owner = OriginalDeclarationDocument {
            uri: "file:///owner.tcl",
            source,
            analysis: &analysis,
        };
        assert_eq!(
            original_command_rename_edits(&[owner, owner], &identity, "changed"),
            Err(OriginalCommandRenameRefusal::AmbiguousOwner)
        );
        let equal_source_owner = OriginalDeclarationDocument {
            uri: "file:///independent.tcl",
            source,
            analysis: &analysis,
        };
        assert_eq!(
            original_command_rename_edits(&[owner, equal_source_owner], &identity, "changed"),
            Err(OriginalCommandRenameRefusal::AmbiguousOwner)
        );
        let displaced = format!("# displaced\n{source}");
        assert_eq!(
            original_command_rename_edits(
                &[OriginalDeclarationDocument {
                    source: &displaced,
                    ..owner
                }],
                &identity,
                "changed"
            ),
            Err(OriginalCommandRenameRefusal::StaleSource)
        );
        assert_eq!(
            original_command_rename_edits(&[owner], &identity, "return"),
            Err(OriginalCommandRenameRefusal::Collision)
        );
        assert_eq!(
            original_command_rename_edits(&[owner], &identity, "N::changed"),
            Err(OriginalCommandRenameRefusal::InvalidTail)
        );
        for source in [
            "proc helper {argument} {return $argument}\ninterp alias {} link {} helper\nlink value\n",
            "proc helper {argument} {return $argument}\nset selected helper\n$selected value\n",
            "proc helper {argument} {return $argument}\nafter idle {helper value}\n",
            "proc helper {argument} {return $argument}\ninfo body helper\n",
            "proc ::tcl::mathfunc::helper {argument} {return $argument}\nexpr {helper(1)}\n",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let identity = OriginalDeclarationIdentity::for_procedure(
                "file:///owner.tcl",
                source,
                &analysis,
                analysis.original_procedure_declarations().next().unwrap(),
            )
            .unwrap();
            let result = original_command_rename_edits(
                &[OriginalDeclarationDocument {
                    uri: "file:///owner.tcl",
                    source,
                    analysis: &analysis,
                }],
                &identity,
                "changed",
            );
            assert!(
                result.is_err(),
                "unclosed name links must not emit a partial edit: {source}"
            );
        }
    }
}
