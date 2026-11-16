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

//! Renaming class members: method targets and access, override families, safety hazards and the spans to rewrite.

use super::{PendingRenameEdit, PrepareRename, TextEdit, finish_rename_edits, span_to_range};
use crate::hover::find_word_span_at_position;
use crate::references::{MemberSel, resolve_member_span};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

/// The other declarations of the instance member family selected at the
/// cursor, which an original member rename edits together.
pub(super) fn member_family_spans(
    source: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    (line, character): (u32, u32),
) -> Vec<tcl_lexer::Span> {
    if analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
    let std::ops::ControlFlow::Break(Some(identity)) =
        crate::original_declaration::select_at_offset("", source, analysis, cursor)
    else {
        return Vec::new();
    };
    crate::original_member_rename::original_member_family_spans(
        crate::original_declaration::OriginalDeclarationDocument {
            uri: "",
            source,
            analysis,
        },
        &identity,
    )
    .into_iter()
    .filter(|span| *span != identity.span())
    .collect()
}

pub(super) fn prepare_list_built_self_method_rename(
    source: &str,
    _line: u32,
    _word_range: (u32, u32),
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<PrepareRename> {
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)?;
    if selected.method.name != word {
        return None;
    }
    let selector = selected.editable_selector()?;
    Some(PrepareRename {
        range: span_to_range(source, &LineIndex::new(source), selector),
        placeholder: selected.method.name.clone(),
    })
}

pub(super) fn rename_list_built_self_method(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    names: (&str, &str),
    cursor_offset: u32,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (word, new_name) = names;
    let (provider, is_classmethod) = crate::references::list_built_self_method_target_at_cursor(
        source,
        dialect,
        analysis,
        word,
        cursor_offset,
    )?;
    rename_method_in_class(
        source,
        dialect,
        (&provider, word, is_classmethod),
        new_name,
        analysis,
        line_index,
    )
}

/// The **untargeted** member-rename tier, behind the safety gate — the last
/// tier [`rename_with_diagnosis`] tries, and the one that answers every
/// cursor position no trusted dispatch tier claims.
///
/// `Err` is a refusal, `Ok(None)` means the cursor names no member here.
///
/// The gate runs *here* rather than in [`pre_edit_refusal`] so the tier order
/// is preserved exactly: a cursor an earlier tier claims (a local variable, a
/// proc, a class) is renamed by that tier and never consulted about a
/// same-spelled member. And it runs at all because `pre_edit_refusal` can
/// only speak for a cursor [`method_rename_target`] resolves — the member's
/// own declaration name, a `my` dispatch, a tracked `$obj` dispatch. The
/// positions this tier owns (the method word inside an untracked `[$other
/// X]`, the bareword in `export … X …`) are precisely the ones from which the
/// hazard emits a declaration-only edit set that breaks the
/// program once applied. Asking the same question about the same member here
/// makes the verdict independent of *which occurrence* the rename was
/// triggered from.
pub(super) fn gated_untargeted_member_rename(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    (word, new_name): (&str, &str),
    analysis: &AnalysisResult,
    cursor_offset: u32,
    line_index: &LineIndex,
) -> Result<Option<Vec<TextEdit>>, crate::rename_safety::RenameRefusal> {
    if let Some((seed_class, method, is_classmethod)) =
        untargeted_member_rename_target(analysis, word, cursor_offset)
        && let Some(refusal) = member_rename_hazard(
            source,
            dialect,
            analysis,
            (&seed_class, &method, is_classmethod),
            new_name,
            line_index,
        )
    {
        return Err(refusal);
    }
    rename_method(
        source,
        dialect,
        word,
        new_name,
        analysis,
        cursor_offset,
        line_index,
    )
    .transpose()
}

/// The safety refusal for a `TclOO` member rename anchored at the cursor, if
/// any — the single in-document entry point to [`crate::rename_safety`].
///
/// Resolves the cursor to a member and its whole override family exactly the
/// way the rename itself does ([`method_rename_target`] + [`override_family`]),
/// so the gate is asked about precisely the declarations the edit set would
/// rewrite — never a wider or narrower set.
pub(super) fn method_rename_refusal(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    let (seed_class, method, is_classmethod) =
        method_rename_target(source, line, character, analysis)?;
    member_rename_hazard(
        source,
        dialect,
        analysis,
        (&seed_class, &method, is_classmethod),
        new_name,
        line_index,
    )
}

