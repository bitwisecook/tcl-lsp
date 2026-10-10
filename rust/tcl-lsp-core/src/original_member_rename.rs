// SPDX-License-Identifier: AGPL-3.0-or-later
//! Atomic instance-member edits from a current source roster and actual calls.
//!
//! The roster owns collisions; selected entries own call identity. Source
//! declarations, static edit containers and complete executable-region coverage
//! remain separate obligations. This supplies no native inventory or execution.

use std::collections::HashSet;

use tcl_compiler::analyser::types::{MemberSide, OriginalSourceMethodMetadata};
use tcl_compiler::command_binding::OriginalClassInstanceMethodRoster;
use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::NameBytes;
use tcl_lexer::{SourceImage, Span};

use crate::original_declaration::{
    OriginalDeclarationDocument, OriginalDeclarationIdentity, OriginalDeclarationRole,
};
use crate::rename::WorkspaceTextEdit;

/// A separately missing source-family, table, lookup or editable obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalMemberRenameRefusal {
    /// The actual complete source/configuration no longer matches.
    StaleSource,
    /// The member has no unique independent source owner.
    AmbiguousOwner,
    /// The selected table has no bounded current roster.
    UnknownRoster,
    /// The workspace's override/family obligations exceed this selected scope.
    IncompleteFamily,
    /// The selected declaration lacks static editable naming provenance.
    UneditableDeclaration,
    /// The proposed name has no selected counted method-name purpose.
    InvalidName,
    /// Another source or builtin method occupies the proposed name.
    Collision,
    /// The declaration's implicit export rule would change.
    ChangesVisibility,
    /// A call lacks a current exact receiver/selector or executable coverage.
    IncompleteCoverage,
    /// Reflected names, links, callbacks or generated routes lack atomic edits.
    UneditableLinks,
    /// A selected name has no complete original source replacement container.
    UneditableInput,
}

impl OriginalMemberRenameRefusal {
    /// Stable host code, independent of reporting labels.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StaleSource => "stale-original-member-source",
            Self::AmbiguousOwner => "ambiguous-original-member-owner",
            Self::UnknownRoster => "unknown-current-source-member-roster",
            Self::IncompleteFamily => "unclosed-original-member-family",
            Self::UneditableDeclaration => "readonly-original-member-declaration",
            Self::InvalidName => "invalid-original-member-name",
            Self::Collision => "original-member-collision",
            Self::ChangesVisibility => "changed-original-member-visibility",
            Self::IncompleteCoverage => "incomplete-original-member-coverage",
            Self::UneditableLinks => "unclosed-original-member-name-links",
            Self::UneditableInput => "uneditable-original-member-input",
        }
    }

    /// The independently missing obligation, not a substitute lookup rule.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::StaleSource => "the complete original source and lexer configuration changed",
            Self::AmbiguousOwner => "the member lacks one unique current declaring document",
            Self::UnknownRoster => {
                "the current class has no complete source member roster including builtin names"
            }
            Self::IncompleteFamily => {
                "the typed override family, configurations or receiver side are not closed for this edit"
            }
            Self::UneditableDeclaration => "the declaration lacks a genuine static naming word",
            Self::InvalidName => "the proposed value has no exact counted method naming purpose",
            Self::Collision => {
                "another retained source or builtin method occupies the proposed name"
            }
            Self::ChangesVisibility => {
                "the proposed declaration name would change its selected default export rule"
            }
            Self::IncompleteCoverage => {
                "a receiver, selector or executable source region lacks complete original coverage"
            }
            Self::UneditableLinks => {
                "reflected names, links or callbacks lack separately authorised source edits"
            }
            Self::UneditableInput => "a selected naming input lacks an original editable container",
        }
    }
}

fn same_method(left: &OriginalSourceMethodMetadata, right: &OriginalSourceMethodMetadata) -> bool {
    left.side() == right.side()
        && left.declaration() == right.declaration()
        && left.metadata() == right.metadata()
}

fn registry_coverage(
    selected: &tcl_compiler::registry_invocation::ResolvedStatementInvocation,
) -> Result<(), OriginalMemberRenameRefusal> {
    crate::original_command_rename::reject_link_obligations(selected).map_err(|refusal| {
        if refusal == crate::original_command_rename::OriginalCommandRenameRefusal::UneditableLinks
        {
            OriginalMemberRenameRefusal::UneditableLinks
        } else {
            OriginalMemberRenameRefusal::IncompleteCoverage
        }
    })
}

