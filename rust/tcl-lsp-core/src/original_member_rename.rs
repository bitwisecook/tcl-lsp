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

/// A Tcl 9.0 `classmethod` is one member reached from both sides (tclsh
/// 9.0.4: `Factory make`, `$f make` and `my make` all run it), so its calls
/// are joined by declaration alone; every other member keeps its side.
fn same_member(
    left: &OriginalSourceMethodMetadata,
    right: &OriginalSourceMethodMetadata,
    classmethod: bool,
) -> bool {
    if classmethod {
        left.declaration() == right.declaration() && left.metadata() == right.metadata()
    } else {
        same_method(left, right)
    }
}

/// A native `classmethod` declaration, as distinct from a class-only
/// `self method` sharing its member kind.
fn native_classmethod(
    identity: &OriginalDeclarationIdentity,
    method: &OriginalSourceMethodMetadata,
) -> bool {
    identity.role() == OriginalDeclarationRole::Method(MemberSide::ClassObject)
        && method.side() == MemberSide::ClassObject
        && native_classmethod_row(method)
}

/// Either side's row of a native `classmethod`: tclsh 9.0 installs it on the
/// class's delegate, so it is listed for the class object and its instances.
fn native_classmethod_row(method: &OriginalSourceMethodMetadata) -> bool {
    // registry-axis-ok: definition_body — a native classmethod row's recorded kind; until member metadata carries a typed kind
    method.metadata().kind == "classmethod" && !method.metadata().is_self_method
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
    let (SignatureSourceNameInput::OriginalWord(_) | SignatureSourceNameInput::OriginalValue(_)) =
        declaration.original_name_input()
    else {
        return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
    };
    script_body_region(
        declaration.body_word(),
        words,
        declaration.original_name_input().policy().string_protocol(),
    )
}

/// The original script extent of a whole body word of the walked command,
/// only where its formed value is exactly that source text.
fn script_body_region(
    body: Option<&tcl_lexer::NativeWord>,
    words: &[tcl_lexer::NativeWord],
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Span, OriginalMemberRenameRefusal> {
    let body = body
        .filter(|body| !body.group().expand && words.contains(body))
        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let content = body
        .content_span()
        .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(body),
        protocol,
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
    let literal =
        tcl_syntax::backslash::native_source_literal_bytes(raw, body.image().channel(), protocol)
            .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
    if literal.as_ref() != value {
        return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
    }
    Ok(content)
}

/// The script body of the method, constructor or destructor one of `records`
/// declares at this command, with that record's index and whether the body
/// runs on the record's instances. The independently retained definition
/// worker supplies source body geometry, never an ordinary runtime call.
pub(crate) fn declared_member_body(
    records: &[&ClassRecord],
    command_offset: u32,
    words: &[tcl_lexer::NativeWord],
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Option<(Span, usize, bool)>, OriginalMemberRenameRefusal> {
    if let Some((class, declaration)) = records
        .iter()
        .enumerate()
        .flat_map(|(index, record)| {
            record
                .metadata()
                .original_members
                .declarations()
                .map(move |row| (index, row))
        })
        .find(|(_, row)| row.declaration().site().offset == command_offset)
    {
        if declaration.declaration().static_occurrence().is_none()
            || !words.contains(declaration.declaration().original_word())
        {
            return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
        }
        // The selected whole body word and shared delimiter/value owners
        // provide source geometry; report spans supply no script.
        let body = original_method_body_region(declaration, words)?;
        let instance =
            declaration.side() == MemberSide::Instance && !declaration.native_class_delegate();
        return Ok(Some((body, class, instance)));
    }
    // A constructor or destructor body runs with the new or dying object as
    // its receiver, an instance of its class or a descendant.
    if let Some((class, special)) = records
        .iter()
        .enumerate()
        .flat_map(|(index, record)| {
            record
                .metadata()
                .original_special_members
                .declarations()
                .map(move |row| (index, row))
        })
        .find(|(_, row)| row.site().offset == command_offset)
    {
        if !words.contains(special.keyword_word()) {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        let body = script_body_region(Some(special.body_word()), words, protocol)?;
        return Ok(Some((body, class, special.side() == MemberSide::Instance)));
    }
    Ok(None)
}

type ClassRecord = tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
    tcl_compiler::analyser::ClassDef,
>;

/// One closed source relation component of the owner document and every
/// same-named instance declaration it carries. tclsh 8.6–9.1: `my speak`
/// written in a base body runs a subclass override on a subclass instance, so
/// the member keeps its behaviour only when the whole family is renamed.
struct MemberFamily<'b> {
    owner: usize,
    records: Vec<&'b ClassRecord>,
    parents: Vec<Vec<usize>>,
    component: Vec<usize>,
    orders: Vec<Option<Vec<usize>>>,
    members: Vec<(usize, &'b OriginalSourceMethodMetadata)>,
}

impl MemberFamily<'_> {
    fn index_of(&self, class: &tcl_compiler::analyser::ClassDef) -> Option<usize> {
        let mut found = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| std::ptr::eq(record.metadata(), class));
        let (index, _) = found.next()?;
        found.next().is_none().then_some(index)
    }

    fn in_component(&self, index: usize) -> bool {
        self.component.contains(&index)
    }

    fn declares(&self, index: usize) -> bool {
        self.members.iter().any(|(owner, _)| *owner == index)
    }

    fn contains(&self, method: &OriginalSourceMethodMetadata) -> bool {
        self.members
            .iter()
            .any(|(_, member)| same_method(member, method))
    }

    /// Every component receiver whose order reaches `body` finds a family
    /// declaration on it, so `my NAME` written in that body selects the family
    /// on every receiver and never falls through to `unknown`.
    fn self_dispatch_closed(&self, body: usize) -> bool {
        self.component
            .iter()
            .filter_map(|&index| self.orders[index].as_ref())
            .filter(|order| order.contains(&body))
            .all(|order| order.iter().any(|class| self.declares(*class)))
    }

    /// The first instance declaration of `selector` along the order of
    /// `body`, the row `my selector` on a `body` instance runs.
    fn first_declaration(
        &self,
        body: usize,
        selector: &SignatureSourceNameInput,
    ) -> Result<Option<&OriginalSourceMethodMetadata>, OriginalMemberRenameRefusal> {
        let order = self.orders[body]
            .as_ref()
            .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
        for &index in order {
            let mut rows = self.records[index]
                .metadata()
                .original_members
                .declarations()
                .filter(|row| {
                    row.side() == MemberSide::Instance
                        && row.original_name_input().policy() == selector.policy()
                        && row.original_name_input().bytes() == selector.bytes()
                });
            if let Some(row) = rows.next() {
                if rows.next().is_some() {
                    return Err(OriginalMemberRenameRefusal::IncompleteFamily);
                }
                return Ok(Some(row));
            }
        }
        Ok(None)
    }

    fn roots(&self) -> impl Iterator<Item = usize> + '_ {
        self.component
            .iter()
            .copied()
            .filter(|index| self.parents[*index].is_empty())
    }
}

