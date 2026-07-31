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

//! Rename provider.
//!
//! Computes a workspace edit that renames the symbol at the
//! cursor across the current document.  Handles:
//!
//! * `$var` references → rewrite the `VarDef.definition_span`
//!   and every `VarDef.references` span to the new name.
//! * proc references → rewrite `ProcDef.name_span` and every
//!   matching command-invocation head to the new name.
//!
//! With **safety gating**: the caller may pass a
//! [`tcl_registry::CommandRegistry`].  When provided, the new
//! name is validated against:
//!
//! * `is_safe_symbol_name(name)` — must match
//!   `^[A-Za-z_][A-Za-z0-9_]*$`.  Applies to every rename
//!   target.
//! * `is_builtin_command_name(name, registry)` — proc renames
//!   refuse to overwrite a built-in command name.
//!
//! When the new name fails either check, [`rename`] returns an
//! empty `Vec<TextEdit>` so the editor refuses the rename
//! rather than producing a partial edit set.
//!
//! Class renames are also supported: when the cursor sits on a
//! `oo::class create ClassName` declaration name (or on any
//! `ClassName new` / `ClassName create instance` invocation
//! head), the provider rewrites the declaration's name span
//! and every invocation that targets the class.  The same
//! `is_safe_symbol_name` / `is_builtin_command_name` gates
//! apply.
//!
//! Method / classmethod / property renames: when
//! the cursor sits on a member name inside the class body
//! (either at the declaration or at a call site), the
//! provider rewrites the declaration's name span plus every
//! same-name command invocation inside the class body, **and**
//! every external `$obj method` / `[$obj method]` call site
//! where `$obj`'s class (per `analysis.instance_classes`)
//! matches.  Renaming can also be triggered from an external
//! call site.  See [`crate::references::find_obj_method_call_sites`]
//! for the external-site scan's coverage.
//!
//! Cross-document rename: procs and classes rewrite their sibling-document
//! call / definition sites via [`cross_document_symbol_edits`] over the
//! workspace index.  **Methods** rename across their whole override family,
//! including sibling documents: the server resolves the family via
//! [`crate::workspace_index::WorkspaceIndex::method_override_family`] and
//! calls [`method_spans_in_document`] per family-member document.
//! [`method_rename_target`] identifies the seed class + method under the
//! cursor for that path.
//!
//! Limitations:
//!
//! * A `$obj method` site is only rewritten in a document that also
//!   *defines* the receiver's class — the same single-document constraint
//!   under which the analyser resolves `$obj`'s class at all.  A `$obj
//!   method` call in a file that only *uses* a class (never defines it) is
//!   not resolvable, so it is neither found nor (mis)renamed.
//! * `$obj method` sites embedded in quoted / word tokens
//!   (`"prefix[$d bark]"`) — the external scan descends into
//!   command-substitution args and proc / method bodies but
//!   not into string interpolation.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

use crate::definition::LspRange;
use crate::hover::{find_var_at_position, find_word_span_at_position};
use crate::references::{MemberSel, resolve_member_span};

/// One text edit in a rename — span plus replacement text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Range to replace (byte spans translated to LSP
    /// line/character coordinates).
    pub range: LspRange,
    /// Replacement text.
    pub new_text: String,
}

/// `true` when `name` matches the safe-symbol shape
/// `^[A-Za-z_][A-Za-z0-9_]*$`.
///
/// Used by [`rename`] to reject new names that contain
/// whitespace, punctuation, leading digits, or other characters
/// that would produce invalid Tcl symbol references.
#[must_use]
pub fn is_safe_symbol_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `true` when `name` is registered as a built-in command in
/// `registry`.
///
/// Used by [`rename`] to refuse proc renames that would
/// overwrite a built-in command name (the editor would then
/// silently dispatch the user's calls to the built-in instead
/// of the renamed proc).
#[must_use]
pub fn is_builtin_command_name(name: &str, registry: &CommandRegistry) -> bool {
    registry.command_names().any(|n| n == name)
}

/// Compute the text edits for a rename of the symbol at the
/// cursor.
///
/// Returns an empty vector when no recognisable symbol is at
/// the position, the new name fails the safety gate, or a
/// proc rename would shadow a built-in command.  The caller
/// (server) is responsible for wrapping the output in a
/// `WorkspaceEdit { changes: { uri: edits } }`.
///
/// `registry`, when `Some`, enables safety
/// gating:
///
/// * Every rename target rejects new names that don't match
///   `is_safe_symbol_name` — invalid syntax is refused.
/// * Proc renames additionally refuse names registered as
///   built-in commands via [`is_builtin_command_name`].
///
/// Result of [`prepare_rename`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareRename {
    /// Range of the symbol the rename will affect.
    pub range: LspRange,
    /// Suggested placeholder (the symbol's current tail name)
    /// the editor pre-fills in its rename input box.
    pub placeholder: String,
}

/// The cursor word's own range + text as a [`PrepareRename`] — the
/// consumer-document fall-through (M8).  [`prepare_rename`] anchors on a
/// *local* declaration; when the document has none (the symbol is defined in
/// a sibling / autoloaded library file), the server may still accept the
/// rename after resolving the word through the workspace index — the range
/// the editor highlights is then the call-site word itself.
#[must_use]
pub fn word_prepare_at(source: &str, line: u32, character: u32) -> Option<PrepareRename> {
    let (word, start, end) = find_word_span_at_position(source, line, character)?;
    Some(PrepareRename {
        range: crate::definition::LspRange {
            start_line: line,
            start_character: start,
            end_line: line,
            end_character: end,
        },
        placeholder: word,
    })
}

/// Validate that the cursor sits on a renameable symbol and
/// return the symbol's range + placeholder text.  Editors
/// call this before `rename` to determine whether to show the
/// rename UI.
#[must_use]
pub fn prepare_rename(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<PrepareRename> {
    let line_index = LineIndex::new(source);
    // Variable?
    if let Some(var_name) = find_var_at_position(source, line, character) {
        let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
        // A `$name`-shaped substring in a comment or a data brace is not a
        // reference, so it is not renameable either (issue #923 idx 24; see
        // `lookup_var_read_at`).
        if let Some(var_def) = crate::definition::lookup_var_read_at(
            &analysis.global_scope,
            source,
            "",
            byte_offset,
            &var_name,
            analysis.ns_var_global_fallback(),
        ) {
            return Some(PrepareRename {
                range: span_to_range(source, &line_index, var_def.definition_span),
                placeholder: var_def.name.clone(),
            });
        }
    }
    // Variable definition / same-cell write site (`set x` / `variable x` /
    // a proc-param / a `catch` result-var) — no `$`, so resolve directly
    // via `var_def_at_declaration_offset`'s byte-offset span search (see
    // its own doc for why the ordinary scope-chain walk can't reach a
    // parameter's own declaring token).
    let def_byte = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(var_def) =
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, def_byte)
    {
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, var_def.definition_span),
            placeholder: var_def.name.clone(),
        });
    }
    // Proc?  Declaration-span hit, else the namespace-aware candidate
    // resolution — never a namespace-blind `p.name == word` first-hit scan
    // (which could seed the rename with an arbitrary same-named proc).
    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;
    let word_off = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some((_, proc_def)) =
        crate::definition::resolve_proc_target_at(analysis, source, word_off, &word, None)
    {
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, proc_def.name_span),
            placeholder: proc_def.name.clone(),
        });
    }
    // Class?  Same resolution shape against `all_classes`.
    if let Some((_, class_def)) =
        crate::definition::resolve_class_target_at(analysis, word_off, &word)
    {
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, class_def.name_span),
            placeholder: class_def.name.clone(),
        });
    }
    // Method / classmethod / property inside a class body?
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(class_def) = crate::definition::enclosing_class_at(analysis, cursor_offset)
        .and_then(|q| analysis.all_classes.get(q))
        && let Some((_, span)) = resolve_member_span(class_def, &word, cursor_offset)
    {
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, span),
            placeholder: word.clone(),
        });
    }
    // External `$obj method` call site — editors that gate the
    // rename UI on `prepare_rename` should still see it as
    // renameable.  Resolve `$obj`'s class and confirm a method
    // of that name exists.
    if let Some((inst, method, is_dollar)) =
        crate::definition::instance_method_at_cursor(source, line, character)
        && let Some(class_q) =
            crate::definition::receiver_instance_class(analysis, &inst, is_dollar)
        && let Some(class_def) = analysis.all_classes.get(class_q)
    {
        let member = class_def
            .methods
            .get(&method)
            .or_else(|| class_def.class_methods.get(&method));
        if let Some(m) = member {
            // Anchor the placeholder range on the call
            // site's method token so the editor's rename
            // box opens where the cursor is.
            let (_, mstart, mend) = find_word_span_at_position(source, line, character)?;
            return Some(PrepareRename {
                range: LspRange {
                    start_line: line,
                    start_character: mstart,
                    end_line: line,
                    end_character: mend,
                },
                placeholder: m.name.clone(),
            });
        }
    }
    None
}

/// Compute rename text edits.
///
/// See module-level docs for the dispatch order (variable → proc → class).
#[must_use]
pub fn rename(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Vec<TextEdit> {
    rename_with_diagnosis(
        source, dialect, line, character, new_name, analysis, registry,
    )
    .unwrap_or_default()
}

/// [`rename`], but reporting *why* a rename was refused.
///
/// `Err(refusal)` means the cursor names a real, resolvable symbol whose
/// rename cannot be completed soundly — the safety gate
/// ([`crate::rename_safety`]) proved the edit set would change what the
/// program does.  The server turns that into an LSP error response carrying
/// the reason, so the editor tells the user instead of silently applying a
/// partial edit set.  `Ok(vec![])` keeps its existing meaning: no renameable
/// symbol here, or a shape / collision gate said no.
///
/// The distinction matters because the two answers must not be conflated: an
/// empty edit set falls through to the server's cross-document and
/// workspace-resolved rename branches, and a *refusal* must not.
pub fn rename_with_diagnosis(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    // Shape gate first — applies to every rename target.
    if !is_safe_symbol_name(new_name) {
        return Ok(Vec::new());
    }
    let line_index = LineIndex::new(source);
    // Safety gate: when the cursor names a `TclOO` member, refuse outright if
    // any dispatch of it in this document cannot be proved safe to rewrite.
    // Checked before any edit is built so a hazard can never leak a partial
    // edit set (issue #923 idx 79).
    if let Some(refusal) =
        method_rename_refusal(source, dialect, line, character, analysis, &line_index)
    {
        return Err(refusal);
    }
    // Safety gate: a variable whose *name* can only be written quoted
    // (`set {$n} 1`) cannot be renamed by substituting into the recorded
    // spans — see the gate's own doc (issue #1078).  Checked before
    // `rename_variable_at`, whose `$`-reference cursor search would otherwise
    // read the `$n` *inside* the braces as a reference to the unrelated
    // variable `n` and rename that one instead.
    if let Some(refusal) = crate::rename_safety::literal_name_variable_rename_refusal(
        source,
        line,
        character,
        analysis,
        &line_index,
    ) {
        return Err(refusal);
    }

    if let Some(edits) =
        rename_variable_at(source, line, character, new_name, analysis, &line_index)
    {
        return Ok(edits);
    }
    let def_byte = crate::definition::byte_offset_at(&line_index, source, line, character);

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Ok(Vec::new());
    };

    if let Some(edits) = rename_proc(
        source,
        &word,
        def_byte,
        new_name,
        analysis,
        registry,
        &line_index,
    ) {
        return Ok(edits);
    }
    if let Some(edits) = rename_class(
        source,
        &word,
        def_byte,
        new_name,
        analysis,
        registry,
        &line_index,
    ) {
        return Ok(edits);
    }
    if let Some(edits) = rename_itcl_class_proc(
        source,
        dialect,
        (line, character),
        new_name,
        analysis,
        &line_index,
    ) {
        return Ok(edits);
    }
    // `$obj method` external call site — when the cursor sits
    // on the method-name token of an instance-method call and
    // `$obj`'s class is known, rename the method across its
    // declaration + all call sites (intra-class + external).
    if let Some((inst, method, is_dollar)) =
        crate::definition::instance_method_at_cursor(source, line, character)
        && method == word
        && let Some(class_q) =
            crate::definition::receiver_instance_class(analysis, &inst, is_dollar)
        && let Some(edits) = rename_method_in_class(
            source,
            dialect,
            (
                class_q,
                &method,
                crate::definition::receiver_method_bucket(analysis, &inst, is_dollar)
                    == crate::definition::MethodBucket::Class,
            ),
            new_name,
            analysis,
            &line_index,
        )
    {
        return Ok(edits);
    }
    // Method rename — match `word` against any class's methods
    // / classmethods / properties at the cursor's byte offset.
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(edits) = rename_method(
        source,
        dialect,
        &word,
        new_name,
        analysis,
        cursor_offset,
        &line_index,
    ) {
        return Ok(edits);
    }
    Ok(Vec::new())
}

/// The safety refusal for a `TclOO` member rename anchored at the cursor, if
/// any — the single in-document entry point to [`crate::rename_safety`].
///
/// Resolves the cursor to a member and its whole override family exactly the
/// way the rename itself does ([`method_rename_target`] + [`override_family`]),
/// so the gate is asked about precisely the declarations the edit set would
/// rewrite — never a wider or narrower set.
fn method_rename_refusal(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    let (seed_class, method, is_classmethod) =
        method_rename_target(source, line, character, analysis)?;
    let family = override_family(analysis, &seed_class, &method);
    if family.is_empty() {
        return None;
    }
    crate::rename_safety::method_rename_hazard(
        source,
        dialect,
        analysis,
        crate::rename_safety::MethodRenameTarget {
            family: &family,
            method: &method,
            is_classmethod,
        },
        line_index,
    )
}