/// The safety verdict for renaming `method` on `seed_class`'s whole override
/// family — the one call both member-rename gates make, so a hazard is a
/// property of *which member the rename rewrites*, never of how the cursor
/// happened to name it.
///
/// Split out of [`method_rename_refusal`] because that entry point can only
/// speak for cursors a *trusted dispatch tier* resolves
/// ([`method_rename_target`]: the member's own declaration name, a `my`
/// dispatch, a tracked `$obj` / class-command dispatch). Every other position
/// that still reaches a member — the untracked call site the gate exists to
/// protect, an `export`/`unexport` bareword — is resolved by
/// [`untargeted_member_rename_target`] instead and asked the same question
/// here.
fn member_rename_hazard(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    (seed_class, method, is_classmethod): (&str, &str, bool),
    new_name: &str,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    let family = override_family(analysis, seed_class, method);
    if family.is_empty() {
        return None;
    }
    crate::rename_safety::method_rename_hazard(
        source,
        dialect,
        analysis,
        crate::rename_safety::MethodRenameTarget {
            family: &family,
            method,
            is_classmethod,
            new_name,
        },
        line_index,
    )
}

/// The member the **untargeted** member-rename tier ([`rename_method`]) would
/// rewrite for a cursor at `cursor_offset`, or `None` when that tier declines
/// the cursor.
///
/// This is deliberately the *same* resolution `rename_method` performs —
/// "which class body encloses the cursor, and does `word` name one of its
/// members" ([`crate::definition::enclosing_class_at`] +
/// [`resolve_member_span`]) — and not a re-derivation. That tier answers
/// exactly the cursor positions no trusted dispatch tier claims: the method
/// name inside `[$untracked X]`, and the bareword in an `export … X …` list.
/// Without the check they would reach `rename_method` ungated, so the very
/// shape [`crate::rename_safety`] refuses when the cursor sits on the
/// declaration would silently emit a declaration-only edit set from one line
/// away — applying that and running it under tclsh 9.0.4 / 8.6.16 gives
/// `unknown method "X"` at the untouched dispatch.
///
/// Restricted to `method` / `classmethod`: a *property* has no dispatch
/// (see [`rename_method`]'s own property branch), so it has no dispatch
/// hazard to check.
#[must_use]
pub fn untargeted_member_rename_target(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(String, String, bool)> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let class_q = crate::definition::enclosing_class_at(analysis, cursor_offset)?;
    let class_def = analysis.all_classes.get(class_q)?;
    let (kind, _span) = resolve_member_span(class_def, word, cursor_offset)?;
    match kind {
        MemberSel::Method => Some((class_def.qualified_name.clone(), word.to_owned(), false)),
        MemberSel::ClassMethod => Some((class_def.qualified_name.clone(), word.to_owned(), true)),
        MemberSel::Property => None,
    }
}

/// Rename `method` across the class identified by `class_q` **and every
/// other class in its override family** — rewriting each class's
/// declaration name span, its intra-class `my method` call sites, and its
/// external `$obj method` call sites.  Returns `None` when neither
/// `class_q` nor any ancestor provides `method` (nothing to rename).
///
/// `target` is `(class_q, method, is_classmethod)` — bundled to keep the
/// parameter count under the lint budget; `is_classmethod` reflects which
/// [`crate::definition::MethodBucket`] the caller's own receiver resolved
/// to (`$obj method` vs. a bare `ClassName method` classmethod dispatch),
/// not something re-derivable from `class_q`/`method` alone.
///
/// A `TclOO` method that is (re)defined by a super- or sub-class is a
/// single polymorphic name: `$obj method` dispatch can reach any
/// definition along the chain, so renaming only the class under the
/// cursor would silently break the override relationship.  See
/// [`override_family`].  Shared by the in-class-body and external
/// `$obj method` rename entry points.
/// Rename an [incr Tcl] class-scoped `proc` from a `Factory::make` call site
/// or its declaration.
///
/// Tried after the proc / class rename paths so an ordinary qualified proc
/// call of the same spelling keeps priority.  The edits come from the shared
/// member-reference collector, whose itcl call sites cover only the final
/// `::`-segment — so the as-written qualifier survives the rewrite.
///
/// Single-document: a `Factory::make` call in a sibling file is not
/// rewritten, because the cross-file rename layer still carries only the
/// two-word `Class method` dispatch shape.
pub(super) fn rename_itcl_class_proc(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    cursor: (u32, u32),
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (line, character) = cursor;
    let (class_q, member) =
        crate::definition::itcl_class_proc_target_at(source, dialect, line, character, analysis)?;
    rename_method_in_class(
        source,
        dialect,
        (&class_q, &member, true),
        new_name,
        analysis,
        line_index,
    )
}

