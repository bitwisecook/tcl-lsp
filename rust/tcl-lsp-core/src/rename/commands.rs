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

//! Renaming procedures and classes: declarations, call sites, ensemble dispatch words and cross-document edits.

use super::variables::source_name_qualifier;
use super::{
    PendingRenameEdit, PrepareRename, TextEdit, WorkspaceTextEdit, finish_rename_edits,
    is_builtin_command_name, is_safe_symbol_name, namespace_prefix_of, span_to_range,
};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

pub(super) fn prepare_named_command_rename(
    source: &str,
    line_index: &LineIndex,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    target: (&str, u32),
) -> Option<PrepareRename> {
    let (word, cursor_offset) = target;
    if let Some((_, proc_def)) =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_offset, word, resolution)
    {
        return Some(PrepareRename {
            range: span_to_range(source, line_index, proc_def.name_span),
            placeholder: proc_def.name.clone(),
        });
    }
    let (_, class_def) = crate::definition::resolve_class_target_at(
        analysis,
        source,
        resolution,
        cursor_offset,
        word,
    )?;
    Some(PrepareRename {
        range: span_to_range(source, line_index, class_def.name_span),
        placeholder: class_def.name.clone(),
    })
}

pub(super) fn rename_named_command(
    source: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
    names: (&str, &str),
    cursor_offset: u32,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (word, new_name) = names;
    rename_proc(
        source,
        word,
        cursor_offset,
        new_name,
        analysis,
        resolution,
        line_index,
    )
    .or_else(|| {
        rename_class(
            source,
            word,
            cursor_offset,
            new_name,
            analysis,
            resolution,
            line_index,
        )
    })
}

/// Whether this call site is an ensemble **dispatch word whose spelling the
/// rename must not touch** — the `show` of `::app::widget show` when the
/// ensemble bound it with `-map {show ::app::widget::Show}`.
///
/// A `-map` key is an arbitrary name with no required relationship to the
/// target it dispatches to (tclsh 8.6.14 / 9.0.4: with that map,
/// `::app::widget show` runs the proc and `::app::widget Show` is `unknown or
/// ambiguous subcommand "Show": must be show`).  So renaming
/// `::app::widget::Show` → `Display` must rewrite the declaration and the
/// `-map` *value* — a separate reference this same loop rewrites, because its
/// span really does carry the target's name — while leaving the dispatch word
/// alone.  Rewriting it is the corruption the issue reports: the map still
/// says `show`, and the edited call names a subcommand that does not exist.
///
/// A `-subcommands {alpha}` entry is the opposite case and is deliberately
/// **not** gated: there the subcommand word *is* the target's tail, the
/// ensemble derives `::app::widget::alpha` from it, and leaving it behind
/// gives `invalid command name "alpha"` — so it must be rewritten with the
/// declaration, which is what the ungated path already does.
///
/// This reads the per-site provenance the analyser recorded, not a name
/// comparison against `ensemble_subcommand_targets`: a `-map {Show
/// ::app::widget::Show}` whose key happens to equal the target's tail is
/// textually indistinguishable from an ordinary bare call to the target, and
/// only the recording site knows which one this span is.
///
/// Find-references, code-lens counts and call-hierarchy are untouched by this
/// — a dispatch site is a genuine reference under either provenance, and the
/// gate lives here (in the two rename loops) rather than in
/// [`crate::references::invocation_references_proc`], which all of them
/// share.
#[must_use]
fn ensemble_map_dispatch_word(
    ensemble_dispatch: Option<tcl_compiler::signature_scan::types::EnsembleSubcommandProvenance>,
) -> bool {
    matches!(
        ensemble_dispatch,
        Some(tcl_compiler::signature_scan::types::EnsembleSubcommandProvenance::Map)
    )
}

