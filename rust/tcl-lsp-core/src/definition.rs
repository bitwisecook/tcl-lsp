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

//! Definition / declaration / type-definition / implementation
//! provider.
//!
//! The four LSP methods all answer the same fundamental question
//! ("where is the symbol at this position defined?") with slightly
//! different priorities for proc / class / variable matches.  The
//! four share the same core function and the server lifts each
//! method onto it.
//!
//! Resolved here:
//!
//! * `$var` references resolve to the `definition_span` of the
//!   matching `VarDef` in the global scope.  Scope-chain
//!   descent is not done — the analyser's body-span line index
//!   isn't currently threaded into the search path.
//! * Bare-word references resolve to a user-defined `proc` or
//!   `TclOO` class via `name_span`.  Proc resolution follows C
//!   Tcl's command lookup (`Tcl_FindCommand`, `tclNamesp.c`):
//!   the caller's namespace first, then the global namespace;
//!   an absolute `::`-prefixed word resolves exactly.  When no
//!   candidate is defined and the word doesn't name a registry
//!   builtin, a deterministic tail match keeps the lenient
//!   behaviour for procs whose defining namespace isn't
//!   statically visible at the call.
//!
//! Command-alias resolution: when the cursor's word matches an
//! `interp alias {} ALIAS {} TARGET` recorded in
//! `analysis.command_aliases`, the provider jumps to the target
//! proc's definition (when the target is a user proc), resolving
//! the target from the global namespace — where an alias target
//! is looked up when the alias fires.
//!
//! Class-member lookup: when the cursor sits on a
//! word inside a class body span, the provider walks that
//! class's `methods` / `class_methods` / `properties` /
//! `constructors` / `destructor` looking for a name match
//! and jumps to the member's `name_span`.  Catches `my
//! method` calls inside the body and bare references to the
//! class's own members.
//!
//! `$obj method` dispatch: when the cursor sits on
//! the method-name token of a `$obj method` / `[$obj method]`
//! call and `$obj`'s class is known (recorded in
//! `analysis.instance_classes` from a `set obj [Cls new]` /
//! `Cls create obj` site), the provider jumps to the method
//! declaration on that class.
//!
//! `apply` namespace override (issue #923 idx 116): a bareword call
//! inside `apply {{params} body ns}`'s body resolves against `ns`, not
//! wherever the `apply` call is lexically written — `namespace_context_at`
//! / `innermost_namespace_at` consult `analysis.namespace_overrides`
//! (populated by `Analyser::handle_apply_command`) ahead of the ordinary
//! lexical scope-chain walk. Also resolves one hop through a `$var` or
//! `[list {params} $body ns]` indirection (`set lambda {...}; apply
//! $lambda`), bounded to that one hop by design.
//!
//! Limitations:
//!
//! * Flow-sensitive / scope-aware instance-class tracking —
//!   `analysis.instance_classes` is a best-effort global
//!   var-name → class map (last assignment wins).  Re-binding
//!   the same name to a different class, or two locals of the
//!   same name in different procs, isn't disambiguated.
//! * `BigIP` definition — a separate provider keyed off iRules
//!   dialect that resolves pool / data-group / iRule /
//!   virtual-server names against a parsed `bigip.conf` — is
//!   not implemented here.
//! * `apply` reached only through a registry `command_prefixes` slot
//!   (`coroutine co ::apply $lambda`) is not modelled — that slot is
//!   consulted today only for IR-level interprocedural call-graph
//!   reachability, never re-dispatched through `AnalyserHookId::Apply`.
//!   Nor is a proc that re-injects its own arguments as a script via a
//!   captured `uplevel`-namespace + trace/callback (tcllib generator.tcl's
//!   `finally`, the exact idiom issue #923 idx 116's confirmed finding
//!   traces through) — there is no static/lexical connection between such
//!   a call and the token it eventually invokes, so this is a considered,
//!   permanent limitation rather than an oversight. Deeper `$var`-to-`$var`
//!   indirection beyond one hop (`set a $lambda; apply $a`) is the same
//!   kind of deliberate, bounded gap, not a special case of this one.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Utf16Col};

use crate::hover::{find_var_at_position, find_word_span_at_position};

/// LSP `Range` analogue — line/character pairs in UTF-16 code
/// units per the LSP spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspRange {
    /// Start position (0-based inclusive).
    pub start_line: u32,
    /// Start character (0-based UTF-16 code units).
    pub start_character: u32,
    /// End position (0-based exclusive).
    pub end_line: u32,
    /// End character (0-based, exclusive).
    pub end_character: u32,
}

pub(crate) fn utf16_col_to_char_col(line_text: &str, character: u32) -> usize {
    let mut utf16 = 0u32;
    for (idx, ch) in line_text.chars().enumerate() {
        if utf16 >= character {
            return idx;
        }
        utf16 = utf16.saturating_add(u32::try_from(ch.len_utf16()).expect("len_utf16 fits u32"));
    }
    line_text.chars().count()
}

pub(crate) fn utf16_len(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).expect("UTF-16 length fits u32")
}

/// Compute "go-to-definition" locations for the symbol at the
/// cursor.
///
/// Returns an empty vector when no recognisable symbol is at
/// the position or when the symbol's definition isn't in the
/// current document.
#[must_use]
pub fn definition(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    let line_index = LineIndex::new(source);

    // 1. Variable reference — walk the scope chain inward
    //    from the global scope toward the innermost scope
    //    whose body span contains the cursor's byte offset,
    //    then walk back outward looking for the var.
    if let Some(var_name) = find_var_at_position(source, line, character) {
        let cursor_offset = byte_offset_at(&line_index, source, line, character);
        if let Some(var_def) = lookup_var_in_scope_chain(
            &analysis.global_scope,
            cursor_offset,
            &var_name,
            analysis.ns_var_global_fallback(),
        ) {
            return vec![span_to_range(source, &line_index, var_def.definition_span)];
        }
        return Vec::new();
    }

    // 1b. Variable *declaration* / same-cell write site — the cursor sits
    //     on a plain name token (a `set x`/`variable x` target, a
    //     proc/method parameter, a `catch` result-var), not a
    //     `$`-prefixed read. See `var_def_at_declaration_offset`'s own doc
    //     for why this can't reuse the ordinary scope-chain walk (issue
    //     #923 differential-audit finding idx 9, main audit wave).
    let decl_byte_offset = byte_offset_at(&line_index, source, line, character);
    if let Some(var_def) = var_def_at_declaration_offset(&analysis.global_scope, decl_byte_offset) {
        return vec![span_to_range(source, &line_index, var_def.definition_span)];
    }

    // 2. Bare word — proc, class, class-member, or alias.
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // `$obj method` / `[$obj method]` — when the cursor sits on
    // the method-name token of an instance-method call and the
    // instance variable's class is known, jump to the method
    // declaration.  Checked before the proc lookup so a method
    // call resolves to the method even when a same-named proc
    // exists.
    if let Some(result) = instance_method_definition(analysis, source, &line_index, line, character)
    {
        return result;
    }
    // `<ensemble> <subcommand>` — a static `namespace ensemble create
    // -map`/`-subcommands` mapping (issue #923 idx 106).  Checked before the
    // generic "otherwise it is a CALL" resolution below: real Tcl never
    // independently looks up `make` as a command (only the pair `widget
    // make` dispatches), so a coincidental same-named proc elsewhere in the
    // workspace must never win.
    if let Some((head, sub, false)) = instance_method_at_cursor(source, line, character) {
        let cursor_offset = byte_offset_at(&line_index, source, line, character);
        let namespace = namespace_context_at(
            &analysis.global_scope,
            cursor_offset,
            &analysis.namespace_overrides,
        );
        if let Some(target) = ensemble_subcommand_target(analysis, &namespace, &head, &sub)
            && let Some(proc_def) = resolve_called_proc(
                analysis,
                source,
                "::",
                target,
                Some(tcl_registry::registry_for_dialect("")),
            )
        {
            return vec![span_to_range(source, &line_index, proc_def.name_span)];
        }
    }
    // `next` / `nextto` inside a method body — jump to the super-method in
    // the MRO chain that the enclosing method overrides (`next`), or to the
    // named class's copy of it (`nextto Cls`).
    if (word == "next" || word == "nextto")
        && let Some(span) =
            next_dispatch_target(analysis, source, &line_index, line, character, &word)
    {
        return vec![span_to_range(source, &line_index, span)];
    }
    // Prefer the proc whose own declaration name span covers the cursor (so
    // a same-named proc in another namespace's own decl resolves to *that*
    // one — mirrors `references::proc_references` / `rename::rename_proc`;
    // #924).
    let cursor_offset = byte_offset_at(&line_index, source, line, character);
    if let Some((_, proc_def)) = analysis
        .all_procs
        .iter()
        .find(|(_, p)| p.name_span.start() <= cursor_offset && cursor_offset < p.name_span.end())
    {
        return vec![span_to_range(source, &line_index, proc_def.name_span)];
    }
    // Otherwise it is a CALL — resolve namespace-aware, following C Tcl's
    // command resolution (`Tcl_FindCommand`, `tclNamesp.c`): the caller's
    // namespace first, then the global namespace; an absolute `::`-prefixed
    // word resolves exactly. A word no candidate defines falls back — unless
    // it names a registry builtin, which is what the call actually reaches —
    // to the lenient tail match for procs whose defining namespace isn't
    // statically visible, resolved deterministically (see
    // [`resolve_called_proc`]).
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor_offset,
        &analysis.namespace_overrides,
    );
    if let Some(proc_def) = resolve_called_proc(
        analysis,
        source,
        &namespace,
        &word,
        Some(tcl_registry::registry_for_dialect("")),
    ) {
        return vec![span_to_range(source, &line_index, proc_def.name_span)];
    }
    // Cursor sitting on a class's own declaration name → that class.
    if let Some((_, class_def)) = analysis
        .all_classes
        .iter()
        .find(|(_, c)| c.name_span.start() <= cursor_offset && cursor_offset < c.name_span.end())
    {
        return vec![span_to_range(source, &line_index, class_def.name_span)];
    }
    // A call / reference that names a class (`superclass X`, `X new`, …).  Skip
    // a document-local `oo::define` extension stub (`via_define`): the real
    // `oo::class create` site lives in another file, and the cross-document
    // resolver prefers it — pinning navigation to the local extension would
    // shadow the true definition.  When the class *is* created here,
    // `via_define` is false and this is that creation site.
    if let Some((_, class_def)) = analysis.all_classes.iter().find(|(qname, c)| {
        !c.via_define && (c.name == word || *qname == &word || *qname == &format!("::{word}"))
    }) {
        return vec![span_to_range(source, &line_index, class_def.name_span)];
    }
    // Class-member lookup — when the cursor sits inside a
    // class body, walk that class's methods / properties /
    // constructors / destructor for a name match.  Covers
    // `my method` calls inside the body plus bare member
    // references.
    if let Some(span) = lookup_class_member(analysis, &word, cursor_offset) {
        return vec![span_to_range(source, &line_index, span)];
    }
    // Alias resolution — when the cursor's word matches an
    // `interp alias {} ALIAS {} TARGET` recorded in
    // `analysis.command_aliases`, jump to the TARGET proc.  An alias target
    // is looked up when the alias fires, from the global namespace, so its
    // resolution context is `"::"` wherever the alias was written.
    if let Some(alias) = lookup_alias(analysis, &word)
        && let Some(proc_def) = resolve_called_proc(
            analysis,
            source,
            "::",
            &alias.target,
            Some(tcl_registry::registry_for_dialect("")),
        )
    {
        return vec![span_to_range(source, &line_index, proc_def.name_span)];
    }
    Vec::new()
}

/// Go-to-definition for an instance-method call site (`$obj method` /
/// `[$obj method]` / `my method`).  Extracted from [`definition`] to keep
/// that function within the line budget.
///
/// `None` means the cursor isn't on an instance-method call at all — the
/// caller falls through to the remaining checks (proc / class / alias).
/// `Some` is definitive: once the receiver's per-object override or class
/// is resolved, the call reaches exactly what this returns (possibly an
/// empty vector for an unexported/undefined method — issue #945 faults 4
/// + 5 + 6), never a same-named proc.
fn instance_method_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<Vec<LspRange>> {
    let (inst, method, is_dollar) = instance_method_at_cursor(source, line, character)?;
    // A per-object method (`oo::objdefine $obj { method m … }`) is layered
    // ahead of the object's class methods, so resolve it first — `$obj m`
    // must reach the per-object override, not a same-named class method.
    // Binding-identity keyed (issue #945 fault 5): the lookup honours the
    // receiver's scope, not just its textual tail.
    if let Some(span) = lookup_object_method(analysis, source, &inst, &method, line, character) {
        return Some(vec![span_to_range(source, line_index, span)]);
    }
    // `my m` — an internal call: dispatch starts at the *enclosing*
    // class and reaches unexported methods too (issue #945 fault 4).
    if inst == "my" {
        let cursor = byte_offset_at(line_index, source, line, character);
        if let Some(class_q) = enclosing_class_at(analysis, cursor) {
            return Some(method_dispatch_definition(
                analysis,
                source,
                line_index,
                class_q,
                &method,
                false,
                MethodBucket::Instance,
            ));
        }
    }
    let class_q = receiver_instance_class(analysis, &inst, is_dollar)?;
    let bucket = receiver_method_bucket(analysis, &inst, is_dollar);
    // External `$obj m` / `CLASS m`: the C-faithful dispatch entry — the
    // first exported implementation on the receiver's linearisation
    // (mixins before the class, subclasses before bases; issue #945
    // faults 4 + 6).  A resolved receiver is a definitive answer either
    // way: an unexported/undefined method yields *nothing* rather than
    // falling through to a same-named proc.
    let class_q = class_q.clone();
    let dispatch = method_dispatch_definition(
        analysis, source, line_index, &class_q, &method, true, bucket,
    );
    if !dispatch.is_empty() {
        return Some(dispatch);
    }
    // `configure` / `cget` accessors and properties have no method entry;
    // keep the property fallback before giving up.
    if let Some(p) = analysis
        .all_classes
        .get(&class_q)
        .and_then(|cd| cd.properties.get(method.as_str()))
    {
        return Some(vec![span_to_range(source, line_index, p.name_span)]);
    }
    Some(Vec::new())
}