pub(super) fn rename_method_in_class(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    target: (&str, &str, bool),
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (class_q, method, is_classmethod) = target;
    let family = override_family(analysis, class_q, method);
    if family.is_empty() {
        return None;
    }
    let mut edits = Vec::new();
    for member in &family {
        // Reached via an external call site — either `$obj method` (always
        // instance-context) or a bare `ClassName method` classmethod
        // dispatch — so `is_classmethod` is fixed by
        // the caller's own receiver resolution, never re-derived per family
        // member (a method and a classmethod occupy separate dispatch
        // tables; the receiver picks exactly one for the whole rename).
        let Some((decl_span, call_spans)) = crate::references::method_references_for_class(
            source,
            dialect,
            analysis,
            member,
            method,
            is_classmethod,
        ) else {
            continue;
        };
        edits.push(PendingRenameEdit {
            span: decl_span,
            new_text: new_name.to_owned(),
        });
        for span in call_spans {
            edits.push(PendingRenameEdit {
                span,
                new_text: new_name.to_owned(),
            });
        }
    }
    if edits.is_empty() {
        return None;
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// When the cursor sits on a renameable `TclOO` **method**, return the
/// `(class_qualified_name, method_name)` it targets — the seed the server
/// uses to compute the cross-file override family.  Covers both entry
/// points the in-document rename recognises: an external `$obj method`
/// call site (class resolved via `instance_classes`) and a cursor inside a
/// class body on a method / classmethod name (declaration or `my method`
/// call).  Returns `None` for non-method targets (vars, procs, classes,
/// properties), which stay on the single-document rename path.
#[must_use]
pub fn method_rename_target(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String, bool)> {
    method_target_with_access(source, line, character, analysis)
        .map(|(c, m, is_cm, _)| (c, m, is_cm))
}

/// Like [`method_rename_target`] but also reports the **access context**
/// of the cursor's call shape: an external
/// `$obj m` dispatches through exported methods only, while a `my m`
/// call or a declaration-side cursor inside the class body reaches
/// unexported methods too.
#[must_use]
pub fn method_target_with_access(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String, bool, crate::workspace_index::MethodAccess)> {
    method_target_with_access_in_workspace(source, line, character, analysis, None)
}

fn callback_prefix_target_in_workspace(
    source: &str,
    analysis: &AnalysisResult,
    word: &str,
    cursor: u32,
    index: Option<(&crate::workspace_index::WorkspaceIndex, &str)>,
) -> Option<(String, String, bool, crate::workspace_index::MethodAccess)> {
    use crate::workspace_index::MethodAccess;

    let (receiver, is_classmethod, external) =
        crate::references::callback_prefix_method_receiver_at_cursor(
            source,
            crate::profile_for_analysis(analysis),
            analysis,
            word,
            cursor,
        )?;
    let (workspace, _uri) = index?;
    let access = if external {
        MethodAccess::External
    } else {
        MethodAccess::Internal
    };
    // The chain is the *gate*, not the answer: an empty one means this
    // receiver cannot dispatch the member at all (a `[self]` capture of a name
    // the receiver has unexported), so the position resolves to nothing.  What
    // is returned is the **receiver**, exactly as the bare `$obj method` arms
    // below return theirs — the family, definition and rename tiers each
    // resolve the provider from it through this same chain, and handing them a
    // provider instead would drop the receiver's own `export` / `unexport`
    // stub on the way: when `Base` unexports `tock` and `Child` exports the
    // inherited name, only a chain rooted at `Child` still reaches `Base`'s
    // body.
    let chain = if is_classmethod {
        workspace.class_method_dispatch_chain(&receiver, word, access)
    } else {
        workspace.method_dispatch_chain(&receiver, word, access)
    };
    chain.first()?;
    Some((receiver, word.to_owned(), is_classmethod, access))
}

/// [`method_target_with_access`] with the **workspace tier** available for
/// receiver classification.
///
/// The local analysis classifies every receiver this document knows about. A
/// *pure consumer* file — one holding only `C cm`, declaring no part of `C` —
/// knows none, and the server's workspace-class oracle reanalysis does not
/// close that gap: it supplies class **names** only, never their member
/// tables. So a class-side call from an ordinary consumer fell through both
/// receiver branches and definition / references / rename all answer nothing.
///
/// Passing `index` lets the classmethod arm ask the workspace what it cannot
/// see locally, through the same dispatch chain the resolvers use. The
/// instance arm needs no equivalent: it already crosses files, because the
/// analyser's own class oracle is enough to type `[Cls new]` and populate
/// `instance_classes`.
#[must_use]
pub fn method_target_with_access_in_workspace(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    index: Option<(&crate::workspace_index::WorkspaceIndex, &str)>,
) -> Option<(String, String, bool, crate::workspace_index::MethodAccess)> {
    use crate::workspace_index::MethodAccess;
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let line_index = LineIndex::new(source);
    let (word, _s, _e) = find_word_span_at_position(source, line, character)?;
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    // Only the retained single-field link form couples command and member edits.
    if let Some(linked) =
        tcl_compiler::analyser::types::original_source_linked_member_at(analysis, source, cursor)
        && linked.has_coupled_name()
        && linked.editable_target_span(source).is_some()
        && linked.alias_input().bytes() == word.as_bytes()
        && linked.target_input().bytes() == word.as_bytes()
    {
        return Some((
            linked.class_declaration().metadata().qualified_name.clone(),
            linked.target().metadata().name.clone(),
            linked.target().side() == tcl_compiler::analyser::MemberSide::ClassObject,
            MethodAccess::Internal,
        ));
    }
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor).is_some()
    {
        let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor)?;
        let current = analysis.all_classes.get(&selected.class.qualified_name)?;
        return (current.name_span == selected.class.name_span
            && selected.editable_selector().is_some())
        .then(|| {
            (
                selected.class.qualified_name.clone(),
                selected.method.name.clone(),
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
                MethodAccess::Internal,
            )
        });
    }
    if let Some(target) =
        callback_prefix_target_in_workspace(source, analysis, &word, cursor, index)
    {
        return Some(target);
    }
    if let Some((provider, is_classmethod)) =
        crate::references::list_built_self_method_target_at_cursor(
            source,
            crate::profile_for_analysis(analysis),
            analysis,
            &word,
            cursor,
        )
    {
        return Some((provider, word, is_classmethod, MethodAccess::External));
    }
    if let Some((_, method, _)) =
        crate::definition::original_instance_method_at_cursor(source, analysis, line, character)
        && method == word
    {
        if let Some(selected) = crate::receiver_identity::method_at_cursor(analysis, source, cursor)
        {
            return Some((
                selected.class.qualified_name.clone(),
                selected.method.name.clone(),
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
                MethodAccess::External,
            ));
        }
        return None;
    }
    // Inside a class body on one of its method / classmethod names — the
    // declaration side, an internal context.
    if let Some(class_def) = crate::definition::enclosing_class_at(analysis, cursor)
        .and_then(|q| analysis.all_classes.get(q))
    {
        let method_decl = class_def.methods.get(&word);
        let classmethod_decl = class_def.class_methods.get(&word);
        let on_declaration = |m: &tcl_compiler::analyser::MethodDef| {
            m.name_span.start() <= cursor && cursor < m.name_span.end()
        };
        let cursor_on_method_decl = method_decl.is_some_and(on_declaration);
        let cursor_on_classmethod_decl = classmethod_decl.is_some_and(on_declaration);
        if (method_decl.is_some() || classmethod_decl.is_some())
            && (cursor_on_method_decl || cursor_on_classmethod_decl)
        {
            // A `method` and a `classmethod` of the same name are distinct
            // members; unambiguous when only one exists, otherwise prefer
            // whichever one's declaration the cursor is actually on.
            let is_classmethod =
                classmethod_decl.is_some() && (method_decl.is_none() || cursor_on_classmethod_decl);
            return Some((
                class_def.qualified_name.clone(),
                word,
                is_classmethod,
                MethodAccess::Internal,
            ));
        }
    }
    None
}

