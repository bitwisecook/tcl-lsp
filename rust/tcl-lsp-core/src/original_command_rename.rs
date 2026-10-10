// SPDX-License-Identifier: AGPL-3.0-or-later
//! Atomic command-name edits from current original owners and byte geometry.

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

fn occupied(
    documents: &[OriginalDeclarationDocument<'_>],
    slot: &ByteCommandSlot,
    policy: NamePolicyProtocol,
) -> Result<bool, OriginalCommandRenameRefusal> {
    let mut present = false;
    for document in documents {
        let world = document
            .analysis
            .original_completed_command_world()
            .ok_or(OriginalCommandRenameRefusal::UnknownWorld)?;
        present |= world
            .command_slot_occupied(slot, policy)
            .ok_or(OriginalCommandRenameRefusal::UnknownDestination)?;
    }
    Ok(present)
}

fn destination_matches(
    documents: &[OriginalDeclarationDocument<'_>],
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
            if occupied(documents, &candidate, lookup.policy())? {
                return Err(OriginalCommandRenameRefusal::Collision);
            }
        }
        if !reached {
            return Err(OriginalCommandRenameRefusal::UnknownDestination);
        }
    }
    Ok(())
}

pub(crate) fn reject_link_obligations(
    selected: &tcl_compiler::registry_invocation::ResolvedStatementInvocation,
) -> Result<(), OriginalCommandRenameRefusal> {
    use tcl_registry::{CommandBindingTransition, NamespaceTransition, StateTransition};
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
    let transitions = selected
        .facts
        .state_transitions
        .declared()
        .ok_or(OriginalCommandRenameRefusal::IncompleteCoverage)?;
    for fact in transitions.facts() {
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

fn coverage_trace(offset: u32, stage: &'static str) {
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

fn document_requests(
    document: OriginalDeclarationDocument<'_>,
    documents: &[OriginalDeclarationDocument<'_>],
    declaration: &Declaration<'_>,
    destination: &ByteCommandSlot,
    tail: &[u8],
) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalCommandRenameRefusal> {
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
        let region = document
            .source
            .get(start..end)
            .ok_or(OriginalCommandRenameRefusal::StaleSource)?;
        let offset = u32::try_from(start).map_err(|_| OriginalCommandRenameRefusal::StaleSource)?;
        for command in walk.segment(region, offset) {
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
                    destination_matches(documents, &lookup, destination, tail)?;
                    let wanted = input
                        .policy()
                        .recipe()
                        .replace_command_tail(input.bytes(), tail)
                        .ok_or(OriginalCommandRenameRefusal::InvalidTail)?;
                    requests.push((input.clone(), NameBytes::from(wanted)));
                }
            }
            if let Some(selected) = walk.structure(document.source, &command) {
                reject_link_obligations(&selected).map_err(|refusal| {
                    coverage_trace(command_offset, "registry-name-link-or-effect-coverage");
                    #[cfg(test)]
                    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_RENAME").is_some() {
                        eprintln!("ORIGINAL_COMMAND_RENAME_FACTS offset={command_offset} command={} operation={:?} traits={:?} roles_complete={} roles={:?} transitions={:?} effects={:?} refusal={refusal:?}",
                            selected.facts.canonical_command, selected.facts.operation,
                            selected.facts.traits, selected.facts.arg_roles_complete,
                            selected.written_argument_roles(), selected.facts.state_transitions,
                            selected.facts.effects);
                    }
                    refusal
                })?;
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
                // A genuine direct source procedure with static argv has no
                // Registry-defined executable argument regions. Its own body
                // is visited at the separately selected declaration worker.
                let reference = require_coverage(
                    reference.as_ref(),
                    command_offset,
                    "unselected-registry-or-source-reference",
                )?;
                if !reference.is_direct_definition()
                    || !document
                        .analysis
                        .original_procedure_declarations()
                        .any(|row| {
                            crate::original_declaration::reference_matches_declaration(
                                reference, row, false,
                            )
                        })
                {
                    coverage_trace(command_offset, "direct-procedure-owner");
                    return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
                }
                let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    &words,
                    input.policy().string_protocol(),
                )
                .map_err(|_| OriginalCommandRenameRefusal::IncompleteCoverage)?;
                if words.iter().enumerate().any(|(ordinal, word)| {
                    word.group().expand || captured.literal(ordinal).is_none()
                }) {
                    return Err(OriginalCommandRenameRefusal::IncompleteCoverage);
                }
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
    original_command_rename_units(documents, identity, &tail)
}

fn original_command_rename_units(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    tail: &[u8],
) -> Result<Vec<WorkspaceTextEdit>, OriginalCommandRenameRefusal> {
    let mut uris = HashSet::new();
    for document in documents {
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
        let world = document
            .analysis
            .original_completed_command_world()
            .ok_or(OriginalCommandRenameRefusal::UnknownWorld)?;
        if world.source_image() != &image || world.lexer_config() != config {
            return Err(OriginalCommandRenameRefusal::StaleSource);
        }
    }
    let owner = documents
        .iter()
        .find(|document| document.uri == identity.uri())
        .ok_or(OriginalCommandRenameRefusal::AmbiguousOwner)?;
    if !identity.is_current(owner.uri, owner.source, owner.analysis) {
        return Err(OriginalCommandRenameRefusal::StaleSource);
    }
    let declaration = match identity.role() {
        OriginalDeclarationRole::Procedure => Declaration::Procedure(
            identity
                .procedure_metadata(owner.analysis)
                .ok_or(OriginalCommandRenameRefusal::StaleSource)?,
        ),
        OriginalDeclarationRole::Class => Declaration::Class(
            identity
                .class_metadata(owner.analysis)
                .ok_or(OriginalCommandRenameRefusal::StaleSource)?,
        ),
        _ => return Err(OriginalCommandRenameRefusal::UnsupportedRole),
    };
    let owners = documents
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
        .count();
    if owners != 1 {
        return Err(OriginalCommandRenameRefusal::AmbiguousOwner);
    }
    let original = identity
        .input()
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
    let policy = declaration.name().policy();
    if policy.recipe().command_tail_extent(&tail) != Some(0..tail.len()) {
        return Err(OriginalCommandRenameRefusal::InvalidTail);
    }
    let destination = renamed_slot(declaration.name().slot(), policy, &tail)?;
    let publication = owner
        .analysis
        .original_completed_command_world()
        .ok_or(OriginalCommandRenameRefusal::UnknownWorld)?
        .declaration_at(declaration.name().slot(), policy)
        .ok_or(OriginalCommandRenameRefusal::ChangedPublication)?;
    if Some(publication.declaration_site()) != identity.site() {
        return Err(OriginalCommandRenameRefusal::ChangedPublication);
    }
    if destination != *declaration.name().slot() && occupied(documents, &destination, policy)? {
        return Err(OriginalCommandRenameRefusal::Collision);
    }
    let replacement = policy
        .recipe()
        .replace_command_tail(original.bytes(), &tail)
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
    let mut edits = Vec::new();
    for &document in documents {
        if let Declaration::Procedure(row) = &declaration {
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
        let mut requests =
            document_requests(document, documents, &declaration, &destination, &tail)?;
        if document.uri == owner.uri {
            requests.push((original.clone(), NameBytes::from(replacement.clone())));
        }
        let image = SourceImage::document(document.source);
        let source_edits = crate::original_name_edit::original_name_input_edits(&image, &requests)
            .ok_or(OriginalCommandRenameRefusal::UneditableInput)?;
        for edit in source_edits {
            edits.push(WorkspaceTextEdit {
                uri: document.uri.to_owned(),
                span: edit.span(),
                new_text: edit.text().to_owned(),
            });
        }
    }
    edits.sort_by(|a, b| {
        (&a.uri, a.span.start(), a.span.end()).cmp(&(&b.uri, b.span.start(), b.span.end()))
    });
    for pair in edits.windows(2) {
        if pair[0].uri == pair[1].uri && pair[0].span.end() > pair[1].span.start() {
            return Err(OriginalCommandRenameRefusal::UneditableInput);
        }
    }
    Ok(edits)
}

/// Select a current Proc/Class declaration from its genuine local cursor
/// owner or an actual positioned command reference in another document.
/// This readonly selection supplies no rename, lookup or publication grant;
/// the atomic planner must independently close every edit obligation.
#[must_use]
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
    original_command_rename_units(documents, identity, tail)?;
    crate::original_name_edit::original_static_name_value_span(&input, native)
        .ok_or(OriginalCommandRenameRefusal::UneditableInput)
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