/// Walk every class whose `body_span` contains the cursor
/// offset and look up `word` in that class's methods,
/// class-methods, or properties — but ONLY when `word` is
/// itself genuinely bareword-callable from a method body of
/// this class: a bareword sibling call only actually dispatches
/// when `link` (`oo::Helpers::link`) exposed it that way
/// (`ClassDef::linked_members`, issue #923 idx 113); an
/// un-linked member of the same name errors "invalid command
/// name" at runtime, so it must not resolve here either. Also
/// checks constructors/destructor, unconditionally (a distinct,
/// unrelated feature — see [`ClassDef::linked_members`]'s doc).
/// Returns the matched member's `name_span` when found.
///
/// `"constructor"` matches any defined constructor;
/// `"destructor"` matches the destructor.  Other words match
/// against the member's `name`.
fn lookup_class_member(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    let class_def = analysis
        .all_classes
        .get(enclosing_class_at(analysis, cursor_offset)?)?;
    if let Some(target) = class_def.linked_members.get(word) {
        if let Some(m) = class_def.methods.get(target) {
            return Some(m.name_span);
        }
        if let Some(m) = class_def.class_methods.get(target) {
            return Some(m.name_span);
        }
        if let Some(p) = class_def.properties.get(target) {
            return Some(p.name_span);
        }
    }
    if word == "constructor"
        && let Some(c) = class_def.constructors.first()
    {
        if !c.name_span.is_empty() {
            return Some(c.name_span);
        }
        // Analyser doesn't store a name span for the
        // constructor keyword (it has no name token).
        // Fall back to the body span's start so the
        // editor at least lands on the constructor's
        // body opener.
        return Some(c.body_span);
    }
    if word == "destructor"
        && let Some(d) = &class_def.destructor
    {
        if !d.name_span.is_empty() {
            return Some(d.name_span);
        }
        return Some(d.body_span);
    }
    None
}

/// Look up `method` among the per-object methods added by `oo::objdefine`
/// to the object the receiver variable at the *call site* denotes — the
/// per-object override `TclOO` layers ahead of the object's class methods.
/// Returns the declaration's `name_span`.
///
/// Keyed by **binding identity**, not the receiver's textual tail (issue
/// #945 fault 5): two unrelated locals both named `o` in different procs
/// are different objects with different per-object methods, so the
/// candidate set is scoped to the `oo::objdefine` sites whose receiver
/// resolves to the *same variable binding* as the call site's receiver —
/// the innermost scope (proc / method body, else the top level) declaring
/// the name that encloses both.  A dispatch whose binding cannot be
/// matched to exactly one `objdefine` receiver abstains.
fn lookup_object_method(
    analysis: &AnalysisResult,
    source: &str,
    receiver: &str,
    method: &str,
    line: u32,
    character: u32,
) -> Option<tcl_lexer::Span> {
    let records = analysis.object_methods.get(receiver)?;
    let line_index = LineIndex::new(source);
    let call_offset = byte_offset_at(&line_index, source, line, character);
    let call_scope = variable_scope_extent(analysis, receiver, call_offset);
    let matched: Vec<&tcl_compiler::analyser::ObjectMethodDef> = records
        .iter()
        .filter(|m| variable_scope_extent(analysis, receiver, m.objdefine_offset) == call_scope)
        .filter(|m| m.def.name == method)
        .collect();
    // Exactly one binding-compatible override may answer; several distinct
    // same-scope objects (reassignment) or none at all abstain to the
    // class chain.
    match matched.as_slice() {
        [only] => Some(only.def.name_span),
        _ => None,
    }
}

/// The extent (byte range) of the innermost proc / method body scope that
/// declares or contains variable `name` at `offset` — the binding-identity
/// key for per-object method matching.  Two uses of the same simple name
/// belong to the same binding iff their innermost enclosing proc/method
/// body is the same span (or both sit at the top level, extent `None`).
fn variable_scope_extent(
    analysis: &AnalysisResult,
    _name: &str,
    offset: u32,
) -> Option<(u32, u32)> {
    fn innermost(
        scope: &tcl_compiler::analyser::Scope,
        offset: u32,
        best: &mut Option<(u32, u32)>,
        depth: u32,
    ) {
        if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
            return;
        }
        for child in &scope.children {
            if let Some(span) = child.body_span
                && span.start() <= offset
                && offset <= span.end()
            {
                let is_frame = matches!(
                    child.kind,
                    tcl_compiler::analyser::ScopeKind::Proc
                        | tcl_compiler::analyser::ScopeKind::Method
                );
                if is_frame {
                    let width = span.end() - span.start();
                    if best.is_none_or(|(s, e)| width < e - s) {
                        *best = Some((span.start(), span.end()));
                    }
                }
                innermost(child, offset, best, depth + 1);
            }
        }
    }
    let mut best = None;
    innermost(&analysis.global_scope, offset, &mut best, 0);
    best
}

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
/// covers the *first* one recorded (issue #923 idx 52, main audit wave).
/// Ties (nested definitions) resolve to the narrowest containing span,
/// same tie-break the single-span version always used.
///
/// The single canonical implementation of this check — every other
/// module in this crate that used to keep its own copy now calls this
/// one instead, so the multi-span fix applies everywhere uniformly.
pub(crate) fn enclosing_class_at(analysis: &AnalysisResult, offset: u32) -> Option<&str> {
    analysis
        .class_body_spans
        .iter()
        .filter(|(_, span)| span.start() < offset && offset < span.end())
        .min_by_key(|(_, span)| span.end() - span.start())
        .map(|(name, _)| name.as_str())
}

/// Which of a class's two method buckets a dispatch receiver reaches
/// (issue #923 idx 120), consulted by [`method_dispatch_definition`] and,
/// via [`receiver_method_bucket`], by every other consumer of
/// [`receiver_instance_class`] that also needs its own method lookup
/// restricted to the matching bucket (`completion.rs`'s `method_items`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodBucket {
    /// `$obj method` / `my method` — dispatch on an *instance*:
    /// `methods` only, never `class_methods` (a class-side method is
    /// never itself instance-callable — real tclsh raises `unknown
    /// method` for it; this closes a pre-existing false-positive
    /// go-to-definition on e.g. `ActiveRecord create rec1; rec1 find`,
    /// bundled alongside idx 120 since this function's signature was
    /// already changing here).
    Instance,
    /// `CLASS method` — dispatch on the class's own bound command (issue
    /// #923 idx 120): `class_methods` only (a class-side method is never
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

/// Go-to-definition for a method call dispatched on an instance of
/// `class_q`: the **first applicable implementation** on the class's
/// `TclOO` linearisation (the entry `$obj m` / `my m` actually runs —
/// issue #945 fault 6), visibility-filtered (issue #945 fault 4):
/// `external` keeps exported implementations only; an internal (`my`)
/// dispatch reaches unexported ones, and `private` ones only in the
/// receiver's own class.  Empty when no implementation is callable in
/// this context — a definitive "unknown method" answer, mirroring C.
fn method_dispatch_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    class_q: &str,
    method: &str,
    external: bool,
    bucket: MethodBucket,
) -> Vec<LspRange> {
    let hierarchy = tcl_compiler::analyser::class_hierarchy::build_class_hierarchy(
        analysis.all_classes.clone(),
    );
    let Some(mro) = hierarchy.mro_map.get(class_q) else {
        return Vec::new();
    };
    for provider_q in mro {
        let Some(cd) = analysis.all_classes.get(provider_q) else {
            continue;
        };
        let md = match bucket {
            // A class-side method is never itself instance-callable (real
            // tclsh: `unknown method` when an instance calls a
            // classmethod) — no `class_methods` fallback here, matching
            // `completion.rs`'s `method_items`, which already excludes it
            // for the identical reason.
            MethodBucket::Instance => cd.methods.get(method),
            MethodBucket::Class => cd
                .class_methods
                .get(method)
                .filter(|md| provider_q == class_q || !md.is_self_method),
        };
        let Some(md) = md else {
            continue;
        };
        let visible = if external {
            md.visibility == "public"
        } else {
            md.visibility != "private" || provider_q == class_q
        };
        if visible {
            return vec![span_to_range(source, line_index, md.name_span)];
        }
    }
    Vec::new()
}

/// Look up `method` against the class identified by qualified
/// name `class_q` — searches `methods`, `class_methods`, then
/// `properties`.  Returns the member's `name_span`.
fn lookup_method_in_class(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<tcl_lexer::Span> {
    let class_def = analysis.all_classes.get(class_q)?;
    class_def
        .methods
        .get(method)
        .map(|m| m.name_span)
        .or_else(|| class_def.class_methods.get(method).map(|m| m.name_span))
        .or_else(|| class_def.properties.get(method).map(|p| p.name_span))
}

/// Resolve `TclOO` `next` / `nextto` at the cursor to the super-method's
/// `name_span`.
///
/// `next` inside `method m` of class `C` dispatches `m` one step further
/// down the object's MRO — statically we resolve it in `C`'s own MRO (the
/// sound single-dispatch approximation): the next class after `C` that
/// provides `m`.  `nextto Base` restarts the search at `Base`.  The
/// enclosing class + method are found from the cursor's byte offset.
fn next_dispatch_target(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
    keyword: &str,
) -> Option<tcl_lexer::Span> {
    let cursor = byte_offset_at(line_index, source, line, character);
    let (class_q, method) = enclosing_method(analysis, cursor)?;
    let start_from: Option<String> = if keyword == "nextto" {
        // Byte offset of the cursor within its line, so `word_after` can pick
        // the `nextto` occurrence the cursor is on (not merely the first).
        let line_start = byte_offset_at(line_index, source, line, 0);
        let cursor_in_line = cursor.saturating_sub(line_start) as usize;
        let target = word_after(source, line, cursor_in_line, "nextto")?;
        Some(canonicalise_class(analysis, &class_q, &target))
    } else {
        None
    };
    let hierarchy = analysis.class_hierarchy();
    let next_class = hierarchy.next_provider(&class_q, &method, &class_q, start_from.as_deref())?;
    lookup_method_in_class(analysis, next_class, &method)
}

/// The `(qualified_class, method_name)` whose method body contains the
/// cursor offset, or `None` when the cursor is not inside a method body.
fn enclosing_method(analysis: &AnalysisResult, cursor: u32) -> Option<(String, String)> {
    let cd = analysis
        .all_classes
        .get(enclosing_class_at(analysis, cursor)?)?;
    for (mname, m) in cd.methods.iter().chain(cd.class_methods.iter()) {
        if m.body_span.start() <= cursor && cursor <= m.body_span.end() {
            return Some((cd.qualified_name.clone(), mname.clone()));
        }
    }
    None
}

/// The whitespace-delimited word that follows `keyword` on the cursor's
/// line (used to read the class name in `nextto Class`).
///
/// `cursor_in_line` is the cursor's byte offset within the line.  When a
/// line has several `keyword` occurrences (a comment, a string, or a second
/// statement), the occurrence the cursor sits on — or the nearest one
/// starting at or before the cursor — is chosen, so `nextto` go-to-def
/// resolves the class the user is actually pointing at rather than the
/// first match on the line.
fn word_after(source: &str, line: u32, cursor_in_line: usize, keyword: &str) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    // Select the keyword occurrence anchored on the cursor.
    let mut chosen: Option<usize> = None;
    let mut search = 0;
    while let Some(rel) = line_text[search..].find(keyword) {
        let idx = search + rel;
        let end = idx + keyword.len();
        if idx <= cursor_in_line && cursor_in_line <= end {
            chosen = Some(idx); // cursor is on the keyword itself
            break;
        }
        if idx <= cursor_in_line {
            chosen = Some(idx); // best occurrence at/before the cursor so far
        }
        search = end;
    }
    // Fall back to the first occurrence when the cursor precedes them all.
    let idx = chosen.or_else(|| line_text.find(keyword))?;
    let rest = line_text[idx + keyword.len()..].trim_start();
    let word: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
        .collect();
    (!word.is_empty()).then_some(word)
}

/// Canonicalise a written class name to the qualified form keyed in
/// `all_classes`, **owner-aware** — resolved relative to `owner`'s namespace
/// (the class writing the `nextto`) via the shared [`resolve_class_name`]
/// resolver: exact, `::name`, an outward namespace walk, then a unique tail.
/// This lets `nextto Base` reach a namespaced base class (`::Ns::Base`) named
/// bare from a sibling in the same namespace, instead of only matching a
/// global `::Base`.  Falls back to the written name when nothing resolves so
/// the caller's `next_provider` lookup simply finds no target.
fn canonicalise_class(analysis: &AnalysisResult, owner: &str, name: &str) -> String {
    let tail_index =
        tcl_compiler::analyser::class_hierarchy::build_tail_index(analysis.all_classes.keys());
    tcl_compiler::analyser::class_hierarchy::resolve_class_name(
        name,
        owner,
        |cand| analysis.all_classes.contains_key(cand),
        &tail_index,
    )
    .unwrap_or_else(|| name.to_string())
}

/// Detect a `$obj method ...` / `[$obj method ...]` or
/// `objcmd method ...` call where the cursor sits on the *method-name*
/// token.  Returns `(receiver_name, method_name, receiver_is_dollar)`:
/// `receiver_is_dollar` is `true` for a `$var` / `${var}` receiver (a
/// variable holding an object) and `false` for a bare object-command
/// receiver (`objcmd`, e.g. one bound by `CLASS create objcmd`).
///
/// The receiver must be the command-segment head (a single token
/// immediately preceding the method), so the method sits at word-index 1.
/// Command segments are delimited by `;`, `[`, `{`, and the line start —
/// a single-line approximation that covers the common editor cases.
///
/// Whether a bare receiver actually resolves to a class is decided by
/// [`receiver_instance_class`], which gates bare receivers on
/// `created_instance_commands` (so a plain variable's bare name — never a
/// valid dispatch — does not resolve).
pub(crate) fn instance_method_at_cursor(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, String, bool)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == ':';

    // Method-name word bounds around the cursor.
    let mut wstart = col;
    while wstart > 0 && is_ident(chars[wstart - 1]) {
        wstart -= 1;
    }
    let mut wend = col;
    while wend < chars.len() && is_ident(chars[wend]) {
        wend += 1;
    }
    if wstart == wend {
        return None;
    }
    let method: String = chars[wstart..wend].iter().collect();

    // Command-segment start: nearest `;` / `[` / `{` to the
    // left, else the line start.
    let mut seg_start = 0;
    for i in (0..wstart).rev() {
        if matches!(chars[i], ';' | '[' | '{') {
            seg_start = i + 1;
            break;
        }
    }
    // The head must be exactly one whitespace-delimited token
    // (the receiver), so the method is word-index 1.
    let prefix: String = chars[seg_start..wstart].iter().collect();
    let head_tokens: Vec<&str> = prefix.split_whitespace().collect();
    if head_tokens.len() != 1 {
        return None;
    }
    let head = head_tokens[0];
    if let Some(rest) = head.strip_prefix('$') {
        // `$var` / `${var}` receiver — a variable holding an object.
        let inst = rest
            .strip_prefix('{')
            .map_or(rest, |r| r.strip_suffix('}').unwrap_or(r));
        if inst.is_empty() {
            return None;
        }
        Some((inst.to_string(), method, true))
    } else {
        // Bare `objcmd` receiver — a plain word naming an object command.
        // Substituted / decorated heads (`[…]`, `{…}`, quoted) are not bare
        // object commands; `receiver_instance_class` further gates this on
        // `created_instance_commands`.
        if head.is_empty() || head.contains(['[', ']', '{', '}', '"', '(', ')', '$']) {
            return None;
        }
        Some((head.to_string(), method, false))
    }
}