/// Every span (declaration + intra-class `my method` calls + external
/// `$obj method` sites) that renaming `method` on `class_q` must rewrite
/// **within `source`**.  Empty when `class_q` is not defined in this
/// document.  The server calls this per family-member document to assemble
/// a cross-file method rename.
#[must_use]
pub fn method_spans_in_document(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    match crate::references::method_references_for_class(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
    ) {
        Some((decl, calls)) => {
            let mut spans = Vec::with_capacity(1 + calls.len());
            spans.push(decl);
            spans.extend(calls);
            spans
        }
        None => Vec::new(),
    }
}

/// Every span that renaming `method` must rewrite **within `source`** for a
/// class `class_q` that *inherits* `method` (does not declare it): its
/// intra-class `my method` calls and external `$obj method` sites, with no
/// declaration span.  Empty when `class_q` is not defined in this document.
///
/// The server calls this for documents that contain a purely-inheriting
/// subclass of a rename family member but no definer of their own — the
/// cross-file complement to [`method_spans_in_document`], so an
/// inherited-method rename reaches `my method` / `$obj method` sites that
/// live in a subclass-only file.
///
/// `workspace` carries the two facts a subclass-only document cannot derive
/// from its own (definer-less) `all_classes` — see
/// [`crate::references::InheritedReceiverFacts`].
#[must_use]
pub fn inherited_method_spans_in_document(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    workspace: crate::references::InheritedReceiverFacts<'_>,
) -> Vec<tcl_lexer::Span> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    crate::references::inherited_method_call_sites(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        workspace,
    )
}

