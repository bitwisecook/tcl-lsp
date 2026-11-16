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

//! The class and method bucket of the receiver at the cursor, and `next`/`nextto` targets.

#[cfg(test)]
use super::utf16_col_to_char_col;
use crate::original_context::CurrentSourcePosition;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

/// The qualified name of the class whose body contains `offset`, if any —
/// the enclosing class for `my`-call dispatch and every other "which class
/// am I lexically inside" query in this crate.
///
/// Consults [`AnalysisResult::class_body_spans`] rather than
/// `ClassDef::body_span` alone: a class extended via a *separate*
/// `oo::define ClassName { ... }` block (real corpus shape:
/// `ticklecharts::chart`'s `oo::class create` at one line, every method
/// added via a later, separate `oo::define` block) has textually disjoint
/// body spans, not one contiguous range — `body_span` alone only ever
/// covers the *first* one recorded.  Ties (nested definitions) resolve to
/// the narrowest containing span.
///
/// The single canonical implementation of this check: every other module in
/// this crate calls it rather than keeping its own copy, so the multi-span
/// rule applies everywhere uniformly.
pub(crate) fn enclosing_class_at(analysis: &AnalysisResult, offset: u32) -> Option<&str> {
    analysis
        .class_body_spans
        .iter()
        .filter(|(_, span)| span.start() < offset && offset < span.end())
        .min_by_key(|(_, span)| span.end() - span.start())
        .map(|(name, _)| name.as_str())
}

/// Which of a class's two method buckets a dispatch receiver reaches,
/// consulted by [`method_dispatch_definition`] and,
/// via [`logical_receiver_candidate_at`], by every other consumer of
/// [`logical_receiver_candidate_at`] that also needs its own method lookup
/// restricted to the matching bucket (`completion.rs`'s `method_items`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodBucket {
    /// `$obj method` / `my method` — dispatch on an *instance*:
    /// `methods` only, never `class_methods` (a class-side method is
    /// never itself instance-callable — real tclsh raises `unknown
    /// method` for it, so `ActiveRecord create rec1; rec1 find` must not
    /// resolve to the class-side `find`).
    Instance,
    /// `CLASS method` — dispatch on the class's own bound command:
    /// `class_methods` only (a class-side method is never
    /// itself instance-callable — `completion.rs`'s `method_items` already
    /// documents and implements this same exclusion for its instance-side
    /// suggestions). An ancestor-provided entry is accepted only when it
    /// is NOT `is_self_method` — a plain `TclOO` `self method` is visible
    /// only on the exact class that declared it (confirmed against
    /// tclsh: a subclass with no override does not gain it), unlike
    /// `ooutil`'s `classmethod` keyword, which does propagate to a
    /// subclass's own bound command via its `Delegate`-mixin machinery. A
    /// provider matching the receiver class itself is always accepted
    /// regardless (it's not "inherited" there, it's the direct owner).
    Class,
}