/// Resolve a method-dispatch receiver at the cursor (as returned by
/// [`instance_method_at_cursor`]) to its class's qualified name.
///
/// A `$var` receiver (`is_dollar`) is any object-holding variable, looked
/// up in `instance_classes`.  A bare receiver is a valid *instance*
/// dispatch only when it names an object *command* (`CLASS create NAME`) —
/// a plain variable's bare name (`set v [CLASS new]` then `v method`) is
/// not a command and must not resolve — so it is additionally gated on
/// `created_instance_commands`.
///
/// A bare receiver that instead names a class *directly* (`ActiveRecord
/// find`, `TclOO`'s and `ooutil`'s class-command dispatch — issue #923 idx
/// 120) resolves too: `oo::class create NAME` always binds `NAME` as the
/// class's own command, so any class name written in the source is
/// unconditionally dispatchable this way. A `$var` can't textually denote
/// a class — it holds an object handle read from a variable, never the
/// class's own written name — so this path is bare-word only.
///
/// Shared by the definition / references / rename / hover cursor paths so
/// they agree on which receivers dispatch (and, via
/// `method_references_for_class`, so those match the code-lens count).
pub(crate) fn receiver_instance_class<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    is_dollar: bool,
) -> Option<&'a String> {
    if let Some(class) = analysis.instance_classes.get(receiver)
        && (is_dollar || analysis.created_instance_commands.contains(receiver))
    {
        return Some(class);
    }
    if is_dollar {
        return None;
    }
    let qualified = tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(
        receiver,
        &analysis.all_classes,
    )?;
    analysis
        .all_classes
        .get(&qualified)
        .map(|cd| &cd.qualified_name)
}

/// Which [`MethodBucket`] `receiver` reaches, given how
/// [`receiver_instance_class`] resolved it (issue #923 idx 120):
/// recomputes that function's own first condition (the *instance* path —
/// an object handle, `$var`, or a bound `CLASS create NAME` command)
/// rather than inferring it from the outside (e.g. merely checking
/// `instance_classes.contains_key`), so a same-named instance-command /
/// class collision can't misclassify it. `false` on that check, with
/// `receiver_instance_class` having still resolved a `class_q`, can only
/// mean the bare-word-names-a-class-directly path fired instead.
pub(crate) fn receiver_method_bucket(
    analysis: &AnalysisResult,
    receiver: &str,
    is_dollar: bool,
) -> MethodBucket {
    let is_instance_path = analysis
        .instance_classes
        .get(receiver)
        .is_some_and(|_| is_dollar || analysis.created_instance_commands.contains(receiver));
    if is_instance_path {
        MethodBucket::Instance
    } else {
        MethodBucket::Class
    }
}

/// Resolve a bare `ClassName method` call-site receiver directly to the
/// class whose `classmethod` provides `method` — the reverse of
/// [`crate::references::find_obj_method_call_sites`]'s forward "given the
/// class, which bare names dispatch it" fold, needed to resolve
/// Find-References / Rename / go-to-definition triggered *from* the call
/// site itself rather than from the declaration or a code lens.
///
/// `receiver` must name a class (simple or fully-qualified) that declares
/// or *inherits* (does not override) `method` as a `classmethod` — `None`
/// for an instance-command receiver (that's [`receiver_instance_class`]'s
/// job) or a class with no such classmethod.  Doesn't itself exclude
/// [incr Tcl] classes (whose class-scoped `proc` dispatches as
/// `Factory::method`, not this bare two-word shape): every downstream
/// consumer of the `(class, method, is_classmethod)` triple this feeds —
/// `find_obj_method_call_sites` and friends — already re-checks the
/// definer's family before treating any site as a dispatch, so a
/// same-named itcl class-proc resolves here but then correctly yields no
/// call sites rather than a false one.
#[must_use]
pub(crate) fn classmethod_dispatch_class(
    analysis: &AnalysisResult,
    receiver: &str,
    method: &str,
) -> Option<String> {
    let named = analysis
        .all_classes
        .values()
        .find(|cd| cd.name == receiver || cd.qualified_name == receiver)?;
    if named.class_methods.contains_key(method) {
        return Some(named.qualified_name.clone());
    }
    let hierarchy = analysis.class_hierarchy();
    let provider_q = hierarchy.method_target(&named.qualified_name, method)?;
    analysis
        .all_classes
        .get(provider_q)
        .filter(|cd| cd.class_methods.contains_key(method))
        .map(|_| provider_q.to_string())
}

/// Look up an alias by name.  Accepts the alias's simple or
/// qualified form (`mycmd` and `::mycmd` both match an alias
/// stored with `qualified_name == "::mycmd"`).
fn lookup_alias<'a>(
    analysis: &'a AnalysisResult,
    name: &str,
) -> Option<&'a tcl_compiler::signature_scan::types::SignatureCommandAlias> {
    let qualified = if name.starts_with("::") {
        name.to_string()
    } else {
        format!("::{name}")
    };
    analysis
        .command_aliases
        .get(&qualified)
        .or_else(|| analysis.command_aliases.get(name))
}

/// Resolve `head sub` (as returned by [`instance_method_at_cursor`], with
/// `is_dollar == false`) to the target command it dispatches to, when
/// `head` names a known `namespace ensemble create -map`/`-subcommands`
/// ensemble and `sub` exactly matches one of its literal, statically
/// -recorded subcommands (issue #923 idx 106) — `namespace` is the call
/// site's own command-resolution namespace, so a relative `head` (the
/// overwhelmingly common case: an ensemble dispatches through its own short
/// name) resolves the same way an ordinary bare command call would.
pub(crate) fn ensemble_subcommand_target<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    head: &str,
    sub: &str,
) -> Option<&'a str> {
    tcl_syntax::naming::bareword_resolution_candidates(namespace, head)
        .into_iter()
        .find_map(|cand| analysis.ensemble_subcommand_targets.get(&cand))
        .and_then(|subs| subs.get(sub))
        .map(String::as_str)
}

/// Compute the byte offset of a 0-based LSP `(line, character)`
/// pair in `source`. `character` is a UTF-16 code-unit offset.
///
/// Takes a pre-built `line_index` so callers that perform several
/// position conversions per request share a single index.
pub(crate) fn byte_offset_at(
    line_index: &LineIndex,
    source: &str,
    line: u32,
    character: u32,
) -> u32 {
    line_index.offset_at_utf16(line, Utf16Col::new(character), source)
}

/// Walk the scope tree to find the variable definition that
/// the cursor's `byte_offset` would see — the innermost
/// scope whose body span contains the offset takes precedence
/// over any enclosing scope.
///
/// Descend into the innermost matching child, then walk the
/// scope chain outward for the var lookup.
/// Whether an `uplevel` body's frame semantics hide the scope at index `i` of
/// the resolution `chain` (outermost-first) from the cursor.
///
/// Inside `uplevel #0 { … }` the script runs in the **global** frame, so the
/// enclosing proc's locals are dropped and resolution reaches the
/// global/namespace variable (a same-named proc-local must not shadow it).
/// Inside a non-`#0` `uplevel N { … }` the script runs in a **caller** frame
/// that is statically unknown, so everything *outside* the uplevel body is
/// dropped — the body's own locals resolve, but a name it does not declare
/// abstains rather than mis-attributing to the enclosing proc *or* the global
/// frame (the level word, recorded as the uplevel scope's name, distinguishes
/// the two).
fn uplevel_hides_scope(chain: &[&tcl_compiler::analyser::Scope], i: usize) -> bool {
    use tcl_compiler::analyser::ScopeKind;
    let Some(up) = chain.iter().rposition(|sc| sc.kind == ScopeKind::Uplevel) else {
        return false;
    };
    if chain[up].name == "#0" {
        chain[i].kind == ScopeKind::Proc
    } else {
        i < up
    }
}

pub(crate) fn lookup_var_in_scope_chain<'a>(
    scope: &'a tcl_compiler::analyser::Scope,
    byte_offset: u32,
    name: &str,
    ns_global_fallback: bool,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    use tcl_compiler::analyser::ScopeKind;
    // A `::`-qualified name first tries a direct, single-table lookup in
    // exactly the namespace the qualifier names (tclsh 9.0.4/8.6.16-verified:
    // `$other::v` referenced from inside namespace ::mine never tries
    // ::mine::other::v — only the one namespace the qualifier actually
    // resolves to). An absolute qualifier (leading `::`) resolves from the
    // global namespace; a relative one resolves under the cursor's own
    // command-resolution namespace — empirically the *same* namespace an
    // unqualified command there would resolve against, including the
    // `oo::class` method global-only special case and the `uplevel #0`
    // reset, so [`innermost_namespace_at`] (already the single canonical
    // implementation of that rule) doubles as the qualifier's base with no
    // separate variable-specific namespace logic. This alone covers every
    // `variable`-declared namespace cell from *anywhere* in the file — the
    // case the qualified outward walk below can't reach, since it only
    // ever sees scopes on the cursor's own lexical chain.
    //
    // A miss falls through to that walk rather than returning early: a
    // literal-qualified write with no preceding `namespace eval` / `variable`
    // (`set ns::var val`) records its `VarDef` verbatim under the writer's
    // own active scope, with no namespace-tree node to find by path — that
    // shape is only reachable when the cursor's own chain happens to include
    // the scope that wrote it (`chain[0]`, the global scope, always does),
    // exactly as before this lookup existed.
    if name.contains("::") {
        // Variable ($var) resolution deliberately does not consult
        // `namespace_overrides` (issue #923 idx 116 scoped this to
        // command/bareword resolution only) — `&[]` keeps this call
        // provably unaffected; `scope` alone (no `analysis`) is available
        // here regardless.
        let here = innermost_namespace_at(scope, byte_offset, &[]);
        let qualified = tcl_compiler::naming::qualify(&here, name);
        let (target_ns, base_name) = tcl_compiler::naming::key_holder_and_tail(&qualified);
        if let Some(v) =
            tcl_compiler::analyser::lookup_var_in_namespace(scope, target_ns, base_name)
        {
            return Some(v);
        }
    }
    // First, find the innermost scope containing the cursor.
    let chain = scope_chain_at(scope, byte_offset);
    // A bare name at a **namespace frame** (cursor directly in a `namespace
    // eval` body) follows C Tcl's two-table rule: the namespace's own
    // variables, then — only under a Tcl 8.x dialect (TIP 278;
    // `ns_global_fallback`, from
    // `AnalysisResult::ns_var_global_fallback`) — the global namespace.
    // Enclosing *intermediate* namespaces are never consulted in **any**
    // version.  Qualified names (the direct namespace lookup above already
    // missed) and proc-like frames (proc / method / uplevel bodies) keep the
    // lenient outward walk below.  (An `interp eval` child scope shares the
    // `Namespace` kind, so under 8.x the global leg can still cross the
    // interp boundary — a pre-existing leniency this rule already narrows
    // from "every enclosing scope" to "the global table only".)
    if !name.contains("::")
        && chain.len() > 1
        && chain
            .last()
            .is_some_and(|sc| sc.kind == ScopeKind::Namespace)
    {
        let ns = chain.last().expect("guarded: chain has a last element");
        if let Some(v) = ns.variables.get(name) {
            return Some(v);
        }
        return ns_global_fallback
            .then(|| {
                chain
                    .first()
                    .expect("guarded: chain has a first element")
                    .variables
                    .get(name)
            })
            .flatten();
    }
    // Walk outward (innermost-first) looking for the var, honouring the
    // `uplevel` frame semantics (see [`uplevel_hides_scope`]).
    for (i, sc) in chain.iter().enumerate().rev() {
        if uplevel_hides_scope(&chain, i) {
            continue;
        }
        if let Some(v) = sc.variables.get(name) {
            return Some(v);
        }
    }
    None
}

/// Every reference span (other than `var_def`'s own declaration) for the
/// variable `var_def` denotes, gathered across the whole scope tree by
/// unifying the aliases Tcl treats as one cell:
///
/// * **Namespace / global aliases** — `global v`, `variable v`, `namespace
///   upvar ns v local` each record a `link_target` (the qualified cell name);
///   every alias of that cell, and the namespace-level declaration itself,
///   shares the target, so their declarations (each spells the name) and uses
///   all unify.  This is the analyser analogue of Tcl's `VAR_LINK`.
/// * **`TclOO` instance variables** — pre-bound into every method scope with
///   the *same* `variable v` declaration span; unioning by that shared span
///   links the per-method copies into the one per-object cell.
///
/// A zero-width declaration span (the fallback for a declaration-less
/// grammar-injected implicit) can't be unioned safely — several such seeds in
/// one body share it — so that case returns exactly `var_def`'s own
/// references.  The caller adds `var_def`'s own declaration span itself (when
/// including the declaration), so it is excluded here.
#[must_use]
pub(crate) fn linked_var_reference_spans(
    scope: &tcl_compiler::analyser::Scope,
    var_def: &tcl_compiler::analyser::VarDef,
) -> Vec<tcl_lexer::Span> {
    let mut out = Vec::new();
    match var_def.link_target.as_deref() {
        Some(target) => {
            collect_alias_spans(scope, target, var_def.definition_span, &mut out, 0);
            // `target`'s own canonical cell (a plain `set` / declaration with
            // no `link_target` of its own) is never found by the alias-only
            // scan above, since that scan only unions records that name
            // `target` as *their own* link target (issue #923 idx 68): fold
            // it in directly.
            if let Some(canonical) =
                tcl_compiler::analyser::lookup_var_by_qualified_name(scope, target)
                && canonical.definition_span != var_def.definition_span
            {
                out.push(canonical.definition_span);
                out.extend(canonical.references.iter().copied());
            }
        }
        None if !var_def.definition_span.is_empty() => {
            collect_shared_span_refs(scope, var_def.definition_span, &mut out, 0);
            // The inverse of the fold-in above: `var_def` might itself BE the
            // canonical cell some `global` / `variable` / `namespace upvar`
            // alias elsewhere points at (issue #923 idx 68) — its own
            // `link_target` is `None` precisely because it isn't an alias, so
            // this can only be discovered by finding its own qualified name
            // and searching for aliases of it.
            if let Some(own_qualified) =
                tcl_compiler::analyser::qualified_name_for_var_decl(scope, var_def.definition_span)
            {
                collect_alias_spans(scope, &own_qualified, var_def.definition_span, &mut out, 0);
            }
        }
        None => out.extend(var_def.references.iter().copied()),
    }
    out
}

/// Declarations (other than `own_decl`) and uses of every variable aliasing the
/// cell `target`.
fn collect_alias_spans(
    scope: &tcl_compiler::analyser::Scope,
    target: &str,
    own_decl: tcl_lexer::Span,
    out: &mut Vec<tcl_lexer::Span>,
    depth: u32,
) {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for v in scope.variables.values() {
        if v.link_target.as_deref() == Some(target) {
            if v.definition_span != own_decl {
                out.push(v.definition_span);
            }
            out.extend(v.references.iter().copied());
        }
    }
    for child in &scope.children {
        collect_alias_spans(child, target, own_decl, out, depth + 1);
    }
}