/// The set of classes whose definition of `method` must be renamed
/// together with `seed_class`'s: every class that **directly defines**
/// `method` and sits in the same override-connected component as
/// `seed_class` (or the class that provides `method` to it when
/// `seed_class` only inherits it).
///
/// Two definitions belong to one component when their classes are related
/// by the (mixin-aware) subtype relation — directly, or transitively via
/// another definer.  So a base method plus every subclass override, and
/// sibling overrides of a common base, all rename as one unit; unrelated
/// same-named methods in disjoint hierarchies stay separate (never
/// over-renamed).  The returned vector always contains the seed and is
/// empty only when `method` is neither defined nor inherited by
/// `seed_class`.
fn override_family(analysis: &AnalysisResult, seed_class: &str, method: &str) -> Vec<String> {
    let hierarchy = analysis.class_hierarchy();
    // Classes that *directly* define `method` (own body, any visibility) —
    // constructors/destructors aren't in `methods`/`class_methods` so are
    // naturally excluded.
    let definers: Vec<&String> = analysis
        .all_classes
        .iter()
        .filter(|(_, cd)| cd.methods.contains_key(method) || cd.class_methods.contains_key(method))
        .map(|(q, _)| q)
        .collect();
    // Seed: the class under the cursor when it defines `method`, else the
    // class that provides it (an ancestor, via the MRO).  When neither, the
    // family is empty.
    let seed = if definers.iter().any(|d| d.as_str() == seed_class) {
        seed_class.to_string()
    } else if let Some(provider) = hierarchy.method_target(seed_class, method) {
        provider.to_string()
    } else {
        return Vec::new();
    };
    let connected =
        |a: &str, b: &str| a == b || hierarchy.is_subtype(a, b) || hierarchy.is_subtype(b, a);
    // Grow the weakly-connected component of definers containing `seed` to a
    // fixed point (siblings attach via a shared base already in the family).
    let mut family = vec![seed];
    let mut changed = true;
    while changed {
        changed = false;
        for d in &definers {
            if family.iter().any(|f| f == d.as_str()) {
                continue;
            }
            if family.iter().any(|f| connected(f, d)) {
                family.push((*d).clone());
                changed = true;
            }
        }
    }
    family
}

