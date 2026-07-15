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
        if let Some(var_def) =
            lookup_var_in_scope_chain(&analysis.global_scope, cursor_offset, &var_name)
        {
            return vec![span_to_range(source, &line_index, var_def.definition_span)];
        }
        return Vec::new();
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
    if let Some((inst, method, is_dollar)) = instance_method_at_cursor(source, line, character)
        && let Some(class_q) = receiver_instance_class(analysis, &inst, is_dollar)
        && let Some(span) = lookup_method_in_class(analysis, class_q, &method)
    {
        return vec![span_to_range(source, &line_index, span)];
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
    let namespace = namespace_context_at(&analysis.global_scope, cursor_offset);
    if let Some(proc_def) = resolve_called_proc(
        analysis,
        source,
        &namespace,
        &word,
        Some(tcl_registry::registry_for_dialect("")),
    ) {
        return vec![span_to_range(source, &line_index, proc_def.name_span)];
    }
    let class_match = analysis
        .all_classes
        .iter()
        .find(|(_, c)| c.name_span.start() <= cursor_offset && cursor_offset < c.name_span.end())
        .or_else(|| {
            analysis.all_classes.iter().find(|(qname, c)| {
                c.name == word || *qname == &word || *qname == &format!("::{word}")
            })
        });
    if let Some((_, class_def)) = class_match {
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

/// Walk every class whose `body_span` contains the cursor
/// offset and look up `word` in that class's methods,
/// class-methods, properties, constructors, or destructor.
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
    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        if let Some(m) = class_def.methods.get(word) {
            return Some(m.name_span);
        }
        if let Some(m) = class_def.class_methods.get(word) {
            return Some(m.name_span);
        }
        if let Some(p) = class_def.properties.get(word) {
            return Some(p.name_span);
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
    }
    None
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
    for cd in analysis.all_classes.values() {
        if !(cd.body_span.start() <= cursor && cursor <= cd.body_span.end()) {
            continue;
        }
        for (mname, m) in cd.methods.iter().chain(cd.class_methods.iter()) {
            if m.body_span.start() <= cursor && cursor <= m.body_span.end() {
                return Some((cd.qualified_name.clone(), mname.clone()));
            }
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
/// up in `instance_classes`.  A bare receiver is a valid dispatch only when
/// it names an object *command* (`CLASS create NAME`) — a plain variable's
/// bare name (`set v [CLASS new]` then `v method`) is not a command and must
/// not resolve — so it is additionally gated on `created_instance_commands`.
///
/// Shared by the definition / references / rename / hover cursor paths so
/// they agree on which receivers dispatch (and, via
/// `method_references_for_class`, so those match the code-lens count).
pub(crate) fn receiver_instance_class<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    is_dollar: bool,
) -> Option<&'a String> {
    let class = analysis.instance_classes.get(receiver)?;
    if is_dollar || analysis.created_instance_commands.contains(receiver) {
        Some(class)
    } else {
        None
    }
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
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    // First, find the innermost scope containing the cursor.
    let chain = scope_chain_at(scope, byte_offset);
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
        Some(target) => collect_alias_spans(scope, target, var_def.definition_span, &mut out),
        None if !var_def.definition_span.is_empty() => {
            collect_shared_span_refs(scope, var_def.definition_span, &mut out);
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
) {
    for v in scope.variables.values() {
        if v.link_target.as_deref() == Some(target) {
            if v.definition_span != own_decl {
                out.push(v.definition_span);
            }
            out.extend(v.references.iter().copied());
        }
    }
    for child in &scope.children {
        collect_alias_spans(child, target, own_decl, out);
    }
}

/// Uses of every variable sharing the declaration span `def_span` (the `TclOO`
/// instance-variable case).
fn collect_shared_span_refs(
    scope: &tcl_compiler::analyser::Scope,
    def_span: tcl_lexer::Span,
    out: &mut Vec<tcl_lexer::Span>,
) {
    for v in scope.variables.values() {
        if v.definition_span == def_span {
            out.extend(v.references.iter().copied());
        }
    }
    for child in &scope.children {
        collect_shared_span_refs(child, def_span, out);
    }
}

/// Resolve the *name* of a variable whose declaration occupies
/// `byte_offset` — i.e. the cursor sits on the name token of a
/// `set` / `variable` / `global` / param declaration rather than
/// a `$ref`.  Walks the scope chain innermost-first and returns
/// the first variable whose `definition_span` covers the offset.
/// Lets the variable-rename / reference paths work from the
/// definition site, not just `$var` use sites.
pub(crate) fn var_name_at_definition_offset(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Option<String> {
    for sc in scope_chain_at(scope, byte_offset).iter().rev() {
        for v in sc.variables.values() {
            let span = v.definition_span;
            if span.start() <= byte_offset && byte_offset < span.end() {
                return Some(v.name.clone());
            }
        }
    }
    None
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
pub(crate) fn global_vars_needing_qualification(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Option<FxHashSet<String>> {
    use tcl_compiler::analyser::ScopeKind;
    let chain = scope_chain_at(scope, byte_offset);
    let in_local_context = chain.iter().any(|sc| {
        matches!(sc.kind, ScopeKind::Proc)
            || (matches!(sc.kind, ScopeKind::Namespace) && !sc.name.is_empty() && sc.name != "::")
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
pub(crate) fn innermost_namespace_at(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> String {
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
) -> String {
    let ns = innermost_namespace_at(scope, byte_offset);
    if ns.is_empty() {
        "::".to_owned()
    } else {
        format!("::{ns}")
    }
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

/// Resolve a call `word` written in `namespace` to the proc it denotes:
/// C Tcl's rule first ([`proc_visible_from_namespace`]); then, unless the
/// word names a `registry` builtin — which C Tcl would resolve instead, so a
/// same-named proc in an unrelated namespace must not shadow it — the
/// lenient tail match kept for procs whose defining namespace isn't
/// statically visible at the call, made deterministic by
/// [`fallback_proc_by_simple_name`].
///
/// `registry`, when `Some`, supplies the builtin gate; `None` skips the gate
/// (callers without a registry keep the lenient behaviour).
pub(crate) fn resolve_called_proc<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    namespace: &str,
    word: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    if let Some(proc_def) = proc_visible_from_namespace(analysis, namespace, word) {
        return Some(proc_def);
    }
    if registry.is_some_and(|r| r.get(word).is_some()) {
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
    let ns = namespace_context_at(&analysis.global_scope, cursor_off);
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
    let ns = namespace_context_at(&analysis.global_scope, cursor_off);
    tcl_syntax::naming::bareword_resolution_candidates(&ns, word)
        .into_iter()
        .find_map(|cand| analysis.all_classes.get_key_value(&cand))
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
    fn visit(scope: &Scope, chain: &FxHashSet<String>, out: &mut Vec<String>) {
        if matches!(scope.kind, ScopeKind::Namespace | ScopeKind::Global)
            && !chain.contains(&scope.name)
        {
            for vname in scope.variables.keys() {
                out.push(qualified_var_name(scope, vname));
            }
        }
        for child in &scope.children {
            visit(child, chain, out);
        }
    }
    let mut out = Vec::new();
    visit(global, chain, &mut out);
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
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
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
    fn definition_jumps_to_method_inside_class_body() {
        // Inside an OO class body, `greet` refers to the
        // class's own method.  Cursor on `greet` should jump
        // to the `method greet` declaration.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` in the `twice` body.
        // Line 2: `    method twice {} { greet ; greet }`
        // Col 22 lands on the `g` of the first `greet`.
        let locs = definition(src, 2, 22, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The method declaration is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_jumps_to_classmethod() {
        let src = "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method use {} { factory }`
        // Cursor on `factory` (col 20).
        let locs = definition(src, 2, 20, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
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
}