/// The variable-rename edits for a cursor that names one, or `None` when it
/// names no variable at all.
///
/// Two cursor shapes, both variables: a `$ref` occurrence, and a *declaration
/// / same-cell write* token with no `$` (a `set x` / `variable x`, a
/// proc/method parameter, a `catch` result-var).  The second needs
/// `var_def_at_declaration_offset`'s byte-offset span search — a parameter's
/// own declaring token sits textually *before* its scope's body span even
/// starts, so the ordinary scope-chain walk never reaches it (see that
/// function's own doc).
fn rename_variable_at(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    let name = if let Some(var_name) = find_var_at_position(source, line, character) {
        var_name
    } else {
        let def_byte = crate::definition::byte_offset_at(line_index, source, line, character);
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, def_byte)?
            .name
            .clone()
    };
    Some(rename_var(
        source, line, character, new_name, analysis, line_index, &name,
    ))
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
/// to (`$obj method` vs. a bare `ClassName method` classmethod dispatch,
/// issue #923 idx 120), not something re-derivable from `class_q`/`method`
/// alone.
///
/// A `TclOO` method that is (re)defined by a super- or sub-class is a
/// single polymorphic name: `$obj method` dispatch can reach any
/// definition along the chain, so renaming only the class under the
/// cursor would silently break the override relationship.  See
/// [`override_family`].  Shared by the in-class-body and external
/// `$obj method` rename entry points.
/// Rename an [incr Tcl] class-scoped `proc` from a `Factory::make` call site
/// or its declaration (issue #990).
///
/// Tried after the proc / class rename paths so an ordinary qualified proc
/// call of the same spelling keeps priority.  The edits come from the shared
/// member-reference collector, whose itcl call sites cover only the final
/// `::`-segment — so the as-written qualifier survives the rewrite.
///
/// Single-document: a `Factory::make` call in a sibling file is not
/// rewritten, because the cross-file rename layer still carries only the
/// two-word `Class method` dispatch shape.
fn rename_itcl_class_proc(
    source: &str,
    dialect: &str,
    cursor: (u32, u32),
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
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

fn rename_method_in_class(
    source: &str,
    dialect: &str,
    target: (&str, &str, bool),
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    let (class_q, method, is_classmethod) = target;
    let family = override_family(analysis, class_q, method);
    if family.is_empty() {
        return None;
    }
    let mut edits = Vec::new();
    for member in &family {
        // Reached via an external call site — either `$obj method` (always
        // instance-context) or a bare `ClassName method` classmethod
        // dispatch (issue #923 idx 120) — so `is_classmethod` is fixed by
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
        edits.push(TextEdit {
            range: span_to_range(source, line_index, decl_span),
            new_text: new_name.to_owned(),
        });
        for span in call_spans {
            edits.push(TextEdit {
                range: span_to_range(source, line_index, span),
                new_text: new_name.to_owned(),
            });
        }
    }
    if edits.is_empty() {
        return None;
    }
    dedup_edits(&mut edits);
    Some(edits)
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
/// of the cursor's call shape (issue #945 fault 4): an external
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
    use crate::workspace_index::MethodAccess;
    let line_index = LineIndex::new(source);
    let (word, _s, _e) = find_word_span_at_position(source, line, character)?;
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some((inst, method, is_dollar)) =
        crate::definition::instance_method_at_cursor(source, line, character)
        && method == word
    {
        // `my method` — an internal call from inside the enclosing class.
        // Always instance-context: `my` never reaches a classmethod.
        if crate::definition::is_self_dispatch_keyword(&inst)
            && let Some(class_q) = crate::definition::enclosing_class_at(analysis, cursor)
        {
            return Some((class_q.to_owned(), method, false, MethodAccess::Internal));
        }
        // External `$obj method` — resolve `$obj`'s class.  Always
        // instance-context too: a classmethod is never reached via `$obj`.
        if let Some(class_q) =
            crate::definition::receiver_instance_class(analysis, &inst, is_dollar)
        {
            return Some((class_q.clone(), method, false, MethodAccess::External));
        }
        // Bare `ClassName method` — a classmethod dispatches on the class's
        // own command, never an instance, so it's tried only when the
        // receiver isn't `$`-prefixed (a `$var` can never name a class).
        if !is_dollar
            && let Some(class_q) =
                crate::definition::classmethod_dispatch_class(analysis, &inst, &method)
        {
            return Some((class_q, method, true, MethodAccess::External));
        }
    }
    // Inside a class body on one of its method / classmethod names — the
    // declaration side, an internal context.
    if let Some(class_def) = crate::definition::enclosing_class_at(analysis, cursor)
        .and_then(|q| analysis.all_classes.get(q))
    {
        let method_decl = class_def.methods.get(&word);
        let classmethod_decl = class_def.class_methods.get(&word);
        // A member-name-shaped word anywhere inside the class body is not by
        // itself a reference to that member.  It only is when the cursor sits
        // on the member's **own declaration name**, or on a bareword that
        // `link` genuinely made bareword-callable and pointed back at this
        // same member (`ClassDef::linked_members`, issue #923 idx 113 — an
        // un-linked bareword errors "invalid command name" in real Tcl, so it
        // names nothing).  This is the gate the in-document provider's
        // `definition::lookup_class_member` already applies; without it here,
        // any same-spelled word — a parameter, a literal, an un-linked
        // sibling call — was read as the member and handed to the workspace
        // resolver, which then answered non-deterministically (issue #1028).
        let on_declaration = |m: &tcl_compiler::analyser::MethodDef| {
            m.name_span.start() <= cursor && cursor < m.name_span.end()
        };
        let cursor_on_method_decl = method_decl.is_some_and(on_declaration);
        let cursor_on_classmethod_decl = classmethod_decl.is_some_and(on_declaration);
        let linked_to_self = class_def
            .linked_members
            .get(&word)
            .is_some_and(|target| *target == word);
        if (method_decl.is_some() || classmethod_decl.is_some())
            && (cursor_on_method_decl || cursor_on_classmethod_decl || linked_to_self)
        {
            // A `method` and a `classmethod` of the same name are distinct
            // members; unambiguous when only one exists, otherwise prefer
            // whichever one's declaration the cursor is actually on (a linked
            // bareword that could be either falls back to the method — the
            // pre-existing preference).
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
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
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
/// `extra_classmethod_cmd_names`: see
/// [`crate::references::inherited_method_call_sites`]'s identical parameter —
/// the workspace-wide classmethod-dispatch names this subclass-only document
/// cannot derive from its own (definer-less) `all_classes`.
#[must_use]
pub fn inherited_method_spans_in_document(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    extra_classmethod_cmd_names: &[String],
) -> Vec<tcl_lexer::Span> {
    crate::references::inherited_method_call_sites(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        extra_classmethod_cmd_names,
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

/// Variable-rename path — declaration span + every read site,
/// with brace-ref escaping (`$x` / `${x}` / `$ns::x` /
/// `${ns::x}` keep their leader characters and any namespace
/// qualifier).
fn rename_var(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    var_name: &str,
) -> Vec<TextEdit> {
    let byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    // The ordinary scope-chain lookup resolves a `$ref` cursor (and a
    // definition-site cursor that happens to sit inside its own scope's
    // body span); a proc/method parameter's own declaring token sits
    // *before* its scope's body span even starts, so fall back to the
    // byte-offset span search there (see `var_def_at_declaration_offset`'s
    // own doc).
    let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
        &analysis.global_scope,
        byte_offset,
        var_name,
        analysis.ns_var_global_fallback(),
    )
    .or_else(|| {
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, byte_offset)
            .filter(|v| v.name == var_name)
    }) else {
        return Vec::new();
    };
    // Collision gate: refuse when `new_name` already resolves to a *different*
    // variable visible at the cursor (e.g. a sibling `set y` in the same proc
    // scope) — renaming would merge two distinct variables.
    if let Some(existing) = crate::definition::lookup_var_in_scope_chain(
        &analysis.global_scope,
        byte_offset,
        new_name,
        analysis.ns_var_global_fallback(),
    ) && existing.definition_span != var_def.definition_span
    {
        return Vec::new();
    }
    let mut edits = Vec::with_capacity(1 + var_def.references.len());
    // Preserve any namespace qualifier on the declaration token itself
    // (`set myns::count` → `set myns::total`).  Derive the prefix from the
    // actual source token so it is independent of how the analyser stores
    // the name.
    let def_text = source
        .get(var_def.definition_span.start() as usize..var_def.definition_span.end() as usize)
        .unwrap_or("");
    // The declaration token can carry an array-element suffix
    // (`set arr(0) 1` → the span covers `arr(0)`); renaming must rewrite only
    // the base name and keep both any namespace qualifier and the `(idx)`
    // suffix, exactly like `build_var_ref_replacement` does for references.
    // Dropping the suffix here produced `set data 1` and turned every `$arr(0)`
    // ref into `$data(0)`, so the script then errored "variable isn't array".
    let (def_name_part, def_suffix) = split_array_suffix(def_text);
    let def_ns_prefix = match def_name_part.rfind("::") {
        Some(idx) => &def_name_part[..idx + 2],
        None => "",
    };
    let def_new_text = format!("{def_ns_prefix}{new_name}{def_suffix}");
    edits.push(TextEdit {
        range: span_to_range(source, line_index, var_def.definition_span),
        new_text: def_new_text,
    });
    // Rewrite every alias Tcl treats as one cell together — namespace/global
    // aliases (`global`/`variable`/`namespace upvar`) and a class instance
    // variable's per-method copies — so a rename never leaves a sibling use or
    // an aliasing declaration pointing at the old name.
    let ref_spans = crate::definition::linked_var_reference_spans(&analysis.global_scope, var_def);
    let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    for r in ref_spans {
        if !seen.insert((r.start(), r.end())) {
            continue;
        }
        // Brace-ref escaping — see
        // [`build_var_ref_replacement`].
        let replacement = build_var_ref_replacement(source, r, new_name);
        edits.push(TextEdit {
            range: span_to_range(source, line_index, var_ref_edit_span(source, r)),
            new_text: replacement,
        });
    }
    edits
}

/// Proc-rename path — declaration name span + every matching
/// call site.  Namespace-aware: the declaration keeps its
/// prefix; call sites pick the rewrite that matches the form
/// the source wrote (qualified ↔ short).  `Some(edits)` when
/// a proc matched `word` (even if the edit set ended up empty
/// from a safety gate); `None` when no proc matched.
fn rename_proc(
    source: &str,
    word: &str,
    cursor_off: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    // Resolve the target the same way go-to-definition does: the proc whose
    // declaration covers the cursor, else C Tcl's namespace-aware call-site
    // resolution — never a namespace-blind `p.name == word` scan, which let a
    // rename from a bareword call site pick an arbitrary same-named proc in
    // another namespace and rewrite the wrong definition (matches
    // `references::references`).
    let (qname, proc_def) =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_off, word, registry)?;
    if let Some(registry) = registry
        && is_builtin_command_name(new_name, registry)
    {
        return Some(Vec::new());
    }
    let namespace_prefix = namespace_prefix_of(&proc_def.qualified_name);
    let (new_qualified, qualified_decl) = qualified_and_decl_text(namespace_prefix, new_name);
    // The declaration rewrite follows the *form the source wrote* — a proc
    // declared short inside a `namespace eval` (`proc helper`) stays short
    // (`assist`); one declared qualified (`proc ::ns::greet`) stays qualified.
    let decl_was_qualified = source
        .get(proc_def.name_span.start() as usize..proc_def.name_span.end() as usize)
        .is_some_and(|t| t.contains("::"));
    let new_decl_text = if decl_was_qualified {
        qualified_decl
    } else {
        new_name.to_owned()
    };
    // Collision gate: renaming onto an existing proc of the same qualified
    // name would shadow it — refuse.  Compare names normalised to the
    // leading-`::` form.
    let target_q = format!("::{}", proc_def.qualified_name.trim_start_matches("::"));
    if analysis.all_procs.keys().any(|qn| {
        let q = format!("::{}", qn.trim_start_matches("::"));
        q != target_q && q == new_qualified
    }) {
        return Some(Vec::new());
    }
    // Provenance gate (issue #945 fault 1): an indirect dispatch of this
    // proc whose contributing constants are not all source-writable cannot
    // be kept alive by any edit set — refuse the whole rename rather than
    // emit edits that leave the dispatch running the old name.
    if analysis.command_invocations.iter().any(|inv| {
        !inv.rename_safe
            && crate::references::invocation_references_proc(analysis, inv, qname, proc_def)
    }) {
        return Some(Vec::new());
    }
    let mut edits = vec![TextEdit {
        range: span_to_range(source, line_index, proc_def.name_span),
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        // An indirect site's span does not carry the written name (a constant
        // `$cmd` head, M7) — rewriting it would splice the new name over
        // unrelated text.  References still report it; rename must not.
        if inv.indirect {
            continue;
        }
        // Use the *same* invocation-matching rule as Find-All-References so a
        // rename never rewrites a call the reference finder wouldn't report —
        // in particular the namespace gate that keeps a bare `helper` call in
        // `namespace eval ::b` from matching `::a::helper` (RUST_ISSUE_035).
        if !crate::references::invocation_references_proc(analysis, inv, qname, proc_def) {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(TextEdit {
            range: span_to_range(source, line_index, inv.range),
            new_text: replacement,
        });
    }
    dedup_edits(&mut edits);
    Some(edits)
}

/// Class-rename path — `oo::class create ClassName`
/// declaration plus every `ClassName new` / `ClassName create
/// instance` invocation head.  Same shape as [`rename_proc`]:
/// prefers the class whose declaration name span covers the cursor (so
/// renaming from a same-named class's own decl in another namespace
/// resolves to *that* class, not the first one found), else the first
/// class matching the word.
fn rename_class(
    source: &str,
    word: &str,
    cursor_off: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    // Declaration under the cursor, else namespace-aware resolution — never a
    // namespace-blind `c.name == word` scan (which from a call site could
    // rename the wrong same-named class in another namespace).
    let (qname, class_def) =
        crate::definition::resolve_class_target_at(analysis, cursor_off, word)?;
    if let Some(registry) = registry
        && is_builtin_command_name(new_name, registry)
    {
        return Some(Vec::new());
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
        .is_some_and(|t| t.contains("::"));
    let new_decl_text = if decl_was_qualified {
        qualified_decl
    } else {
        new_name.to_owned()
    };
    // Provenance gate (issue #945 fault 1) — mirrors `rename_proc`: an
    // indirect dispatch with unwritable contributing constants cannot
    // follow the rename, so refuse it outright.
    if analysis.command_invocations.iter().any(|inv| {
        !inv.rename_safe
            && crate::references::invocation_references_class(analysis, inv, qname, class_def)
    }) {
        return Some(Vec::new());
    }
    let mut edits = vec![TextEdit {
        range: span_to_range(source, line_index, class_def.name_span),
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        // Indirect sites (M7) are references, never rename targets — the span
        // does not carry the written name.
        if inv.indirect {
            continue;
        }
        // Use the *same* invocation-matching rule as Find-All-References so a
        // rename never rewrites a call the reference finder wouldn't report —
        // in particular the namespace gate that keeps a bare `ClassName new`
        // call in a *different* namespace from being rewritten (RUST_ISSUE_035).
        if !crate::references::invocation_references_class(analysis, inv, qname, class_def) {
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
        edits.push(TextEdit {
            range: span_to_range(source, line_index, inv.range),
            new_text: replacement,
        });
    }
    dedup_edits(&mut edits);
    Some(edits)
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
fn rename_method(
    source: &str,
    dialect: &str,
    word: &str,
    new_name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
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
    let mut edits = vec![TextEdit {
        range: span_to_range(source, line_index, name_span),
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
                edits.push(TextEdit {
                    range: span_to_range(source, line_index, decl_span),
                    new_text: new_name.to_owned(),
                });
            }
            for span in call_spans {
                edits.push(TextEdit {
                    range: span_to_range(source, line_index, span),
                    new_text: new_name.to_owned(),
                });
            }
        }
    } else {
        // Properties have no `$obj prop` dispatch and no inheritance model
        // (see `references::find_class_member_references`), so a
        // class-local `my <prop>` scan is the whole story.
        for span in
            crate::references::scan_my_method_sites(source, dialect, &body_spans, word, None)
        {
            edits.push(TextEdit {
                range: span_to_range(source, line_index, span),
                new_text: new_name.to_owned(),
            });
        }
    }
    dedup_edits(&mut edits);
    Some(edits)
}

/// Compute the qualified rewrite (`prefix::new`) and the
/// declaration-site rewrite (qualified when the original lived
/// in a namespace, short at the top level).  Shared by the
/// proc and class rename paths.
fn qualified_and_decl_text(namespace_prefix: &str, new_name: &str) -> (String, String) {
    let new_qualified = if namespace_prefix.is_empty() {
        format!("::{new_name}")
    } else {
        format!("{namespace_prefix}::{new_name}")
    };
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
fn invocation_replacement(
    _namespace_prefix: &str,
    _new_qualified: &str,
    new_name: &str,
    inv_name: &str,
) -> String {
    // Preserve the call's *as-written* qualifier: a qualified call replaces
    // only its final `::`-segment (`utils::helper` → `utils::assist`,
    // `::myns::greet` → `::myns::hello`); a short call becomes the short name.
    match inv_name.rfind("::") {
        Some(idx) => format!("{}::{new_name}", &inv_name[..idx]),
        None => new_name.to_owned(),
    }
}

/// A rename edit targeting a specific document, expressed as a
/// byte span (the caller resolves it to an LSP range against
/// that document's source).  Used for cross-document rename,
/// where the core provider can't see sibling documents'
/// sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTextEdit {
    /// Document the edit applies to.
    pub uri: String,
    /// Byte span in `uri`'s source to replace.
    pub span: tcl_lexer::Span,
    /// Replacement text.
    pub new_text: String,
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
    let namespace_prefix = namespace_prefix_of(qualified_name);
    let (new_qualified, new_decl_text) = qualified_and_decl_text(namespace_prefix, new_name);
    let mut edits = Vec::new();
    // Call sites.
    for inv in index.invocations_of(qualified_name, current_uri) {
        // Indirect sites (constant `$cmd` heads, M7) are references, never
        // rename targets — their span is not the written name.
        if inv.indirect {
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
    // not rewritten (and moved into the target's namespace) (RUST_ISSUE_036).
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
/// This is the consumer-document rename path (M8): the cursor sits on a call
/// whose definition lives in another document (a workspace sibling, or a
/// library file the autoload tier merged), so the in-document rename had no
/// local definition to resolve against and produced nothing.  The index
/// supplies everything instead.  Returns `None` when the rename would shadow
/// an existing workspace command — the same collision discipline as the
/// in-document rename's local gate.
#[must_use]
pub fn workspace_symbol_rename_edits(
    qualified_name: &str,
    new_name: &str,
    index: &crate::workspace_index::WorkspaceIndex,
) -> Option<Vec<WorkspaceTextEdit>> {
    let namespace_prefix = namespace_prefix_of(qualified_name);
    let (new_qualified, _) = qualified_and_decl_text(namespace_prefix, new_name);
    if index.workspace_command_exists(&new_qualified) {
        return None;
    }
    // Provenance gate (issue #945 fault 1): an indirect dispatch of this
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

/// Every edit renaming the **namespace-variable cell** `cell` to `new_name`
/// within `source` — one document's share of the workspace-wide rename the
/// cross-document tier drives.
///
/// `cell` is a `::`-rooted qualified name
/// ([`crate::definition::qualified_variable_cell_at`] is the single entry
/// point that decides which cell a cursor names, shared with go-to-definition
/// / hover / find-references so the four cannot disagree).  Three occurrence
/// kinds are rewritten, and they are the complete static set for a namespace
/// cell:
///
/// * the **namespace-scoped declaration(s)** this document holds — the
///   `variable v` / `set v …` sitting directly in a `namespace eval` body,
///   read off the scope tree via `namespace_variables` rather than re-derived;
/// * every **alias** Tcl treats as the same cell — a `variable v` / `global v`
///   / `namespace upvar ns v local` inside a proc or method body, enumerated
///   by [`tcl_compiler::analyser::variable_alias_links`] over
///   `VarDef::link_target` (the analyser's analogue of Tcl's `VAR_LINK`).
///   Missing these is what makes a naive "declaration + qualified
///   occurrences" rename break a program: `namespace eval ns { proc p {} {
///   variable v; puts $v } }` keeps working only if that `variable v` and its
///   `$v` are renamed too;
/// * every **`::`-qualified occurrence** (`$::ns::v`, `set app::ns::v 1`) —
///   `analysis.qualified_var_refs`, the same table the workspace index's
///   cross-document reference set is built from.
///
/// Deliberately *not* rewritten: an unqualified `$v` that is not an alias of
/// this cell.  A bare name means whatever the local scope chain supplies,
/// which is a per-document question this cell's identity cannot answer.
///
/// Also deliberately *not* rewritten: the **local spelling of a
/// differently-named alias**.  `namespace upvar ::ns v local` names the cell
/// in its `otherVar` word (`v`); `local` is an independent local name the
/// cell's identity does not determine.  See
/// [`cell_rename_spans`] for the rule and its oracle.
///
/// The result is unsorted and deduplicated by range; the caller merges it
/// with the other documents' shares.
#[must_use]
pub fn namespace_variable_rename_edits(
    source: &str,
    analysis: &AnalysisResult,
    cell: &str,
    new_name: &str,
) -> Vec<TextEdit> {
    if !is_safe_symbol_name(new_name) {
        return Vec::new();
    }
    let target = cell.trim_start_matches("::");
    // Collision gate, the in-document half of the workspace one the server
    // applies: renaming onto a cell this document already declares would
    // merge two distinct variables into one.  Same discipline (and same
    // "answer nothing" shape) as the single-document variable rename's own
    // scope-chain collision check.
    let new_cell = match cell.rfind("::") {
        Some(idx) => format!("{}::{new_name}", &cell[..idx]),
        None => format!("::{new_name}"),
    };
    if tcl_compiler::analyser::lookup_var_by_qualified_name(&analysis.global_scope, &new_cell)
        .is_some()
    {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let mut spans: Vec<tcl_lexer::Span> = Vec::new();
    for (qualified, var) in tcl_compiler::analyser::namespace_variables(&analysis.global_scope) {
        if qualified.trim_start_matches("::") != target {
            continue;
        }
        // The declaration itself, plus its own unqualified reads in the
        // namespace frame.  Aliases are *not* unioned in here — they are
        // handled by `cell_rename_spans`, which knows which of an alias's
        // words names the cell and which is merely a local spelling.
        spans.push(var.definition_span);
        spans.extend(var.references.iter().copied());
    }
    cell_rename_spans(&analysis.global_scope, target, &mut spans);
    for vref in &analysis.qualified_var_refs {
        if vref.qualified_name.trim_start_matches("::") == target {
            spans.push(vref.span);
        }
    }
    let mut edits: Vec<TextEdit> = spans
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| TextEdit {
            range: span_to_range(source, &line_index, var_ref_edit_span(source, s)),
            new_text: build_var_ref_replacement(source, s, new_name),
        })
        .collect();
    dedup_edits(&mut edits);
    edits
}

/// The spans a rename of the `::`-rooted cell `target` must rewrite in every
/// variable *aliasing* it — a proc-local `variable v` / `global v` /
/// `namespace upvar ns v local` / `upvar #0 ::ns::v local`, anywhere in the
/// scope tree.
///
/// The namespace-scoped declaration walk above only sees namespace and global
/// scopes, so an alias declared inside a proc or method body (the ordinary way
/// a namespace variable is used) is invisible to it; without this the
/// cross-document rename would rewrite the declaration and leave every
/// `variable v; … $v` body naming a cell that no longer exists.
///
/// # Which word an alias contributes
///
/// An alias binds a **local spelling** to a **cell**, and only one of the two
/// words involved is determined by the cell's name.
/// [`VarDef::link_target_span`](tcl_compiler::analyser::VarDef::link_target_span)
/// says which word names the cell, and this walk splits on whether that is the
/// declaration word itself:
///
/// * **Same word** — `variable v`, `global ::ns::v`.  The local spelling *is*
///   the cell's tail name, so renaming the cell renames the declaration and
///   every unqualified read with it.  Rewriting only some of them breaks the
///   program: with `variable total` against a body still reading `$v`, tclsh
///   9.0.4 and 8.6.16 both give `can't read "v": no such variable`.
///
/// * **Different words** — `namespace upvar ::ns v local`, `upvar #0 ::ns::v
///   local`.  The cell is named by `otherVar`; `local` is an independent
///   spelling the cell's name does not determine.  Only `otherVar` is
///   rewritten, and `local` and all its reads are left exactly as written.
///   Rewriting `local` instead — which is what the declaration span alone
///   gives — produces `namespace upvar ::ns v total; … $total`, and both
///   interpreters then fail `can't read "total": no such variable`, because
///   the alias still points at the cell that was renamed away.
///
/// A same-*spelled* `namespace upvar ::ns v v` is the second case, not the
/// first: its two `v`s are distinct words at distinct offsets, and the local
/// one is still an independent spelling.  Both interpreters accept either
/// answer (`namespace upvar ::ns total v; … $v` and `namespace upvar ::ns
/// total total; … $total` both print `42` on 9.0.4 and 8.6.16), so the
/// **minimal** edit is taken: rewrite the word that names the cell, and touch
/// nothing whose meaning the rename does not change.
fn cell_rename_spans(
    scope: &tcl_compiler::analyser::Scope,
    target: &str,
    out: &mut Vec<tcl_lexer::Span>,
) {
    for link in tcl_compiler::analyser::variable_alias_links(scope) {
        if link.cell.trim_start_matches("::") != target {
            continue;
        }
        let var = link.var;
        match var.link_target_span {
            // The cell is named by a word other than the declaration: rewrite
            // only that word, and leave the local spelling (and its reads)
            // alone.
            Some(span) if span != var.definition_span => out.push(span),
            // Declaration word and cell-naming word are one and the same.
            _ => {
                out.push(var.definition_span);
                out.extend(var.references.iter().copied());
            }
        }
    }
}

/// Extend a `${name}` reference span to cover its own closing
/// brace when the lexer's recorded span stops one byte short of it.
///
/// `tcl-lexer`'s `Var` token span deliberately excludes the closing
/// `}` for a non-degenerate braced name — `${a{b}}` names `a{b}`,
/// whose content can legitimately end in `}` itself, so the span
/// convention leaves the outer delimiter unconsumed rather than risk
/// misreading it as content (see `SourceMap::token_text`'s doc). That
/// makes `span` unsafe to use directly as a rename *edit range*:
/// `build_var_ref_replacement` already emits a self-closed `${new}`
/// string, so replacing only the short span leaves the source's own
/// original `}` sitting right after it, corrupting `${new}` into
/// `${new}}` (issue #923 idx 95 — this broke `tk.tcl`'s `${dir}view`
/// idiom badly enough to fail to parse post-rename). Mirrors
/// `token_text`'s own degenerate-`${}`-empty-name check so this never
/// mis-fires on a span that already legitimately includes the brace.
fn var_ref_edit_span(source: &str, span: tcl_lexer::Span) -> tcl_lexer::Span {
    let start = span.start() as usize;
    let end = span.end() as usize;
    let bytes = source.as_bytes();
    if start >= bytes.len() || end > bytes.len() {
        return span;
    }
    let Some(after_prefix) = source[start..end].strip_prefix("${") else {
        return span;
    };
    if after_prefix == "}" {
        return span;
    }
    if bytes.get(end) == Some(&b'}') {
        return tcl_lexer::Span::new(span.start(), span.end() + 1);
    }
    span
}

/// Build a replacement string for a variable reference span.
///
/// The reference span covers the full Var token (`$x`,
/// `${name}`, `$ns::var`, `${ns::var}`).  We read the source
/// bytes at the span to decide which leader characters (`$`,
/// `${`, `}`) to preserve, then splice in `new_tail` in place
/// of the existing tail name.
///
/// For `${name}` and `$name`, we strip the prefix to find the
/// inner text, find its namespace prefix (`ns::`), and emit
/// `${ns::new_tail}` / `$ns::new_tail` so namespace-qualified
/// refs keep their qualification.
fn build_var_ref_replacement(source: &str, span: tcl_lexer::Span, new_tail: &str) -> String {
    let start = span.start() as usize;
    let end = span.end() as usize;
    let bytes = source.as_bytes();
    if start >= bytes.len() || end > bytes.len() {
        return new_tail.to_owned();
    }
    let text = &source[start..end];
    if let Some(rest) = text.strip_prefix("${") {
        // `${arr(idx)}` is recorded against base `arr` (the analyser's
        // `normalise_var_name` strips the index for the braced form
        // too), so preserve the index here as well: `${arr(idx)}` →
        // `${<new>(idx)}` rather than clobbering it to `${<new>}`.
        let inner = rest.strip_suffix('}').unwrap_or(rest);
        let (name_part, suffix) = split_array_suffix(inner);
        let ns_prefix = match name_part.rfind("::") {
            Some(idx) => &name_part[..idx + 2],
            None => "",
        };
        return format!("${{{ns_prefix}{new_tail}{suffix}}}");
    }
    if let Some(rest) = text.strip_prefix('$') {
        // Preserve an array-index suffix so renaming the base array
        // variable keeps the element index: `$arr(idx)` → `$<new>(idx)`
        // rather than clobbering it to `$<new>`.  The index text is
        // copied verbatim (any `$`/`[` substitution inside it is
        // renamed independently via its own reference).
        let (name_part, suffix) = split_array_suffix(rest);
        let ns_prefix = match name_part.rfind("::") {
            Some(idx) => &name_part[..idx + 2],
            None => "",
        };
        return format!("${ns_prefix}{new_tail}{suffix}");
    }
    let (name_part, suffix) = split_array_suffix(text);
    let ns_prefix = match name_part.rfind("::") {
        Some(idx) => &name_part[..idx + 2],
        None => "",
    };
    format!("{ns_prefix}{new_tail}{suffix}")
}

/// Split a (already `$`-stripped) variable reference into its base
/// name and any trailing array-index suffix.  `arr(idx)` →
/// `("arr", "(idx)")`; `name` → `("name", "")`.
fn split_array_suffix(rest: &str) -> (&str, &str) {
    match rest.find('(') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, ""),
    }
}

/// Return the namespace prefix of a qualified name — everything
/// before the final `::`.  `"::myns::greet"` → `"::myns"`;
/// `"::greet"` → `""` (proc lives at global scope, no enclosing
/// namespace).  `"greet"` → `""` likewise.
fn namespace_prefix_of(qualified: &str) -> &str {
    // `qualified` is a constructed key: its holder comes from the
    // construction-inverse split (#934) — a repeated-strip + `rfind` would
    // misread a lone-colon segment.  The root holder maps to `""` (a global
    // proc has no enclosing-namespace prefix).
    let (holder, _tail) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder == "::" { "" } else { holder }
}

fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

fn dedup_edits(edits: &mut Vec<TextEdit>) {
    let mut seen: FxHashSet<(u32, u32, u32, u32)> = FxHashSet::default();
    edits.retain(|e| {
        let key = (
            e.range.start_line,
            e.range.start_character,
            e.range.end_line,
            e.range.end_character,
        );
        seen.insert(key)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// Apply every edit's `(range, new_text)` back onto `source` — the
    /// same thing an editor does — sorted back-to-front so earlier byte
    /// offsets stay valid as later edits are spliced in. Some tests below
    /// use this to check the *result*, not just that `new_text` looks
    /// right in isolation: idx 95's own bug (a rename edit range short by
    /// one byte) shipped uncaught precisely because every prior brace-ref
    /// test only asserted `new_text`, never applied it.
    fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
        let line_index = LineIndex::new(source);
        let mut spans: Vec<(usize, usize, &str)> = edits
            .iter()
            .map(|e| {
                let start = crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    e.range.start_line,
                    e.range.start_character,
                ) as usize;
                let end = crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    e.range.end_line,
                    e.range.end_character,
                ) as usize;
                (start, end, e.new_text.as_str())
            })
            .collect();
        spans.sort_by_key(|s| std::cmp::Reverse(s.0));
        let mut result = source.to_string();
        for (start, end, new_text) in spans {
            result.replace_range(start..end, new_text);
        }
        result
    }

    #[test]
    fn rename_proc_includes_decl_and_calls() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 6, "hi", &analysis, None);
        assert!(!edits.is_empty());
        assert!(edits.iter().all(|e| e.new_text == "hi"));
        // First edit is the declaration on line 0 col 5.
        assert_eq!(edits[0].range.start_line, 0);
        assert_eq!(edits[0].range.start_character, 5);
    }

    #[test]
    fn rename_rewrites_bare_calls_to_a_proc_installed_into_oo_helpers() {
        // Issue #923 idx 56 (main audit wave, high severity): the finding's
        // own demonstrated failure mode — renaming `::oo::Helpers::classvar`
        // previously produced a `WorkspaceEdit` with only the declaration
        // rewritten, leaving the bare `classvar hits` call site inside the
        // method body pointed at the now-nonexistent old name. Applying
        // that edit would crash the very next invocation with "invalid
        // command name" at runtime, while the tool reported it as a
        // complete, safe rename.
        let src = "proc ::oo::Helpers::classvar {name} {\n    set ns [uplevel 1 {my getONSClass}]\n    tailcall namespace upvar $ns $name $name\n}\noo::class create Counter {\n    variable _label\n    constructor {label} { set _label $label }\n    method getONSClass {} { return [self class] }\n    method bump {} {\n        classvar hits\n        incr hits\n        return \"$_label:$hits\"\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `classvar` declaration (line 0, col 20).
        let edits = rename(src, "tcl", 0, 20, "renamedClassvar", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {edits:?}");
        assert!(
            lines.contains(&9),
            "the bare classvar call site inside the method body must be rewritten too: {edits:?}"
        );
        // The qualified declaration gets the qualified replacement; the
        // bare call site gets the short replacement — same convention as
        // any other namespaced-proc rename.
        let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(
            replacements.contains(&"::oo::Helpers::renamedClassvar"),
            "expected the qualified replacement at decl; got {replacements:?}"
        );
        assert!(
            replacements.contains(&"renamedClassvar"),
            "expected the short replacement at the bare call site; got {replacements:?}"
        );
    }

    #[test]
    fn rename_from_the_in_proc_global_alias_rewrites_the_callers_canonical_set_too() {
        // Issue #923 idx 68 (main audit wave, high severity, pix corpus):
        // the finding's own stated consequence of the `references()` gap —
        // triggering Rename from the in-proc `global tolComp` alias
        // previously rewrote only the proc's own 4 spans, leaving the
        // caller's `set ::tolComp 0.05` pointed at the now-decoupled old
        // name. Applying that edit would silently fall back to `isEqual`'s
        // hardcoded 0.01 default instead of the caller-intended tolerance —
        // a real runtime-behavior break introduced by a "successful" rename.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
        let edits = rename(src, "tcl", 2, 14, "tolerance", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&1), "the `global tolComp` decl: {edits:?}");
        assert!(lines.contains(&2), "the in-proc $tolComp read: {edits:?}");
        assert!(
            lines.contains(&4),
            "the caller's canonical `set ::tolComp 0.05` must be rewritten too: {edits:?}"
        );
        let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(
            replacements.contains(&"::tolerance"),
            "the canonical cell's own `::` qualifier must be preserved; got {replacements:?}"
        );
    }

    #[test]
    fn rename_from_the_callers_canonical_set_rewrites_every_in_proc_global_alias_too() {
        // The reverse direction of the test above (issue #923 idx 68):
        // triggering Rename from the caller's own `set ::tolComp` must
        // rewrite every in-proc `global tolComp` occurrence too, not just
        // the caller's own 2 spans.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the `set ::tolComp` declaration (line 4, col 8).
        let edits = rename(src, "tcl", 4, 8, "tolerance", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&4),
            "the caller's own decl must be rewritten: {edits:?}"
        );
        assert!(
            lines.contains(&1),
            "the in-proc `global tolComp` decl must be rewritten too: {edits:?}"
        );
        assert!(
            lines.contains(&2),
            "the in-proc $tolComp read must be rewritten too: {edits:?}"
        );
    }

    #[test]
    fn rename_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(rename(src, "tcl", 0, 6, "x", &analysis, None).is_empty());
    }

    #[test]
    fn rename_leaves_wildcard_imported_bareword_call_site_untouched_same_document() {
        // Same-document analogue of
        // `cross_document_symbol_edits_leaves_wildcard_imported_call_site_untouched`:
        // renaming `proc bar` (reached bare elsewhere only through a
        // wildcard `namespace import`) must rewrite the declaration but
        // leave the imported bareword call site alone — it names the
        // *local* imported command, which keeps its own spelling.
        // `rename_proc` calls `invocation_references_proc` directly (not
        // the wildcard-aware `proc_reference_spans`), so it is unaffected
        // by `references()`'s newly widened same-document reference set
        // (see `references::tests::references_include_wildcard_imported_bareword_call_same_document`).
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the `bar` declaration (line 1, col 9).
        let edits = rename(src, "tcl", 1, 9, "baz", &analysis, None);
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].range.start_line, 1);
        assert_eq!(edits[0].new_text, "baz");
    }

    #[test]
    fn rename_rewrites_the_renames_own_old_word_too() {
        // TP — issue #923 idx 39 (main audit wave), the critical half: a
        // rename that skips this occurrence is worse than a no-op — the
        // real corpus repro shows the resulting edit set leaves a genuine
        // `rename OLD ""` pointing at a now-nonexistent command, crashing a
        // previously-passing tcltest at runtime with "can't delete ...:
        // command doesn't exist" and no diagnostic warning anywhere.
        let src = "proc helperFunc {x} { return [expr {$x * 2}] }\nhelperFunc 21\nrename helperFunc \"\"\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 6, "newName", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert_eq!(edits.len(), 3, "{edits:?}");
        assert!(lines.contains(&0), "decl missing: {edits:?}");
        assert!(lines.contains(&1), "call site missing: {edits:?}");
        assert!(
            lines.contains(&2),
            "the rename statement's own OLD word must be rewritten too: {edits:?}"
        );
        assert!(edits.iter().all(|e| e.new_text == "newName"));
    }

    #[test]
    fn cross_document_symbol_edits_rewrite_import_pattern_and_rename_old_word() {
        use crate::workspace_index::WorkspaceIndex;
        // `::mymod::helper` is imported by `::app` and renamed-away in a third
        // file; renaming the source must rewrite the `namespace import` pattern
        // and the `rename` OLD word so both stay bound to the new name — but
        // never the local imported/renamed *usages*.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
        let ren = analyse("rename ::mymod::helper legacy\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
            ("file:///ren.tcl", &ren),
        ]);
        let edits =
            cross_document_symbol_edits("::mymod::helper", "helper2", &index, "file:///mymod.tcl");
        // The import pattern (app.tcl) and the rename OLD word (ren.tcl) both
        // become the new fully-qualified name.
        assert_eq!(edits.len(), 2, "{edits:?}");
        let app_edit = edits
            .iter()
            .find(|e| e.uri == "file:///app.tcl")
            .expect("import pattern edit");
        assert_eq!(app_edit.new_text, "::mymod::helper2");
        let ren_edit = edits
            .iter()
            .find(|e| e.uri == "file:///ren.tcl")
            .expect("rename OLD edit");
        assert_eq!(ren_edit.new_text, "::mymod::helper2");
    }

    #[test]
    fn cross_document_symbol_edits_leaves_wildcard_imported_call_site_untouched() {
        use crate::workspace_index::WorkspaceIndex;
        // A wildcard `namespace import ::mymod::*` creates no textual link to
        // rewrite (issue #923 idx 18, unlike the exact-import case tested
        // above): renaming `::mymod::helper` must rewrite its own
        // declaration but leave `app.tcl`'s bareword call site untouched —
        // it names the *local* imported command, which keeps its own
        // spelling, and a glob pattern has no literal occurrence of
        // `helper` to rewrite in the first place.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        // `current_uri` is a third, unrelated file so `mymod.tcl`'s own
        // declaration is not excluded from this cross-document sweep.
        let edits =
            cross_document_symbol_edits("::mymod::helper", "helper2", &index, "file:///caller.tcl");
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].uri, "file:///mymod.tcl");
    }

    #[test]
    fn rename_var_includes_decl_span() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor inside `$x`.
        let edits = rename(src, "tcl", 1, 7, "y", &analysis, None);
        assert!(!edits.is_empty());
        // Declaration replaces just `x` → `y`; reference
        // replaces `$x` → `$y` so the `$` prefix is preserved.
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains(&"y"), "{texts:?}");
        assert!(texts.contains(&"$y"), "{texts:?}");
    }

    #[test]
    fn rename_from_proc_param_bareword_declaration_rewrites_every_use() {
        // TP — differential-audit finding idx 9 (main audit wave): renaming
        // from a cursor placed directly on a proc parameter's own bareword
        // name (not a `$`-prefixed read) previously produced *zero* edits
        // — an LSP that silently no-ops a rename request is worse than one
        // that fails loudly, since the user has no signal anything went
        // wrong. Both the parameter's own declaration and its `$name` read
        // must be rewritten.
        let src = "proc greet {name} { return $name }\n";
        let analysis = analyse(src);
        // Cursor on `name` inside the parameter list (col 12-16).
        let edits = rename(src, "tcl", 0, 13, "label", &analysis, None);
        assert_eq!(edits.len(), 2, "{edits:?}");
        // The declaration rewrites bare `name` -> `label`; the `$name` read
        // preserves its `$` prefix -> `$label` (mirrors
        // `rename_var_includes_decl_span`'s established convention).
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains(&"label"), "{texts:?}");
        assert!(texts.contains(&"$label"), "{texts:?}");
        assert!(edits.iter().any(|e| e.range.start_line == 0), "{edits:?}");
    }

    #[test]
    fn rename_from_catch_resultvar_bareword_rewrites_the_original_declaration_too() {
        // TP — the finding's other confirmed shape: a `catch script name`
        // result-var reuses an existing variable; renaming from a cursor
        // on its own bareword token must rewrite every occurrence,
        // including the original declaration elsewhere in the proc.
        let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
        let analysis = analyse(src);
        // Cursor on the catch result-var `name` (line 1, col 16-20).
        let edits = rename(src, "tcl", 1, 17, "resolved", &analysis, None);
        assert!(
            edits.iter().any(|e| e.range.start_line == 0),
            "original param declaration must be rewritten too: {edits:?}"
        );
        assert!(
            edits.iter().any(|e| e.range.start_line == 1),
            "the catch result-var site itself must be rewritten: {edits:?}"
        );
        assert!(
            edits.iter().any(|e| e.range.start_line == 2),
            "the later $name read must be rewritten: {edits:?}"
        );
        // The bareword sites (decl + catch result-var) rewrite to plain
        // `resolved`; the `$name` read preserves its `$` prefix.
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains(&"resolved"), "{texts:?}");
        assert!(texts.contains(&"$resolved"), "{texts:?}");
    }

    // brace-ref escaping

    #[test]
    fn rename_var_preserves_braced_reference_form() {
        let src = "set x 1\nputs ${x}\n";
        let analysis = analyse(src);
        // Cursor inside `${x}` on the `x`.
        let edits = rename(src, "tcl", 1, 7, "y", &analysis, None);
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains(&"y"), "{texts:?}");
        assert!(texts.contains(&"${y}"), "{texts:?}");
    }

    #[test]
    fn var_ref_edit_span_extends_the_ordinary_braced_form() {
        let src = "${x}";
        // The lexer's own span for a non-degenerate name stops one byte
        // short of the closing `}` (see `var_ref_edit_span`'s doc).
        assert_eq!(
            var_ref_edit_span(src, tcl_lexer::Span::new(0, 3)),
            tcl_lexer::Span::new(0, 4)
        );
    }

    #[test]
    fn var_ref_edit_span_leaves_the_degenerate_empty_name_span_untouched() {
        // `${}` immediately followed by a literal (unrelated) `}` — the
        // pathological case that would fool a naive "does the next byte
        // look like `}`" check. The lexer already extends `${}`'s own
        // span to include its closing brace, so `var_ref_edit_span` must
        // recognise the degenerate case and stop, not also swallow the
        // following literal `}` into the edit range.
        let src = "${}}";
        assert_eq!(
            var_ref_edit_span(src, tcl_lexer::Span::new(0, 3)),
            tcl_lexer::Span::new(0, 3)
        );
    }

    #[test]
    fn var_ref_edit_span_extends_a_tcl9_nested_brace_name() {
        // `${a{b}}` names `a{b}` (Tcl 9's brace-nesting rule) — the span
        // stops before the *outer* `}`, one byte short, same as the
        // ordinary case.
        let src = "${a{b}}";
        assert_eq!(
            var_ref_edit_span(src, tcl_lexer::Span::new(0, 6)),
            tcl_lexer::Span::new(0, 7)
        );
    }

    #[test]
    fn var_ref_edit_span_leaves_non_braced_forms_untouched() {
        let src = "$x";
        let span = tcl_lexer::Span::new(0, 2);
        assert_eq!(var_ref_edit_span(src, span), span);
    }

    #[test]
    fn var_ref_edit_span_leaves_an_unterminated_reference_untouched() {
        // No closing brace anywhere in the source to extend to.
        let src = "${x";
        let span = tcl_lexer::Span::new(0, 3);
        assert_eq!(var_ref_edit_span(src, span), span);
    }

    #[test]
    fn rename_array_variable_applying_the_braced_index_edit_does_not_duplicate_the_closing_brace() {
        let src = "set arr(0) 1\nputs ${arr(0)}\n";
        let analysis = analyse(src);
        // Cursor on `arr` inside `${arr(0)}` (line 1, col 7).
        let edits = rename(src, "tcl", 1, 7, "data", &analysis, None);
        assert_eq!(apply_edits(src, &edits), "set data(0) 1\nputs ${data(0)}\n");
    }

    #[test]
    fn rename_var_applying_the_braced_reference_edit_does_not_duplicate_the_closing_brace() {
        // Issue #923 idx 95. The `Var` token's own lexer span for a
        // non-degenerate `${name}` form stops one byte short of the
        // closing `}` (`${a{b}}` names `a{b}`, whose content can itself
        // legitimately end in `}` — see `var_ref_edit_span`'s doc), so
        // using that raw span as the *edit range* (rather than just for
        // matching/reference bookkeeping) left the source's own original
        // `}` sitting right after the replacement once applied, silently
        // corrupting `${x}` into `${y}}`. `rename_var_preserves_braced_reference_form`
        // above already pins `new_text`; this applies the edit for real.
        let src = "set x 1\nputs ${x}\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 1, 7, "y", &analysis, None);
        assert_eq!(apply_edits(src, &edits), "set y 1\nputs ${y}\n");
    }

    #[test]
    fn rename_var_applying_the_dir_view_idiom_edit_produces_valid_tcl() {
        // The real `tk/library/tk.tcl:594-596` idiom this finding traces
        // through (`$w ${dir}view scroll ...`, subcommand synthesized by
        // concatenating `$dir` with literal `view`): renaming `dir`
        // previously produced `$w ${direction}}view ...` once applied —
        // tclsh8.6/9.0 both fail to even parse the enclosing proc ("extra
        // characters after close-brace"), since the stray extra `}` shifts
        // Tcl's own brace-counting scan for where the proc body ends.
        let src = "proc ::tk::MouseWheel {w dir amount {factor -120.0} {units units}} {\n    $w ${dir}view scroll [expr {$amount/$factor}] $units\n}\n";
        let analysis = analyse(src);
        // Cursor on the `d` of `dir` inside `${dir}view` (line 1, col 9).
        let edits = rename(src, "tcl", 1, 9, "direction", &analysis, None);
        let applied = apply_edits(src, &edits);
        assert!(
            applied.contains("${direction}view"),
            "expected a single, correctly-closed brace: {applied}"
        );
        assert!(
            !applied.contains("${direction}}view"),
            "must not duplicate the closing brace: {applied}"
        );
    }

    #[test]
    fn rename_array_variable_preserves_element_index() {
        // Renaming the base array variable `arr` must keep each
        // element index intact: `$arr(0)` → `$data(0)`, not `$data`.
        let src = "set arr(0) 1\nputs $arr(0)\n";
        let analysis = analyse(src);
        // Cursor on `arr` in the `set` target (line 0, col 4).
        let edits = rename(src, "tcl", 1, 6, "data", &analysis, None);
        assert!(!edits.is_empty(), "expected array rename edits");
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(
            texts.contains(&"$data(0)"),
            "reference edit must preserve the array index, got {texts:?}",
        );
    }

    #[test]
    fn rename_array_variable_preserves_index_in_braced_reference() {
        // The `${arr(0)}` braced form is recorded against base `arr`,
        // so renaming must rewrite it to `${data(0)}` (index kept).
        let src = "set arr(0) 1\nputs ${arr(0)}\n";
        let analysis = analyse(src);
        // Cursor on `arr` inside `${arr(0)}` (line 1, col 7).
        let edits = rename(src, "tcl", 1, 7, "data", &analysis, None);
        assert!(!edits.is_empty(), "expected braced array rename edits");
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(
            texts.contains(&"${data(0)}"),
            "braced reference edit must preserve the array index, got {texts:?}",
        );
    }

    #[test]
    fn prepare_rename_returns_range_for_proc() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 0, 6, &analysis).expect("expected prepare_rename on proc name");
        assert_eq!(p.placeholder, "greet");
        // Anchored at the proc name span.
        assert_eq!(p.range.start_line, 0);
        assert_eq!(p.range.start_character, 5);
    }

    #[test]
    fn prepare_rename_returns_range_for_var() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 1, 7, &analysis).expect("expected prepare_rename on var");
        assert_eq!(p.placeholder, "x");
    }

    #[test]
    fn prepare_rename_returns_none_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        // `hello` isn't a proc or var declaration.
        assert!(prepare_rename(src, 0, 6, &analysis).is_none());
    }

    #[test]
    fn build_var_ref_replacement_handles_all_forms() {
        // Pure helper exercise — bare `$x`, braced `${x}`, and
        // qualified forms all produce the right replacement text.
        // Namespace-qualified refs preserve the prefix so e.g.
        // `$ns::z` → `$ns::c` even if the analyser doesn't yet
        // surface namespace-scoped variable lookups (the helper
        // is the building block; the lookup side isn't wired
        // yet).
        let src = "  $x  ${y}  $ns::z  ${ns::w}  ";
        let span_x = tcl_lexer::Span::new(2, 4);
        let span_braced_y = tcl_lexer::Span::new(6, 10);
        let span_qualified_z = tcl_lexer::Span::new(12, 18);
        let span_braced_w = tcl_lexer::Span::new(20, 28);
        assert_eq!(build_var_ref_replacement(src, span_x, "a"), "$a");
        assert_eq!(build_var_ref_replacement(src, span_braced_y, "b"), "${b}");
        assert_eq!(
            build_var_ref_replacement(src, span_qualified_z, "c"),
            "$ns::c"
        );
        assert_eq!(
            build_var_ref_replacement(src, span_braced_w, "d"),
            "${ns::d}"
        );
    }

    #[test]
    fn build_var_ref_replacement_preserves_array_index() {
        // Renaming the base array variable must keep the element
        // index: `$arr(idx)` → `$data(idx)`, not `$data`.
        let src = "$arr(idx)  $arr($i)  $ns::a(k)";
        assert_eq!(
            build_var_ref_replacement(src, tcl_lexer::Span::new(0, 9), "data"),
            "$data(idx)"
        );
        // A substituted index is copied verbatim (the inner `$i`
        // reference is renamed on its own if `i` is renamed).
        assert_eq!(
            build_var_ref_replacement(src, tcl_lexer::Span::new(11, 19), "data"),
            "$data($i)"
        );
        // Namespace prefix + array index together.
        assert_eq!(
            build_var_ref_replacement(src, tcl_lexer::Span::new(21, 30), "b"),
            "$ns::b(k)"
        );
    }

    #[test]
    fn build_var_ref_replacement_preserves_array_index_in_braced_form() {
        // `${arr(idx)}` is recorded against base `arr`, so renaming it
        // must keep the index inside the braces: `${data(idx)}`.
        let src = "${arr(idx)}  ${ns::a(k)}";
        assert_eq!(
            build_var_ref_replacement(src, tcl_lexer::Span::new(0, 11), "data"),
            "${data(idx)}"
        );
        assert_eq!(
            build_var_ref_replacement(src, tcl_lexer::Span::new(13, 23), "b"),
            "${ns::b(k)}"
        );
    }

    // safety gating

    #[test]
    fn is_safe_symbol_name_accepts_canonical_identifiers() {
        assert!(is_safe_symbol_name("foo"));
        assert!(is_safe_symbol_name("Foo"));
        assert!(is_safe_symbol_name("_underscore"));
        assert!(is_safe_symbol_name("a1"));
        assert!(is_safe_symbol_name("snake_case_42"));
    }

    #[test]
    fn is_safe_symbol_name_rejects_invalid_shapes() {
        assert!(!is_safe_symbol_name(""), "empty name should be invalid");
        assert!(!is_safe_symbol_name("1leading_digit"));
        assert!(!is_safe_symbol_name("has space"));
        assert!(!is_safe_symbol_name("has-dash"));
        assert!(!is_safe_symbol_name("has::colon"));
        assert!(!is_safe_symbol_name("with$dollar"));
        // Tcl-valid but rename-rejects (forces editor to refuse
        // partially-edited symbols).
        assert!(!is_safe_symbol_name("dotted.name"));
    }

    #[test]
    fn rename_returns_empty_for_unsafe_new_name() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        // Whitespace in the new name fails the shape gate.
        assert!(rename(src, "tcl", 0, 6, "bad name", &analysis, None).is_empty());
        // Leading digit fails.
        assert!(rename(src, "tcl", 0, 6, "1lead", &analysis, None).is_empty());
        // Dash fails.
        assert!(rename(src, "tcl", 0, 6, "with-dash", &analysis, None).is_empty());
    }

    #[test]
    fn rename_var_also_rejects_unsafe_new_name() {
        // The shape gate applies to variable renames too.
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        assert!(rename(src, "tcl", 1, 7, "bad name", &analysis, None).is_empty());
    }

    #[test]
    fn rename_proc_to_builtin_command_name_blocked() {
        // Renaming `greet` to `puts` would shadow the built-in
        // `puts` command — the safety gate must reject the
        // rename when a registry is provided.
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let edits = rename(src, "tcl", 0, 6, "puts", &analysis, Some(&registry));
        assert!(
            edits.is_empty(),
            "rename to built-in `puts` must produce no edits, got {edits:?}",
        );
    }

    #[test]
    fn rename_proc_to_non_builtin_succeeds_with_registry() {
        // Renaming `greet` to a non-built-in name should still
        // succeed when the registry is provided.
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let edits = rename(src, "tcl", 0, 6, "salut", &analysis, Some(&registry));
        assert!(
            !edits.is_empty(),
            "rename to non-built-in `salut` should produce edits",
        );
        assert!(edits.iter().all(|e| e.new_text == "salut"));
    }

    #[test]
    fn rename_var_to_builtin_name_allowed() {
        // The built-in-shadow gate is proc-only.  Variable
        // names live in a separate namespace and can use any
        // shape-valid identifier — including `puts`.
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let edits = rename(src, "tcl", 1, 7, "puts", &analysis, Some(&registry));
        assert!(
            !edits.is_empty(),
            "variable rename to `puts` should succeed (different namespace)",
        );
    }

    // namespace-aware proc renames

    #[test]
    fn namespace_prefix_of_qualified_names() {
        assert_eq!(namespace_prefix_of("::myns::greet"), "::myns");
        assert_eq!(namespace_prefix_of("::a::b::c"), "::a::b");
        assert_eq!(namespace_prefix_of("::greet"), "");
        assert_eq!(namespace_prefix_of("greet"), "");
    }

    #[test]
    fn rename_namespaced_proc_keeps_namespace_prefix_at_decl() {
        // `proc ::myns::greet {} {}` renamed to `hello` should
        // rewrite the declaration's name span to
        // `::myns::hello` (keeping the namespace prefix), not
        // just `hello` (which would clobber the prefix).
        let src = "proc ::myns::greet {} {}\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 14, "hello", &analysis, None);
        assert!(!edits.is_empty(), "{edits:?}");
        assert!(
            edits.iter().any(|e| e.new_text == "::myns::hello"),
            "expected qualified replacement at decl, got {:?}",
            edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn rename_namespaced_proc_rewrites_call_sites_appropriately() {
        // Source has both a qualified call (`::myns::greet`)
        // and a short call (`greet` inside a `namespace eval`
        // block).  The qualified call gets the qualified
        // replacement; the short call gets the short
        // replacement.
        let src = "proc ::myns::greet {} {}\n\
                   ::myns::greet\n\
                   namespace eval ::myns {\n\
                       greet\n\
                   }\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 14, "hello", &analysis, None);
        let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        // Should include both `::myns::hello` (qualified) and
        // `hello` (short).
        assert!(
            replacements.contains(&"::myns::hello"),
            "expected `::myns::hello` somewhere; got {replacements:?}",
        );
        assert!(
            replacements.contains(&"hello"),
            "expected `hello` somewhere; got {replacements:?}",
        );
    }

    #[test]
    fn rename_top_level_proc_uses_short_form_at_decl() {
        // For a top-level proc (`proc greet {} {}` →
        // `proc.qualified_name == "::greet"`, no enclosing
        // namespace prefix), the declaration rewrite stays
        // unqualified (`hello`, not `::hello`).
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 6, "hello", &analysis, None);
        assert!(
            edits.iter().any(|e| e.new_text == "hello"),
            "expected short `hello` at decl; got {:?}",
            edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
        );
        // No qualified `::hello` rewrites either — the source
        // never uses a qualified call.
        assert!(
            edits.iter().all(|e| e.new_text == "hello"),
            "expected every edit to be `hello`; got {:?}",
            edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
        );
    }

    // class rename

    #[test]
    fn rename_class_at_decl_rewrites_decl_and_calls() {
        // `oo::class create MyClass { ... }` plus a `MyClass new`
        // invocation — both should be rewritten.
        let src = "oo::class create MyClass {\n\
                       method greet {} {}\n\
                   }\n\
                   MyClass new\n";
        let analysis = analyse(src);
        // Cursor on the `MyClass` declaration name (column 17).
        let edits = rename(src, "tcl", 0, 17, "Renamed", &analysis, None);
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(!edits.is_empty(), "expected non-empty edits");
        assert!(
            texts.iter().all(|t| *t == "Renamed"),
            "every replacement should be `Renamed`; got {texts:?}",
        );
        // The decl site + the `MyClass new` call site = 2 edits
        // (more if the analyser surfaces inheritance refs).
        assert!(edits.len() >= 2, "{edits:?}");
    }

    #[test]
    fn rename_class_at_call_site_also_works() {
        // Cursor on the `MyClass` head of an invocation — same
        // outcome as renaming from the declaration.
        let src = "oo::class create MyClass {\n}\nMyClass new\n";
        let analysis = analyse(src);
        // Cursor on the `MyClass` in `MyClass new` (line 2, col 3).
        let edits = rename(src, "tcl", 2, 3, "Renamed", &analysis, None);
        assert!(!edits.is_empty(), "{edits:?}");
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(
            texts.iter().all(|t| *t == "Renamed"),
            "expected every edit to be `Renamed`; got {texts:?}",
        );
    }

    #[test]
    fn rename_class_rejects_unsafe_new_name() {
        let src = "oo::class create MyClass {}\nMyClass new\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 17, "1bad", &analysis, None);
        assert!(edits.is_empty(), "{edits:?}");
    }

    #[test]
    fn rename_class_blocked_when_shadowing_builtin() {
        let src = "oo::class create MyClass {}\nMyClass new\n";
        let analysis = analyse(src);
        let mut r = tcl_registry::CommandRegistry::build_default();
        r.load_dialect(tcl_dialect::DialectSet::IRULES);
        let edits = rename(src, "tcl", 0, 17, "if", &analysis, Some(&r));
        assert!(
            edits.is_empty(),
            "renaming to a built-in should be blocked; got {edits:?}",
        );
    }

    #[test]
    fn prepare_rename_returns_range_for_class() {
        let src = "oo::class create MyClass {}\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 0, 17, &analysis).expect("expected prepare_rename on class");
        assert_eq!(p.placeholder, "MyClass");
        // Anchored at the class name span.
        assert_eq!(p.range.start_line, 0);
        // The span sits at column 17.
        assert_eq!(p.range.start_character, 17);
    }

    // method rename

    #[test]
    fn rename_method_at_decl_rewrites_decl_and_calls() {
        // Method declared on line 1, dispatched via `my greet` from line 2's
        // body twice — should rewrite the declaration plus both call sites.
        // A bare `greet` call is never valid `TclOO` dispatch (only `my
        // greet` reaches the method), so the fixture must use `my`.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1 col 11).
        let edits = rename(src, "tcl", 1, 11, "salute", &analysis, None);
        assert!(!edits.is_empty(), "{edits:?}");
        // All three sites should be present.
        assert!(edits.len() >= 3, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "salute");
        }
    }

    #[test]
    fn rename_method_rewrites_a_my_dispatch_call_inside_a_switch_arm() {
        // Issue #923 idx 63 (main audit wave, high severity): the real
        // corpus's `ticklecharts::chart::Add` dispatcher shape — `switch
        // ... { barSeries { my AddBarSeries {*}$args } ... }`. `rename`
        // reaches `my`-dispatch call sites via `references::
        // method_references_for_class` (`scan_my_method_region`), so this
        // is the same fix as `references_for_method_reach_a_my_dispatch_
        // call_inside_a_switch_arm`, verified end-to-end through rename.
        let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `bar` declaration (line 1, col 11).
        let edits = rename(src, "tcl", 1, 11, "baz", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {edits:?}");
        assert!(
            lines.contains(&4),
            "the my bar call site inside the switch arm must be rewritten too: {edits:?}"
        );
        assert!(edits.iter().all(|e| e.new_text == "baz"));
    }

    #[test]
    fn rename_method_rewrites_my_dispatch_when_class_extended_via_separate_oo_define() {
        // Issue #923 idx 52 (main audit wave, high severity): `Gadget` is
        // created via `oo::class create` with no body; every method
        // (including the `my Helper` call site) is added via a *separate*,
        // later `oo::define Gadget { ... }` block — the real corpus shape
        // (`ticklecharts::chart`). Renaming from the `Helper` declaration
        // must rewrite the `my Helper` call site living in that separate
        // block too, not silently skip it (which would leave the program
        // calling a now-nonexistent method after the rename).
        let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `Helper` declaration (line 4, col 11).
        let edits = rename(src, "tcl", 4, 11, "Assist", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&4), "decl missing: {edits:?}");
        assert!(
            lines.contains(&5),
            "the my Helper call site inside the separate oo::define block must be rewritten too: {edits:?}"
        );
        assert!(edits.iter().all(|e| e.new_text == "Assist"));
    }

    #[test]
    fn rename_method_does_not_rewrite_next_dispatch() {
        // `next` is a keyword, not the method name.  Find-references counts the
        // super-dispatch as a reference, but a rename must never rewrite it —
        // the reference paths add `next` sites, the rename path must not.
        let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
        let analysis = analyse(src);
        // Cursor on `Sub::greet`'s declaration (line 5, col 11).
        let edits = rename(src, "tcl", 5, 11, "salute", &analysis, None);
        assert!(!edits.is_empty(), "{edits:?}");
        // The declaration (line 5, col 11) is rewritten; nothing in the `{ next
        // }` body region (col >= 20 on line 5) is.
        assert!(
            edits
                .iter()
                .all(|e| !(e.range.start_line == 5 && e.range.start_character >= 20)),
            "a rename edit landed on the `next` keyword: {edits:?}",
        );
    }

    #[test]
    fn rename_method_at_call_site_also_works() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` call site (line 2 col 25, after `my `).
        let edits = rename(src, "tcl", 2, 25, "salute", &analysis, None);
        assert!(edits.len() >= 3, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "salute");
        }
    }

    /// Regression: `rename_method` used to also run an unconditional
    /// bare-head scan that rewrote *any* command invocation whose head text
    /// matched the renamed method's name — including an unrelated builtin
    /// call that merely happens to share the name.  A `TclOO` method is
    /// never a bare-callable command (only `my <method>` dispatches it), so
    /// a bare call is always a call to something else and must never be
    /// touched by a method rename.
    #[test]
    fn rename_method_does_not_rewrite_unrelated_bare_command_with_same_name() {
        let src = "oo::class create C {\n    method format {} {}\n    method show {} { format %d 1 }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `format` method's declaration (line 1, col 11).
        let edits = rename(src, "tcl", 1, 11, "render", &analysis, None);
        // Only the declaration is renamed — the bare `format %d 1` call
        // inside `show` invokes the builtin, not this method.
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].range.start_line, 1);
    }

    /// Regression: methods, classmethods, and properties are independent
    /// tables, so a name shared by more than one (rare, but `TclOO` never
    /// merges them) must resolve to whichever declaration the cursor
    /// actually sits on — not always the same one by blind priority, which
    /// would silently rename the wrong declaration.
    #[test]
    fn rename_disambiguates_property_and_method_sharing_a_name_by_cursor() {
        let src = "oo::class create C {\n    property color\n    method color {} { return c }\n}\n";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        // Cursor on the *property* declaration (line 1, col 13).
        let property_edits = rename(src, "tcl9.0", 1, 13, "shade", &analysis, None);
        assert_eq!(
            property_edits.len(),
            1,
            "expected only the property decl: {property_edits:?}"
        );
        assert_eq!(property_edits[0].range.start_line, 1, "{property_edits:?}");
        // Cursor on the *method* declaration (line 2) picks the method
        // instead, leaving the same-named property untouched.
        let method_edits = rename(src, "tcl9.0", 2, 11, "shade", &analysis, None);
        assert!(
            method_edits.iter().any(|e| e.range.start_line == 2),
            "{method_edits:?}"
        );
        assert!(
            method_edits.iter().all(|e| e.range.start_line != 1),
            "must not touch the same-named property: {method_edits:?}"
        );
    }

    /// Regression: an instance `method` and a `classmethod` sharing a name
    /// (rare, but `TclOO` keeps them in independent tables, so it's legal)
    /// must never cross-link during rename — `my <word>` dispatch scope
    /// depends on which table the *caller's own body* belongs to (`self` is
    /// the class object inside a `classmethod`, the instance everywhere
    /// else), so renaming the classmethod must not touch a `my greet` site
    /// that actually dispatches to the unrelated instance method — doing so
    /// would corrupt otherwise-working code (the call would then reference
    /// a name that no longer exists on the instance).
    #[test]
    fn rename_classmethod_does_not_corrupt_unrelated_instance_method_call_site() {
        let src = "oo::class create C {\n    method greet {} {}\n    classmethod greet {} {}\n    method twice {} { my greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the *classmethod* declaration (line 2, col 16).
        let edits = rename(src, "tcl", 2, 16, "hail", &analysis, None);
        assert_eq!(
            edits.len(),
            1,
            "only the classmethod's own decl — `twice`'s `my greet` targets \
             the unrelated instance method and must be untouched: {edits:?}"
        );
        assert_eq!(edits[0].range.start_line, 2, "{edits:?}");
    }

    // `method_target_with_access`'s class-body fallback — the idx-113 link
    // gate (issue #1028).

    /// TP: the cursor on a method's own declaration name is the member.
    #[test]
    fn method_target_fires_on_the_members_own_declaration_name() {
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
        let analysis = analyse(src);
        let target = method_target_with_access(src, 1, 11, &analysis);
        assert_eq!(
            target,
            Some((
                "::Widget".to_owned(),
                "foo".to_owned(),
                false,
                crate::workspace_index::MethodAccess::Internal
            )),
            "cursor on the `foo` declaration must resolve to Widget's `foo`"
        );
    }

    /// TP: a bareword sibling call that `link` genuinely made callable is a
    /// reference to the member — tclsh9.0-verified (`link foo` in the
    /// constructor makes `[foo 42]` inside another method dispatch to
    /// `Widget`'s `foo`; `link` does not exist in 8.6's `TclOO`).
    #[test]
    fn method_target_fires_on_a_linked_bareword_call() {
        let src = "oo::class create Widget {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        let target = method_target_with_access(src, 3, 28, &analysis);
        assert_eq!(
            target,
            Some((
                "::Widget".to_owned(),
                "foo".to_owned(),
                false,
                crate::workspace_index::MethodAccess::Internal
            )),
            "a linked bareword sibling call must resolve to the member"
        );
    }

    /// FP guard (issue #1028): an **un-linked** bareword sibling call names
    /// nothing — real Tcl raises `invalid command name "foo"` there
    /// (tclsh9.0-verified) — so the resolver must abstain rather than hand
    /// the word to the workspace resolver, which used to answer it
    /// non-deterministically.
    #[test]
    fn method_target_abstains_on_an_unlinked_bareword_sibling_call() {
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
        let analysis = analyse(src);
        assert_eq!(
            method_target_with_access(src, 2, 28, &analysis),
            None,
            "an un-linked bareword sibling call is not a reference to the method"
        );
    }

    /// FP guard: a *parameter* that happens to share a sibling method's name
    /// is not that method either — the old fallback fired on any
    /// member-name-shaped word anywhere in the class body span.
    #[test]
    fn method_target_abstains_on_a_parameter_sharing_a_method_name() {
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {foo} { return $foo }\n}\n";
        let analysis = analyse(src);
        assert_eq!(
            method_target_with_access(src, 2, 16, &analysis),
            None,
            "a parameter word must not resolve to the same-named method"
        );
    }

    // property rename

    /// Regression: renaming a property must rewrite every `my <property>`
    /// call site alongside the declaration.  Properties have no `$obj
    /// prop` dispatch and no override family — but they *are* read via
    /// `my <property>` inside the class's own methods, and a bare
    /// `<property>` occurrence is never valid `TclOO` dispatch (only `my
    /// <property>` reads it), so the bare-head scan `rename_method` also
    /// runs never matches this shape — without the dedicated `my`-aware
    /// scan, only the declaration was renamed, leaving call sites pointing
    /// at the old name.
    #[test]
    fn rename_property_rewrites_decl_and_my_dispatch_call_sites() {
        let src = "oo::class create C {\n    property color\n    method describe {} {\n        return [my color]\n    }\n}\n";
        // `property` is Tcl 9.0+ — the shared `analyse()` helper fixes the
        // dialect at 8.6, so this test analyses at 9.0 directly.
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        // Cursor on the `color` property declaration (line 1, col 13).
        let edits = rename(src, "tcl9.0", 1, 13, "shade", &analysis, None);
        assert_eq!(edits.len(), 2, "decl + `my color` call site: {edits:?}");
        assert!(edits.iter().all(|e| e.new_text == "shade"));
        assert_eq!(edits[0].range.start_line, 1);
        assert_eq!(edits[1].range.start_line, 3);
    }

    /// FN→TP (issue #957's general form): a `my <property>` read nested
    /// inside `if` control flow is renamed too — the property rename path
    /// shares `scan_my_method_sites`'s control-flow recursion with methods.
    #[test]
    fn rename_property_rewrites_control_flow_nested_call_site() {
        let src = "oo::class create C {\n    property color\n    method describe {} {\n        if {1} {\n            return [my color]\n        }\n    }\n}\n";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        let edits = rename(src, "tcl9.0", 1, 13, "shade", &analysis, None);
        assert_eq!(
            edits.len(),
            2,
            "decl + nested `my color` call site: {edits:?}"
        );
        assert!(edits.iter().all(|e| e.new_text == "shade"));
        assert_eq!(edits[1].range.start_line, 4);
    }

    #[test]
    fn rename_method_skipped_outside_class_body() {
        // Cursor on bare `greet` outside the class — no class
        // method named `greet` is visible, so the rename
        // returns empty.
        let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 3, 2, "salute", &analysis, None);
        assert!(edits.is_empty(), "{edits:?}");
    }

    /// Regression for issue #957's general form: renaming a method must
    /// rewrite a `my method` call site nested inside `if` / `switch`
    /// control flow, not just a top-level or `[...]`-nested call — `rename`
    /// delegates to the same `method_references_for_class` resolver
    /// `references`/the code lens use, so the fix there covers rename too.
    #[test]
    fn rename_method_rewrites_control_flow_nested_my_dispatch() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        if {1} {\n            switch -- $k {\n                default {\n                    my getOptions $k\n                }\n            }\n        }\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `getOptions` declaration (line 1, col 11).
        let edits = rename(src, "tcl", 1, 11, "fetchOptions", &analysis, None);
        assert_eq!(edits.len(), 2, "decl + nested `my` site: {edits:?}");
        assert!(edits.iter().all(|e| e.new_text == "fetchOptions"));
        assert_eq!(edits[0].range.start_line, 1);
        // The nested call site, six lines further down.
        assert_eq!(edits[1].range.start_line, 6);
    }

    #[test]
    fn rename_method_rejects_unsafe_new_name() {
        let src = "oo::class create C {\n    method greet {} {}\n}\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 1, 11, "1bad", &analysis, None);
        assert!(edits.is_empty(), "{edits:?}");
    }

    #[test]
    fn prepare_rename_returns_range_for_method() {
        let src = "oo::class create C {\n    method greet {} {}\n}\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 1, 11, &analysis).expect("expected prepare_rename on method");
        assert_eq!(p.placeholder, "greet");
    }

    // external $obj method rename

    #[test]
    fn rename_method_from_decl_rewrites_external_obj_sites() {
        // Renaming `bark` from its declaration also rewrites the
        // external `$d bark` / `[$d bark]` call sites.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 1, 11, "yip", &analysis, None);
        // Declaration + 2 external sites = 3 edits, all "yip".
        assert!(edits.len() >= 3, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "yip");
        }
        // One edit on line 4 (`$d bark`) and one on line 5
        // (`[$d bark]`).
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&4) && lines.contains(&5), "{edits:?}");
    }

    #[test]
    fn rename_method_rewrites_bare_created_instance_command_site() {
        // `Dog create rex` binds `rex` as an object command, so renaming
        // `bark` must also rewrite the bare `rex bark` dispatch — the same
        // shared resolver drives the peek, the lens, and the rename.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 1, 11, "yip", &analysis, None);
        for e in &edits {
            assert_eq!(e.new_text, "yip");
        }
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&1), "decl not renamed: {edits:?}");
        assert!(
            lines.contains(&4),
            "bare `rex bark` site not renamed: {edits:?}"
        );
    }

    #[test]
    fn rename_classmethod_from_call_site_rewrites_decl_and_inheriting_subclass_call() {
        // TP — issue #923 idx 120: renaming `find` with the cursor on the
        // `ActiveRecord find foo bar` call site must rewrite the
        // declaration, that same call, AND the inherited `Table find`
        // call — three edits, none missed (rename.rs has no separate
        // scan of its own; it delegates fully to
        // `references::method_references_for_class`, so this exercises
        // Part 2 + Part 3 together end-to-end).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
        let analysis = analyse(src);
        // Cursor on `find` in `ActiveRecord find foo bar` (line 7, col 13).
        let edits = rename(src, "tcl", 7, 13, "lookup", &analysis, None);
        assert_eq!(edits.len(), 3, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "lookup");
        }
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&1), "decl not renamed: {edits:?}");
        assert!(lines.contains(&6), "Table find call not renamed: {edits:?}");
        assert!(
            lines.contains(&7),
            "ActiveRecord find call not renamed: {edits:?}"
        );
    }

    #[test]
    fn rename_method_from_cursor_on_bare_obj_command_call_site() {
        // Codex #881 (symmetry): triggering rename with the cursor ON the
        // `bark` token of a bare `rex bark` dispatch rewrites the declaration
        // and the call site — not only the declaration-triggered rename.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        // Line 4 `rex bark` — cursor on `bark` (col 4).
        let edits = rename(src, "tcl", 4, 4, "yip", &analysis, None);
        assert!(edits.len() >= 2, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "yip");
        }
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&1), "decl not renamed: {edits:?}");
        assert!(lines.contains(&4), "call site not renamed: {edits:?}");
    }

    #[test]
    fn rename_method_from_external_call_site() {
        // Triggering rename from the external `$d bark` site
        // rewrites the declaration + all sites.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        // Cursor on `bark` in `$d bark` (line 4, col 3).
        let edits = rename(src, "tcl", 4, 3, "yip", &analysis, None);
        assert!(edits.len() >= 2, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "yip");
        }
        // Declaration (line 1) is rewritten too.
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    }

    #[test]
    fn rename_method_external_rewrites_proc_body_sites() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { $d bark }\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 1, 11, "yip", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        // Declaration (1) + proc-body call (4).
        assert!(lines.contains(&1) && lines.contains(&4), "{edits:?}");
    }

    #[test]
    fn rename_var_from_definition_site_resolves_without_dollar() {
        // Cursor on the `x` in `set x 42` (no `$`) — the def-site
        // resolver must find the variable and rewrite decl + reads.
        let src = "set x 42\nputs $x\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 4, "newvar", &analysis, None);
        let texts: std::collections::HashSet<&str> =
            edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains("newvar"), "decl edit missing: {edits:?}");
        assert!(texts.contains("$newvar"), "ref edit missing: {edits:?}");
    }

    #[test]
    fn rename_var_preserves_namespace_qualifier_at_decl() {
        let src = "set myns::count 0\nputs $myns::count\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 10, "total", &analysis, None);
        let texts: std::collections::HashSet<&str> =
            edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains("myns::total"), "decl ns lost: {edits:?}");
        assert!(texts.contains("$myns::total"), "ref ns lost: {edits:?}");
    }

    #[test]
    fn rename_var_rejected_on_same_scope_collision() {
        let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
        let analysis = analyse(src);
        // Rename `x` (read site, line 3) to `y` which already exists in scope.
        let edits = rename(src, "tcl", 3, 10, "y", &analysis, None);
        assert!(edits.is_empty(), "collision not rejected: {edits:?}");
    }

    #[test]
    fn rename_proc_rejected_on_existing_proc_collision() {
        let src = "proc greet {} {}\nproc hello {} {}\ngreet\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl", 0, 6, "hello", &analysis, None);
        assert!(edits.is_empty(), "proc collision not rejected: {edits:?}");
    }

    // method rename across overrides

    #[test]
    fn rename_method_renames_subclass_override() {
        // `Animal::speak` overridden by `Dog::speak`.  Renaming from the
        // base declaration must rewrite *both* declarations — leaving the
        // override behind would silently break polymorphic dispatch.
        let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method speak {} { return woof }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `speak` in Animal's declaration (line 1 col 7).
        let edits = rename(src, "tcl", 1, 7, "vocalise", &analysis, None);
        assert!(edits.iter().all(|e| e.new_text == "vocalise"));
        // Both declarations rewritten: line 1 (Animal) and line 5 (Dog).
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&1) && lines.contains(&5),
            "expected both Animal (l1) and Dog override (l5) renamed; got {edits:?}",
        );
    }

    #[test]
    fn rename_method_renames_from_subclass_up_to_base() {
        // Symmetric to the above: renaming from the *subclass* override
        // must also rewrite the base declaration.
        let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method speak {} { return woof }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `speak` in Dog's declaration (line 5 col 7).
        let edits = rename(src, "tcl", 5, 7, "vocalise", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&1) && lines.contains(&5),
            "expected base (l1) renamed from subclass override; got {edits:?}",
        );
    }

    #[test]
    fn rename_method_renames_sibling_overrides_of_common_base() {
        // Two siblings override a common base method.  Renaming any one
        // must rewrite the base and *both* siblings — they are one
        // polymorphic name reachable via the base's static type.
        let src = "oo::class create Shape {\n\
                       method area {} { return 0 }\n\
                   }\n\
                   oo::class create Circle {\n\
                       superclass Shape\n\
                       method area {} { return 3 }\n\
                   }\n\
                   oo::class create Square {\n\
                       superclass Shape\n\
                       method area {} { return 4 }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `area` in Circle (line 5 col 7).
        let edits = rename(src, "tcl", 5, 7, "measure", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        // Shape (l1), Circle (l5), Square (l9) all rewritten.
        assert!(
            lines.contains(&1) && lines.contains(&5) && lines.contains(&9),
            "expected Shape + both siblings renamed; got {edits:?}",
        );
    }

    #[test]
    fn rename_method_leaves_unrelated_same_name_method_untouched() {
        // Two classes in *disjoint* hierarchies both define `run`.
        // Renaming one must not touch the other.
        let src = "oo::class create Engine {\n\
                       method run {} { return e }\n\
                   }\n\
                   oo::class create Task {\n\
                       method run {} { return t }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `run` in Engine (line 1 col 7).
        let edits = rename(src, "tcl", 1, 7, "start", &analysis, None);
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&1),
            "Engine::run should be renamed: {edits:?}"
        );
        assert!(
            !lines.contains(&4),
            "unrelated Task::run must NOT be renamed; got {edits:?}",
        );
    }

    #[test]
    fn rename_inherited_method_from_external_obj_site() {
        // `$d speak` where `d` is a `Dog` that *inherits* `speak` from
        // `Animal` (no override).  Renaming from the external call site
        // must rewrite the base declaration (previously produced nothing).
        let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                   }\n\
                   set d [Dog new]\n\
                   $d speak\n";
        let analysis = analyse(src);
        // Cursor on `speak` in `$d speak` (line 7 col 3).
        let edits = rename(src, "tcl", 7, 3, "vocalise", &analysis, None);
        assert!(
            !edits.is_empty(),
            "inherited-method rename produced nothing"
        );
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&1),
            "expected base Animal::speak decl (l1) renamed; got {edits:?}",
        );
    }

    #[test]
    fn rename_inherited_method_rewrites_subclass_instance_obj_sites() {
        // Renaming an inherited method from the *base declaration* must also
        // rewrite `$d method` sites where `$d` is an instance of a subclass
        // that inherits (does not override) it — the exact-class-equality
        // filter used to drop these, leaving them pointing at the old name.
        let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                   }\n\
                   set d [Dog new]\n\
                   $d speak\n";
        let analysis = analyse(src);
        // Cursor on `speak` in Animal's declaration (line 1 col 11).
        let edits = rename(src, "tcl", 1, 11, "vocalise", &analysis, None);
        assert!(edits.iter().all(|e| e.new_text == "vocalise"));
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&1) && lines.contains(&7),
            "expected decl (l1) + inheriting-subclass instance site (l7); got {edits:?}",
        );
    }

    #[test]
    fn rename_inherited_method_rewrites_pure_inheritor_my_sites() {
        // A subclass that inherits `speak` (no override) calls `my speak`
        // from one of its own method bodies.  That call dispatches to the
        // base definition, so renaming the base must rewrite it — even
        // though the subclass is not itself a rename family member.
        let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method describe {} { my speak }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `speak` in Animal's declaration (line 1 col 11).
        let edits = rename(src, "tcl", 1, 11, "vocalise", &analysis, None);
        assert!(edits.iter().all(|e| e.new_text == "vocalise"));
        let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
        assert!(
            lines.contains(&1) && lines.contains(&5),
            "expected decl (l1) + `my speak` in inheriting subclass (l5); got {edits:?}",
        );
    }

    /// TP — the `export` list is part of the rename.
    ///
    /// Oracle (tclsh 9.0.4 and 8.6.16, identical): with `method Foo` +
    /// `export Foo`, `[$a Foo]` prints `foo`.  Rename `Foo` → `Bar` touching
    /// only the declaration and the call site, leaving `export Foo` behind,
    /// and both interpreters answer
    /// `unknown method "Bar": must be destroy` — the renamed method is no
    /// longer exported.  A `method` whose name begins with an upper-case
    /// letter is unexported by default (probed on 9.0.4: `$a Foo` errors,
    /// `$a bar` works), which is exactly why the corpus writes the list.
    #[test]
    fn tp_rename_method_rewrites_its_export_list() {
        let src = "oo::class create A {\n    method Foo {} { return 1 }\n    export Foo\n}\nset a [A new]\nputs [$a Foo]\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl8.6", 1, 12, "Bar", &analysis, None);
        let applied = apply_edits(src, &edits);
        assert!(
            applied.contains("export Bar"),
            "the export list must be renamed with the method: {applied}"
        );
        assert!(
            !applied.contains("Foo"),
            "no stale name may survive: {applied}"
        );
    }

    /// TP — an `export` written in a separate `oo::define` block is found
    /// through the definer grammar, not the class's own body span.
    #[test]
    fn tp_rename_method_rewrites_an_export_in_a_separate_oo_define_block() {
        let src = "oo::class create A {\n    method Foo {} { return 1 }\n}\noo::define A {\n    export Foo\n}\n";
        let analysis = analyse(src);
        let edits = rename(src, "tcl8.6", 1, 12, "Bar", &analysis, None);
        let applied = apply_edits(src, &edits);
        assert!(
            applied.contains("export Bar"),
            "an `oo::define` export list must rename too: {applied}"
        );
    }

    /// TP + TN, issue #981's object-command half.
    ///
    /// Oracle (tclsh 9.0.4 / 8.6.16, identical): `::a::Factory create rex`
    /// and `::b::Widget create rex` bind two different commands; `rex make`
    /// inside `::a` prints `a-made` and inside `::b` prints `b-made`, and
    /// `::a::rex make` / `::b::rex make` both work from the top level.
    /// Renaming `::b::Widget::make` → `produce` and also rewriting `::a`'s
    /// call site makes both interpreters fail with
    /// `unknown method "produce": must be destroy or make` at `rex produce`
    /// inside `::a` — which is exactly what the LSP emitted before this fix.
    #[test]
    fn tn_object_command_dispatch_is_scoped_to_its_creation_namespace() {
        let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory {\n\
                   \x20       method make {} { return \"a-made\" }\n\
                   \x20   }\n\
                   \x20   Factory create rex\n\
                   \x20   puts [rex make]\n\
                   }\n\
                   namespace eval ::b {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method make {} { return \"b-made\" }\n\
                   \x20   }\n\
                   \x20   Widget create rex\n\
                   \x20   puts [rex make]\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `make` in `::b::Widget`'s declaration.
        let edits = rename(src, "tcl8.6", 9, 15, "produce", &analysis, None);
        let applied = apply_edits(src, &edits);
        assert!(
            applied.contains("method produce {} { return \"b-made\" }"),
            "::b's declaration must rename: {applied}"
        );
        assert!(
            applied.contains("method make {} { return \"a-made\" }"),
            "::a's declaration must be untouched: {applied}"
        );
        // `::a`'s own call site must survive verbatim; `::b`'s must rename.
        let a_call = applied
            .split("namespace eval ::b")
            .next()
            .expect("split keeps the ::a half");
        assert!(
            a_call.contains("puts [rex make]"),
            "::a's `rex make` must not be rewritten (issue #981): {applied}"
        );
        assert!(
            applied
                .split("namespace eval ::b")
                .nth(1)
                .is_some_and(|b| b.contains("puts [rex produce]")),
            "::b's own `rex make` must rename (the finding's lost-site half): {applied}"
        );
    }

    /// TP — the namespace-variable cell's declaration, its proc-local
    /// `variable` alias, that alias's unqualified reads, and every qualified
    /// occurrence rename as one unit.
    ///
    /// Oracle (tclsh 9.0.4 / 8.6.16, identical): the script prints `1` before
    /// and after the complete rename; renaming only the declaration and the
    /// qualified read leaves `p` reading a cell that no longer exists.
    #[test]
    fn tp_namespace_variable_rename_covers_declaration_alias_and_qualified_use() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} {\n\
                   \x20       variable v\n\
                   \x20       return $v\n\
                   \x20   }\n\
                   }\n\
                   puts $::ns::v\n\
                   puts [::ns::p]\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(applied.contains("variable total 1"), "{applied}");
        assert!(applied.contains("        variable total\n"), "{applied}");
        assert!(applied.contains("return $total"), "{applied}");
        assert!(applied.contains("puts $::ns::total"), "{applied}");
        assert!(
            !applied.contains("$v\n"),
            "no stale read may survive: {applied}"
        );
    }

    /// FP (Codex review of PR #1091, finding 3) — `namespace upvar ::ns v
    /// local` names the cell in its *third* word; `local` is an independent
    /// local spelling.  Rewriting the alias token and leaving the target word
    /// behind re-points the alias at a cell that no longer exists.
    ///
    /// Oracle (tclsh 9.0.4 / 8.6.16, byte-identical):
    ///   before                         -> `p -> 42`, `cell -> 42`, rc 0
    ///   `namespace upvar ::ns v total; … $total` (alias rewritten, target
    ///   word left)                     -> `can't read "total": no such
    ///                                     variable` at `"expr {$total + 41}"`,
    ///                                     rc 1
    ///   `namespace upvar ::ns total local; … $local` (target word rewritten,
    ///   local spelling kept)           -> `p -> 42`, `cell -> 42`, rc 0
    #[test]
    fn fp_namespace_variable_rename_keeps_a_differently_named_alias_spelling() {
        let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   namespace upvar ::ns v local\n\
                   \x20   set local [expr {$local + 41}]\n\
                   \x20   return $local\n\
                   }\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(applied.contains("variable total 1"), "{applied}");
        assert!(
            applied.contains("namespace upvar ::ns total local"),
            "the word naming the cell is the word rewritten: {applied}"
        );
        assert!(
            applied.contains("set local [expr {$local + 41}]") && applied.contains("return $local"),
            "the local spelling and its reads must survive verbatim: {applied}"
        );
        assert!(
            !applied.contains("$total"),
            "no read may be re-spelled to the cell's new name: {applied}"
        );
    }

    /// TN (same finding) — a *same-spelled* `namespace upvar ::ns v v` is
    /// still two independent words.  Both interpreters accept either answer
    /// (`namespace upvar ::ns total v; … $v` and `namespace upvar ::ns total
    /// total; … $total` both print `42` on 9.0.4 and 8.6.16), so the minimal
    /// edit is taken: rewrite the word that names the cell, and leave the
    /// local spelling — whose meaning the rename does not change — alone.
    #[test]
    fn tn_namespace_variable_rename_takes_the_minimal_edit_for_a_same_spelled_alias() {
        let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   namespace upvar ::ns v v\n\
                   \x20   return $v\n\
                   }\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(applied.contains("variable total 1"), "{applied}");
        assert!(
            applied.contains("namespace upvar ::ns total v"),
            "only the cell-naming word changes: {applied}"
        );
        assert!(
            applied.contains("return $v"),
            "the local read is unaffected by the cell's name: {applied}"
        );
    }

    /// TP (same finding) — `upvar #0 ::ns::v local` is the other
    /// differently-named shape, and resolves the same way: the `otherVar`
    /// word is qualified, so the rewrite must preserve its qualifier.
    #[test]
    fn tp_namespace_variable_rename_rewrites_a_global_upvar_target_word() {
        let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   upvar #0 ::ns::v local\n\
                   \x20   return $local\n\
                   }\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(applied.contains("variable total 1"), "{applied}");
        assert!(
            applied.contains("upvar #0 ::ns::total local"),
            "the qualified target word keeps its qualifier: {applied}"
        );
        assert!(
            applied.contains("return $local"),
            "the local spelling survives: {applied}"
        );
    }

    /// TP (same finding) — `global ::ns::v` is the *same*-word shape: its one
    /// declaration word both names the cell and introduces the local alias
    /// (whose name is the cell's tail), so the rename must rewrite the word
    /// **and** every unqualified read it enables.
    ///
    /// Oracle (9.0.4 / 8.6.16): leaving the reads behind gives `can't read
    /// "v": no such variable`.
    #[test]
    fn tp_namespace_variable_rename_rewrites_a_global_alias_and_its_reads() {
        let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   global ::ns::v\n\
                   \x20   return $v\n\
                   }\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(applied.contains("variable total 1"), "{applied}");
        assert!(applied.contains("global ::ns::total"), "{applied}");
        assert!(
            applied.contains("return $total"),
            "the local spelling is the cell's tail, so it travels: {applied}"
        );
    }

    /// TN — a same-tailed cell in a *different* namespace is a different
    /// variable and must be untouched (`$other::v` never searches enclosing
    /// namespaces; tclsh 9.0.4 / 8.6.16 keep the two independent).
    #[test]
    fn tn_namespace_variable_rename_leaves_a_same_tailed_sibling_cell_alone() {
        let src = "namespace eval ::a {\n    variable v 1\n}\n\
                   namespace eval ::b {\n    variable v 2\n}\n\
                   puts $::a::v\nputs $::b::v\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::a::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(applied.contains("puts $::a::total"), "{applied}");
        assert!(
            applied.contains("puts $::b::v"),
            "::b's cell must survive: {applied}"
        );
        assert!(
            applied.contains("namespace eval ::b {\n    variable v 2\n}"),
            "::b's declaration must survive: {applied}"
        );
    }

    /// FP guard — renaming a cell onto a name the same namespace already
    /// declares would merge two distinct variables, so the whole rename
    /// answers nothing (the same discipline the single-document variable
    /// rename's scope-chain collision check applies).
    #[test]
    fn fp_namespace_variable_rename_refuses_a_collision_in_its_own_namespace() {
        let src =
            "namespace eval ::ns {\n    variable v 1\n    variable total 2\n}\nputs $::ns::v\n";
        let analysis = analyse(src);
        assert!(
            namespace_variable_rename_edits(src, &analysis, "::ns::v", "total").is_empty(),
            "renaming `v` onto the existing `total` must produce no edits"
        );
    }

    /// TN — an unqualified local that merely shares the tail name is not the
    /// cell, so it is not renamed.
    #[test]
    fn tn_namespace_variable_rename_leaves_an_unrelated_local_alone() {
        let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc other {} {\n    set v 9\n    return $v\n}\n\
                   puts $::ns::v\n";
        let analysis = analyse(src);
        let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
        let applied = apply_edits(src, &edits);
        assert!(
            applied.contains("set v 9"),
            "the proc local must survive: {applied}"
        );
        assert!(applied.contains("return $v"), "{applied}");
        assert!(applied.contains("puts $::ns::total"), "{applied}");
    }

    /// FP guard, end of the in-document path: the idx-79 shape refuses with a
    /// reason rather than emitting the declaration-only edit set that broke
    /// the program on both interpreters.
    #[test]
    fn fp_rename_refuses_the_untracked_receiver_dispatch() {
        let src = "oo::class create Vector3d {\n\
                   \x20   variable _x\n\
                   \x20   constructor {args} {\n\
                   \x20       set other [lindex $args 0]\n\
                   \x20       set _x [$other X]\n\
                   \x20   }\n\
                   \x20   method X {} { return $_x }\n\
                   }\n";
        let analysis = analyse(src);
        let err = rename_with_diagnosis(src, "tcl8.6", 6, 11, "GetX", &analysis, None)
            .expect_err("idx 79's shape must refuse");
        assert!(err.reason.contains("not tracked"), "{}", err.reason);
        assert!(err.range.is_some(), "the refusal must point at the site");
        // And the plain entry point must produce no edits at all — never a
        // partial set.
        assert!(rename(src, "tcl8.6", 6, 11, "GetX", &analysis, None).is_empty());
    }
}