fn original_method_body_region(
    declaration: &OriginalSourceMethodMetadata,
    words: &[tcl_lexer::NativeWord],
) -> Result<Span, OriginalMemberRenameRefusal> {
    let body = declaration
        .body_word()
        .filter(|body| !body.group().expand && words.contains(body))
        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let content = body
        .content_span()
        .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(body),
        declaration.original_name_input().policy().string_protocol(),
    )
    .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let value = captured
        .literal(0)
        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let raw = body
        .image()
        .bytes()
        .get(content.as_range())
        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let literal = tcl_syntax::backslash::native_source_literal_bytes(
        raw,
        body.image().channel(),
        declaration.original_name_input().policy().string_protocol(),
    )
    .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
    if literal.as_ref() != value {
        return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
    }
    Ok(content)
}

fn requests(
    document: OriginalDeclarationDocument<'_>,
    class: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    method: &OriginalSourceMethodMetadata,
    roster: &OriginalClassInstanceMethodRoster,
    wanted: &[u8],
) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalMemberRenameRefusal> {
    let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)
        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let mut requests = vec![(
        method.declaration().original_name_input().clone(),
        wanted.into(),
    )];
    let mut pending = vec![(0_usize, document.source.len(), 0_u32)];
    let mut visited = HashSet::new();
    while let Some((start, end, depth)) = pending.pop() {
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth)
            || start > end
            || end > document.source.len()
        {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        if !visited.insert((start, end)) {
            continue;
        }
        let region = document
            .source
            .get(start..end)
            .ok_or(OriginalMemberRenameRefusal::StaleSource)?;
        let offset = u32::try_from(start).map_err(|_| OriginalMemberRenameRefusal::StaleSource)?;
        for command in walk.segment(region, offset) {
            let words = walk
                .native_words(document.source, &command)
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
            let command_offset = words
                .first()
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?
                .span()
                .start();
            let tokens = walk.tokens(document.source, &command);
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_MEMBER_RENAME").is_some() {
                eprintln!(
                    "ORIGINAL_MEMBER_RENAME site={command_offset} command={} region={start}..{end} original_members={:?}",
                    command.name(),
                    class
                        .metadata()
                        .original_members
                        .declarations()
                        .map(|row| (row.declaration().site().offset, row.metadata().body_span))
                        .collect::<Vec<_>>()
                );
            }
            // The independently retained definition worker supplies source
            // body geometry. It does not supply an ordinary runtime call.
            if let Some(declaration) = class
                .metadata()
                .original_members
                .declarations()
                .find(|row| row.declaration().site().offset == command_offset)
            {
                if declaration.declaration().static_occurrence().is_none()
                    || !words.contains(declaration.declaration().original_word())
                {
                    return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
                }
                // The selected whole body word and shared delimiter/value
                // owners provide source geometry; report spans supply no script.
                let body = original_method_body_region(declaration, &words)?;
                pending.push((body.start() as usize, body.end() as usize, depth + 1));
                continue;
            }
            let binding = tokens
                .source_binding
                .as_ref()
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
            if let Some(selected) = crate::receiver_identity::method_at_command(
                document.analysis,
                document.source,
                &command,
            ) {
                if selected.own_entry.is_some()
                    || selected.receiver
                        != tcl_compiler::command_binding::SourceMethodReceiver::Instance
                    || selected.class != class.metadata()
                    || selected.receiver_class != class.metadata()
                {
                    return Err(OriginalMemberRenameRefusal::IncompleteFamily);
                }
                let input = selected.original_selector_input();
                if input.original_word_key().is_none()
                    || !crate::original_name_edit::original_input_matches_source(
                        document.source,
                        document.analysis,
                        input,
                        selected.selector,
                    )
                {
                    return Err(OriginalMemberRenameRefusal::UneditableInput);
                }
                let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    &words,
                    input.policy().string_protocol(),
                )
                .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
                if words.iter().enumerate().skip(1).any(|(ordinal, word)| {
                    word.group().expand || captured.literal(ordinal).is_none()
                }) {
                    return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
                }
                if same_method(selected.metadata, method) {
                    requests.push((input.clone(), wanted.into()));
                }
                continue;
            }
            if let Some(shape) = binding.original_default_manufacture_shape(
                &tokens,
                document
                    .analysis
                    .resolved_registry()
                    .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?,
            ) {
                if shape.class_definition() != roster.class_definition() {
                    return Err(OriginalMemberRenameRefusal::IncompleteFamily);
                }
                continue;
            }
            let selected = walk
                .structure(document.source, &command)
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
            registry_coverage(&selected).map_err(|refusal| {
                #[cfg(test)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_MEMBER_RENAME").is_some() {
                    eprintln!("ORIGINAL_MEMBER_RENAME_REFUSAL site={command_offset} facts={:?} refusal={refusal:?}", selected.facts);
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
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_MEMBER_RENAME").is_some() {
                eprintln!(
                    "ORIGINAL_MEMBER_RENAME_REGIONS site={command_offset} complete={} pending={pending:?}",
                    walk.complete()
                );
            }
            if !walk.complete() {
                return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
            }
        }
    }
    Ok(requests)
}