/// The relation component of `class` with its same-named instance family.
/// Every source class's relations must resolve to source records, so a
/// receiver outside the component cannot reach it.
fn member_family<'b>(
    analysis: &'b tcl_compiler::analyser::AnalysisResult,
    class: &ClassRecord,
    method: &OriginalSourceMethodMetadata,
) -> Result<MemberFamily<'b>, OriginalMemberRenameRefusal> {
    use tcl_compiler::analyser::class_hierarchy::original_metadata::{
        original_direct_metadata_relations_for_records,
        original_instance_metadata_order_for_inventory,
    };
    use tcl_compiler::analyser::types::OriginalSourceClassRelationKind;
    let records = analysis.original_class_declarations().collect::<Vec<_>>();
    let occupied = analysis
        .original_procedure_declarations()
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
        .collect::<Vec<_>>();
    let mut roots = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.declaration_site() == class.declaration_site());
    let (root, record) = roots
        .next()
        .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
    if roots.next().is_some() || **record != *class {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    let mut parents = Vec::with_capacity(records.len());
    let mut edges = vec![Vec::new(); records.len()];
    for index in 0..records.len() {
        let supers = original_direct_metadata_relations_for_records(
            &records,
            index,
            OriginalSourceClassRelationKind::Superclass,
            &occupied,
        )
        .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
        let mixins = original_direct_metadata_relations_for_records(
            &records,
            index,
            OriginalSourceClassRelationKind::Mixin,
            &occupied,
        )
        .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
        for &target in supers.iter().chain(&mixins) {
            edges[index].push(target);
            edges
                .get_mut(target)
                .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?
                .push(index);
        }
        parents.push(supers);
    }
    let mut component = vec![root];
    let mut pending = vec![root];
    while let Some(index) = pending.pop() {
        for &next in &edges[index] {
            if !component.contains(&next) {
                component.push(next);
                pending.push(next);
            }
        }
    }
    component.sort_unstable();
    let orders = (0..records.len())
        .map(|index| original_instance_metadata_order_for_inventory(&records, index, &occupied))
        .collect::<Vec<_>>();
    let mut members = Vec::new();
    let name = method.original_name_input();
    for &index in &component {
        if orders[index].is_none() {
            return Err(OriginalMemberRenameRefusal::IncompleteFamily);
        }
        let mut declared = records[index]
            .metadata()
            .original_members
            .declarations()
            .filter(|row| {
                row.side() == MemberSide::Instance
                    && row.original_name_input().policy() == name.policy()
                    && row.original_name_input().bytes() == name.bytes()
            });
        if let Some(row) = declared.next() {
            if declared.next().is_some() {
                return Err(OriginalMemberRenameRefusal::IncompleteFamily);
            }
            members.push((index, row));
        }
    }
    let family = MemberFamily {
        owner: root,
        records,
        parents,
        component,
        orders,
        members,
    };
    if method.side() == MemberSide::Instance && !family.contains(method) {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    Ok(family)
}

/// A pending script region: its extent, nesting depth and, for an original
/// instance method body, the record declaring it.
type PendingRegion = (usize, usize, u32, Option<usize>);

struct MemberRenameScan<'a, 'b> {
    document: OriginalDeclarationDocument<'a>,
    family: &'b MemberFamily<'b>,
    definitions: &'b [Option<tcl_compiler::command_binding::SourceCommandDefinition>],
    syntax: &'b crate::source_structure::SourceSyntaxStructure,
    method: &'b OriginalSourceMethodMetadata,
    wanted: &'b [u8],
    walk: &'b crate::refactor::FrameWalk<'a>,
    classmethod: bool,
    census: bool,
}

impl MemberRenameScan<'_, '_> {
    fn record_selected_request(
        &self,
        selected: &crate::receiver_identity::RetainedReceiverMethod<'_>,
        words: &[tcl_lexer::NativeWord],
        requests: &mut Vec<(SignatureSourceNameInput, NameBytes)>,
    ) -> Result<(), OriginalMemberRenameRefusal> {
        let Self {
            document,
            family,
            method,
            wanted,
            ..
        } = *self;
        let receiver_admitted = selected.receiver
            == tcl_compiler::command_binding::SourceMethodReceiver::Instance
            || (self.classmethod
                && selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class);
        let (Some(class), Some(receiver_class)) = (
            family.index_of(selected.class),
            family.index_of(selected.receiver_class),
        ) else {
            return Err(OriginalMemberRenameRefusal::IncompleteFamily);
        };
        let related = family.in_component(class);
        if selected.own_entry.is_some()
            || !receiver_admitted
            || related != family.in_component(receiver_class)
            || (self.classmethod && (class != family.owner || receiver_class != family.owner))
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
            words,
            input.policy().string_protocol(),
        )
        .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
        if words
            .iter()
            .enumerate()
            .skip(1)
            .any(|(ordinal, word)| word.group().expand || captured.literal(ordinal).is_none())
        {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        let selects = if self.classmethod {
            same_member(selected.metadata, method, true)
        } else {
            related && family.contains(selected.metadata)
        };
        if selects {
            requests.push((input.clone(), wanted.into()));
        }
        Ok(())
    }

    /// `my NAME …` written directly in an original instance method body that
    /// no actual receiver entered. The compiler's declared self candidates
    /// must agree with the body's own record, and the selector, like every
    /// literal argument, must be static. A family selector is renamed only
    /// when every receiver that can run the body reaches the family.
    fn declared_self_dispatch(
        &self,
        binding: &tcl_compiler::command_binding::SourceInvocationBinding,
        (tokens, words): (&tcl_compiler::ir::CommandTokens, &[tcl_lexer::NativeWord]),
        owner: Option<usize>,
        requests: &mut Vec<(SignatureSourceNameInput, NameBytes)>,
    ) -> Result<bool, OriginalMemberRenameRefusal> {
        let Self {
            document,
            family,
            method,
            wanted,
            ..
        } = *self;
        let registry = document
            .analysis
            .resolved_registry()
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        let old = method.original_name_input();
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            words,
            old.policy().string_protocol(),
        )
        .map_err(|_| OriginalMemberRenameRefusal::IncompleteCoverage)?;
        let Some(head) = captured
            .literal(0)
            .and_then(|head| std::str::from_utf8(head).ok())
        else {
            return Ok(false);
        };
        if head.contains("::")
            || registry.method_dispatch_keyword(head)
                != Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            || words.len() < 2
        {
            return Ok(false);
        }
        let (Some(body), false) = (owner, self.classmethod) else {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        };
        if words.get(1).is_some_and(|word| word.group().expand) {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        let Some(selector) = captured.literal(1) else {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        };
        if selector == wanted && wanted != old.bytes() {
            return Err(OriginalMemberRenameRefusal::Collision);
        }
        for word in words.iter().skip(2) {
            if mentions_member(document.source, word, old.bytes(), wanted)? {
                return Err(OriginalMemberRenameRefusal::UneditableLinks);
            }
        }
        let input = binding
            .original_written_name_input(tokens, 1)
            .filter(|input| input.bytes() == selector)
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        // The walked declaration body owns the receiver frame. The compiler's
        // declared candidates, where it retains them, must select the row the
        // body's own order selects.
        let first = family.first_declaration(body, &input)?;
        let declared = binding.declared_self_method_candidates(registry);
        if !declared.is_empty()
            && !first.is_some_and(|row| {
                declared.iter().all(|entry| {
                    entry.receiver()
                        == tcl_compiler::command_binding::SourceMethodReceiver::Instance
                        && entry.declaration() == row.declaration().site()
                        && entry.name_source().span == row.metadata().name_span
                })
            })
        {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        if selector != old.bytes() || !family.in_component(body) {
            return Ok(true);
        }
        if !family.self_dispatch_closed(body) {
            return Err(OriginalMemberRenameRefusal::IncompleteFamily);
        }
        if input.original_word_key().is_none()
            || !crate::original_name_edit::original_input_matches_source(
                document.source,
                document.analysis,
                &input,
                words[1].span(),
            )
        {
            return Err(OriginalMemberRenameRefusal::UneditableInput);
        }
        requests.push((input, wanted.into()));
        Ok(true)
    }

    /// `next`/`nextto` in an instance method body continues the current
    /// method's own chain and never spells a member name, so a closed family
    /// rename leaves each step of it on the renamed family (tclsh 8.6–9.1).
    fn next_chain(
        &self,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        words: &[tcl_lexer::NativeWord],
    ) -> Result<bool, OriginalMemberRenameRefusal> {
        let Self {
            document,
            method,
            wanted,
            census,
            ..
        } = *self;
        // A reference census keeps method-chain helpers open, as the source
        // census does; only an edit plan relies on the chain staying closed.
        if census {
            return Ok(false);
        }
        let Some(context) = document
            .analysis
            .resolved_input
            .as_ref()
            .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry)
        else {
            return Ok(false);
        };
        let next = crate::original_invocation::source_registry_words(
            document.source,
            document.analysis,
            command,
        )
        .and_then(|selected| {
            selected.with_source_schema(&context, |schema| {
                schema
                    .semantics
                    .traits
                    .contains(tcl_registry::Traits::TCLOO_NEXT_CHAIN)
                    && schema
                        .authored_source_descriptors()
                        .command
                        .implementation_namespace
                        .is_none()
            })
        })
        .unwrap_or(false);
        if !next {
            return Ok(false);
        }
        for word in words.iter().skip(1) {
            if word.group().expand
                || word.executable_parts().all_parts().any(|part| {
                    !matches!(
                        part.part,
                        tcl_lexer::ExecutablePart::Text(_)
                            | tcl_lexer::ExecutablePart::Variable { .. }
                    )
                })
                || mentions_member(
                    document.source,
                    word,
                    method.original_name_input().bytes(),
                    wanted,
                )?
            {
                return Err(OriginalMemberRenameRefusal::UneditableLinks);
            }
        }
        Ok(true)
    }

    /// The script region of a member declaration at this command, with the
    /// record whose instances it runs on when it is an instance body.
    fn member_body(
        &self,
        command_offset: u32,
        words: &[tcl_lexer::NativeWord],
    ) -> Result<Option<(Span, Option<usize>)>, OriginalMemberRenameRefusal> {
        let Self { family, method, .. } = *self;
        Ok(declared_member_body(
            &family.records,
            command_offset,
            words,
            method.original_name_input().policy().string_protocol(),
        )?
        .map(|(body, class, instance)| (body, instance.then_some(class))))
    }

    fn command(
        &self,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        region: PendingRegion,
        pending: &mut Vec<PendingRegion>,
        requests: &mut Vec<(SignatureSourceNameInput, NameBytes)>,
    ) -> Result<(), OriginalMemberRenameRefusal> {
        let Self {
            document,
            definitions,
            syntax,
            method,
            wanted,
            walk,
            ..
        } = *self;
        let (_, _, depth, owner) = region;
        let words = walk
            .native_words(document.source, command)
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        let command_offset = words
            .first()
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?
            .span()
            .start();
        let tokens = walk.tokens(document.source, command);
        if let Some(body) = self.member_body(command_offset, &words)? {
            pending.push((
                body.0.start() as usize,
                body.0.end() as usize,
                depth + 1,
                body.1,
            ));
            return Ok(());
        }
        let binding = tokens
            .source_binding
            .as_ref()
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        if let Some(selected) =
            crate::receiver_identity::method_at_command(document.analysis, document.source, command)
        {
            self.record_selected_request(&selected, &words, requests)?;
            return Ok(());
        }
        if self.declared_self_dispatch(binding, (&tokens, &words), owner, requests)? {
            return Ok(());
        }
        if owner.is_some() && self.next_chain(command, &words)? {
            return Ok(());
        }
        if crate::original_declaration::class_relation_member(syntax, command) {
            return Ok(());
        }
        if let Some(shape) = binding.original_default_manufacture_shape(
            &tokens,
            document
                .analysis
                .resolved_registry()
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?,
        ) {
            if !definitions
                .iter()
                .flatten()
                .any(|definition| definition == shape.class_definition())
            {
                return Err(OriginalMemberRenameRefusal::IncompleteFamily);
            }
            return Ok(());
        }
        let selected = walk
            .structure(document.source, command)
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        registry_coverage(&selected)?;
        let same_frame = walk.same_frame_regions(document.source, command);
        let shifted = walk.frame_shifted_regions(document.source, command);
        if !walk.complete() {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        // Literal text naming either member outside a script this walk
        // classifies could be a captured selector dispatched later.
        let old = method.original_name_input().bytes();
        for word in words.iter().skip(1) {
            for name in [old, wanted] {
                let mentions =
                    crate::original_declaration::word_member_mentions(document.source, word, name)
                        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
                if mentions.iter().any(|mention| {
                    !same_frame.iter().chain(&shifted).any(|&(start, end)| {
                        start <= mention.start() as usize && mention.end() as usize <= end
                    })
                }) {
                    return Err(OriginalMemberRenameRefusal::UneditableLinks);
                }
            }
        }
        pending.extend(
            same_frame
                .into_iter()
                .map(|(start, end)| (start, end, depth + 1, owner)),
        );
        pending.extend(
            shifted
                .into_iter()
                .map(|(start, end)| (start, end, depth + 1, None)),
        );
        Ok(())
    }
}

fn requests(
    document: OriginalDeclarationDocument<'_>,
    family: &MemberFamily<'_>,
    method: &OriginalSourceMethodMetadata,
    wanted: &[u8],
    (classmethod, census): (bool, bool),
) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalMemberRenameRefusal> {
    let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)
        .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let definitions = family
        .records
        .iter()
        .map(|record| class_definition(document, record))
        .collect::<Vec<_>>();
    let mut requests = if classmethod {
        vec![(
            method.declaration().original_name_input().clone(),
            wanted.into(),
        )]
    } else {
        family
            .members
            .iter()
            .map(|(_, member)| {
                (
                    member.declaration().original_name_input().clone(),
                    NameBytes::from(wanted),
                )
            })
            .collect()
    };
    let syntax =
        crate::source_structure::SourceSyntaxStructure::capture(document.source, document.analysis)
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
    let mut pending: Vec<PendingRegion> = vec![(0, document.source.len(), 0, None)];
    let mut visited = HashSet::new();
    let scan = MemberRenameScan {
        document,
        family,
        definitions: &definitions,
        syntax: &syntax,
        method,
        wanted,
        walk: &walk,
        classmethod,
        census,
    };
    while let Some(region) = pending.pop() {
        let (start, end, depth, owner) = region;
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth)
            || start > end
            || end > document.source.len()
        {
            return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
        }
        if !visited.insert((start, end, owner)) {
            continue;
        }
        let text = document
            .source
            .get(start..end)
            .ok_or(OriginalMemberRenameRefusal::StaleSource)?;
        let offset = u32::try_from(start).map_err(|_| OriginalMemberRenameRefusal::StaleSource)?;
        for command in if depth == 0 {
            walk.segment_document(text)
        } else {
            walk.segment_nested(text, offset)
        } {
            scan.command(&command, region, &mut pending, &mut requests)?;
        }
    }
    Ok(requests)
}