/// Method / property rename — locate the class whose body
/// contains the cursor, match `word` against its members,
/// then rewrite the declaration's name span plus every call
/// site inside any sibling method body that targets the same
/// name.
///
/// The analyser's `command_invocations` collection records
/// invocations at the top level only; method bodies aren't
/// walked.  To find call sites we re-segment each sibling
/// method body via [`tcl_compiler::segmenter::segment_commands_with_offset`]
/// and match on the segmented command's head token.
///
/// Returns `Some(edits)` when a class member matched `word`,
/// `None` otherwise.  `cursor_offset` is the byte offset for
/// containment filtering.
pub(super) fn rename_method(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    word: &str,
    new_name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    use tcl_lexer::Span;

    let class_def = analysis
        .all_classes
        .get(crate::definition::enclosing_class_at(
            analysis,
            cursor_offset,
        )?)?;
    // Methods, classmethods, and properties are independent tables; a
    // name shared by more than one (rare, but real — `TclOO` never
    // merges them) disambiguates by which declaration's own span the
    // cursor sits on rather than a fixed priority — see
    // `resolve_member_span`.
    let (selected_kind, name_span) = resolve_member_span(class_def, word, cursor_offset)?;
    let is_classmethod = selected_kind == MemberSel::ClassMethod;
    let mut edits = vec![PendingRenameEdit {
        span: name_span,
        new_text: new_name.to_owned(),
    }];
    // `command_invocations` records top-level invocations only; method
    // bodies aren't walked there.  `body_spans` (every method /
    // classmethod / constructor / destructor body) is the re-segmentable
    // material both branches below scan via the shared `my`-aware
    // matcher — a `TclOO` member is never a bare-callable command (a
    // bare `word` errors "invalid command name" at runtime; only `my
    // word` dispatches), so there is no separate bare-head scan here.
    let body_spans: Vec<Span> = crate::references::collect_member_bodies(class_def);
    // Methods / classmethods (not properties — those aren't dispatched
    // through the MRO) are one polymorphic name across the whole override
    // family: a method (re)defined by a super- or sub-class renames as a
    // unit.  For every family member — **including the class under the
    // cursor** — pull its declaration, intra-class `my method` sites (its
    // own bodies *and* any purely-inheriting subclass's), and external
    // `$obj method` sites via the shared resolver.  Routing the cursor
    // class through it too (rather than an ad-hoc self-only scan) is what
    // catches an inheriting subclass's `my method` / `$obj method` sites
    // when renaming from the base declaration.
    if matches!(selected_kind, MemberSel::Method | MemberSel::ClassMethod) {
        for member in override_family(analysis, &class_def.qualified_name, word) {
            let Some((decl_span, call_spans)) = crate::references::method_references_for_class(
                source,
                dialect,
                analysis,
                &member,
                word,
                is_classmethod,
            ) else {
                continue;
            };
            // The cursor class's own declaration edit is already queued.
            if member != class_def.qualified_name {
                edits.push(PendingRenameEdit {
                    span: decl_span,
                    new_text: new_name.to_owned(),
                });
            }
            for span in call_spans {
                edits.push(PendingRenameEdit {
                    span,
                    new_text: new_name.to_owned(),
                });
            }
        }
    } else {
        // Properties have no `$obj prop` dispatch and no inheritance model
        // (see `references::find_class_member_references`), so a
        // class-local `my <prop>` scan is the whole story.
        for span in
            crate::references::scan_my_method_sites(source, analysis, &body_spans, word, None)
        {
            edits.push(PendingRenameEdit {
                span,
                new_text: new_name.to_owned(),
            });
        }
    }
    Some(finish_rename_edits(source, line_index, edits))
}