/// Resolve `TclOO` `next` / `nextto` at the cursor to the super-method's
/// `name_span`.
///
/// `next` inside `method m` of class `C` dispatches `m` one step further
/// down the object's MRO, resolved in `C`'s own MRO (the sound
/// single-dispatch approximation): the next class after `C` that provides
/// `m`.  `nextto Base` restarts the search at `Base`.  The enclosing class
/// and method are found from the cursor's byte offset.
pub(super) fn next_dispatch_target(
    analysis: &AnalysisResult,
    source: &str,
    _line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<tcl_lexer::Span> {
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let input = analysis.resolved_input.as_ref()?;
    let (class_q, method, body) =
        tcl_compiler::analyser::class_hierarchy::original_source_member_at(
            analysis,
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config(),
            cursor,
        )?;
    let (_, target) =
        crate::references::scan_next_dispatch_sites_with_target(source, analysis, body)
            .into_iter()
            .find(|(head, _)| head.start() <= cursor && cursor < head.end())?;
    let start_from = match target {
        Some(target) => Some(canonicalise_class(analysis, cursor, &target)?),
        None => None,
    };
    let hierarchy = analysis.class_hierarchy();
    let next_class = hierarchy.member_next_provider(
        &class_q,
        &method,
        &class_q,
        start_from.as_deref(),
        source,
    )?;
    let cd = analysis.all_classes.get(next_class)?;
    tcl_compiler::analyser::class_hierarchy::class_member_def(cd, &method).map(|md| md.name_span)
}

/// Canonicalise a written `nextto` class name to the qualified form keyed in
/// `all_classes`, or `None` when the name does not resolve unambiguously.
///
/// A `TclOO` member body resolves a command word through the object's own
/// namespace, the registry's implicit member-body path, then `::` — never the
/// class's declaring namespace (tclsh 8.6.18 / 9.0.4: `nextto A` inside
/// `::Ns::B` raises `A does not refer to an object`). A class created inside
/// the object namespace at run time is not distinguished from a root class,
/// so the root lookup answers unless the name reaches a class under a path
/// namespace first; that case is refused rather than guessed. Outside a
/// member body the written scope resolves it.
pub(crate) fn canonicalise_class(analysis: &AnalysisResult, at: u32, name: &str) -> Option<String> {
    use tcl_compiler::analyser::class_hierarchy::{
        resolve_written_class_name, resolve_written_class_name_in_scope, source_namespace_at,
    };
    let path = tcl_compiler::analyser::scope::implicit_command_namespace_path_at(
        &analysis.global_scope,
        at,
    );
    if path.is_empty() {
        let namespace = source_namespace_at(&analysis.global_scope, at)?;
        return resolve_written_class_name_in_scope(name, namespace, &analysis.all_classes);
    }
    let shadowed = !tcl_syntax::naming::is_absolute(name)
        && path.iter().any(|namespace| {
            resolve_written_class_name(
                &tcl_syntax::naming::qualify(namespace, name),
                &analysis.all_classes,
            )
            .is_some()
        });
    if shadowed {
        return None;
    }
    resolve_written_class_name(name, &analysis.all_classes)
}

/// Current complete command-head and selector syntax. Actual receiver identity,
/// method visibility and source edits require their independent owners.
pub(crate) fn original_instance_method_at_cursor(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<(String, String, bool)> {
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    crate::source_structure::original_method_site_at(source, analysis, cursor)
}

#[cfg(test)]
pub(crate) fn instance_method_at_cursor(
    source: &str,
    line: u32,
    character: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, String, bool)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());

    // Method-name word bounds around the cursor.
    let (wstart, wend) = crate::hover::word_char_bounds(&chars, col)?;
    let method: String = chars[wstart..wend].iter().collect();
    if !method.chars().any(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    // A complete variable at this line's head already has canonical geometry.
    // FirstClose names may contain a literal `{`; it is not an enclosing script.
    let line_prefix: String = chars[..wstart].iter().collect();
    let line_head = line_prefix.trim();
    let sole_line_head = tcl_lexer::word_parts::whole_var_ref(line_head.as_bytes(), config)
        .ok()
        .flatten()
        .map(|_| line_head);

    // Command-segment start: nearest *unmatched* `;` / `[` / `{` to the
    // left, else the line start. Depth-tracked — a `]`/`}`
    // seen first means the following `[`/`{` closes a balanced
    // substitution that is itself part of the head (`[self] m`'s own
    // brackets), not a boundary to stop at; only a `[`/`{` with no
    // pending closer is a genuine enclosing scope.
    let mut seg_start = 0;
    let mut depth: i32 = 0;
    for i in (0..wstart).rev() {
        match chars[i] {
            ']' | '}' => depth += 1,
            '[' | '{' if depth > 0 => depth -= 1,
            '[' | '{' => {
                seg_start = i + 1;
                break;
            }
            ';' if depth == 0 => {
                seg_start = i + 1;
                break;
            }
            _ => {}
        }
    }
    // The head must be exactly one whitespace-delimited token (the
    // receiver), so the method is word-index 1 — *except* a command
    // substitution, which is one token by Tcl's own quoting rules even
    // though it can contain internal whitespace (`[self object]`):
    // checked as a whole before falling back to
    // `split_whitespace`, which would otherwise split it in two.
    let prefix: String = chars[seg_start..wstart].iter().collect();
    let trimmed = prefix.trim();
    let head = if let Some(head) = sole_line_head {
        head
    } else if tcl_compiler::value_shapes::parse_command_substitution_with_config(trimmed, config)
        .is_some()
    {
        trimmed
    } else {
        let head_tokens: Vec<&str> = prefix.split_whitespace().collect();
        if head_tokens.len() != 1 {
            return None;
        }
        head_tokens[0]
    };
    if head.starts_with('$') {
        // A complete sole substitution is source geometry, not receiver identity.
        // Its configured closer must consume the whole head before a reporting
        // name can reach the independent method/source owner below.
        tcl_lexer::word_parts::whole_var_ref(head.as_bytes(), config).ok()??;
        let inst = tcl_syntax::naming::var_reference_for_style(head, config.braced_var);
        if inst.is_empty() {
            return None;
        }
        Some((inst.to_string(), method, true))
    } else if tcl_compiler::value_shapes::parse_command_substitution_with_config(head, config)
        .is_some()
    {
        // A well-formed single command substitution (`[cmd ?args?]`) — e.g.
        // `TclOO`'s `[self]`/`[self object]`, a receiver the registry
        // resolves at compile time regardless of the object's runtime name
        // This function only extracts the receiver text;
        // whether `head` names anything real is answered downstream by the
        // original selected descriptor (`original_internal_dispatch`)
        // exactly as for a bareword receiver below — most substitutions
        // resolve to nothing there, which is the correct outcome, not a
        // reason to reject the shape here.
        Some((head.to_string(), method, false))
    } else {
        // Bare `objcmd` receiver — a plain word naming an object command.
        // Any other decorated head (`{…}`, quoted, an ill-formed `[…]`) is
        // not a bare object command; `logical_receiver_candidate_at` further
        // gates this on `created_instance_commands`.
        if head.is_empty() || head.contains(['[', ']', '{', '}', '"', '(', ')', '$']) {
            return None;
        }
        Some((head.to_string(), method, false))
    }
}

/// Logical method-completion candidates retain their class and bucket together.
/// Bare names require an authored instance-command declaration or class name;
/// a variable receiver requires its positioned unique class projection. Native
/// source construction and temporal receiver identity retain separate owners.
pub(crate) fn logical_receiver_candidate_at<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    is_dollar: bool,
    offset: u32,
) -> Option<(&'a String, MethodBucket)> {
    // Logical declaration advice is the sole compatibility ingress. Native
    // source construction and current receiver identity retain separate owners.
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    if is_dollar {
        return lattice_singleton_class(analysis, receiver, offset)
            .map(|class| (class, MethodBucket::Instance));
    }
    if analysis.created_instance_commands.contains(receiver)
        && let Some(class) = analysis.instance_classes.get(receiver)
    {
        return Some((class, MethodBucket::Instance));
    }
    let qualified = tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(
        receiver,
        &analysis.all_classes,
    )?;
    analysis
        .all_classes
        .get(&qualified)
        .map(|class| (&class.qualified_name, MethodBucket::Class))
}

/// The unique class of the original variable read containing `offset`, or
/// `None` when its reaching contents are unknown or admit multiple classes.
pub(crate) fn lattice_singleton_class<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    offset: u32,
) -> Option<&'a String> {
    let classes = analysis
        .object_handle_facts
        .classes_in_scope(offset, receiver)?;
    if classes.len() != 1 {
        return None;
    }
    classes.iter().next()
}
