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

//! Cards for class members: declarations, constructors, method dispatch and resolution-order notes.

use super::Hover;
use super::classes::class_hover_text;
use tcl_compiler::analyser::AnalysisResult;

/// The card for a class member whose declaring class this document's analysis
/// holds: the class's qualified name, the declared parameters and the MRO
/// note. Display only; the candidate's own card states its purpose.
#[must_use]
pub fn declared_member_hover(
    source: &str,
    analysis: &AnalysisResult,
    candidate: &crate::method_symbol::OriginalMethodCandidate,
) -> Option<Hover> {
    candidate.own_object().is_none().then_some(())?;
    let class = candidate.declaring_class_in(source, analysis)?.metadata();
    let method = candidate.method();
    let name = crate::original_oo::method_label(method);
    let kind = match method.side() {
        tcl_compiler::analyser::MemberSide::Instance => "method",
        tcl_compiler::analyser::MemberSide::ClassObject => "classmethod",
    };
    let parameters = if method.metadata().params_computed {
        "unknown parameters".to_owned()
    } else {
        format!("{} param(s)", method.metadata().params.len())
    };
    let note = oo_method_resolution_note(analysis, &class.qualified_name, &name)
        .map_or(String::new(), |note| format!("  \n{note}"));
    let purpose = if candidate.has_call_point_entry() {
        "Resolved at this call"
    } else {
        "Possible source declaration"
    };
    Some(Hover::markdown(format!(
        "**{kind}** `{}` ({parameters}){note}\n\n{purpose}.",
        tcl_syntax::naming::rooted_member_key(&class.qualified_name, &name)
    )))
}

/// The card for a member of the analysed class whose body holds the cursor,
/// on a release profile where the strict path had no answer: a self-dispatch
/// selector, or the member's own declaration name. Display only.
pub(super) fn declared_class_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<Hover> {
    if analysis.allows_lexical_declaration_advice() {
        return None;
    }
    if let Some((receiver, provider, method)) =
        crate::definition::declared_self_dispatch_at(source, analysis, cursor_offset)
    {
        let note = oo_resolution_note_for_provider(
            analysis,
            &receiver.qualified_name,
            &provider.qualified_name,
            &method.name,
        )
        .map_or(String::new(), |note| format!("  \n{note}"));
        return Some(Hover::markdown(format!(
            "**method** `{}` ({} param(s)){note}",
            tcl_syntax::naming::rooted_member_key(&provider.qualified_name, &method.name),
            method.params.len(),
        )));
    }
    let class = crate::definition::declared_class_at(analysis, cursor_offset)?;
    let word =
        crate::source_structure::original_complete_cursor_word(source, analysis, cursor_offset)?
            .static_unicode_value()?;
    member_declaration_hover_text(analysis, class, &word, cursor_offset).map(Hover::markdown)
}

// Constructor heads carry their own site-specific class selection.
pub(super) fn source_constructor_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<Hover> {
    let prefix =
        crate::source_structure::original_word_cursor_prefix(source, analysis, cursor_offset)
            .ok()??;
    (prefix.word_index() == 0).then_some(())?;
    let call = tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
        source,
        analysis,
        prefix.container_span().start(),
    )?;
    call.original_words().first().filter(|head| {
        head.span() == prefix.container_span()
            && head.span().as_range().contains(&(cursor_offset as usize))
    })?;
    let class = call.class_declaration().source_class(analysis)?;
    Some(Hover::markdown(class_hover_text(
        analysis,
        class.metadata(),
    )))
}