/// Edits proved by the complete single-document member census instead of the
/// executable walk: every command of every reachable region is classified,
/// and every self dispatch corresponds one-to-one to the compiler's typed
/// member tape. A family selector is renamed only when every receiver that
/// can run its body reaches the family.
fn census_requests(
    document: OriginalDeclarationDocument<'_>,
    family: &MemberFamily<'_>,
    method: &OriginalSourceMethodMetadata,
    wanted: &[u8],
) -> Option<Vec<(SignatureSourceNameInput, NameBytes)>> {
    use tcl_compiler::analyser::class_hierarchy::original_metadata::{
        original_source_self_method_references, original_source_self_property_references,
    };
    let methods = original_source_self_method_references(document.source, document.analysis)?;
    let properties = original_source_self_property_references(document.source, document.analysis)?;
    let old = method.original_name_input();
    if !crate::original_declaration::complete_member_name_census(
        old.bytes(),
        document.source,
        document.analysis,
        &methods,
        &properties,
    ) {
        return None;
    }
    let mut requests = family
        .members
        .iter()
        .map(|(_, member)| {
            (
                member.declaration().original_name_input().clone(),
                NameBytes::from(wanted),
            )
        })
        .collect::<Vec<_>>();
    let mut spans = methods
        .iter()
        .filter(|reference| {
            reference.selector_input().policy() == old.policy()
                && reference.selector_input().bytes() == old.bytes()
        })
        .map(tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfMethodReference::selector_span)
        .collect::<Vec<_>>();
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    for span in spans {
        let rows = methods
            .iter()
            .filter(|reference| reference.selector_span() == span)
            .collect::<Vec<_>>();
        let first = rows.first()?;
        let mut bodies = family
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| ***record == *first.body_class());
        let (body, _) = bodies.next()?;
        if bodies.next().is_some()
            || rows.iter().any(|row| {
                row.body_class() != first.body_class()
                    || row.selector_input() != first.selector_input()
            })
        {
            return None;
        }
        let in_family = rows
            .iter()
            .filter(|row| family.contains(row.method()))
            .count();
        if !family.in_component(body) {
            if in_family > 0 {
                return None;
            }
            continue;
        }
        if in_family != rows.len() || !family.self_dispatch_closed(body) {
            return None;
        }
        first.selector_input().original_word_key()?;
        requests.push((first.selector_input().clone(), NameBytes::from(wanted)));
    }
    Some(requests)
}