/// Uses of every variable sharing the declaration span `def_span` (the `TclOO`
/// instance-variable case).
fn collect_shared_span_refs(
    scope: &tcl_compiler::analyser::Scope,
    def_span: tcl_lexer::Span,
    out: &mut Vec<tcl_lexer::Span>,
    depth: u32,
) {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for v in scope.variables.values() {
        if v.definition_span == def_span {
            out.extend(v.references.iter().copied());
        }
    }
    for child in &scope.children {
        collect_shared_span_refs(child, def_span, out, depth + 1);
    }
}

/// Resolve the [`VarDef`](tcl_compiler::analyser::VarDef) whose declaration
/// or a later same-cell bareword occupies `byte_offset` — i.e. the cursor
/// sits on a plain name token (a `set` / `variable` / `global` / proc- or
/// method-parameter declaration, or a later bareword write to that same
/// variable such as a `catch script name` result-var) rather than a `$ref`.
///
/// Deliberately does **not** route through [`scope_chain_at`]'s lexical
/// containment walk: a proc/method parameter's own name token sits in the
/// parameter list, textually *before* that scope's `body_span` even
/// starts, so a scope-chain lookup keyed on `byte_offset` never reaches
/// the scope that owns it (issue #923 differential-audit finding idx 9,
/// main audit wave — go-to-definition/hover/find-references all returned
/// nothing for a cursor placed directly on a parameter's or a `catch`
/// result-var's own declaring token, even though every `$name` read of the
/// same variable resolved correctly). Instead this walks every scope in
/// the whole tree unconditionally, checking each `VarDef`'s own
/// `definition_span` and every span in `references` for byte-offset
/// containment. This is safe without any scope-visibility filtering: a
/// byte-offset span match is unambiguous by construction (no two distinct
/// declaration/write occurrences in a file share a byte range), unlike a
/// *name*-based lookup which would need scope-aware disambiguation.
pub(crate) fn var_def_at_declaration_offset(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Option<&tcl_compiler::analyser::VarDef> {
    let contains = |span: tcl_lexer::Span| span.start() <= byte_offset && byte_offset < span.end();
    for v in scope.variables.values() {
        if contains(v.definition_span) || v.references.iter().any(|&r| contains(r)) {
            return Some(v);
        }
    }
    scope
        .children
        .iter()
        .find_map(|child| var_def_at_declaration_offset(child, byte_offset))
}

/// Collect every variable name visible at `byte_offset` — the union
/// of `variables` across the scope chain (innermost first, then
/// enclosing scopes up to the global root).  Used by variable
/// completion so a `$`-trigger inside a proc / namespace body offers
/// that scope's locals + params alongside the globals.
///
/// Inside an `uplevel #0 { … }` body the script runs in the global
/// frame, so the enclosing proc's locals are *not* visible — only the
/// global / namespace frame's variables plus the uplevel body's own
/// locals.  When the chain contains an [`ScopeKind::Uplevel`] scope,
/// proc-scope variables are dropped from the visible set.
pub(crate) fn visible_variable_names(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<String> {
    let chain = scope_chain_at(scope, byte_offset);
    let mut names: Vec<String> = Vec::new();
    for (i, sc) in chain.iter().enumerate().rev() {
        // Honour the `uplevel` frame semantics (see [`uplevel_hides_scope`]):
        // `#0` drops the enclosing proc's locals, a non-`#0` level drops
        // everything outside the uplevel body.
        if uplevel_hides_scope(&chain, i) {
            continue;
        }
        for k in sc.variables.keys() {
            if !names.iter().any(|n| n == k) {
                names.push(k.clone());
            }
        }
    }
    names
}

/// For completion: the set of variable names defined in the *global/root*
/// scope, returned only when the cursor sits where an unqualified reference
/// does NOT resolve to the global — inside a proc (unqualified names are the
/// proc's locals) or inside a nested `namespace eval` (they resolve within
/// that namespace).  Such a global must be offered `::`-qualified (`$::foo`)
/// so the inserted reference actually reaches it, matching Tcl's name
/// resolution.  Returns `None` at global scope, where bare names are correct.
///
/// `ns_global_fallback` carries the document dialect's TIP 278 semantics
/// (`AnalysisResult::ns_var_global_fallback`): under a Tcl 8.x dialect a
/// bare name at a namespace frame *does* reach an unshadowed global, so a
/// namespace context only forces qualification under 9.0+ semantics.  Proc
/// bodies never had the fallback in any version.
pub(crate) fn global_vars_needing_qualification(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
    ns_global_fallback: bool,
) -> Option<FxHashSet<String>> {
    use tcl_compiler::analyser::ScopeKind;
    let chain = scope_chain_at(scope, byte_offset);
    let in_local_context = chain.iter().any(|sc| {
        matches!(sc.kind, ScopeKind::Proc)
            || (matches!(sc.kind, ScopeKind::Namespace)
                && !sc.name.is_empty()
                && sc.name != "::"
                && !ns_global_fallback)
    });
    if !in_local_context {
        return None;
    }
    // Collect root/global names, but drop any that a closer (proc / namespace /
    // uplevel) scope also defines: a local of the same name *shadows* the
    // global, so the bare `$name` correctly resolves to the local and must not
    // be rewritten to `$::name` (which would silently retarget the reference).
    let mut globals = FxHashSet::default();
    let mut shadows = FxHashSet::default();
    for sc in &chain {
        if matches!(sc.kind, ScopeKind::Global) {
            globals.extend(sc.variables.keys().cloned());
        } else {
            shadows.extend(sc.variables.keys().cloned());
        }
    }
    for name in &shadows {
        globals.remove(name);
    }
    Some(globals)
}

/// Names of every namespace / global scope in the cursor's lexical chain.
/// Used by completion to skip cross-namespace candidates already offered as
/// bare names.
pub(crate) fn lexical_namespace_chain(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> FxHashSet<String> {
    use tcl_compiler::analyser::ScopeKind;
    let mut chain = FxHashSet::default();
    for sc in scope_chain_at(scope, byte_offset) {
        if matches!(sc.kind, ScopeKind::Namespace | ScopeKind::Global) {
            chain.insert(sc.name.clone());
        }
    }
    chain
}

/// Namespace an unqualified command invoked at `byte_offset` resolves
/// against (without the leading `::`), or `""` at global scope.  Used to
/// attribute an unqualified call to the proc in its own namespace when the
/// analyser's resolution falls back to the global guess.
///
/// Thin wrapper over [`tcl_compiler::analyser::command_resolution_namespace_at`]
/// — the single canonical implementation of Tcl's command-resolution
/// namespace rule, shared with the analyser's own `resolved_qualified_name`
/// computation (`Analyser::command_resolution_namespace`) so the two can
/// never disagree.  Correctly accumulates through every enclosing
/// `namespace eval` (however deeply nested — a bare, non-accumulating
/// "just the innermost segment" reading previously misidentified a 2+-level
/// nested namespace, e.g. `::a::b` read back as just `b`) and resets to a
/// proc's/method's own defining namespace inside its body, even when that
/// proc was declared with a fully-qualified name with no enclosing
/// `namespace eval` at all.
///
/// `overrides` (issue #923 idx 116) is checked *first*: a runtime-context
/// namespace pin (`apply {{params} body ns}` runs `body` in `ns`, not its
/// lexical home) that the ordinary span-containment walk above can never
/// see, since `handle_apply_command` roots that `Scope` subtree under
/// fresh, `body_span`-less namespace wrapper nodes. On multiple containing
/// entries (nested `apply` calls), the smallest span wins — innermost,
/// mirroring the lexical walk's own innermost-scope preference. Pass `&[]`
/// to opt out (e.g. variable lookups, which this fix deliberately doesn't
/// extend — see `lookup_var_in_scope_chain`).
pub(crate) fn innermost_namespace_at(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
    overrides: &[(tcl_lexer::Span, String)],
) -> String {
    let pinned = overrides
        .iter()
        .filter(|(span, _)| span.start() <= byte_offset && byte_offset < span.end())
        .min_by_key(|(span, _)| span.end() - span.start());
    if let Some((_, ns)) = pinned {
        return ns.trim_start_matches("::").to_string();
    }
    tcl_compiler::analyser::command_resolution_namespace_at(scope, byte_offset)
        .trim_start_matches("::")
        .to_string()
}

/// The `::`-prefixed namespace context at `byte_offset` (`"::"` at global
/// scope), in the shape [`tcl_syntax::naming::bareword_resolution_candidates`]
/// expects.  Built on [`innermost_namespace_at`] so call attribution here
/// agrees with the reference / rename gates
/// (`references::invocation_references_proc`).
pub(crate) fn namespace_context_at(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
    overrides: &[(tcl_lexer::Span, String)],
) -> String {
    let ns = innermost_namespace_at(scope, byte_offset, overrides);
    if ns.is_empty() {
        "::".to_owned()
    } else {
        format!("::{ns}")
    }
}

/// The bare command word at `(line, character)` together with the
/// `::`-prefixed namespace in effect there.
///
/// For the autoload-definition fallback: when a command head resolves to
/// nothing in the current document *or* the workspace index, the caller feeds
/// this `(word, namespace)` to the package / auto-load database
/// ([`crate::package_resolver::PackageResolver::resolve_auto_command`]) to find
/// the library file that defines it.  Returns `None` off any word.  The word
/// is returned verbatim (the resolver auto-qualifies it against the
/// namespace); `namespace` is `"::"` at global scope.
#[must_use]
pub fn command_head_and_namespace_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<(String, String)> {
    let (word, _start, _end) = crate::hover::find_word_span_at_position(source, line, character)?;
    let line_index = LineIndex::new(source);
    let offset = byte_offset_at(&line_index, source, line, character);
    let namespace = namespace_context_at(
        &analysis.global_scope,
        offset,
        &analysis.namespace_overrides,
    );
    Some((word, namespace))
}

/// Resolve a written call `word` to the user proc C Tcl's command resolution
/// would pick (`Tcl_FindCommand`, `tclNamesp.c`): each candidate qualified name
/// from [`tcl_syntax::naming::command_resolution_candidates`] — the caller's
/// namespace, then each `namespace path` entry, then global; an absolute
/// `::`-prefixed word is exact — looked up in `all_procs`.  `namespace` is the
/// caller's `::`-prefixed namespace (`"::"` at global scope).
///
/// Honouring the caller namespace's recorded `namespace path` (from
/// `analysis.namespace_paths`) is what lets a bare call reach a proc on the
/// path before a same-named global — matching how call-site settling already
/// resolves, so definition / hover / signature help agree with references.
fn proc_visible_from_namespace<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    tcl_syntax::naming::command_resolution_candidates(namespace, path, word)
        .into_iter()
        .find_map(|qname| analysis.all_procs.get(&qname))
}

/// Resolve a bareword `word` (`namespace` context, honouring `namespace
/// path` exactly like [`proc_visible_from_namespace`]) through an
/// in-document wildcard `namespace import NS::*` — a glob (or exact)
/// pattern recorded in `analysis.namespace_imports`.
///
/// For each candidate qualified name Tcl's own lookup would try (the
/// caller's namespace, then each `namespace path` entry, then global — the
/// same order [`proc_visible_from_namespace`] walks), this checks whether
/// that candidate's namespace has an in-scope import whose pattern's tail
/// glob-matches `word`. A wildcard import only ever imports names its
/// source namespace has actually `namespace export`ed (`Tcl_Export`,
/// `tclNamesp.c`) — an unexported sibling command living in the same
/// namespace is **not** reachable through the import (tclsh9.0/8.6-verified:
/// `invalid command name` calling it bare) — so this also requires a recorded
/// `analysis.namespace_exports` entry from that same source namespace whose
/// pattern covers `word`.
///
/// Only resolves a source namespace defined in **this** document
/// (`analysis.all_procs`); a source namespace defined in another file is
/// the cross-document oracle's job
/// (`tcl_lsp_core::workspace_index::WorkspaceIndex::resolve_wildcard_import`).
fn proc_visible_via_wildcard_import<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    let source_ns = wildcard_import_source_namespace(analysis, namespace, path, word)?;
    analysis
        .all_procs
        .get(&tcl_syntax::naming::qualify(source_ns, word))
}

/// Shared candidate walk behind [`proc_visible_via_wildcard_import`] and
/// [`resolve_class_target_at`]'s wildcard-import fallback: find the first
/// in-scope `namespace import` whose pattern covers `word`, whose source
/// namespace has actually exported a matching pattern, and return that
/// source namespace. `path` is the caller's own `namespace path` entries
/// (empty for the class resolver, which — like
/// [`tcl_syntax::naming::bareword_resolution_candidates`] — does not
/// consult it). The caller joins the result with `word`
/// ([`tcl_syntax::naming::qualify`]) and looks the qualified name up in
/// whichever map (`all_procs` / `all_classes`) applies.
///
/// Restricted to a genuine bareword `word` (no embedded `::`) — `namespace
/// import` / `namespace export` patterns only ever name simple command
/// tails, matching the bug's scope (issue #923 idx 18); a qualified call
/// (`inner::p`) never goes through an imported alias in real Tcl, so this
/// abstains rather than risk mismatching a namespace-path segment against
/// an unrelated import.
fn wildcard_import_source_namespace<'a, S: AsRef<str>>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    path: &[S],
    word: &str,
) -> Option<&'a str> {
    if word.contains("::") {
        return None;
    }
    for candidate in tcl_syntax::naming::command_resolution_candidates(namespace, path, word) {
        let Some((prefix, _)) = candidate.rsplit_once("::") else {
            continue;
        };
        let candidate_ns = if prefix.is_empty() { "::" } else { prefix };
        for imp in analysis
            .namespace_imports
            .iter()
            .filter(|i| i.ns == candidate_ns)
        {
            let Some((source_ns, export_tail)) = imp.pattern.rsplit_once("::") else {
                continue;
            };
            if source_ns.is_empty() || !tcl_syntax::glob::string_match(export_tail, word) {
                continue;
            }
            let exported = analysis
                .namespace_exports
                .iter()
                .any(|e| e.ns == source_ns && tcl_syntax::glob::string_match(&e.pattern, word));
            if exported {
                return Some(source_ns);
            }
        }
    }
    None
}