/// Proc-rename path — declaration name span + every matching
/// call site.  Namespace-aware: the declaration keeps its
/// prefix; call sites pick the rewrite that matches the form
/// the source wrote (qualified ↔ short).  `Some(edits)` when
/// a proc matched `word` (even if the edit set ended up empty
/// from a safety gate); `None` when no proc matched.
pub(super) fn rename_proc(
    source: &str,
    word: &str,
    cursor_off: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let registry = resolution.registry;
    // Resolve the target the same way go-to-definition does: the proc whose
    // declaration covers the cursor, else C Tcl's namespace-aware call-site
    // resolution — never a namespace-blind `p.name == word` scan, which let a
    // rename from a bareword call site pick an arbitrary same-named proc in
    // another namespace and rewrite the wrong definition (matches
    // `references::references`).
    let (qname, proc_def) =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_off, word, resolution)?;
    if let Some(registry) = registry
        && is_builtin_command_name(new_name, registry)
    {
        return Some(Ok(Vec::new()));
    }
    let namespace_prefix = namespace_prefix_of(&proc_def.qualified_name);
    let (new_qualified, qualified_decl) = qualified_and_decl_text(namespace_prefix, new_name);
    // The declaration rewrite follows the *form the source wrote* — a proc
    // declared short inside a `namespace eval` (`proc helper`) stays short
    // (`assist`); one declared qualified (`proc ::ns::greet`) stays qualified.
    let decl_was_qualified = source
        .get(proc_def.name_span.start() as usize..proc_def.name_span.end() as usize)
        .is_some_and(|t| tcl_syntax::naming::is_qualified(t.as_bytes()));
    let new_decl_text = if decl_was_qualified {
        qualified_decl
    } else {
        new_name.to_owned()
    };
    // Collision gate: renaming onto an existing proc of the same qualified
    // name would shadow it — refuse.  Compare names normalised to the
    // leading-`::` form.
    if analysis
        .all_procs
        .keys()
        .any(|qn| qn != &proc_def.qualified_name && qn == &new_qualified)
    {
        return Some(Ok(Vec::new()));
    }
    // Provenance gate: an indirect dispatch of this
    // proc whose contributing constants are not all source-writable cannot
    // be kept alive by any edit set — refuse the whole rename rather than
    // emit edits that leave the dispatch running the old name.
    if analysis.command_invocations.iter().any(|inv| {
        !inv.rename_safe
            && crate::references::invocation_references_proc(analysis, inv, qname, proc_def, source)
    }) {
        return Some(Ok(Vec::new()));
    }
    let mut edits = vec![PendingRenameEdit {
        span: proc_def.name_span,
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        // An indirect site's span does not carry the written name (a constant
        // `$cmd` head) — rewriting it would splice the new name over
        // unrelated text.  References still report it; rename must not.
        if inv.indirect {
            continue;
        }
        // A `-map` key is an arbitrary name the target's rename does not
        // change — see [`ensemble_map_dispatch_word`].
        if ensemble_map_dispatch_word(inv.ensemble_dispatch) {
            continue;
        }
        // Use the *same* invocation-matching rule as Find-All-References so a
        // rename never rewrites a call the reference finder wouldn't report —
        // in particular the namespace gate that keeps a bare `helper` call in
        // `namespace eval ::b` from matching `::a::helper`.
        if !crate::references::invocation_references_proc(analysis, inv, qname, proc_def, source) {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(PendingRenameEdit {
            span: inv.range,
            new_text: replacement,
        });
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// Class-rename path — `oo::class create ClassName`
/// declaration plus every `ClassName new` / `ClassName create
/// instance` invocation head.  Same shape as [`rename_proc`]:
/// prefers the class whose declaration name span covers the cursor (so
/// renaming from a same-named class's own decl in another namespace
/// resolves to *that* class, not the first one found), else the first
/// class matching the word.
pub(super) fn rename_class(
    source: &str,
    word: &str,
    cursor_off: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let registry = resolution.registry;
    // Declaration under the cursor, else namespace-aware resolution — never a
    // namespace-blind `c.name == word` scan (which from a call site could
    // rename the wrong same-named class in another namespace).
    let (qname, class_def) =
        crate::definition::resolve_class_target_at(analysis, source, resolution, cursor_off, word)?;
    if let Some(registry) = registry
        && is_builtin_command_name(new_name, registry)
    {
        return Some(Ok(Vec::new()));
    }
    let namespace_prefix = namespace_prefix_of(&class_def.qualified_name);
    let (new_qualified, qualified_decl) = qualified_and_decl_text(namespace_prefix, new_name);
    // The declaration rewrite follows the *form the source wrote* — a class
    // declared short inside a `namespace eval` (`oo::class create Widget`)
    // stays short (`Panel`), one declared qualified
    // (`oo::class create ::ns::Widget`) stays qualified. Mirrors
    // `rename_proc`'s identical `decl_was_qualified` handling.
    let decl_was_qualified = source
        .get(class_def.name_span.start() as usize..class_def.name_span.end() as usize)
        .is_some_and(|t| tcl_syntax::naming::is_qualified(t.as_bytes()));
    let new_decl_text = if decl_was_qualified {
        qualified_decl
    } else {
        new_name.to_owned()
    };
    // Provenance gate — mirrors `rename_proc`: an
    // indirect dispatch with unwritable contributing constants cannot
    // follow the rename, so refuse it outright.
    if analysis.command_invocations.iter().any(|inv| {
        !inv.rename_safe
            && crate::references::invocation_references_class(
                analysis, inv, qname, class_def, source,
            )
    }) {
        return Some(Ok(Vec::new()));
    }
    let mut edits = vec![PendingRenameEdit {
        span: class_def.name_span,
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        // Indirect sites are references, never rename targets — the span
        // does not carry the written name.
        if inv.indirect {
            continue;
        }
        // A `-map` key is an arbitrary name the target's rename does not
        // change — see [`ensemble_map_dispatch_word`].
        if ensemble_map_dispatch_word(inv.ensemble_dispatch) {
            continue;
        }
        // Use the *same* invocation-matching rule as Find-All-References so a
        // rename never rewrites a call the reference finder wouldn't report —
        // in particular the namespace gate that keeps a bare `ClassName new`
        // call in a *different* namespace from being rewritten.
        if !crate::references::invocation_references_class(analysis, inv, qname, class_def, source)
        {
            continue;
        }
        // Skip an invocation whose range contains the class
        // declaration site (defence in depth — the analyser's
        // recording shape doesn't usually file this here).
        if inv.range.start() <= class_def.name_span.start()
            && class_def.name_span.end() <= inv.range.end()
        {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(PendingRenameEdit {
            span: inv.range,
            new_text: replacement,
        });
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// Compute the qualified rewrite (`prefix::new`) and the
/// declaration-site rewrite (qualified when the original lived
/// in a namespace, short at the top level).  Shared by the
/// proc and class rename paths.
pub(super) fn qualified_and_decl_text(namespace_prefix: &str, new_name: &str) -> (String, String) {
    let new_qualified = tcl_syntax::naming::qualify(namespace_prefix, new_name);
    let new_decl_text = if namespace_prefix.is_empty() {
        new_name.to_owned()
    } else {
        new_qualified.clone()
    };
    (new_qualified, new_decl_text)
}

/// Pick the rewrite shape for a single call/invocation site —
/// short form at the top level, qualified form when the call
/// itself was qualified.  Shared by the proc and class rename
/// paths.
pub(super) fn invocation_replacement(
    _namespace_prefix: &str,
    _new_qualified: &str,
    new_name: &str,
    inv_name: &str,
) -> String {
    // Preserve the call's *as-written* qualifier: a qualified call replaces
    // only its final `::`-segment (`utils::helper` → `utils::assist`,
    // `::myns::greet` → `::myns::hello`); a short call becomes the short name.
    format!("{}{new_name}", source_name_qualifier(inv_name))
}

/// Compute the cross-document rename edits for a proc / class
/// identified by `(simple_name, qualified_name)` renamed to
/// `new_name`.  Walks the workspace index for every invocation
/// site (and definition site) in documents *other than*
/// `current_uri`, applying the same namespace-aware
/// replacement shape the in-document proc rename uses.
///
/// The caller (server) is responsible for the current document
/// — this returns only the sibling-document edits — and for
/// resolving each [`WorkspaceTextEdit`] span to an LSP range
/// against the target document's source.
#[must_use]
pub fn cross_document_symbol_edits(
    qualified_name: &str,
    new_name: &str,
    index: &crate::workspace_index::WorkspaceIndex,
    current_uri: &str,
) -> Vec<WorkspaceTextEdit> {
    if !index.allows_lexical_rename_advice() || !is_safe_symbol_name(new_name) {
        return Vec::new();
    }
    let namespace_prefix = namespace_prefix_of(qualified_name);
    let (new_qualified, new_decl_text) = qualified_and_decl_text(namespace_prefix, new_name);
    let mut edits = Vec::new();
    // Call sites.
    for inv in index.invocations_of(qualified_name, current_uri) {
        // Indirect sites (constant `$cmd` heads) are references, never
        // rename targets — their span is not the written name.
        if inv.indirect {
            continue;
        }
        // The cross-document half of the same `-map` gate the in-document
        // rename applies — see [`ensemble_map_dispatch_word`].  An ensemble
        // declared in one document and dispatched from another is exactly the
        // shape the recording covers.
        if ensemble_map_dispatch_word(inv.ensemble_dispatch) {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(WorkspaceTextEdit {
            uri: inv.uri.clone(),
            span: inv.range,
            new_text: replacement,
        });
    }
    // Name-link declarations that reference the command by qualified name —
    // the `rename OLD NEW` `OLD` word, the `namespace import` pattern.  Each
    // names the renamed command and must follow it to stay bound; the new
    // fully-qualified name is always a valid replacement for a qualified
    // reference.  (The `interp alias` `TARGET` word is already a call site
    // rewritten above, so it is not among these spans.)  The local imported /
    // aliased *usages* are deliberately left alone — they name the local
    // command, which keeps its own name.
    for (link_uri, span) in index.link_target_spans(qualified_name, current_uri) {
        edits.push(WorkspaceTextEdit {
            uri: link_uri,
            span,
            new_text: new_qualified.clone(),
        });
    }
    // Definition sites (proc + class) in other documents — matched by
    // *qualified* name so a same-simple-name proc in a different namespace is
    // not rewritten (and moved into the target's namespace).
    for p in index.proc_definitions_qualified(qualified_name, current_uri) {
        edits.push(WorkspaceTextEdit {
            uri: p.uri.clone(),
            span: p.name_span,
            new_text: new_decl_text.clone(),
        });
    }
    for c in index.class_definitions_qualified(qualified_name, current_uri) {
        edits.push(WorkspaceTextEdit {
            uri: c.uri.clone(),
            span: c.name_span,
            new_text: new_decl_text.clone(),
        });
    }
    edits
}

/// Compute the **whole-workspace** edit set for renaming the command whose
/// qualified name is `qualified_name` — every call site, name-link word, and
/// definition site the index knows, the current document included.
///
/// This is the consumer-document rename path: the cursor sits on a call
/// whose definition lives in another document (a workspace sibling, or a
/// library file the autoload tier merged), so the in-document rename has no
/// local definition to resolve against and produces nothing.  The index
/// supplies everything instead.  Returns `None` when the rename would shadow
/// an existing workspace command — the same collision discipline as the
/// in-document rename's local gate.
#[must_use]
pub fn workspace_symbol_rename_edits(
    qualified_name: &str,
    new_name: &str,
    index: &crate::workspace_index::WorkspaceIndex,
) -> Option<Vec<WorkspaceTextEdit>> {
    if !index.allows_lexical_rename_advice() || !is_safe_symbol_name(new_name) {
        return None;
    }
    let namespace_prefix = namespace_prefix_of(qualified_name);
    let (new_qualified, _) = qualified_and_decl_text(namespace_prefix, new_name);
    if index.workspace_command_exists(&new_qualified) {
        return None;
    }
    // Provenance gate: an indirect dispatch of this
    // command anywhere in the workspace whose contributing constants are
    // not all source-writable cannot follow any edit set — abstain.
    if index.rename_blocked(qualified_name) {
        return None;
    }
    Some(cross_document_symbol_edits(
        qualified_name,
        new_name,
        index,
        "",
    ))
}