/// Original declaration name extents of the instance family a member rename
/// edits together, for consumers that join the family's reference sets.
pub(crate) fn original_member_family_spans(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
) -> Vec<Span> {
    let (Some(class), Some(method)) = (
        identity.class_metadata(document.analysis),
        identity.method_metadata(document.analysis),
    ) else {
        return Vec::new();
    };
    if identity.role() != OriginalDeclarationRole::Method(MemberSide::Instance) {
        return Vec::new();
    }
    member_family(document.analysis, class, method).map_or_else(
        |_| Vec::new(),
        |family| {
            family
                .members
                .iter()
                .filter_map(|(_, member)| {
                    member
                        .declaration()
                        .static_occurrence()
                        .map(|occurrence| occurrence.name_input().span())
                })
                .collect()
        },
    )
}

/// The complete single-document selector occurrences of an instance member
/// whose family is the member alone, proved by the walk that plans its
/// rename. A member with overrides or an unproved walk has no census here.
pub(crate) fn original_member_occurrence_census(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
) -> Option<Vec<Span>> {
    if identity.role() != OriginalDeclarationRole::Method(MemberSide::Instance) {
        return None;
    }
    let class = identity.class_metadata(document.analysis)?;
    let method = identity.method_metadata(document.analysis)?;
    let family = member_family(document.analysis, class, method).ok()?;
    if family.members.len() != 1 {
        return None;
    }
    let declaration = method.declaration().original_name_input();
    let [(_, requests)] = original_member_rename_plan(
        &[document],
        identity,
        method.original_name_input().bytes(),
        true,
    )
    .ok()?
    .try_into()
    .ok()?;
    let mut spans = requests
        .iter()
        .filter(|(input, _)| input != declaration)
        .map(|(input, _)| {
            crate::original_name_edit::original_static_name_value_span(
                input,
                0..input.bytes().len(),
            )
        })
        .collect::<Option<Vec<_>>>()?;
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    Some(spans)
}