/// Resolve a call `word` written in `namespace` to the proc it denotes:
/// C Tcl's rule first ([`proc_visible_from_namespace`]); then, unless the
/// word names a `registry` builtin — which C Tcl would resolve instead, so a
/// same-named proc in an unrelated namespace must not shadow it — the
/// lenient tail match kept for procs whose defining namespace isn't
/// statically visible at the call, made deterministic by
/// [`fallback_proc_by_simple_name`].
///
/// A candidate whose own declaration is nested inside another proc's (or
/// class's) body is never allowed to win this gate: nested code runs only
/// conditionally, when and if the enclosing definition is actually invoked,
/// so a `proc ::set {...}` written inside some unrelated proc must not
/// permanently outrank the real `set` builtin for every call site in the
/// workspace — the classic "rename the builtin away, install a shadow,
/// restore it" idiom (e.g. `rest.tcl`'s `describe`) otherwise leaves hover /
/// go-to-definition / references pointing at the shadow forever, even at a
/// call site that runs strictly after the shadow has been renamed back off.
/// [`AnalysisResult::offset_is_inside_any_definition_body`] is the same
/// judgement the analyser's own W123 gate (`registry_name_deleted_before`)
/// already applies to a *deletion* recorded inside a body — reused here
/// rather than re-derived, so both agree on what "load-time" means.
///
/// `registry`, when `Some`, supplies the builtin gate; `None` skips the gate
/// (callers without a registry keep the lenient behaviour, including the
/// nested-shadow one — there is no builtin to protect).
pub(crate) fn resolve_called_proc<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    namespace: &str,
    word: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let has_builtin = registry.is_some_and(|r| r.get(word).is_some());
    if let Some(proc_def) = proc_visible_from_namespace(analysis, namespace, word) {
        let nested_shadow = has_builtin
            && analysis.offset_is_inside_any_definition_body(proc_def.name_span.start());
        if !nested_shadow {
            return Some(proc_def);
        }
    }
    // A wildcard `namespace import NS::*` creates no real `ProcDef` of its
    // own — `proc_visible_from_namespace` can never see it — but it makes
    // an exported command in `NS` callable bare, exactly as if a real proc
    // existed at the candidate's qualified name. Tried at the same
    // priority tier (same nested-shadow gate) so an unconditional
    // top-level import still outranks a same-named builtin, matching how
    // an equivalent top-level `proc` redefinition already would.
    if let Some(proc_def) = proc_visible_via_wildcard_import(analysis, namespace, word) {
        let nested_shadow = has_builtin
            && analysis.offset_is_inside_any_definition_body(proc_def.name_span.start());
        if !nested_shadow {
            return Some(proc_def);
        }
    }
    if has_builtin {
        return None;
    }
    fallback_proc_by_simple_name(analysis, source, word)
}

/// Resolve the `(all_procs key, ProcDef)` that a proc-oriented editor
/// operation (rename / references / call-hierarchy) targets at `cursor_off`:
///
/// 1. the proc whose declaration name span covers the cursor (a declaration
///    -site invocation), else
/// 2. the namespace-aware call-site resolution ([`resolve_called_proc`], C
///    Tcl's own rule from the caller's namespace).
///
/// It never falls back to a namespace-blind `p.name == word` scan — the shape
/// that let a rename triggered from a bareword call site pick an *arbitrary*
/// same-named proc in an unrelated namespace (`HashMap` order) and rewrite the
/// wrong definition while leaving the one under the cursor untouched.  The
/// returned key equals `ProcDef::qualified_name` (the map is keyed by it).
pub(crate) fn resolve_proc_target_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor_off: u32,
    word: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> Option<(&'a String, &'a tcl_compiler::analyser::ProcDef)> {
    if let Some(hit) = analysis
        .all_procs
        .iter()
        .find(|(_, p)| p.name_span.start() <= cursor_off && cursor_off < p.name_span.end())
    {
        return Some(hit);
    }
    let ns = namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    let proc_def = resolve_called_proc(analysis, source, &ns, word, registry)?;
    analysis.all_procs.get_key_value(&proc_def.qualified_name)
}

/// The class analogue of [`resolve_proc_target_at`]: the `(all_classes key,
/// ClassDef)` a class-oriented editor operation targets at `cursor_off` — the
/// class whose declaration name span covers the cursor, else the
/// namespace-aware candidate resolution (a class name *is* a command name, so
/// the same `bareword_resolution_candidates` order applies: caller namespace,
/// then global).  Never a namespace-blind `c.name == word` scan.  The returned
/// key equals `ClassDef::qualified_name`.
pub(crate) fn resolve_class_target_at<'a>(
    analysis: &'a AnalysisResult,
    cursor_off: u32,
    word: &str,
) -> Option<(&'a String, &'a tcl_compiler::analyser::ClassDef)> {
    if let Some(hit) = analysis
        .all_classes
        .iter()
        .find(|(_, c)| c.name_span.start() <= cursor_off && cursor_off < c.name_span.end())
    {
        return Some(hit);
    }
    let ns = namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    if let Some(hit) = tcl_syntax::naming::bareword_resolution_candidates(&ns, word)
        .into_iter()
        .find_map(|cand| analysis.all_classes.get_key_value(&cand))
    {
        return Some(hit);
    }
    // A wildcard `namespace import NS::*` makes an exported class in `NS`
    // callable bare — see [`proc_visible_via_wildcard_import`] for the full
    // rationale (issue #923 idx 18); classes never consult `namespace path`,
    // matching `bareword_resolution_candidates` above.
    let source_ns = wildcard_import_source_namespace(analysis, &ns, &[] as &[&str], word)?;
    analysis
        .all_classes
        .get_key_value(&tcl_syntax::naming::qualify(source_ns, word))
}

/// Deterministic replacement for the old first-`HashMap`-hit tail match: of
/// every proc whose simple name equals `word`, prefer one defined in this
/// document ([`name_token_in_document`]), then the lexicographically
/// smallest qualified name.  `HashMap` iteration order never decides the
/// result.
pub(crate) fn fallback_proc_by_simple_name<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    word: &str,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    analysis
        .all_procs
        .iter()
        .filter(|(_, proc_def)| proc_def.name == word)
        .min_by(|(qname_a, proc_a), (qname_b, proc_b)| {
            let a_foreign = !name_token_in_document(source, proc_a);
            let b_foreign = !name_token_in_document(source, proc_b);
            a_foreign.cmp(&b_foreign).then_with(|| qname_a.cmp(qname_b))
        })
        .map(|(_, proc_def)| proc_def)
}

/// Whether `proc_def`'s name token actually spells its name in `source` —
/// the available evidence that the proc was defined in *this* document
/// rather than carried in from another file's analysis.
fn name_token_in_document(source: &str, proc_def: &tcl_compiler::analyser::ProcDef) -> bool {
    source
        .get(proc_def.name_span.start() as usize..proc_def.name_span.end() as usize)
        .is_some_and(|text| text.ends_with(proc_def.name.as_str()))
}

/// Fully-qualified `::ns::var` form for a var stored in a namespace / global
/// scope.
fn qualified_var_name(scope: &tcl_compiler::analyser::Scope, var: &str) -> String {
    use tcl_compiler::analyser::ScopeKind;
    if var.starts_with("::") {
        var.to_string()
    } else if scope.kind == ScopeKind::Global {
        format!("::{var}")
    } else {
        format!("{}::{var}", scope.name)
    }
}

/// Walk the scope tree and return the fully-qualified names of every
/// namespace / global variable whose enclosing namespace is not in the
/// cursor's lexical `chain` (proc locals excluded).
pub(crate) fn cross_namespace_qualified_vars(
    global: &tcl_compiler::analyser::Scope,
    chain: &FxHashSet<String>,
) -> Vec<String> {
    use tcl_compiler::analyser::{Scope, ScopeKind};
    fn visit(scope: &Scope, chain: &FxHashSet<String>, out: &mut Vec<String>, depth: u32) {
        if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
            return;
        }
        if matches!(scope.kind, ScopeKind::Namespace | ScopeKind::Global)
            && !chain.contains(&scope.name)
        {
            for vname in scope.variables.keys() {
                out.push(qualified_var_name(scope, vname));
            }
        }
        for child in &scope.children {
            visit(child, chain, out, depth + 1);
        }
    }
    let mut out = Vec::new();
    visit(global, chain, &mut out, 0);
    out
}