/// Plan a bounded instance-table rename in one complete current source owner.
/// The current ordinary class roster includes builtin collisions. Inherited
/// families, class-object tables, own-object configurations, generated members,
/// name mutations and unknown receivers remain separate unsupported obligations.
/// Every direct call is joined by its actual selected canonical declaration.
///
/// # Errors
/// Returns the missing typed obligation before issuing any partial edit.
pub fn original_member_rename_edits(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    new_name: &str,
) -> Result<Vec<WorkspaceTextEdit>, OriginalMemberRenameRefusal> {
    let input = identity
        .input()
        .ok_or(OriginalMemberRenameRefusal::UneditableDeclaration)?;
    let wanted = tcl_syntax::backslash::native_source_literal_bytes(
        new_name.as_bytes(),
        SourceImage::document(new_name).channel(),
        input.policy().string_protocol(),
    )
    .map_err(|_| OriginalMemberRenameRefusal::InvalidName)?;
    original_member_rename_units(documents, identity, &wanted)
}

fn original_member_rename_units(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    wanted: &[u8],
) -> Result<Vec<WorkspaceTextEdit>, OriginalMemberRenameRefusal> {
    let [document] = documents else {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    };
    if document.uri != identity.uri() {
        return Err(OriginalMemberRenameRefusal::AmbiguousOwner);
    }
    if !identity.is_current(document.uri, document.source, document.analysis) {
        return Err(OriginalMemberRenameRefusal::StaleSource);
    }
    if identity.role() != OriginalDeclarationRole::Method(MemberSide::Instance) {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    let mut classes = document.analysis.original_class_declarations();
    let class = classes
        .next()
        .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
    if classes.next().is_some()
        || identity.class_metadata(document.analysis) != Some(class)
        || document
            .analysis
            .original_procedure_declarations()
            .next()
            .is_some()
        || document
            .analysis
            .original_class_configurations()
            .next()
            .is_some()
        || document
            .analysis
            .original_object_configurations()
            .next()
            .is_some()
        || class
            .metadata()
            .original_relations
            .effects()
            .next()
            .is_some()
        || class.metadata().original_members.effects().next().is_some()
        || class
            .metadata()
            .original_properties
            .declarations()
            .next()
            .is_some()
        || class
            .metadata()
            .original_special_members
            .declarations()
            .next()
            .is_some()
        || class.metadata().original_members.declarations().any(|row| {
            row.side() != MemberSide::Instance
                || row.native_class_delegate()
                || row.forward_prefix().is_some()
                || row.declaration().static_occurrence().is_none()
        })
    {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    let method = identity
        .method_metadata(document.analysis)
        .ok_or(OriginalMemberRenameRefusal::StaleSource)?;
    if method.declaration().static_occurrence().is_none()
        || method.native_class_delegate()
        || method.forward_prefix().is_some()
        || method.original_name_input() != method.declaration().original_name_input()
    {
        return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
    }
    if class
        .metadata()
        .original_members
        .declarations()
        .filter(|row| {
            row.side() == method.side()
                && row.original_name_input().policy() == method.original_name_input().policy()
                && row.original_name_input().bytes() == method.original_name_input().bytes()
        })
        .count()
        != 1
    {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    let mut methods = class
        .metadata()
        .original_members
        .declarations()
        .filter(|row| same_method(row, method));
    methods
        .next()
        .ok_or(OriginalMemberRenameRefusal::UneditableDeclaration)?;
    if methods.next().is_some() {
        return Err(OriginalMemberRenameRefusal::AmbiguousOwner);
    }
    let world = document
        .analysis
        .original_completed_command_world()
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    if world.source_image() != &SourceImage::document(document.source)
        || world.lexer_config() != identity.config()
    {
        return Err(OriginalMemberRenameRefusal::StaleSource);
    }
    let publication = world
        .declaration_at(class.name().slot(), class.name().policy())
        .filter(|row| row.declaration_site() == class.declaration_site())
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    let definition = publication
        .definition()
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    let roster = world
        .class_instance_method_roster(definition)
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    let input = method.declaration().original_name_input();
    if roster.policy() != input.policy()
        || input
            .policy()
            .recipe()
            .oo_method_input(wanted)
            .ok()
            .is_none_or(|name| name.selected() != wanted)
    {
        return Err(OriginalMemberRenameRefusal::InvalidName);
    }
    let current_entry = roster
        .source_method_for_input(input)
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    if current_entry.declaration() != method.declaration().site()
        || current_entry.name_source().span != method.metadata().name_span
        || current_entry.is_exported() != method.exported()
    {
        return Err(OriginalMemberRenameRefusal::UnknownRoster);
    }
    if roster.default_exported(input.bytes()) != method.exported()
        || roster.default_exported(wanted) != method.exported()
    {
        return Err(OriginalMemberRenameRefusal::ChangesVisibility);
    }
    if !roster.contains(input.bytes()) {
        return Err(OriginalMemberRenameRefusal::UnknownRoster);
    }
    if wanted != input.bytes() && roster.contains(wanted) {
        return Err(OriginalMemberRenameRefusal::Collision);
    }
    let requests = requests(*document, class, method, &roster, wanted)?;
    let edits = crate::original_name_edit::original_name_input_edits(
        &SourceImage::document(document.source),
        &requests,
    )
    .ok_or(OriginalMemberRenameRefusal::UneditableInput)?;
    Ok(edits
        .into_iter()
        .map(|edit| WorkspaceTextEdit {
            uri: document.uri.to_owned(),
            span: edit.span(),
            new_text: edit.text().to_owned(),
        })
        .collect())
}

/// Workspace adapter for the same bounded current source-family protocol.
/// A foreign or multi-owner family remains an explicit missing obligation;
/// this never selects members by reporting names across documents.
///
/// # Errors
/// Returns the edit planner's independently missing obligation.
pub fn original_member_prepare_span_in(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    cursor_uri: &str,
    cursor: u32,
) -> Result<Span, OriginalMemberRenameRefusal> {
    let [document] = documents else {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    };
    if document.uri != cursor_uri {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    original_member_prepare_span(*document, identity, cursor)
}

/// Validate the same atomic family and coverage obligations before offering an
/// original static selector/name extent. Readonly values cannot supply edits.
///
/// # Errors
/// Returns the edit planner's independently missing obligation.
pub fn original_member_prepare_span(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
    cursor: u32,
) -> Result<Span, OriginalMemberRenameRefusal> {
    let method = identity
        .method_metadata(document.analysis)
        .ok_or(OriginalMemberRenameRefusal::StaleSource)?;
    original_member_rename_units(&[document], identity, method.original_name_input().bytes())?;
    let input = if identity.span().start() <= cursor && cursor < identity.span().end() {
        identity
            .input()
            .ok_or(OriginalMemberRenameRefusal::UneditableInput)?
            .clone()
    } else {
        let selected =
            crate::receiver_identity::method_at_cursor(document.analysis, document.source, cursor)
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        if !same_method(selected.metadata, method) {
            return Err(OriginalMemberRenameRefusal::IncompleteFamily);
        }
        selected.original_selector_input().clone()
    };
    crate::original_name_edit::original_static_name_value_span(&input, 0..input.bytes().len())
        .ok_or(OriginalMemberRenameRefusal::UneditableInput)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn identity(
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> OriginalDeclarationIdentity {
        let class = analysis.original_class_declarations().next().unwrap();
        let method = class
            .metadata()
            .original_members
            .declarations()
            .next()
            .unwrap();
        OriginalDeclarationIdentity::for_method(
            "file:///owner.tcl",
            source,
            analysis,
            class,
            method,
        )
        .unwrap()
    }

    fn document<'a>(
        source: &'a str,
        analysis: &'a tcl_compiler::analyser::AnalysisResult,
    ) -> OriginalDeclarationDocument<'a> {
        OriginalDeclarationDocument {
            uri: "file:///owner.tcl",
            source,
            analysis,
        }
    }

    // Implementation contract: naming.editor.original-instance-member-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
    #[test]
    fn original_member_rename_keeps_actual_selectors_and_opaque_declarations_after_ui_clear() {
        // Implementation contract: naming.editor.original-instance-member-rename-plans
        // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for name in ["pick", r"m\uD800"] {
                let source = format!(
                    "oo::class create C {{method {name} {{}} {{return body}}}}\nC create object\nobject {name}\n"
                );
                let mut analysis = Analyser::new().analyse(&source, dialect);
                analysis.all_classes.clear();
                analysis.global_scope.classes.clear();
                analysis.instance_classes.clear();
                analysis.created_instance_commands.clear();
                let identity = identity(&source, &analysis);
                let doc = document(&source, &analysis);
                let edits = original_member_rename_edits(&[doc], &identity, "changed")
                    .unwrap_or_else(|refusal| panic!("{dialect}: {name}: {refusal:?}"));
                assert_eq!(edits.len(), 2, "{dialect}: {name}");
                let mut changed = source.clone();
                for edit in edits.iter().rev() {
                    changed.replace_range(edit.span.as_range(), &edit.new_text);
                }
                assert_eq!(
                    changed,
                    "oo::class create C {method changed {} {return body}}\nC create object\nobject changed\n"
                );
                let selector = u32::try_from(source.rfind(name).unwrap()).unwrap();
                let span = original_member_prepare_span(doc, &identity, selector).unwrap();
                assert_eq!(source.get(span.as_range()), Some(name));
                assert!(analysis.all_classes.is_empty());
            }
        }
    }

    #[test]
    fn original_member_body_coverage_uses_whole_words_and_formed_content() {
        // Implementation contract: naming.editor.original-instance-member-rename-plans
        // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (body, expected) in [
                ("{return body}", Some("return body")),
                ("\"return body\"", Some("return body")),
                ("\"return\\ body\"", None),
                ("{return {λ 😀}}", Some("return {λ 😀}")),
                ("\"return {λ 😀}\"", Some("return {λ 😀}")),
                ("{return {\r\nλ 😀\r\n}}", Some("return {\r\nλ 😀\r\n}")),
            ] {
                let source = format!("oo::class create C {{method pick {{}} {body}}}");
                let analysis = Analyser::new().analyse(&source, dialect);
                let class = analysis.original_class_declarations().next().unwrap();
                let method = class
                    .metadata()
                    .original_members
                    .declarations()
                    .next()
                    .unwrap();
                let word = method.body_word().unwrap();
                let result = original_method_body_region(method, std::slice::from_ref(word));
                if let Some(expected) = expected {
                    let content = result.unwrap();
                    assert_eq!(
                        source.get(content.as_range()),
                        Some(expected),
                        "{dialect}/{body}"
                    );
                    assert!(
                        content.start() > word.span().start(),
                        "wrappers stay outside the script"
                    );
                    assert_eq!(
                        original_method_body_region(method, &[]),
                        Err(OriginalMemberRenameRefusal::IncompleteCoverage),
                        "a declaration body cannot substitute for the current worker's word"
                    );
                } else {
                    assert_eq!(
                        result,
                        Err(OriginalMemberRenameRefusal::IncompleteCoverage),
                        "transformed value has no affine original script region"
                    );
                }
            }
        }
    }

    // Implementation contract: naming.editor.original-instance-member-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
    #[test]
    fn original_member_rename_checks_builtin_roster_visibility_and_source_family_closure() {
        // Implementation contract: naming.editor.original-instance-member-rename-plans
        // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
        let source =
            "oo::class create C {method pick {} {return body}}\nC create object\nobject pick\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let target = identity(source, &analysis);
        let doc = document(source, &analysis);
        assert_eq!(
            original_member_rename_edits(&[doc], &target, "destroy"),
            Err(OriginalMemberRenameRefusal::Collision)
        );
        assert_eq!(
            original_member_rename_edits(&[doc], &target, "Upper"),
            Err(OriginalMemberRenameRefusal::ChangesVisibility)
        );
        assert_eq!(
            original_member_rename_edits(&[doc, doc], &target, "changed"),
            Err(OriginalMemberRenameRefusal::IncompleteFamily)
        );
        let displaced = format!("# displaced\n{source}");
        assert_eq!(
            original_member_rename_edits(&[document(&displaced, &analysis)], &target, "changed"),
            Err(OriginalMemberRenameRefusal::StaleSource)
        );
        for source in [
            "oo::class create C {method pick {} {}; method pick {} {return body}}",
            "oo::class create C {method pick {} {return body}; foreach name {other} {method $name {} {}}}",
            "oo::class create C {method pick {} {return body}}; C create object; after idle {object pick}",
            "oo::class create C {method pick {} {return body}}; oo::class create D {superclass C}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let target = identity(source, &analysis);
            assert!(
                original_member_rename_edits(&[document(source, &analysis)], &target, "changed")
                    .is_err(),
                "{source}"
            );
        }
    }
}