/// Whether literal text of `word` may name the old or the proposed member.
fn mentions_member(
    source: &str,
    word: &tcl_lexer::NativeWord,
    old: &[u8],
    wanted: &[u8],
) -> Result<bool, OriginalMemberRenameRefusal> {
    for name in [old, wanted] {
        if !crate::original_declaration::word_member_mentions(source, word, name)
            .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?
            .is_empty()
        {
            return Ok(true);
        }
    }
    Ok(false)
}
/// Plan a bounded instance-table rename in one complete current source owner
/// and the consumers that dispatch to it through a class loaded from that
/// owner. The current ordinary class roster includes builtin collisions. Inherited
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
    let (SignatureSourceNameInput::OriginalWord(_) | SignatureSourceNameInput::OriginalValue(_)) =
        input
    else {
        return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
    };
    let wanted = tcl_syntax::backslash::native_source_literal_bytes(
        new_name.as_bytes(),
        SourceImage::document(new_name).channel(),
        input.policy().string_protocol(),
    )
    .map_err(|_| OriginalMemberRenameRefusal::InvalidName)?;
    original_member_rename_units(documents, identity, &wanted)
}

fn incomplete_member_family(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
    class: &ClassRecord,
    family: &MemberFamily<'_>,
) -> bool {
    identity.class_metadata(document.analysis) != Some(class)
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
        || family.component.iter().any(|&index| {
            let class = family.records[index].metadata();
            // Resolved source relations are the component's own edges.
            (family.component.len() == 1 && class.original_relations.effects().next().is_some())
                || class.original_members.effects().next().is_some()
                || class.original_properties.declarations().next().is_some()
                || class.original_members.declarations().any(|row| {
                    (row.side() != MemberSide::Instance || row.native_class_delegate())
                        && !native_classmethod_row(row)
                        || row.forward_prefix().is_some()
                        || row.declaration().static_occurrence().is_none()
                })
        })
}