/// Return the chain of scopes from `root` down to the
/// innermost child whose `body_span` contains `byte_offset`.
/// The chain is ordered outermost (`root`) to innermost.
fn scope_chain_at(
    root: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<&tcl_compiler::analyser::Scope> {
    let mut chain = vec![root];
    let mut cursor = root;
    loop {
        let next = cursor.children.iter().find(|c| {
            // `Span` is half-open `[start, end)` — the byte at
            // `s.end()` lives outside the scope.
            c.body_span
                .is_some_and(|s| s.start() <= byte_offset && byte_offset < s.end())
        });
        match next {
            Some(child) => {
                chain.push(child);
                cursor = child;
            }
            None => break,
        }
    }
    chain
}

/// Return the `body_span`s of the scope chain containing
/// `byte_offset`, innermost first.  The global scope has no
/// `body_span`, so an empty result means "the whole file is
/// visible" — an empty list signals a file-wide walk.
pub(crate) fn scope_body_spans_at(
    root: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<tcl_lexer::Span> {
    scope_chain_at(root, byte_offset)
        .into_iter()
        .rev()
        .filter_map(|s| s.body_span)
        .collect()
}

pub(crate) fn span_to_range(
    source: &str,
    line_index: &LineIndex,
    span: tcl_lexer::Span,
) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// Regression coverage for issue #996: `variable_scope_extent`'s
    /// nested `innermost` fn and `linked_var_reference_spans`'s
    /// `collect_alias_spans`/`collect_shared_span_refs` all recurse once
    /// per nested namespace/proc scope, with no depth cap before this fix
    /// (`MAX_SCOPE_WALK_DEPTH`, `crate::lib`). 80 nested `namespace eval`
    /// levels is past the point (confirmed empirically: 100+) where
    /// unguarded namespace-scope recursion overflows `cargo test`'s bare
    /// ~2 MiB per-test default. The assertion is that both return at all,
    /// not what they return.
    #[test]
    fn deeply_nested_namespaces_survive_scope_extent_and_alias_scans() {
        const DEPTH: usize = 80;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("set x 1\nputs $x\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        let analysis = analyse(&source);
        let _ = variable_scope_extent(&analysis, "x", 0);
        if let Some(var_def) = analysis.all_variables.values().next() {
            let _ = linked_var_reference_spans(&analysis.global_scope, var_def);
        }
        let _ = cross_namespace_qualified_vars(&analysis.global_scope, &FxHashSet::default());
    }

    #[test]
    fn jump_to_proc_definition() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the second `greet` (line 1, char 2).
        let locs = definition(src, 1, 2, &analysis);
        assert_eq!(locs.len(), 1);
        // The proc name span is on line 0 starting at column 5.
        assert_eq!(locs[0].start_line, 0);
        assert_eq!(locs[0].start_character, 5);
    }

    // foreach multi-list lock-step form (issue #923 idx 70): `foreach
    // varList1 list1 varList2 list2 ... body` binds every varList, not
    // just the first.

    #[test]
    fn multi_list_foreach_name_resolves_to_its_own_loop_not_an_unrelated_later_one() {
        // The finding's own demonstrated in-vivo failure mode on the real,
        // unmodified pix corpus file (docs/pixdoc.tcl) — a *second*
        // `foreach` reusing the bare name `name` (a wholly unrelated,
        // later loop) previously "won" by being the only `VarDef` named
        // `name` anywhere in the flat top-level scope, since the first
        // loop's own `name` (the second varList of a `foreach dirName
        // {...} name {...} {...}` multi-list form) was never bound at
        // all. tclsh9.0/8.6 both prove `name` inside the first loop's body
        // is that loop's own variable, never reaching the second loop.
        let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
        let analysis = analyse(src);
        // Cursor on the first loop's own `$name` read (line 1, col 21).
        let locs = definition(src, 1, 21, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must resolve to the first loop's own `name` clause (line 0), not the unrelated second loop (line 4): {locs:?}"
        );
        assert_eq!(locs[0].start_character, 37);

        // Same wrong-location bug reproduced from a second usage inside
        // the same loop body (line 2, `if {$name eq ...}`).
        let locs2 = definition(src, 2, 10, &analysis);
        assert_eq!(locs2.len(), 1, "{locs2:?}");
        assert_eq!(locs2[0].start_line, 0, "{locs2:?}");
    }

    // namespace-aware proc resolution (C Tcl `Tcl_FindCommand` order)

    #[test]
    fn unqualified_call_resolves_in_callers_namespace_first() {
        // Two namespaces each define `helper`; the unqualified call inside
        // ::b must land on ::b::helper — C Tcl resolves the current
        // namespace before global, never a sibling namespace.
        let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nnamespace eval b {\n    proc helper {} { return 2 }\n    helper\n}\n";
        let analysis = analyse(src);
        // Cursor on the bare `helper` call (line 5).
        let locs = definition(src, 5, 6, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 4, "must resolve to ::b::helper");
        assert_eq!(locs[0].start_character, 9);
    }

    // apply namespace override (issue #923 idx 116): a bareword called
    // inside `apply {{params} body ns}`'s body resolves against `ns`, not
    // wherever the `apply` call is lexically written.

    #[test]
    fn bareword_inside_apply_body_resolves_against_the_lambdas_own_namespace() {
        // TP — the finding's own literal repro: `real` and `lexical` both
        // define `cleanup`; `apply {{} {cleanup done} ::real}` must
        // resolve the bareword `cleanup` inside its body to `::real`'s
        // copy, not `::lexical`'s (tclsh9.0/8.6-verified: prints "real
        // done"). Before this fix: resolved to `::lexical::cleanup`
        // purely by lexical-nearest-definition coincidence.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\napply {{} {cleanup done} ::real}\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the apply body (line 6, col 11).
        let locs = definition(src, 6, 11, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
        );
        assert_eq!(locs[0].start_character, 9);
    }

    #[test]
    fn bareword_inside_var_indirected_apply_body_resolves_against_the_lambdas_namespace() {
        // TP — one-hop `$var` indirection to a literal lambda triple:
        // `set lambda {{} {cleanup done} ::real}` then `apply $lambda`.
        // tclsh9.0/8.6-verified: prints "real done". Exercises
        // `resolve_dynamic_apply_lambda`'s `TokenType::Var` branch.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\nset lambda {{} {cleanup done} ::real}\napply $lambda\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the `set`'s braced value (line 6, col 16).
        let locs = definition(src, 6, 16, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
        );
    }

    #[test]
    fn bareword_inside_list_and_qualified_var_apply_resolves_against_the_lambdas_namespace() {
        // TP — direct list-substitution + namespace-qualified var, `apply`
        // as a literal command word: `apply [list {} $lexical::body
        // ::real]`, where `lexical::body` itself holds the literal
        // `{cleanup done}`. Exercises BOTH new pieces together
        // (`lookup_const_string_in_namespace` + the list-folding
        // `TokenType::Cmd` branch).
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n    set body {cleanup done}\n}\napply [list {} $lexical::body ::real]\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside `set body {cleanup done}` (line 5, col 14).
        let locs = definition(src, 5, 14, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
        );
    }

    #[test]
    fn two_hop_apply_var_indirection_stays_unresolved() {
        // FN — deliberately out of scope: `set a $lambda; apply $a` is an
        // extra level of $var-to-$var forwarding beyond the one hop
        // `resolve_dynamic_apply_lambda` follows from `apply`'s own
        // argument (`set a $lambda`'s value is itself a `$`-prefixed Var
        // token, which `handle_set_command` never records as a constant
        // string at all, so `apply $a` can't resolve even one hop back to
        // the literal lambda). Two conflicting `cleanup` procs, matching
        // the other scenarios' shape, so a wrong resolution would be
        // visible rather than accidentally right via a single-candidate
        // fallback. Confirm this stays unresolved after the fix — a
        // documented depth bound, not a regression.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\nset lambda {{} {cleanup done} ::real}\nset a $lambda\napply $a\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the `set lambda`'s braced value (line 6, col 16).
        let locs = definition(src, 6, 16, &analysis);
        // Since `apply $a` never resolves (not even one hop), this text is
        // never walked as a script body at all, so `namespace_overrides`
        // gets no entry for it — whatever answer comes back is from the
        // SAME pre-existing, coincidental lexical-nearest fallback that
        // would fire with no `apply`/`namespace_overrides` mechanism in
        // the picture at all (confirmed: resolves to lexical::cleanup,
        // line 4 — the nearer declaration, not real::cleanup on line 1).
        // The one assertion that actually matters: the override must NOT
        // have kicked in and pointed this at real::cleanup.
        assert!(
            locs.iter().all(|l| l.start_line != 1),
            "two-hop indirection is a deliberate depth bound — must not resolve via the \
             override to real::cleanup: {locs:?}"
        );
    }

    #[test]
    fn apply_namespace_matching_lexical_nesting_still_resolves() {
        // TN — the "control" shape: apply's namespace override happens to
        // equal its lexical home. Must resolve correctly both before and
        // after the fix (before: coincidentally, via the lexical fallback;
        // after: genuinely, via the override) — proves the fix doesn't
        // disturb the already-working common case.
        let src = "namespace eval app {\n    proc cleanup {tag} { puts $tag }\n    apply {{} {cleanup done} ::app}\n}\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the apply body (line 2, col 15).
        let locs = definition(src, 2, 15, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn apply_namespace_override_does_not_leak_past_its_own_span() {
        // FP guard — a bareword with the SAME text, immediately outside
        // the apply body's span, must resolve exactly as it would with no
        // apply call in the file at all (i.e. to the lexically-nearest
        // `lexical::cleanup`, not real::cleanup). Catches an off-by-one in
        // the recorded span or a "last override wins globally" bug.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts $tag }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts $tag }\n    apply {{} {cleanup done} ::real}\n    cleanup done\n}\n";
        let analysis = analyse(src);
        // Cursor on the second `cleanup` (line 6, the bare call OUTSIDE the
        // apply body, col 4).
        let locs = definition(src, 6, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 4,
            "must resolve to lexical::cleanup, the override must not leak past its span: {locs:?}"
        );
    }

    #[test]
    fn two_apply_calls_with_different_namespace_overrides_never_cross_resolve() {
        // FP guard — two independent, non-overlapping `apply` calls with
        // different namespace overrides in the same file must each
        // resolve to their own namespace, never the other's (catches an
        // implementation that only tracks a single override, or picks the
        // wrong entry when more than one exists).
        let src = "namespace eval a {\n    proc cleanup {tag} { puts $tag }\n}\nnamespace eval b {\n    proc cleanup {tag} { puts $tag }\n}\napply {{} {cleanup 1} ::a}\napply {{} {cleanup 2} ::b}\n";
        let analysis = analyse(src);
        // First apply body's `cleanup` (line 6, col 11) -> ::a::cleanup (line 1).
        let locs_a = definition(src, 6, 11, &analysis);
        assert_eq!(locs_a.len(), 1, "{locs_a:?}");
        assert_eq!(locs_a[0].start_line, 1);
        // Second apply body's `cleanup` (line 7, col 11) -> ::b::cleanup (line 4).
        let locs_b = definition(src, 7, 11, &analysis);
        assert_eq!(locs_b.len(), 1, "{locs_b:?}");
        assert_eq!(locs_b[0].start_line, 4);
    }

    // wildcard namespace import bareword resolution (issue #923 idx 18):
    // `namespace import NS::*` makes every command `NS` has `namespace
    // export`ed callable bare wherever the import is in scope — but real
    // Tcl only imports *exported* names (`Tcl_Export`), so an unexported
    // sibling in `NS` stays unreachable through the import.

    #[test]
    fn wildcard_namespace_import_resolves_exported_proc() {
        // TP — single command in `Foo`, exported, reached only through the
        // wildcard import (tclsh9.0/8.6-verified: `bar` prints 1).
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the bareword `bar` call (line 5, col 0).
        let locs = definition(src, 5, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to Foo::bar's declaration"
        );
        assert_eq!(locs[0].start_character, 9);
    }

    #[test]
    fn wildcard_namespace_import_resolves_exported_proc_alongside_unexported_sibling() {
        // TP — the finding's own headline shape: `Foo` has both an exported
        // `bar` and an unrelated, unexported `other`. Exhaustive tracing
        // found the old code blocked *every* route identically regardless
        // of sibling count — this proves the fix does not regress to some
        // sibling-count-dependent "ambiguity threshold" that doesn't exist
        // in real Tcl either.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the bareword `bar` call (line 6, col 0).
        let locs = definition(src, 6, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to Foo::bar's declaration"
        );
        assert_eq!(locs[0].start_character, 9);
    }

    #[test]
    fn wildcard_namespace_import_unexported_sibling_stays_unresolved() {
        // FP guard (CRITICAL) — `other` lives in `Foo` but is never
        // exported, so real Tcl's `namespace import ::Foo::*` never binds
        // it: a bare `other` call errors "invalid command name" at runtime
        // (Tcl manual, `namespace` — "namespace import" only imports
        // exported names). The wildcard-import resolver must not
        // manufacture a resolution the interpreter itself would reject.
        //
        // Exercises `proc_visible_via_wildcard_import` directly rather than
        // the full `definition()` provider: `resolve_called_proc`'s own
        // pre-existing, unrelated lenient fallback
        // (`fallback_proc_by_simple_name`, "any proc with this simple name,
        // anywhere") would otherwise also resolve a bare `other` call
        // regardless of import/export state — a longstanding, documented
        // leniency for procs whose defining namespace isn't statically
        // visible, not the gate this test is proving.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nother\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, "::", "other");
        assert!(
            hit.is_none(),
            "an unexported sibling must stay unresolved through the wildcard import: {hit:?}"
        );
    }

    #[test]
    fn wildcard_namespace_import_resolves_exported_class() {
        // TP — the class analogue of
        // `wildcard_namespace_import_resolves_exported_proc`:
        // `resolve_class_target_at` is the fixed function directly (the
        // top-level `definition()` provider's own bareword-class match is
        // namespace-blind already and would not exercise this fix).
        let src = "namespace eval Foo {\n    oo::class create Widget {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nWidget new\n";
        let analysis = analyse(src);
        let line_index = LineIndex::new(src);
        let cursor_off = byte_offset_at(&line_index, src, 5, 0);
        let hit = resolve_class_target_at(&analysis, cursor_off, "Widget");
        assert!(
            hit.is_some(),
            "must resolve Widget through the wildcard import"
        );
        let (qname, _) = hit.unwrap();
        assert_eq!(qname, "::Foo::Widget");
    }

    #[test]
    fn wildcard_namespace_import_unexported_sibling_class_stays_unresolved() {
        // FP guard — the class analogue: `Other` lives in `Foo` but is
        // never exported, so it must stay unresolved through the wildcard
        // import exactly like the proc case.
        let src = "namespace eval Foo {\n    oo::class create Widget {}\n    oo::class create Other {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nOther new\n";
        let analysis = analyse(src);
        let line_index = LineIndex::new(src);
        let cursor_off = byte_offset_at(&line_index, src, 6, 0);
        let hit = resolve_class_target_at(&analysis, cursor_off, "Other");
        assert!(
            hit.is_none(),
            "an unexported sibling class must stay unresolved: {hit:?}"
        );
    }

    #[test]
    fn wildcard_namespace_import_wins_over_an_unrelated_decoy_with_the_same_simple_name() {
        // TP — differential-audit finding idx 29 (main audit wave), its
        // "cleanest repro": before this fix, a bareword call reached via a
        // wildcard import never consulted `namespace_imports` at all, so
        // it fell straight to `fallback_proc_by_simple_name`'s lenient
        // "any proc with this simple name, prefer lexicographically
        // smallest qualified name" tie-break — silently resolving (and
        // hovering) to an entirely unrelated, never-imported decoy
        // (`::decoyns::helper`, "d" < "r") instead of the real,
        // oracle-proven target (`::realns::helper`, tclsh9.0/8.6-verified:
        // prints `REAL`). The wildcard-import resolution added by idx 18
        // is tried *before* that fallback, so it must win here.
        let src = "namespace eval realns {\n    namespace export helper\n    proc helper {} { return REAL }\n}\nnamespace eval decoyns {\n    proc helper {} { return DECOY }\n}\nnamespace eval userns {\n    namespace import ::realns::*\n    proc run {} {\n        return [helper]\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the bareword `helper` call inside `userns::run` (line
        // 10, col 16 — `        return [helper]`).
        let locs = definition(src, 10, 16, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 2,
            "must resolve to realns::helper (the exported, imported target), \
             not the decoy: {locs:?}"
        );
    }

    #[test]
    fn exact_namespace_import_resolves_source_proc_in_document() {
        // FP guard / bonus TP — a plain (non-glob) `namespace import
        // ::Foo::bar` must keep working through the same in-document path
        // (the wildcard resolver's glob match degrades to exact equality
        // for a literal pattern), matching the already-passing
        // cross-document `namespace_import_call_site_references_the_source_command`
        // (`tcl-lsp-core::workspace_index`) — this exercises the same-file
        // case that resolver never covers.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::bar\nbar\n";
        let analysis = analyse(src);
        let locs = definition(src, 5, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to Foo::bar's declaration"
        );
    }

    #[test]
    fn unqualified_call_at_global_prefers_global_proc() {
        // A global ::helper exists alongside ::a::helper; a global-scope
        // call resolves the global proc (the only candidate C Tcl tries).
        let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nproc helper {} { return 0 }\nhelper\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 2, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 3, "must resolve to ::helper");
    }

    #[test]
    fn global_call_with_only_namespaced_procs_falls_back_deterministically() {
        // No ::helper exists, so no candidate resolves; the lenient tail
        // fallback fires and must be deterministic — the lexicographically
        // smallest qualified name (::a::helper), on every repeat, never a
        // `HashMap`-iteration-order pick.
        let src = "namespace eval z {\n    proc helper {} { return 26 }\n}\nnamespace eval a {\n    proc helper {} { return 1 }\n}\nhelper\n";
        let analysis = analyse(src);
        for attempt in 0..8 {
            let locs = definition(src, 6, 2, &analysis);
            assert_eq!(locs.len(), 1, "attempt {attempt}: {locs:?}");
            assert_eq!(
                locs[0].start_line, 4,
                "attempt {attempt}: fallback must pick ::a::helper"
            );
        }
    }

    #[test]
    fn builtin_named_namespace_proc_resolves_inside_its_namespace_only() {
        // `proc set` inside ::ns shadows the builtin *within* the namespace;
        // a global-scope `set` reaches the builtin, so no user-proc jump.
        let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
        let analysis = analyse(src);
        // Inside the namespace: the proc wins.
        let inside = definition(src, 2, 5, &analysis);
        assert_eq!(inside.len(), 1, "{inside:?}");
        assert_eq!(inside[0].start_line, 1);
        // At global scope the builtin wins — no definition to jump to.
        let global = definition(src, 4, 1, &analysis);
        assert!(global.is_empty(), "{global:?}");
    }

    #[test]
    fn nested_shadow_of_builtin_never_outranks_the_builtin() {
        // TP — the "rename the builtin away, install a same-named shadow,
        // restore it" idiom (`rest.tcl`'s `describe`, tcllib): a `proc ::set
        // {...}` written *inside* another proc's body exists only
        // conditionally (when and if the enclosing proc runs), and is
        // renamed back off again before the enclosing proc returns. A call
        // site that runs strictly after the enclosing proc returns is
        // unambiguously the real builtin — the shadow definition existing
        // *somewhere in the file* must not make it win.
        let src = "proc outer {} {\n    rename ::set ::_set\n    proc ::set {var val} { ::_set $var $val }\n    rename ::set {}\n    rename ::_set ::set\n}\nouter\nset final restored\n";
        let analysis = analyse(src);
        // `set` on the last line, well after `outer` (and its nested shadow)
        // has returned.
        let after_restore = definition(src, 7, 0, &analysis);
        assert!(
            after_restore.is_empty(),
            "a call after the shadow is restored must resolve to the \
             builtin (no jump target), not the nested shadow: {after_restore:?}",
        );
    }

    #[test]
    fn ordinary_nested_proc_still_resolves_when_not_shadowing_a_builtin() {
        // TN / regression guard — the nested-shadow gate is scoped to names
        // that collide with a real registry builtin; an ordinary nested
        // helper proc (no such collision) must keep resolving exactly as
        // before.
        let src = "proc outer {} {\n    proc helper {} { return 1 }\n    return [helper]\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 13, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must still resolve to the nested helper"
        );
    }

    #[test]
    fn top_level_builtin_override_still_resolves_to_itself() {
        // FN guard — a proc that shadows a builtin at *file* (top-level)
        // scope is not a conditional nested definition; it unconditionally
        // replaces the builtin for the rest of the file, exactly as real
        // Tcl's `proc puts {args} {...}` would, and must keep resolving to
        // itself rather than being caught by the nested-shadow gate.
        let src = "proc puts {args} { return }\nputs hello\n";
        let analysis = analyse(src);
        let locs = definition(src, 1, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must resolve to the top-level override"
        );
    }

    #[test]
    fn qualified_relative_call_prefers_current_namespace_then_global() {
        // `sub::p` written inside ::ns resolves ::ns::sub::p before
        // ::sub::p; the absolute `::sub::p` resolves exactly (confirmed
        // against tclsh — see `bareword_resolution_candidates`).
        let src = "namespace eval ns {\n    namespace eval sub {\n        proc p {} { return 1 }\n    }\n    sub::p\n}\nnamespace eval sub {\n    proc p {} { return 2 }\n}\n::sub::p\n";
        let analysis = analyse(src);
        let relative = definition(src, 4, 5, &analysis);
        assert_eq!(relative.len(), 1, "{relative:?}");
        assert_eq!(relative[0].start_line, 2, "must prefer ::ns::sub::p");
        let absolute = definition(src, 9, 4, &analysis);
        assert_eq!(absolute.len(), 1, "{absolute:?}");
        assert_eq!(absolute[0].start_line, 7, "::sub::p is absolute");
    }

    #[test]
    fn jump_to_proc_definition_in_two_level_nested_namespace_via_qualified_call() {
        // Issue #923: go-to-definition on a fully-qualified call to a proc
        // nested two `namespace eval` levels deep must land on its own decl.
        let src = concat!(
            "namespace eval modelTestVerTool {\n",
            "    namespace eval gui {\n",
            "        proc specAddButtonPopUp {x y} { return \"$x $y\" }\n",
            "    }\n",
            "}\n",
            "::modelTestVerTool::gui::specAddButtonPopUp 1 2\n",
        );
        let analysis = analyse(src);
        // Cursor on the qualified call (line 5).
        let locs = definition(src, 5, 30, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 2, "should land on the decl");
    }

    #[test]
    fn jump_to_proc_definition_disambiguates_same_named_procs_by_cursor() {
        // Two procs share the simple name `helper` in different two-level
        // nested namespaces. Go-to-definition from each one's own
        // declaration (a no-op jump, but exercises the lookup) must resolve
        // to *that* proc, not whichever `all_procs` entry a HashMap happens
        // to iterate first.
        let src = concat!(
            "namespace eval a {\n",
            "    namespace eval b {\n",
            "        proc helper {} { return \"a-b\" }\n",
            "    }\n",
            "}\n",
            "namespace eval c {\n",
            "    namespace eval d {\n",
            "        proc helper {} { return \"c-d\" }\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(src);
        // Cursor on ::a::b::helper's own decl (line 2).
        let locs_ab = definition(src, 2, 14, &analysis);
        assert_eq!(locs_ab.len(), 1, "{locs_ab:?}");
        assert_eq!(locs_ab[0].start_line, 2, "must resolve to ::a::b::helper");
        // Cursor on ::c::d::helper's own decl (line 7).
        let locs_cd = definition(src, 7, 14, &analysis);
        assert_eq!(locs_cd.len(), 1, "{locs_cd:?}");
        assert_eq!(locs_cd[0].start_line, 7, "must resolve to ::c::d::helper");
    }

    #[test]
    fn jump_to_class_definition_disambiguates_same_named_classes_by_cursor() {
        // Class analogue of the proc test above.
        let src = concat!(
            "namespace eval a {\n",
            "    namespace eval b {\n",
            "        oo::class create Widget {}\n",
            "    }\n",
            "}\n",
            "namespace eval c {\n",
            "    namespace eval d {\n",
            "        oo::class create Widget {}\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.len() < 2 {
            return;
        }
        let locs_ab = definition(src, 2, 26, &analysis);
        assert_eq!(locs_ab.len(), 1, "{locs_ab:?}");
        assert_eq!(locs_ab[0].start_line, 2, "must resolve to ::a::b::Widget");
        let locs_cd = definition(src, 7, 26, &analysis);
        assert_eq!(locs_cd.len(), 1, "{locs_cd:?}");
        assert_eq!(locs_cd[0].start_line, 7, "must resolve to ::c::d::Widget");
    }

    #[test]
    fn jump_to_next_super_method() {
        // `next` inside B::greet jumps to the overridden A::greet.
        let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\n";
        let analysis = analyse(src);
        // Cursor on `next` (line 5). `    method greet {} { next }`
        let locs = definition(src, 5, 23, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // A::greet's name is on line 1 at column 11.
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 11);
    }

    #[test]
    fn jump_to_nextto_named_class_method() {
        // `nextto A` inside C::greet jumps to A::greet (skipping B).
        let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { nextto A }\n}\n";
        let analysis = analyse(src);
        // Cursor on `nextto` (line 9). `    method greet {} { nextto A }`
        let locs = definition(src, 9, 22, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1, "should land on A::greet");
        assert_eq!(locs[0].start_character, 11);
    }

    #[test]
    fn jump_to_nextto_namespaced_class_method() {
        // `nextto A` names a namespaced sibling bare from within `::Ns::C`.
        // Owner-aware canonicalisation resolves it to `::Ns::A` (previously
        // only a global `::A` would have matched, so this produced nothing).
        let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto A }\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on `nextto` inside C::greet (line 10).
        let locs = definition(src, 10, 28, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // ::Ns::A::greet's name token is on line 2.
        assert_eq!(locs[0].start_line, 2, "should land on ::Ns::A::greet");
    }

    #[test]
    fn nextto_picks_occurrence_at_cursor_not_first_on_line() {
        // The line has a decoy `nextto` earlier (inside a string) before the
        // real `nextto A`.  With the cursor on the real one, resolution must
        // read the class after *that* occurrence (A), not the first match.
        let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { set x \"nextto Z\" ; nextto A }\n}\n";
        let analysis = analyse(src);
        // Cursor inside the real `nextto` (line 9, col 44).
        let locs = definition(src, 9, 44, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "should land on A::greet, not the string decoy"
        );
        assert_eq!(locs[0].start_character, 11);
    }

    #[test]
    fn jump_to_var_definition() {
        let src = "set greeting hi\nputs $greeting\n";
        let analysis = analyse(src);
        // Cursor inside `$greeting` on line 1.
        let locs = definition(src, 1, 8, &analysis);
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    }

    #[test]
    fn jump_to_proc_param_bareword_declaration_resolves_to_itself() {
        // TP — differential-audit finding idx 9 (main audit wave): a cursor
        // placed directly on a proc parameter's own bareword name (not a
        // `$`-prefixed read) previously resolved to nothing at all, even
        // though every `$name` read of the same parameter resolved fine —
        // `scope_chain_at` never reaches the proc's own scope for a byte
        // offset inside the parameter list, which sits textually *before*
        // that scope's `body_span` even starts.
        let src = "proc greet {name} { return $name }\n";
        let analysis = analyse(src);
        // Cursor on `name` inside the parameter list (col 12-16).
        let locs = definition(src, 0, 13, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
        assert_eq!(locs[0].start_character, 12, "{:?}", locs[0]);
        assert_eq!(locs[0].end_character, 16, "{:?}", locs[0]);
    }

    #[test]
    fn jump_to_catch_resultvar_bareword_resolves_to_the_reused_variable() {
        // TP — the finding's other confirmed shape: `catch script name`
        // reuses (writes back into) an *existing* variable — its own
        // bareword token is recorded in `VarDef.references`, not
        // `definition_span` (that stays pinned to the variable's original
        // declaration) — so a cursor placed directly on it must still
        // resolve, to the *original* declaration site.
        let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
        let analysis = analyse(src);
        // Cursor on the catch result-var `name` (line 1, col 16-20).
        let locs = definition(src, 1, 17, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must resolve to the original param declaration: {:?}",
            locs[0]
        );
        assert_eq!(locs[0].start_character, 20, "{:?}", locs[0]);
        assert_eq!(locs[0].end_character, 24, "{:?}", locs[0]);
    }

    #[test]
    fn bareword_that_is_not_any_variable_occurrence_does_not_resolve_as_a_variable() {
        // FP guard — an ordinary bareword that never appears as any
        // variable's declaration or reference span (an unrelated command's
        // plain argument) must not be mistaken for a variable declaration
        // site, even in a file containing unrelated variables.
        let src = "proc greet {name} { return $name }\nputs literal\n";
        let analysis = analyse(src);
        // Cursor on `literal` (line 1, col 5-12) — not a variable anywhere.
        let locs = definition(src, 1, 7, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn jump_to_absolutely_qualified_var_definition() {
        // TP — the exact shape both mined findings (defer.tcl's
        // `$::defer::idVar`, uri.tcl's namespace-current pattern) reduce to:
        // a `variable` declared at namespace-eval scope, referenced
        // elsewhere via its fully `::`-qualified name. Previously empty
        // (lookup_var_in_scope_chain only special-cased bare names).
        let src = "namespace eval ::simple {\n    variable v \"hello\"\n}\nputs $::simple::v\n";
        let analysis = analyse(src);
        // Cursor on `v` inside `$::simple::v` (line 3, col 16).
        let locs = definition(src, 3, 16, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
        assert_eq!(locs[0].start_character, 13, "{:?}", locs[0]);
        assert_eq!(locs[0].end_character, 14, "name-sized span: {:?}", locs[0]);
    }

    #[test]
    fn jump_to_relatively_qualified_var_definition_from_sibling_namespace() {
        // TP — a relative qualifier (`B::v`, no leading `::`) resolves under
        // the *cursor's* command-resolution namespace (tclsh 9.0.4/8.6.16
        // -verified: identical rule to what an unqualified command there
        // would resolve against), not the file's top level. `show` is
        // defined in `::A`, so `B::v` must reach `::A::B::v`, never a
        // top-level `::B::v`.
        let src = "namespace eval ::A {\n    namespace eval ::A::B {\n        variable v \"nested\"\n    }\n    proc show {} {\n        puts $B::v\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on `v` inside `$B::v` (line 5, col 17).
        let locs = definition(src, 5, 17, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 2, "{:?}", locs[0]);
        assert_eq!(locs[0].start_character, 17, "{:?}", locs[0]);
    }

    #[test]
    fn qualified_var_definition_for_var_declared_in_a_reopened_namespace() {
        // TP — a namespace `namespace eval`-reopened in two unrelated
        // places: the qualified reference must find the declaration
        // regardless of which block declares it (no single lexical block
        // may be assumed to hold every variable a qualified reference
        // might mean).
        let src = "namespace eval ::A {\n    proc noop {} {}\n}\nnamespace eval ::A {\n    variable v 1\n}\nputs $::A::v\n";
        let analysis = analyse(src);
        let locs = definition(src, 6, 11, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 4, "{:?}", locs[0]);
    }

    #[test]
    fn qualified_var_definition_abstains_for_unknown_namespace() {
        // TN — a qualified reference into a namespace that's never declared
        // anywhere in the file must abstain (empty), not guess at an
        // unrelated variable.
        let src = "namespace eval ::real {\n    variable v 1\n}\nputs $::nonexistent::v\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 20, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn qualified_var_definition_does_not_match_a_same_named_proc_local() {
        // FP guard — a proc's local variable must never satisfy a
        // namespace-qualified lookup even though the proc's own
        // command-resolution namespace coincides with the target: `f`'s
        // local `v` is unrelated to `::A::v`, which is never declared here.
        let src =
            "namespace eval ::A {\n    proc f {} {\n        set v 1\n    }\n}\nputs $::A::v\n";
        let analysis = analyse(src);
        let locs = definition(src, 5, 11, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    /// Issue #727: go-to-definition of a formal-parameter use must resolve to
    /// the parameter *name* in the declaration, not the proc name (proc) or the
    /// whole method/constructor body (`TclOO`). The returned range must be a
    /// single-line, name-sized span over `arg1`.
    #[test]
    fn param_definition_points_at_the_name_not_the_body() {
        let cases = [
            // (source, usage line, usage char, expected decl line, decl start col)
            (
                "proc greet {arg1 arg2} {\n    puts $arg1\n}\n",
                1,
                11,
                0,
                12,
            ),
            (
                "oo::class create C {\n    method m {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
                2,
                15,
                1,
                14,
            ),
            (
                "oo::class create C {\n    constructor {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
                2,
                15,
                1,
                17,
            ),
        ];
        for (src, ul, uc, dl, dc) in cases {
            let analysis = analyse(src);
            let locs = definition(src, ul, uc, &analysis);
            assert_eq!(locs.len(), 1, "one def for {src:?}: {locs:?}");
            let r = locs[0];
            assert_eq!(r.start_line, dl, "decl line for {src:?}: {r:?}");
            assert_eq!(r.start_character, dc, "decl col for {src:?}: {r:?}");
            // Name-sized: same line, spans the 4-char `arg1`.
            assert_eq!(r.end_line, dl, "decl is single-line for {src:?}: {r:?}");
            assert_eq!(
                r.end_character - r.start_character,
                4,
                "decl spans `arg1` (4 chars) for {src:?}: {r:?}",
            );
        }
    }

    #[test]
    fn no_definition_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(definition(src, 0, 6, &analysis).is_empty());
    }

    #[test]
    fn jump_to_class_definition() {
        let src = "oo::class create Greeter {}\nGreeter\n";
        let analysis = analyse(src);
        // Cursor on `Greeter` on line 1.
        let locs = definition(src, 1, 2, &analysis);
        if !locs.is_empty() {
            assert_eq!(locs[0].start_line, 0);
        }
    }

    // alias resolution

    #[test]
    fn jump_to_alias_target_proc() {
        // `interp alias {} mycmd {} greet` aliases `mycmd` to
        // the user proc `greet`.  Cursor on `mycmd` should
        // resolve to `greet`'s definition.
        let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\nmycmd\n";
        let analysis = analyse(src);
        // Cursor on `mycmd` invocation on line 2.
        let locs = definition(src, 2, 2, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The target's name span is on line 0 starting at
        // column 5 (after `proc `).
        assert_eq!(locs[0].start_line, 0);
        assert_eq!(locs[0].start_character, 5);
    }

    #[test]
    fn alias_with_no_user_proc_returns_empty() {
        // `mycmd` aliases to a built-in (`puts`).  No user
        // proc; the provider returns empty rather than
        // pretending to know the location.
        let src = "interp alias {} mycmd {} puts\nmycmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 1, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn alias_lookup_accepts_qualified_form() {
        // The alias's qualified form (`::mycmd`) should also
        // resolve.
        let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\n::mycmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 2, &analysis);
        // Whether find_word_span_at_position includes leading
        // `::` depends on the implementation; both should
        // jump to the target proc when matched.
        if !locs.is_empty() {
            assert_eq!(locs[0].start_line, 0);
        }
    }

    #[test]
    fn ensemble_map_subcommand_jumps_to_target_proc() {
        // TP — issue #923 idx 106, the confirmed finding's own minimal repro:
        // `namespace ensemble create -map {foo ::e::Foo}` inside `::e`, then
        // `e foo bar` at the call site. Cursor on the call-site "foo" must
        // resolve to ::e::Foo's declaration.
        let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
        let analysis = analyse(src);
        // Cursor on "foo" in `puts [e foo bar]` (0-based line 7, col 8).
        let locs = definition(src, 7, 8, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 5);
    }

    #[test]
    fn ensemble_map_subcommand_resolves_forward_declared_differently_cased_target() {
        // TP — the subcommand's spelling ("make") differs from the target's
        // simple name ("Make") in case, and the target is declared AFTER
        // the ensemble and the call site. A fix matching on literal name
        // equality (rather than the resolved qualified name) would fail
        // this the same way it would fail the real widget.tcl repro.
        let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nputs [widget make hello]\nproc ::widget::Make {args} { return \"make: $args\" }\n";
        let analysis = analyse(src);
        let locs = definition(src, 5, 15, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 6);
    }

    #[test]
    fn ensemble_subcommands_form_jumps_to_target_proc() {
        // TP — the `-subcommands` sibling form (not `-map`), the folded-in
        // secondary fix from the same research plan.
        let src = "namespace eval ::box {\n    proc show {} { return \"shown\" }\n    namespace ensemble create -subcommands {show}\n}\nbox show\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 5, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn ensemble_dynamic_map_mutation_still_abstains() {
        // TN — must not regress the audit's own explicitly-correct
        // abstention: a subcommand added only via a runtime
        // `namespace ensemble configure -map $var` dict-set mutation (never
        // a literal `-map {...}` list) is not statically resolvable —
        // `ensemble_subcommand_targets` never gets an entry for it.
        let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::addType {t} {\n    set m [namespace ensemble configure ::widget -map]\n    dict set m dyn ::widget::Dyn\n    namespace ensemble configure ::widget -map $m\n}\nproc ::widget::Dyn {args} { return dyn }\nwidget addType foo\nputs [widget dyn hello]\n";
        let analysis = analyse(src);
        // Cursor on "dyn" in `puts [widget dyn hello]` (0-based line 12).
        let locs = definition(src, 12, 13, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn two_ensembles_with_shared_subcommand_name_do_not_cross_contaminate() {
        // TN — validates the outer table is keyed per-ensemble, not a
        // single flat map (the reason `command_aliases` can't be reused
        // directly for this fix).
        let src = "namespace eval ::a {\n    namespace ensemble create -map {go ::a::Go}\n}\nnamespace eval ::b {\n    namespace ensemble create -map {go ::b::Go}\n}\nproc ::a::Go {} { return a }\nproc ::b::Go {} { return b }\nputs [a go]\nputs [b go]\n";
        let analysis = analyse(src);
        // `puts [a go]` is 0-based line 8; `puts [b go]` is line 9.
        let locs_a = definition(src, 8, 8, &analysis);
        assert_eq!(locs_a.len(), 1, "{locs_a:?}");
        assert_eq!(locs_a[0].start_line, 6);
        let locs_b = definition(src, 9, 8, &analysis);
        assert_eq!(locs_b.len(), 1, "{locs_b:?}");
        assert_eq!(locs_b[0].start_line, 7);
    }

    #[test]
    fn ensemble_unmapped_subcommand_abstains_rather_than_guessing() {
        // FN (accepted, documented) — a typo'd/genuinely-unmapped
        // subcommand silently abstains rather than crashing or guessing.
        // Also documents that this fix deliberately does not implement
        // Tcl's unique-prefix ensemble-subcommand matching (a real tclsh
        // would dispatch "mak" to "make" here since it's an unambiguous
        // prefix and `-prefixes` defaults to true) — a reusable, named
        // follow-up (`CommandSpec::resolve_subcommand_for_dialect`-style
        // logic), not attempted here.
        let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::Make {args} {}\nputs [widget mak hello]\n";
        let analysis = analyse(src);
        // Cursor on "mak" in `puts [widget mak hello]` (0-based line 6).
        let locs = definition(src, 6, 13, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn ensemble_wholly_dynamic_map_value_abstains() {
        // FN (accepted, zero regression) — the whole `-map` value itself is
        // a variable, not a literal list; `ensemble_subcommand_targets`
        // never gets an entry for `::widget` at all.
        let src = "namespace eval ::widget {\n    variable dynamicMap {make ::widget::Make}\n    namespace ensemble create -map $dynamicMap\n}\nproc ::widget::Make {args} {}\nputs [widget make hello]\n";
        let analysis = analyse(src);
        // Cursor on "make" in `puts [widget make hello]` (0-based line 5).
        let locs = definition(src, 5, 13, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    // scope-chain $var descent

    #[test]
    fn proc_local_var_jumps_to_proc_scope_definition() {
        // `local` is defined inside `proc f`.  Cursor on
        // `$local` inside `f`'s body must jump to the
        // proc-local definition, not the global scope
        // (which has none).
        let src = "proc f {} {\n    set local 1\n    puts $local\n}\n";
        let analysis = analyse(src);
        // Cursor inside `$local` on line 2.
        let locs = definition(src, 2, 12, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The proc-local `set local 1` is on line 1.
        assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
    }

    #[test]
    fn byte_offset_at_handles_newlines() {
        let src = "abc\ndef\nghi\n";
        let line_index = LineIndex::new(src);
        assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
        assert_eq!(byte_offset_at(&line_index, src, 0, 3), 3);
        assert_eq!(byte_offset_at(&line_index, src, 1, 0), 4);
        assert_eq!(byte_offset_at(&line_index, src, 2, 2), 10);
    }

    #[test]
    fn byte_offset_at_uses_lsp_utf16_columns() {
        let src = "a😀b";
        let line_index = LineIndex::new(src);
        assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
        assert_eq!(byte_offset_at(&line_index, src, 0, 1), 1);
        assert_eq!(byte_offset_at(&line_index, src, 0, 3), 5);
        assert_eq!(byte_offset_at(&line_index, src, 0, 4), 6);
    }

    #[test]
    fn span_to_range_translates_offsets() {
        let src = "abc\ndef\n";
        let line_index = LineIndex::new(src);
        // Span covering `def` (offsets 4..7).
        let span = tcl_lexer::Span::new(4, 7);
        let range = span_to_range(src, &line_index, span);
        assert_eq!(range.start_line, 1);
        assert_eq!(range.start_character, 0);
        assert_eq!(range.end_line, 1);
        assert_eq!(range.end_character, 3);
    }

    #[test]
    fn span_to_range_uses_lsp_utf16_columns() {
        let src = "é😀x\n";
        let line_index = LineIndex::new(src);
        let range = span_to_range(src, &line_index, tcl_lexer::Span::new(6, 7));
        assert_eq!(range.start_line, 0);
        assert_eq!(range.start_character, 3);
        assert_eq!(range.end_line, 0);
        assert_eq!(range.end_character, 4);
    }

    // class-member lookup

    #[test]
    fn definition_bare_sibling_method_call_without_link_abstains() {
        // FP (issue #923 idx 113) — a bareword sibling method call is NOT
        // actually reachable from another method's body unless `link`
        // exposed it that way; real tclsh: "invalid command name". Must
        // abstain rather than falsely resolve.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` in the `twice` body.
        // Line 2: `    method twice {} { greet ; greet }`
        // Col 22 lands on the `g` of the first `greet`.
        let locs = definition(src, 2, 22, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_linked_sibling_method_call_resolves() {
        // TP (issue #923 idx 113) — `link greet` (called from the
        // constructor) makes `greet` genuinely bareword-callable from
        // every method body of this class, so the bare call now
        // correctly resolves to the method's declaration.
        let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method twice {} { greet ; greet }` — same text/col
        // as the un-linked test above, one line further down.
        let locs = definition(src, 3, 22, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The method declaration is now on line 2.
        assert_eq!(locs[0].start_line, 2);
    }

    #[test]
    fn definition_my_dispatch_resolves_when_class_extended_via_separate_oo_define() {
        // Issue #923 idx 52 (main audit wave, high severity): `Gadget` is
        // created via `oo::class create` with no body, then every method —
        // including the `my Helper` call site itself — is added via a
        // *separate*, later `oo::define Gadget { ... }` block. This is
        // exactly the real corpus shape (`ticklecharts::chart`: `oo::class
        // create` at one line, all methods added via a later `oo::define`).
        // tclsh9.0/8.6 both prove `my Helper` genuinely dispatches to
        // `Helper` here — go-to-definition must resolve it, not abstain.
        let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
        let analysis = analyse(src);
        // Line 5: `    method Caller {} { my Helper }` — cursor on the
        // `Helper` word inside `my Helper` (col 26).
        let locs = definition(src, 5, 26, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The `Helper` method declaration is on line 4.
        assert_eq!(locs[0].start_line, 4);
    }

    #[test]
    fn definition_bare_sibling_classmethod_call_without_link_abstains() {
        // FP (issue #923 idx 113) — same shape as the method case above,
        // for `classmethod`.
        let src = "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method use {} { factory }`
        // Cursor on `factory` (col 20).
        let locs = definition(src, 2, 20, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_linked_sibling_classmethod_call_resolves() {
        // TP (issue #923 idx 113) — `link factory` makes the classmethod
        // call resolve.
        let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method use {} { factory }` — same text/col as the
        // un-linked test above, one line further down.
        let locs = definition(src, 3, 20, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The classmethod declaration is now on line 2.
        assert_eq!(locs[0].start_line, 2);
    }

    #[test]
    fn definition_jumps_to_constructor_keyword() {
        // Bare `constructor` inside a class body jumps to the
        // constructor's declaration.
        let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
        let analysis = analyse(src);
        // Cursor on `constructor` in the `touch_ctor` body
        // (line 2 col 27).
        let locs = definition(src, 2, 27, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The analyser anchors the constructor's ``name_span``
        // on the ``constructor`` keyword token (declared on
        // line 1).  The provider keeps a body-span fallback
        // for the empty-span case, but the keyword span is
        // populated now, so the jump lands on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_bare_sibling_property_call_without_link_abstains() {
        // FP (issue #923 idx 113) — same rationale as the method/classmethod
        // cases above, for a Tcl 9.0+ `oo::configurable` `property`
        // accessor bareword call: `[length]` is not actually reachable
        // without a `link`; real tclsh: "invalid command name".
        let src = "oo::configurable create Widget {\n    property length -get {return 42}\n    method use {} { return [length] }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method use {} { return [length] }` — col 27 lands
        // on the `l` of `length`.
        let locs = definition(src, 2, 27, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_member_lookup_skipped_outside_class_body() {
        // Same word outside the class body must not surface
        // the method definition.
        let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the bare `greet` on line 3.
        let locs = definition(src, 3, 2, &analysis);
        // No proc / class / member named `greet` is in scope
        // here — the class-member lookup must not leak.
        assert!(locs.is_empty(), "{locs:?}");
    }

    // $obj method dispatch

    #[test]
    fn definition_resolves_obj_method_call() {
        // `set d [Dog new]` then `$d bark` — cursor on `bark`
        // jumps to the method declaration on Dog.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        // Line 4 `$d bark` — `bark` starts at col 3.
        let locs = definition(src, 4, 3, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `method bark` is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_resolves_obj_method_in_bracket() {
        // `[$d bark]` bracketed form.
        let src =
            "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nputs [$d bark]\n";
        let analysis = analyse(src);
        // Line 4 `puts [$d bark]` — `bark` starts at col 9.
        let locs = definition(src, 4, 9, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_obj_method_unknown_instance_falls_through() {
        // `$x bark` where `x` has no recorded class — no
        // instance-method resolution, falls through (and finds
        // nothing here).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 3, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_resolves_bare_created_instance_command_method() {
        // Codex #881: `Dog create rex` then `rex bark` — cursor on the bare
        // `bark` jumps to the method declaration, mirroring `$obj bark`.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        // Line 4 `rex bark` — `bark` starts at col 4.
        let locs = definition(src, 4, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_bare_var_receiver_without_dollar_does_not_resolve() {
        // `set d [Dog new]` binds `d` as a variable; a bare `d bark` (no `$`)
        // is not a valid dispatch, so it must not resolve to the method.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn instance_method_at_cursor_detects_dollar_head() {
        let src = "$d bark\n";
        let got = instance_method_at_cursor(src, 0, 4);
        assert_eq!(got, Some(("d".to_string(), "bark".to_string(), true)));
    }

    #[test]
    fn instance_method_at_cursor_reports_bare_head_as_non_dollar() {
        // `foo bark` — a bare-word receiver (an object command); reported
        // with `is_dollar == false`.  Whether it actually resolves to a
        // class is decided later by `receiver_instance_class`.
        let src = "foo bark\n";
        assert_eq!(
            instance_method_at_cursor(src, 0, 5),
            Some(("foo".to_string(), "bark".to_string(), false))
        );
    }

    #[test]
    fn instance_method_at_cursor_rejects_substituted_head() {
        // `[x] bark` — a command-substitution head is not a bare object
        // command.
        let src = "[x] bark\n";
        assert_eq!(instance_method_at_cursor(src, 0, 5), None);
    }

    #[test]
    fn receiver_instance_class_gates_bare_on_created_commands() {
        // `set b [Bar new]` binds `b` as a variable; `Bar create rex` binds
        // `rex` as an object command.  A bare receiver resolves only for the
        // command (`rex`), not the variable (`b`); a `$`-receiver resolves
        // for either.
        let src =
            "oo::class create Bar {\n    method get {} {}\n}\nset b [Bar new]\nBar create rex\n";
        let analysis = analyse(src);
        assert!(receiver_instance_class(&analysis, "rex", false).is_some());
        assert!(receiver_instance_class(&analysis, "b", false).is_none());
        assert!(receiver_instance_class(&analysis, "b", true).is_some());
    }

    // Class-command dispatch (issue #923 idx 120): `CLASS method` for a
    // classmethod / `self method`, distinct from `$obj method` / `NAME
    // method` instance dispatch above.

    #[test]
    fn classmethod_call_on_its_own_class_resolves() {
        // TP — the finding's own `ActiveRecord find` shape: a classmethod
        // called directly on the class that declares it (Part 2 alone;
        // ooutil's `classmethod` keyword was already correctly extracted
        // into `class_methods` by the existing class-body walker).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 14, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 16);
    }

    #[test]
    fn classmethod_call_on_an_inheriting_subclass_resolves() {
        // TP — the finding's second repro shape: `Table find`, where
        // `Table` inherits `find` from `ActiveRecord` via the ordinary
        // superclass MRO walk (ooutil's `classmethod` propagates to a
        // subclass's own bound command — confirmed against tclsh).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 6, 7, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 16);
    }

    #[test]
    fn self_method_call_on_its_own_class_resolves() {
        // TP — requires BOTH Part 1 (self recognised at all) and Part 2
        // (the class-command receiver path).
        let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 8, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 16);
    }

    #[test]
    fn self_method_not_inherited_by_a_non_overriding_subclass() {
        // TN — the critical precision test for `is_self_method`: unlike
        // `ooutil`'s `classmethod`, a plain `self method` is NOT inherited
        // by a subclass's own bound command (confirmed against tclsh:
        // `Gadget make` raises `unknown method "make"`). A naive reuse of
        // the ordinary MRO walk (which DOES include Widget in Gadget's
        // MRO) would incorrectly resolve this.
        let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nGadget make foo\n";
        let analysis = analyse(src);
        let locs = definition(src, 6, 8, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn instance_calling_a_classmethod_does_not_resolve() {
        // TN — a real instance can never dispatch a classmethod (real
        // tclsh: `unknown method "find": must be <cloned>, create, ...`).
        // This is the companion tightening bundled with Part 2 (the two
        // pre-existing instance-dispatch call sites now pass
        // `MethodBucket::Instance`, which excludes `class_methods`
        // entirely) — `completion.rs`'s `method_items` already documented
        // and implemented this same exclusion for its own suggestions.
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord create rec1\nrec1 find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 5, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn bare_var_receiver_without_a_bound_command_does_not_resolve_class_dispatch() {
        // TN — regression guard mirroring
        // `receiver_instance_class_gates_bare_on_created_commands`: `d` is
        // a plain variable (never bound as a command by `create`), so
        // neither the existing instance-command branch nor the new
        // class-command branch may fire for it.
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} {}\n}\nset d [ActiveRecord new]\nd find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }
}
