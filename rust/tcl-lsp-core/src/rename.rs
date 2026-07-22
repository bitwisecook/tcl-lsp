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
        if let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
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
    // Variable definition site (`set x` / `variable x`) — no `$`, so resolve
    // by the declaration span covering the cursor.
    let def_byte = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(var_name) =
        crate::definition::var_name_at_definition_offset(&analysis.global_scope, def_byte)
        && let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            def_byte,
            &var_name,
            analysis.ns_var_global_fallback(),
        )
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
    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        if let Some((_, span)) = resolve_member_span(class_def, &word, cursor_offset) {
            return Some(PrepareRename {
                range: span_to_range(source, &line_index, span),
                placeholder: word.clone(),
            });
        }
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
    // Shape gate first — applies to every rename target.
    if !is_safe_symbol_name(new_name) {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);

    if let Some(var_name) = find_var_at_position(source, line, character) {
        return rename_var(
            source,
            line,
            character,
            new_name,
            analysis,
            &line_index,
            &var_name,
        );
    }

    // Definition-site rename: the cursor sits on a `set x` / `variable x`
    // declaration name (no `$`), so the `$ref` scan above missed it.  Resolve
    // the variable by the declaration span that covers the cursor.
    let def_byte = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(var_name) =
        crate::definition::var_name_at_definition_offset(&analysis.global_scope, def_byte)
    {
        return rename_var(
            source,
            line,
            character,
            new_name,
            analysis,
            &line_index,
            &var_name,
        );
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
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
        return edits;
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
        return edits;
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
            class_q,
            &method,
            new_name,
            analysis,
            &line_index,
        )
    {
        return edits;
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
        return edits;
    }
    Vec::new()
}

/// Rename `method` across the class identified by `class_q` **and every
/// other class in its override family** — rewriting each class's
/// declaration name span, its intra-class `my method` call sites, and its
/// external `$obj method` call sites.  Returns `None` when neither
/// `class_q` nor any ancestor provides `method` (nothing to rename).
///
/// A `TclOO` method that is (re)defined by a super- or sub-class is a
/// single polymorphic name: `$obj method` dispatch can reach any
/// definition along the chain, so renaming only the class under the
/// cursor would silently break the override relationship.  See
/// [`override_family`].  Shared by the in-class-body and external
/// `$obj method` rename entry points.
fn rename_method_in_class(
    source: &str,
    dialect: &str,
    class_q: &str,
    method: &str,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    let family = override_family(analysis, class_q, method);
    if family.is_empty() {
        return None;
    }
    let mut edits = Vec::new();
    for member in &family {
        // Only ever reached via an external `$obj method` call site (see the
        // call site below), which is always an instance-method receiver —
        // never a classmethod, which is called on the class object itself
        // (confirmed against tclsh 9.0.4, see
        // `tcl_compiler::analyser::diagnostics::var_command`'s dispatch-scope
        // note).
        let Some((decl_span, call_spans)) = crate::references::method_references_for_class(
            source, dialect, analysis, member, method, false,
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
        if inst == "my"
            && let Some(class_q) = analysis
                .all_classes
                .values()
                .filter(|c| c.body_span.start() < cursor && cursor < c.body_span.end())
                .min_by_key(|c| c.body_span.end() - c.body_span.start())
                .map(|c| c.qualified_name.clone())
        {
            return Some((class_q, method, false, MethodAccess::Internal));
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
    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        let has_method = class_def.methods.contains_key(&word);
        let has_classmethod = class_def.class_methods.contains_key(&word);
        if body.start() < cursor && cursor < body.end() && (has_method || has_classmethod) {
            // A `method` and a `classmethod` of the same name are distinct
            // members; unambiguous when only one exists, otherwise prefer
            // whichever one's declaration the cursor is actually on (falls
            // back to the method when the cursor is elsewhere in the body,
            // e.g. a `my word` call site — the pre-existing preference).
            let is_classmethod = has_classmethod
                && (!has_method
                    || class_def.class_methods.get(&word).is_some_and(|m| {
                        m.name_span.start() <= cursor && cursor < m.name_span.end()
                    }));
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
    let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
        &analysis.global_scope,
        byte_offset,
        var_name,
        analysis.ns_var_global_fallback(),
    ) else {
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
            range: span_to_range(source, line_index, r),
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

    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
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
        return Some(edits);
    }
    None
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
    fn rename_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(rename(src, "tcl", 0, 6, "x", &analysis, None).is_empty());
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
}