fn validate_member_roster(
    method: &OriginalSourceMethodMetadata,
    roster: &OriginalClassInstanceMethodRoster,
    input: &SignatureSourceNameInput,
    (wanted, classmethod): (&[u8], bool),
) -> Result<(), OriginalMemberRenameRefusal> {
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
    let current_entry = if classmethod {
        roster.source_class_object_method_for_input(input)
    } else {
        roster.source_method_for_input(input)
    }
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
    Ok(())
}

/// A consumer reaches the member only through a class it loaded from the
/// owner's current source, so each call is joined by that declaration site.
/// A selector the consumer may dispatch with an unknown receiver withdraws
/// the edit rather than leaving a call that no longer names the member.
fn consumer_requests(
    document: OriginalDeclarationDocument<'_>,
    members: &[&OriginalSourceMethodMetadata],
    (wanted, class_name): (&[u8], &str),
    classmethod: bool,
) -> Result<Vec<(SignatureSourceNameInput, NameBytes)>, OriginalMemberRenameRefusal> {
    let Some(method) = members.first() else {
        return Ok(Vec::new());
    };
    let site = method.declaration().site();
    let old = method.original_name_input().bytes();
    let mut requests = Vec::new();
    let mut touched = false;
    let mut refusal = None;
    let proved = std::cell::RefCell::new(HashSet::new());
    let visit = crate::executable_regions::visit_checked_analysis_executable_commands_proving(
        document.source,
        document.analysis,
        &mut |command, _, _| {
            let class_dispatch = crate::receiver_identity::class_at_command_head(
                document.analysis,
                document.source,
                command,
            )
            .is_some()
                || crate::receiver_identity::class_command_at(
                    document.analysis,
                    document.source,
                    command,
                );
            if class_dispatch {
                proved.borrow_mut().insert(command.span);
            }
            let Some(foreign) = crate::receiver_identity::foreign_method_at_command(
                document.analysis,
                document.source,
                command,
            ) else {
                // A class command dispatching the old selector that no member
                // owns must not be left behind by the edit.
                let names_class = command.texts.first().is_some_and(|head| {
                    tcl_syntax::naming::key_tail(head) == tcl_syntax::naming::key_tail(class_name)
                });
                if (class_dispatch || names_class)
                    && command.texts.get(1).map(String::as_bytes) == Some(old)
                {
                    refusal = Some(OriginalMemberRenameRefusal::IncompleteCoverage);
                    return true;
                }
                if crate::receiver_identity::method_at_command(
                    document.analysis,
                    document.source,
                    command,
                )
                .is_none()
                    && command
                        .texts
                        .first()
                        .is_some_and(|head| head.contains(['$', '[']))
                    && command.texts.get(1).map(String::as_bytes) == Some(old)
                {
                    refusal = Some(OriginalMemberRenameRefusal::IncompleteCoverage);
                    return true;
                }
                return false;
            };
            proved.borrow_mut().insert(command.span);
            if foreign.declaration.source != site.source {
                return false;
            }
            touched = true;
            if !members.iter().any(|member| {
                foreign.declaration == *member.declaration().site()
                    && foreign.name_span == member.metadata().name_span
                    && (foreign.side == member.side() || classmethod)
            }) {
                return false;
            }
            if foreign.selector_input.original_word_key().is_none()
                || !crate::original_name_edit::original_input_matches_source(
                    document.source,
                    document.analysis,
                    &foreign.selector_input,
                    foreign.selector,
                )
            {
                refusal = Some(OriginalMemberRenameRefusal::UneditableInput);
                return true;
            }
            requests.push((foreign.selector_input, wanted.into()));
            false
        },
        |span| proved.borrow().contains(&span),
    );
    if let Some(refusal) = refusal {
        return Err(refusal);
    }
    if touched && visit != crate::executable_regions::PotentialSourceVisit::Complete {
        return Err(OriginalMemberRenameRefusal::IncompleteCoverage);
    }
    Ok(requests)
}