/// Original linked command heads retain their conditional member owner.
pub(super) fn original_linked_member_hover_text(
    source: &str,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<String> {
    if let Some(linked) = tcl_compiler::analyser::types::original_source_linked_member_at(
        analysis,
        source,
        cursor_offset,
    ) {
        let class = linked.class_declaration().metadata();
        let method = linked.target().metadata();
        let qname = &class.qualified_name;
        let kind = if linked.target().side() == tcl_compiler::analyser::MemberSide::ClassObject {
            "classmethod"
        } else {
            "method"
        };
        let linked_note = if linked.alias_input().bytes() == linked.target_input().bytes() {
            String::new()
        } else {
            format!("  \nlinked from `{word}`")
        };
        let note = oo_method_resolution_note(analysis, qname, &method.name)
            .map_or(String::new(), |note| format!("  \n{note}"));
        return Some(format!(
            "**{kind}** `{qname}::{name}` ({nparam} param(s)){note}{linked_note}",
            name = method.name,
            nparam = method.params.len()
        ));
    }
    None
}

/// Original linked heads hover as their retained target member; declaration
/// names and constructor/destructor keywords keep their own source paths.
pub(super) fn class_member_hover_text(
    source: &str,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<String> {
    if analysis.allows_retained_logical_declaration_advice() {
        let config = analysis.body_lexer_config?;
        let image = tcl_lexer::SourceImage::document(source);
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        let selected = analysis.logical_source_member_at(&image, config, cursor_offset)?;
        let class = analysis.logical_source_member_class(&selected)?;
        let method = selected.metadata();
        let side = match selected.side() {
            tcl_compiler::analyser::MemberSide::Instance => "method",
            tcl_compiler::analyser::MemberSide::ClassObject => "classmethod",
        };
        let mut text = format!(
            "**{side}** `{}`",
            tcl_syntax::naming::rooted_member_key(&class.qualified_name, &method.name)
        );
        if !method.doc.is_empty() {
            text.push_str("\n\n");
            text.push_str(&method.doc);
        }
        return Some(text);
    }
    if let Some(text) = original_linked_member_hover_text(source, analysis, word, cursor_offset) {
        return Some(text);
    }
    let class_def = analysis
        .all_classes
        .get(crate::definition::enclosing_class_at(
            analysis,
            cursor_offset,
        )?)?;
    let qname = &class_def.qualified_name;
    // The member's **own declaration** token — `method reopened {v} {…}`'s
    // `reopened` — hovers as that member, wherever in the class the
    // declaration was written.  A class body is not a
    // single lexical region: `oo::define Cls { … }` reopens the class and
    // its members are just as much `Cls`'s own, so this hangs off
    // `enclosing_class_at` (which already consults
    // `AnalysisResult::class_body_spans`, the multi-span record) rather than
    // any single `body_span`, and the reopening block behaves exactly like
    // the creation block.  Without a declaration-site path only *call* sites
    // (`my m`, `$obj m`) and `link`-exposed barewords render, which would
    // leave a class member the one declared symbol in the language that
    // cannot be hovered where it is declared, while `proc` and the class name
    // itself both can.
    //
    // Gated on the cursor genuinely sitting inside the member's own
    // `name_span`, so this never fires for a same-named word elsewhere in
    // the body (those keep falling through to the tiers below).
    if let Some(text) = member_declaration_hover_text(analysis, class_def, word, cursor_offset) {
        return Some(text);
    }
    // A constructor or destructor keyword: the member it declared says what
    // it is.
    if let Some(ctor) = class_def
        .constructors
        .first()
        .filter(|c| c.is_declared_by_keyword(word))
    {
        let nparam = ctor.params.len();
        return Some(format!(
            "**{}** of `{qname}` ({nparam} param(s))",
            ctor.kind
        ));
    }
    if let Some(dtor) = class_def
        .destructor
        .as_ref()
        .filter(|d| d.is_declared_by_keyword(word))
    {
        return Some(format!("**{}** of `{qname}`", dtor.kind));
    }
    None
}

/// Hover text for a class member's **own declaration** name token, or
/// `None` when the cursor is not sitting on one.
///
/// Which member a name belongs to is
/// [`crate::references::resolve_member_span`] — the very same
/// method/classmethod/property disambiguation find-references uses, so
/// hover and references can never disagree about *which* declaration a
/// cursor is on when a name appears in more than one of a class's three
/// independent member tables.  That helper falls back to the first
/// candidate when the cursor is on no declaration at all (it is also used
/// from call sites), so the containment test is re-applied here: a
/// declaration hover must be anchored on the declaration.
///
/// The rendered summary is byte-identical to the call-site
/// (`my m` / `$obj m`) rendering, including the MRO note, so hovering a
/// method at its declaration and at a call site describe the same member
/// the same way.
fn member_declaration_hover_text(
    analysis: &AnalysisResult,
    class_def: &tcl_compiler::analyser::types::ClassDef,
    word: &str,
    cursor_offset: u32,
) -> Option<String> {
    use crate::references::MemberSel;
    let (kind, span) = crate::references::resolve_member_span(class_def, word, cursor_offset)?;
    if !(span.start() <= cursor_offset && cursor_offset <= span.end()) {
        return None;
    }
    let qname = &class_def.qualified_name;
    let note = |label: &str, m: &tcl_compiler::analyser::types::MethodDef| {
        let suffix = oo_method_resolution_note(analysis, qname, &m.name)
            .map_or(String::new(), |n| format!("  \n{n}"));
        format!(
            "**{label}** `{name}` ({nparam} param(s)){suffix}",
            name = tcl_syntax::naming::rooted_member_key(qname, &m.name),
            nparam = m.params.len(),
        )
    };
    match kind {
        MemberSel::Method => class_def.methods.get(word).map(|m| note("method", m)),
        MemberSel::ClassMethod => class_def
            .class_methods
            .get(word)
            .map(|m| note("classmethod", m)),
        MemberSel::Property => class_def.properties.get(word).map(|p| {
            format!(
                "**property** `{}`",
                tcl_syntax::naming::rooted_member_key(qname, &p.name)
            )
        }),
    }
}

/// The `$obj method` / `my method` dispatch arms of the hover walk, plus
/// the per-object visibility mask that precedes them.
///
/// Three-valued so the caller's tier order survives the extraction:
/// `Break(Some(_))` is a rendered method hover, `Break(None)` is a
/// **definitive no-hover** — a per-object mask (`oo::objdefine $o {
/// unexport m }`, or an unexported per-object member) makes `$obj m` answer
/// `unknown method` regardless of the class chain, never a
/// fall-through to a same-named proc or command — and `Continue(())` means
/// the cursor is not a method-dispatch site at all, so the remaining hover
/// tiers run.
pub(super) fn method_dispatch_hover(
    source: &str,
    line: u32,
    character: u32,
    cursor_offset: u32,
    analysis: &AnalysisResult,
) -> std::ops::ControlFlow<Option<Hover>> {
    use std::ops::ControlFlow;
    if let Some(selected) =
        crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)
    {
        let label = match selected.receiver {
            tcl_compiler::command_binding::SourceMethodReceiver::Instance => "method",
            tcl_compiler::command_binding::SourceMethodReceiver::Class => "classmethod",
        };
        return ControlFlow::Break(Some(Hover::markdown(format!(
            "**{label}** `{}::{}` ({} param(s))",
            selected.class.qualified_name,
            selected.method.name,
            selected.method.params.len(),
        ))));
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor_offset)
        .is_some()
        || crate::definition::object_dispatch_mask(analysis, source, line, character)
            .withholds_workspace_fallback()
    {
        return ControlFlow::Break(None);
    }

    ControlFlow::Continue(())
}

/// The one-line method summary for a dispatch whose **provider was resolved
/// elsewhere** — the server's workspace tier, walking the same C-faithful
/// linearisation `definition.rs`'s cross-file method tier walks.
///
/// [`method_dispatch_hover`] can only answer when the whole class chain is
/// visible in the requesting document's own analysis. The real corpus shape
/// is the opposite: `my ArgsPreprocess` in one file, the `mixin`/`superclass`
/// that provides `ArgsPreprocess` in another (the `SpiceGenTcl` `RModel` /
/// `Utility` split). Go-to-definition and
/// find-references cross that boundary through the workspace index;
/// `hover.rs` has no cross-document tier of its
/// own. Rather than grow one here — the index and the document store live in
/// the server — the server resolves the provider and asks this function to
/// render it, so both tiers emit the identical heading and MRO note.
///
/// `receiver_class_q` is the class the call dispatches *on*; `provider_q` the
/// class the chain landed on. They differ exactly when the member is
/// inherited, which is the note this renders — there is no `next`-chain
/// lookup to do here, because the chain walk that produced `provider_q`
/// already applied the visibility rule and the instance/class-side split.
#[must_use]
pub fn cross_document_method_hover(
    receiver_class_q: &str,
    provider_q: &str,
    method_name: &str,
    nparams: usize,
    is_classmethod: bool,
) -> Hover {
    let label = if is_classmethod {
        "classmethod"
    } else {
        "method"
    };
    let note = if provider_q == receiver_class_q {
        String::new()
    } else {
        format!("  \n_inherited from `{provider_q}`_")
    };
    Hover::markdown(format!(
        "**{label}** `{provider_q}::{method_name}` ({nparams} param(s)){note}"
    ))
}

/// MRO note for `method` on `class_q`: `inherited from ::Provider` when the
/// method resolves to an ancestor, or `overrides ::Super::method` when it
/// is defined on `class_q` but a superclass also provides it.  `None` for a
/// method defined only here (no note needed) or an unresolvable one.
pub(super) fn oo_method_resolution_note(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<String> {
    let provider = analysis.class_hierarchy().method_target(class_q, method)?;
    oo_resolution_note_for_provider(analysis, class_q, provider, method)
}

/// Render the inheritance or override note for the provider selected by the
/// caller. The note uses that exact provider rather than resolving it again.
fn oo_resolution_note_for_provider(
    analysis: &AnalysisResult,
    class_q: &str,
    provider: &str,
    method: &str,
) -> Option<String> {
    if provider == class_q {
        // Defined here — does a superclass further down the MRO also
        // provide it (i.e. this is an override)?
        analysis
            .class_hierarchy()
            .next_provider(class_q, method, class_q, None)
            .map(|sup| {
                format!(
                    "_overrides `{}`_",
                    tcl_syntax::naming::rooted_member_key(sup, method)
                )
            })
    } else {
        Some(format!("_inherited from `{provider}`_"))
    }
}