fn document_edits(
    document: OriginalDeclarationDocument<'_>,
    requests: &[(SignatureSourceNameInput, NameBytes)],
) -> Result<Vec<WorkspaceTextEdit>, OriginalMemberRenameRefusal> {
    let edits = crate::original_name_edit::original_name_input_edits(
        &SourceImage::document(document.source),
        requests,
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

/// Whether any class may inherit from `class`, which would carry a classmethod
/// to its own command (tclsh 9.0.4: `SubFactory make` runs `Factory`'s).
/// Relations are compared by name tail, so a same-tailed class elsewhere also
/// withholds the edit rather than risk leaving a subclass call behind.
fn inherits_from(
    documents: &[OriginalDeclarationDocument<'_>],
    class: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
) -> bool {
    let tail = tcl_syntax::naming::key_tail(&class.metadata().qualified_name);
    documents.iter().any(|document| {
        document
            .analysis
            .original_class_declarations()
            .any(|other| {
                other
                    .metadata()
                    .superclasses
                    .iter()
                    .chain(&other.metadata().mixins)
                    .any(|name| tcl_syntax::naming::key_tail(name) == tail)
            })
    })
}

/// The current completed command world's class definition for `class`.
fn class_definition(
    document: OriginalDeclarationDocument<'_>,
    class: &ClassRecord,
) -> Option<tcl_compiler::command_binding::SourceCommandDefinition> {
    let world = document.analysis.original_completed_command_world()?;
    world
        .declaration_at(class.name().slot(), class.name().policy())
        .filter(|row| row.declaration_site() == class.declaration_site())?
        .definition()
        .cloned()
}

/// The current completed command world's member roster for `class`.
fn class_member_roster(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
    class: &ClassRecord,
) -> Result<OriginalClassInstanceMethodRoster, OriginalMemberRenameRefusal> {
    let world = document
        .analysis
        .original_completed_command_world()
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    if world.source_image() != &SourceImage::document(document.source)
        || world.lexer_config() != identity.config()
    {
        return Err(OriginalMemberRenameRefusal::StaleSource);
    }
    let definition =
        class_definition(document, class).ok_or(OriginalMemberRenameRefusal::UnknownRoster)?;
    world
        .class_instance_method_roster(&definition)
        .ok_or(OriginalMemberRenameRefusal::UnknownRoster)
}

/// Every family declaration must be a static, uniquely named source word.
fn editable_member(
    method: &OriginalSourceMethodMetadata,
    classmethod: bool,
) -> Result<(), OriginalMemberRenameRefusal> {
    if method.declaration().static_occurrence().is_none()
        || (method.native_class_delegate() && !classmethod)
        || method.forward_prefix().is_some()
        || method.original_name_input() != method.declaration().original_name_input()
    {
        return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
    }
    let (SignatureSourceNameInput::OriginalWord(_) | SignatureSourceNameInput::OriginalValue(_)) =
        method.declaration().original_name_input()
    else {
        return Err(OriginalMemberRenameRefusal::UneditableDeclaration);
    };
    Ok(())
}

/// The family's rosters: every root class of the component carries the
/// builtin names its descendants inherit, and every family declaration keeps
/// its default visibility under the proposed name. A source declaration of
/// the proposed name anywhere in the component is a collision.
fn validate_family_rosters(
    document: OriginalDeclarationDocument<'_>,
    identity: &OriginalDeclarationIdentity,
    family: &MemberFamily<'_>,
    method: &OriginalSourceMethodMetadata,
    wanted: &[u8],
) -> Result<(), OriginalMemberRenameRefusal> {
    let input = method.declaration().original_name_input();
    let mut roots = 0_usize;
    for root in family.roots() {
        roots += 1;
        let roster = class_member_roster(document, identity, family.records[root])?;
        if let Some((_, member)) = family.members.iter().find(|(owner, _)| *owner == root) {
            validate_member_roster(
                member,
                &roster,
                member.declaration().original_name_input(),
                (wanted, false),
            )?;
            continue;
        }
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
        if roster.default_exported(input.bytes()) != method.exported()
            || roster.default_exported(wanted) != method.exported()
        {
            return Err(OriginalMemberRenameRefusal::ChangesVisibility);
        }
        if wanted != input.bytes() && roster.contains(wanted) {
            return Err(OriginalMemberRenameRefusal::Collision);
        }
    }
    if roots == 0 {
        return Err(OriginalMemberRenameRefusal::UnknownRoster);
    }
    for (_, member) in &family.members {
        editable_member(member, false)?;
        if member.exported() != method.exported() {
            return Err(OriginalMemberRenameRefusal::ChangesVisibility);
        }
    }
    if wanted != input.bytes()
        && family.component.iter().any(|&index| {
            family.records[index]
                .metadata()
                .original_members
                .declarations()
                .any(|row| {
                    row.side() == MemberSide::Instance
                        && row.original_name_input().policy() == input.policy()
                        && row.original_name_input().bytes() == wanted
                })
        })
    {
        return Err(OriginalMemberRenameRefusal::Collision);
    }
    Ok(())
}

/// Whether another document may extend the component: a class related to,
/// or redefining, one of its classes by name, or any class configuration.
fn consumer_extends_family(
    consumer: OriginalDeclarationDocument<'_>,
    family: &MemberFamily<'_>,
) -> Option<OriginalMemberRenameRefusal> {
    let tails = family
        .component
        .iter()
        .map(|&index| {
            tcl_syntax::naming::key_tail(&family.records[index].metadata().qualified_name)
        })
        .collect::<Vec<_>>();
    for other in consumer.analysis.original_class_declarations() {
        if family
            .component
            .iter()
            .any(|&index| other.name().slot() == family.records[index].name().slot())
        {
            return Some(OriginalMemberRenameRefusal::AmbiguousOwner);
        }
        if other
            .metadata()
            .superclasses
            .iter()
            .chain(&other.metadata().mixins)
            .any(|name| tails.contains(&tcl_syntax::naming::key_tail(name)))
        {
            return Some(OriginalMemberRenameRefusal::IncompleteFamily);
        }
    }
    (consumer
        .analysis
        .original_class_configurations()
        .next()
        .is_some()
        || consumer
            .analysis
            .original_object_configurations()
            .next()
            .is_some())
    .then_some(OriginalMemberRenameRefusal::IncompleteFamily)
}

type DocumentRequests<'a> = (
    OriginalDeclarationDocument<'a>,
    Vec<(SignatureSourceNameInput, NameBytes)>,
);

fn original_member_rename_units(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    wanted: &[u8],
) -> Result<Vec<WorkspaceTextEdit>, OriginalMemberRenameRefusal> {
    let mut edits = Vec::new();
    for (document, requests) in original_member_rename_plan(documents, identity, wanted, false)? {
        edits.extend(document_edits(document, &requests)?);
    }
    Ok(edits)
}

/// Every document's original naming inputs a member rename replaces.
fn original_member_rename_plan<'a>(
    documents: &[OriginalDeclarationDocument<'a>],
    identity: &OriginalDeclarationIdentity,
    wanted: &[u8],
    census: bool,
) -> Result<Vec<DocumentRequests<'a>>, OriginalMemberRenameRefusal> {
    let mut owners = documents
        .iter()
        .filter(|document| document.uri == identity.uri());
    let document = owners
        .next()
        .ok_or(OriginalMemberRenameRefusal::AmbiguousOwner)?;
    if owners.next().is_some() {
        return Err(OriginalMemberRenameRefusal::AmbiguousOwner);
    }
    if !identity.is_current(document.uri, document.source, document.analysis) {
        return Err(OriginalMemberRenameRefusal::StaleSource);
    }
    let classmethod = identity
        .method_metadata(document.analysis)
        .is_some_and(|method| native_classmethod(identity, method));
    if identity.role() != OriginalDeclarationRole::Method(MemberSide::Instance) && !classmethod {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    let class = identity
        .class_metadata(document.analysis)
        .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
    let method = identity
        .method_metadata(document.analysis)
        .ok_or(OriginalMemberRenameRefusal::StaleSource)?;
    let family = member_family(document.analysis, class, method)?;
    // A classmethod keeps the single-class scope: its delegate chain is not
    // part of the instance relation component.
    if (classmethod && family.records.len() != 1)
        || incomplete_member_family(*document, identity, class, &family)
    {
        return Err(OriginalMemberRenameRefusal::IncompleteFamily);
    }
    editable_member(method, classmethod)?;
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
    let input = method.declaration().original_name_input();
    if classmethod {
        let roster = class_member_roster(*document, identity, class)?;
        validate_member_roster(method, &roster, input, (wanted, classmethod))?;
        if wanted != input.bytes() && roster.class_object_contains(wanted) {
            return Err(OriginalMemberRenameRefusal::Collision);
        }
        if inherits_from(documents, class) {
            return Err(OriginalMemberRenameRefusal::IncompleteFamily);
        }
    } else {
        validate_family_rosters(*document, identity, &family, method, wanted)?;
    }
    let requests = match requests(*document, &family, method, wanted, (classmethod, census)) {
        Err(
            refusal @ (OriginalMemberRenameRefusal::IncompleteCoverage
            | OriginalMemberRenameRefusal::UneditableLinks),
        ) if !classmethod => census_requests(*document, &family, method, wanted).ok_or(refusal)?,
        selected => selected?,
    };
    let mut plan = vec![(*document, requests)];
    let members = if classmethod {
        vec![method]
    } else {
        family.members.iter().map(|(_, member)| *member).collect()
    };
    for consumer in documents.iter().filter(|other| other.uri != document.uri) {
        if let Some(refusal) = consumer_extends_family(*consumer, &family) {
            return Err(refusal);
        }
        let requests = consumer_requests(
            *consumer,
            &members,
            (wanted, &class.metadata().qualified_name),
            classmethod,
        )?;
        if !requests.is_empty() {
            plan.push((*consumer, requests));
        }
    }
    Ok(plan)
}

/// Workspace adapter for the same bounded current source-family protocol.
/// The owner's family stays closed; consumers join only by the declaration
/// site their loaded class retains, never by reporting names.
///
/// # Errors
/// Returns the edit planner's independently missing obligation.
pub fn original_member_prepare_span_in(
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    cursor_uri: &str,
    cursor: u32,
) -> Result<Span, OriginalMemberRenameRefusal> {
    let document = documents
        .iter()
        .find(|document| document.uri == cursor_uri)
        .ok_or(OriginalMemberRenameRefusal::IncompleteFamily)?;
    let owner = documents
        .iter()
        .find(|document| document.uri == identity.uri())
        .ok_or(OriginalMemberRenameRefusal::AmbiguousOwner)?;
    let method = identity
        .method_metadata(owner.analysis)
        .ok_or(OriginalMemberRenameRefusal::StaleSource)?;
    original_member_rename_units(documents, identity, method.original_name_input().bytes())?;
    let classmethod = native_classmethod(identity, method);
    let input = if document.uri != identity.uri() {
        consumer_selector_at(*document, method, cursor, classmethod)?
    } else if identity.span().start() <= cursor && cursor < identity.span().end() {
        identity
            .input()
            .ok_or(OriginalMemberRenameRefusal::UneditableInput)?
            .clone()
    } else {
        let selected =
            crate::receiver_identity::method_at_cursor(document.analysis, document.source, cursor)
                .ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)?;
        if !same_member(selected.metadata, method, classmethod) {
            return Err(OriginalMemberRenameRefusal::IncompleteFamily);
        }
        selected.original_selector_input().clone()
    };
    crate::original_name_edit::original_static_name_value_span(&input, 0..input.bytes().len())
        .ok_or(OriginalMemberRenameRefusal::UneditableInput)
}

fn consumer_selector_at(
    document: OriginalDeclarationDocument<'_>,
    method: &OriginalSourceMethodMetadata,
    cursor: u32,
    classmethod: bool,
) -> Result<SignatureSourceNameInput, OriginalMemberRenameRefusal> {
    let mut selected = None;
    crate::executable_regions::visit_analysis_executable_commands(
        document.source,
        document.analysis,
        &mut |command, _, _| {
            if !(command.span.start() <= cursor && cursor <= command.span.end()) {
                return false;
            }
            if let Some(foreign) = crate::receiver_identity::foreign_method_at_command(
                document.analysis,
                document.source,
                command,
            ) && foreign.selector.start() <= cursor
                && cursor <= foreign.selector.end()
                && foreign.declaration == *method.declaration().site()
                && foreign.name_span == method.metadata().name_span
                && (foreign.side == method.side() || classmethod)
            {
                selected = Some(foreign.selector_input);
                return true;
            }
            false
        },
    );
    selected.ok_or(OriginalMemberRenameRefusal::IncompleteCoverage)
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
    original_member_prepare_span_in(&[document], identity, document.uri, cursor)
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
            Err(OriginalMemberRenameRefusal::AmbiguousOwner)
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
            // `B`'s own instances reach no `pick`, so its `my pick` selects
            // the family only on `D` receivers.
            "oo::class create C {method pick {} {return body}}; oo::class create B {method run {} {my pick}}; oo::class create D {superclass B C}",
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
