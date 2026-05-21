//! Find-references / document-highlight provider — Rust port
//! of `lsp/features/references.py`.
//!
//! Locates every usage of the symbol at the cursor:
//!
//! * `$var` references → `VarDef.definition_span` plus every
//!   span in `VarDef.references` (already collected by the
//!   analyser's body walk).
//! * proc references → `ProcDef.name_span` plus every command
//!   invocation in `analysis.command_invocations` whose head
//!   matches the proc's simple or qualified name.
//! * class references → `ClassDef.name_span` plus every command
//!   invocation whose head matches the class's simple or
//!   qualified name.
//!
//! Two entry points:
//!
//! * [`references`] — returns plain `Vec<LspRange>` for the
//!   LSP `textDocument/references` request.
//! * [`document_highlights`] — returns
//!   `Vec<(LspRange, HighlightKind)>` for the LSP
//!   `textDocument/documentHighlight` request.  Variables get
//!   the `Write` / `Read` distinction
//!   (`S-document-highlight-rich`); command-invocation matches
//!   stay `Text` because the analyser's
//!   `command_invocations` doesn't currently surface read /
//!   write semantics on call-head matches.
//!
//! Class-member references also land: when the cursor sits
//! on a method, classmethod, or property name inside the
//! class body, the provider re-segments every sibling method
//! body and surfaces each invocation that names the same
//! member.  `document_highlights` returns the declaration as
//! `Write` and every call site as `Text`.
//!
//! External `$obj method` references also land: when the
//! cursor sits on the method-name token of a `$obj method`
//! call (or inside the class body), the provider additionally
//! scans the whole document for `$v method` / `[$v method]`
//! call sites where `v`'s class (per
//! `analysis.instance_classes`) matches.  See
//! [`find_obj_method_call_sites`] for the scan's coverage.
//!
//! What is *still deferred* (planned as `S-references-rich`
//! follow-ups):
//!
//! * Cross-document references — the workspace-index integration
//!   that surfaces references across every open document; lands
//!   alongside `S-workspace-symbols` and the workspace-index
//!   chunks.
//! * `$obj method` sites embedded in quoted / word tokens
//!   (`"prefix[$d bark]"`) — the scan descends into
//!   command-substitution args and proc / method bodies but
//!   not into string interpolation.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::{find_var_at_position, find_word_span_at_position};

/// Compute the locations of every reference to the symbol at
/// the cursor.
///
/// `include_declaration` mirrors the LSP `ReferenceContext`
/// flag — when `true`, the symbol's defining span is the first
/// element of the returned vector.
#[must_use]
pub fn references(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
) -> Vec<LspRange> {
    let line_index = LineIndex::new(source);

    if let Some(var_name) = find_var_at_position(source, line, character) {
        let byte_offset = crate::definition::byte_offset_at(source, line, character);
        let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            byte_offset,
            &var_name,
        ) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if include_declaration {
            out.push(span_to_range(&line_index, var_def.definition_span));
        }
        for r in &var_def.references {
            out.push(span_to_range(&line_index, *r));
        }
        return out;
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    // Class references (checked first because Python checks
    // class name before proc name in get_references).
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            let mut out = Vec::new();
            if include_declaration {
                out.push(span_to_range(&line_index, class_def.name_span));
            }
            for inv in &analysis.command_invocations {
                if inv.name == class_def.name || inv.name == class_def.qualified_name {
                    out.push(span_to_range(&line_index, inv.range));
                }
            }
            dedup_ranges(&mut out);
            return out;
        }
    }

    // Proc references.
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            let mut out = Vec::new();
            if include_declaration {
                out.push(span_to_range(&line_index, proc_def.name_span));
            }
            let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
            for inv in &analysis.command_invocations {
                if inv.name == proc_def.name
                    || inv.name == proc_def.qualified_name
                    || inv.name == qname_no_prefix
                    || inv
                        .resolved_qualified_name
                        .as_deref()
                        .is_some_and(|r| r == proc_def.qualified_name)
                {
                    out.push(span_to_range(&line_index, inv.range));
                }
            }
            dedup_ranges(&mut out);
            return out;
        }
    }

    // `$obj method` external call site — when the cursor sits
    // on the method-name token of an instance-method call and
    // `$obj`'s class is known, surface the method declaration
    // plus every call site (intra-class + external).
    if let Some((inst, method)) =
        crate::definition::instance_method_at_cursor(source, line, character)
    {
        if let Some(class_q) = analysis.instance_classes.get(&inst) {
            if let Some((decl_span, call_spans)) =
                method_references_for_class(source, analysis, class_q, &method)
            {
                let mut out = Vec::new();
                if include_declaration {
                    out.push(span_to_range(&line_index, decl_span));
                }
                for s in call_spans {
                    out.push(span_to_range(&line_index, s));
                }
                dedup_ranges(&mut out);
                return out;
            }
        }
    }

    // Class-member references — when the cursor sits inside a
    // class body and `word` matches a method / classmethod /
    // property, re-segment the sibling method bodies to find
    // every invocation that names the same member, then append
    // external `$obj method` call sites.  Mirrors the
    // `rename_method` walk in `crate::rename`.
    let cursor_offset = crate::definition::byte_offset_at(source, line, character);
    if let Some(spans) = find_class_member_references(source, &word, analysis, cursor_offset) {
        let (decl_span, call_spans) = spans;
        let mut out = Vec::new();
        if include_declaration {
            out.push(span_to_range(&line_index, decl_span));
        }
        for s in call_spans {
            out.push(span_to_range(&line_index, s));
        }
        dedup_ranges(&mut out);
        return out;
    }

    Vec::new()
}

/// Resolve a method's declaration span plus every call site —
/// intra-class (re-segment the class's own method bodies) and
/// external (`$obj method` across the document).  Returns
/// `None` when `class_q` has no method / classmethod named
/// `method`.
pub(crate) fn method_references_for_class(
    source: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    use tcl_compiler::segmenter::segment_commands_with_offset;
    use tcl_lexer::Span;
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = class_def
        .methods
        .get(method)
        .map(|m| m.name_span)
        .or_else(|| class_def.class_methods.get(method).map(|m| m.name_span))?;

    let mut call_spans: Vec<Span> = Vec::new();
    // Intra-class: re-segment every method / classmethod /
    // ctor / dtor body for bare `method` invocations.
    let mut bodies: Vec<Span> = class_def
        .methods
        .values()
        .map(|m| m.body_span)
        .chain(class_def.class_methods.values().map(|m| m.body_span))
        .chain(class_def.constructors.iter().map(|c| c.body_span))
        .collect();
    if let Some(d) = &class_def.destructor {
        bodies.push(d.body_span);
    }
    for body_span in bodies {
        if body_span.is_empty() {
            continue;
        }
        let mut start = body_span.start() as usize;
        let mut end = body_span.end() as usize;
        if start >= source.len() || end > source.len() || start > end {
            continue;
        }
        if source.as_bytes().get(start) == Some(&b'{') {
            start += 1;
        }
        if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
            end -= 1;
        }
        let body_text = &source[start..end];
        let commands = segment_commands_with_offset(body_text, u32::try_from(start).unwrap_or(0));
        for cmd in &commands {
            let Some(head) = cmd.argv.first() else {
                continue;
            };
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            if h_start >= source.len() || h_end > source.len() {
                continue;
            }
            if &source[h_start..h_end] == method && head.span != decl_span {
                call_spans.push(head.span);
            }
        }
    }
    // External `$obj method` sites.
    call_spans.extend(find_obj_method_call_sites(
        source, analysis, class_q, method,
    ));
    Some((decl_span, call_spans))
}

/// Find a class member's declaration span plus every call
/// site inside any sibling method body.  Returns
/// `Some((decl_span, call_spans))` when the cursor sits
/// inside a class body and `word` matches one of that
/// class's members.
fn find_class_member_references(
    source: &str,
    word: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    use tcl_compiler::segmenter::segment_commands_with_offset;
    use tcl_lexer::Span;

    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        let name_span: Option<Span> = class_def
            .methods
            .get(word)
            .map(|m| m.name_span)
            .or_else(|| class_def.class_methods.get(word).map(|m| m.name_span))
            .or_else(|| class_def.properties.get(word).map(|p| p.name_span));
        let decl_span = name_span?;
        // Collect call-site spans by re-segmenting every
        // method body (the analyser doesn't walk into method
        // bodies for the `command_invocations` collection).
        let mut bodies: Vec<Span> = class_def
            .methods
            .values()
            .map(|m| m.body_span)
            .chain(class_def.class_methods.values().map(|m| m.body_span))
            .chain(class_def.constructors.iter().map(|c| c.body_span))
            .collect();
        if let Some(d) = &class_def.destructor {
            bodies.push(d.body_span);
        }
        let mut call_spans: Vec<Span> = Vec::new();
        for body_span in bodies {
            if body_span.is_empty() {
                continue;
            }
            let mut start = body_span.start() as usize;
            let mut end = body_span.end() as usize;
            if start >= source.len() || end > source.len() || start > end {
                continue;
            }
            if source.as_bytes().get(start) == Some(&b'{') {
                start += 1;
            }
            if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
                end -= 1;
            }
            let body_text = &source[start..end];
            let commands = segment_commands_with_offset(
                body_text,
                u32::try_from(start).unwrap_or(body_span.start()),
            );
            for cmd in &commands {
                let Some(head) = cmd.argv.first() else {
                    continue;
                };
                let h_start = head.span.start() as usize;
                let h_end = head.span.end() as usize;
                if h_start >= source.len() || h_end > source.len() {
                    continue;
                }
                if &source[h_start..h_end] != word {
                    continue;
                }
                // Skip the declaration site itself (cannot
                // happen — declaration sits outside method
                // bodies — but defensive).
                if head.span.start() == decl_span.start() && head.span.end() == decl_span.end() {
                    continue;
                }
                call_spans.push(head.span);
            }
        }
        // Append external `$obj method` call sites for
        // methods / classmethods (not properties — those
        // aren't dispatched as `$obj prop`).
        if class_def.methods.contains_key(word) || class_def.class_methods.contains_key(word) {
            call_spans.extend(find_obj_method_call_sites(
                source,
                analysis,
                &class_def.qualified_name,
                word,
            ));
        }
        return Some((decl_span, call_spans));
    }
    None
}

/// Find every external `$v method` / `[$v method]` call site
/// in the document where `v` is an instance variable whose
/// class qualified-name is `class_q` (per
/// `analysis.instance_classes`).  Returns the spans of the
/// method-name tokens.
///
/// Scans three region kinds — the top-level command stream,
/// each user proc body, and each class method body — and
/// recurses into command-substitution (`[...]`) args at every
/// level.  This covers the common call forms (`$d bark`,
/// `puts [$d bark]`, calls inside procs / methods).  Method
/// names embedded in quoted / word tokens
/// (`"prefix[$d bark]"`) are not descended — a rare form.
pub(crate) fn find_obj_method_call_sites(
    source: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    use std::collections::HashSet;
    // Variables of the target class.
    let var_set: HashSet<&str> = analysis
        .instance_classes
        .iter()
        .filter(|(_, c)| c.as_str() == class_q)
        .map(|(v, _)| v.as_str())
        .collect();
    if var_set.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    let mut seen: HashSet<(u32, u32)> = HashSet::new();

    // Region 1: the whole document.
    scan_obj_method_region(
        source,
        0,
        source.len(),
        &var_set,
        method,
        &mut out,
        &mut seen,
    );
    // Regions 2/3: proc + method bodies (the top-level scan
    // skips braced body args, so descend explicitly).
    for proc_def in analysis.all_procs.values() {
        scan_obj_method_body(
            source,
            proc_def.body_span,
            &var_set,
            method,
            &mut out,
            &mut seen,
        );
    }
    for class_def in analysis.all_classes.values() {
        for m in class_def
            .methods
            .values()
            .chain(class_def.class_methods.values())
            .chain(class_def.constructors.iter())
            .chain(class_def.destructor.iter())
        {
            scan_obj_method_body(source, m.body_span, &var_set, method, &mut out, &mut seen);
        }
    }
    out
}

/// Scan a brace-delimited body span for `$v method` call sites
/// (stripping the surrounding braces first).
fn scan_obj_method_body(
    source: &str,
    body_span: tcl_lexer::Span,
    var_set: &std::collections::HashSet<&str>,
    method: &str,
    out: &mut Vec<tcl_lexer::Span>,
    seen: &mut std::collections::HashSet<(u32, u32)>,
) {
    if body_span.is_empty() {
        return;
    }
    let mut start = body_span.start() as usize;
    let mut end = body_span.end() as usize;
    if start >= source.len() || end > source.len() || start > end {
        return;
    }
    if source.as_bytes().get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    scan_obj_method_region(source, start, end, var_set, method, out, seen);
}

/// Segment `source[start..end]` and record every `$v method`
/// call site, recursing into command-substitution (`[...]`)
/// args.  `var_set` holds the bare names of in-scope instance
/// variables.
fn scan_obj_method_region(
    source: &str,
    start: usize,
    end: usize,
    var_set: &std::collections::HashSet<&str>,
    method: &str,
    out: &mut Vec<tcl_lexer::Span>,
    seen: &mut std::collections::HashSet<(u32, u32)>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset;
    use tcl_lexer::TokenType;
    if start >= end || end > source.len() {
        return;
    }
    let region = &source[start..end];
    let commands = segment_commands_with_offset(region, u32::try_from(start).unwrap_or(0));
    for cmd in &commands {
        // Head `$v` + method at argv[1].
        if let (Some(head), Some(method_tok)) = (cmd.argv.first(), cmd.argv.get(1)) {
            if head.kind == TokenType::Var {
                let h_start = head.span.start() as usize;
                let h_end = head.span.end() as usize;
                if h_start < source.len() && h_end <= source.len() {
                    let raw = &source[h_start..h_end];
                    if let Some(name) = strip_var_decoration(raw) {
                        if var_set.contains(name) {
                            let m_start = method_tok.span.start() as usize;
                            let m_end = method_tok.span.end() as usize;
                            if m_start < source.len()
                                && m_end <= source.len()
                                && &source[m_start..m_end] == method
                            {
                                let key = (method_tok.span.start(), method_tok.span.end());
                                if seen.insert(key) {
                                    out.push(method_tok.span);
                                }
                            }
                        }
                    }
                }
            }
        }
        // Recurse into command-substitution args.
        for arg in &cmd.argv {
            if arg.kind != TokenType::Cmd {
                continue;
            }
            let a_start = arg.span.start() as usize;
            let a_end = arg.span.end() as usize;
            if a_start >= source.len() || a_end > source.len() || a_start >= a_end {
                continue;
            }
            // Strip the surrounding `[` `]`.
            let inner_start = if source.as_bytes().get(a_start) == Some(&b'[') {
                a_start + 1
            } else {
                a_start
            };
            let inner_end =
                if a_end > inner_start && source.as_bytes().get(a_end - 1) == Some(&b']') {
                    a_end - 1
                } else {
                    a_end
                };
            scan_obj_method_region(source, inner_start, inner_end, var_set, method, out, seen);
        }
    }
}

/// Strip a `$name` / `${name}` decoration to the bare variable
/// name.  Returns `None` when the text isn't a `$`-prefixed
/// reference.
fn strip_var_decoration(raw: &str) -> Option<&str> {
    let rest = raw.strip_prefix('$')?;
    let inner = rest
        .strip_prefix('{')
        .map_or(rest, |r| r.strip_suffix('}').unwrap_or(r));
    if inner.is_empty() {
        None
    } else {
        Some(inner)
    }
}

/// Read / write kind for a document-highlight span.  Mirrors
/// `lsprotocol.types.DocumentHighlightKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    /// The cursor's symbol appears here as a read (`$var`,
    /// command-invocation head, etc.).
    Read,
    /// The cursor's symbol is being assigned / defined here
    /// (a `set` / `variable` / `upvar` write site, a proc
    /// declaration's name span, etc.).
    Write,
    /// The match has no read / write distinction — used for
    /// command-invocation heads whose call semantics aren't
    /// surfaced as read/write by the analyser.
    Text,
}

/// Compute the document-highlight spans for the symbol at the
/// cursor with read / write kinds.
///
/// Variables: `VarDef.definition_span` becomes `Write`; every
/// span in `VarDef.references` becomes `Read`.  Procs and
/// classes: the name span is `Write`; every matching command-
/// invocation head is `Text` (the analyser doesn't currently
/// distinguish read vs write semantics on command-invocation
/// heads, so we conservatively emit `Text`).
#[must_use]
pub fn document_highlights(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<(LspRange, HighlightKind)> {
    let line_index = LineIndex::new(source);

    if let Some(var_name) = find_var_at_position(source, line, character) {
        let byte_offset = crate::definition::byte_offset_at(source, line, character);
        let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            byte_offset,
            &var_name,
        ) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(1 + var_def.references.len());
        out.push((
            span_to_range(&line_index, var_def.definition_span),
            HighlightKind::Write,
        ));
        for r in &var_def.references {
            out.push((span_to_range(&line_index, *r), HighlightKind::Read));
        }
        return dedup_kinded(out);
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            let mut out = Vec::new();
            out.push((
                span_to_range(&line_index, class_def.name_span),
                HighlightKind::Write,
            ));
            for inv in &analysis.command_invocations {
                if inv.name == class_def.name || inv.name == class_def.qualified_name {
                    out.push((span_to_range(&line_index, inv.range), HighlightKind::Text));
                }
            }
            return dedup_kinded(out);
        }
    }

    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            let mut out = Vec::new();
            out.push((
                span_to_range(&line_index, proc_def.name_span),
                HighlightKind::Write,
            ));
            let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
            for inv in &analysis.command_invocations {
                if inv.name == proc_def.name
                    || inv.name == proc_def.qualified_name
                    || inv.name == qname_no_prefix
                    || inv
                        .resolved_qualified_name
                        .as_deref()
                        .is_some_and(|r| r == proc_def.qualified_name)
                {
                    out.push((span_to_range(&line_index, inv.range), HighlightKind::Text));
                }
            }
            return dedup_kinded(out);
        }
    }

    // Class-member highlights — re-segment sibling method
    // bodies via `find_class_member_references` and mark the
    // declaration as Write, every call site as Text.
    let cursor_offset = crate::definition::byte_offset_at(source, line, character);
    if let Some((decl_span, call_spans)) =
        find_class_member_references(source, &word, analysis, cursor_offset)
    {
        let mut out = Vec::new();
        out.push((span_to_range(&line_index, decl_span), HighlightKind::Write));
        for s in call_spans {
            out.push((span_to_range(&line_index, s), HighlightKind::Text));
        }
        return dedup_kinded(out);
    }

    Vec::new()
}

/// Deduplicate kinded highlight spans by (start, end) — keeps
/// the highest-kind for each duplicate range.  Write outranks
/// Read which outranks Text, so a span that the analyser
/// records both as a write and as a Read keeps the Write
/// label.
fn dedup_kinded(mut entries: Vec<(LspRange, HighlightKind)>) -> Vec<(LspRange, HighlightKind)> {
    use std::collections::HashMap;
    let mut by_key: HashMap<(u32, u32, u32, u32), HighlightKind> = HashMap::new();
    for (range, kind) in &entries {
        let key = (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        );
        let kind = *kind;
        by_key
            .entry(key)
            .and_modify(|existing| {
                if priority(kind) > priority(*existing) {
                    *existing = kind;
                }
            })
            .or_insert(kind);
    }
    let mut seen: std::collections::HashSet<(u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    entries.retain_mut(|(range, kind)| {
        let key = (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        );
        if !seen.insert(key) {
            return false;
        }
        *kind = by_key[&key];
        true
    });
    entries
}

fn priority(kind: HighlightKind) -> u8 {
    match kind {
        HighlightKind::Write => 2,
        HighlightKind::Read => 1,
        HighlightKind::Text => 0,
    }
}

fn span_to_range(line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LspRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
    }
}

fn dedup_ranges(ranges: &mut Vec<LspRange>) {
    let mut seen: std::collections::HashSet<(u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    ranges.retain(|r| {
        let key = (r.start_line, r.start_character, r.end_line, r.end_character);
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
    fn references_to_proc_include_decl_and_calls() {
        let src = "proc greet {} {}\ngreet\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` reference (line 1).
        let refs = references(src, 1, 2, &analysis, true);
        assert!(refs.len() >= 2, "expected decl + call sites: {refs:?}");
        // First entry is the declaration on line 0.
        assert_eq!(refs[0].start_line, 0);
    }

    #[test]
    fn references_exclude_decl_when_flag_false() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let with_decl = references(src, 1, 2, &analysis, true);
        let without_decl = references(src, 1, 2, &analysis, false);
        assert!(with_decl.len() > without_decl.len());
    }

    #[test]
    fn references_to_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(references(src, 0, 6, &analysis, true).is_empty());
    }

    #[test]
    fn references_to_var_includes_definition_and_uses() {
        let src = "set x 1\nputs $x\nputs $x\n";
        let analysis = analyse(src);
        // Cursor on `$x` first reference.
        let refs = references(src, 1, 7, &analysis, true);
        // The analyser may or may not record the literal `$x`
        // as a reference depending on lowering; at minimum the
        // declaration should land in the result list.
        assert!(!refs.is_empty(), "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 0));
    }

    // -- S-document-highlight-rich: read/write distinction -----------

    #[test]
    fn document_highlights_var_records_write_at_definition() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor inside `$x`.
        let highlights = document_highlights(src, 1, 7, &analysis);
        // The defining `set x` span should be tagged Write.
        let writes: Vec<_> = highlights
            .iter()
            .filter(|(_, k)| *k == HighlightKind::Write)
            .collect();
        assert!(
            !writes.is_empty(),
            "expected at least one Write for `set x 1`; got {highlights:?}",
        );
        // The Write should be on line 0 (the `set` line).
        assert!(
            writes.iter().any(|(r, _)| r.start_line == 0),
            "expected Write on line 0; got {highlights:?}",
        );
    }

    #[test]
    fn document_highlights_var_read_kind_is_correctly_tagged() {
        // The kind-tagging contract: every span in
        // `VarDef.references` becomes Read; the definition
        // span becomes Write.  Whether the analyser actually
        // populates `references` for a given source depends
        // on its body-walk heuristics (single-arg `set x`
        // reads are tracked, `$x` substitutions in arg
        // positions are not in the current Rust port).  This
        // test injects a synthetic `VarDef` with a known
        // `references` entry to verify the tagging logic in
        // isolation from the body-walk gap.
        use tcl_compiler::analyser::{AnalysisResult as Result, Scope, VarDef};
        use tcl_lexer::Span;
        let mut scope = Scope::default();
        scope.variables.insert(
            "x".into(),
            VarDef {
                name: "x".into(),
                definition_span: Span::new(4, 5),
                references: vec![Span::new(13, 14)],
                warn_if_unused: false,
            },
        );
        let a = Result {
            global_scope: scope,
            ..Result::default()
        };
        // Source matches the spans we injected so
        // line/character translation works.
        let src = "set x 1\nputs $x\n";
        let highlights = document_highlights(src, 1, 6, &a);
        // Write at definition.
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
            "expected Write at line 0; got {highlights:?}",
        );
        // Read at the injected reference.
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Read),
            "expected Read at line 1; got {highlights:?}",
        );
    }

    #[test]
    fn document_highlights_proc_decl_is_write() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let highlights = document_highlights(src, 0, 6, &analysis);
        // Declaration on line 0 should be Write.
        let line0_write = highlights
            .iter()
            .find(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write);
        assert!(
            line0_write.is_some(),
            "expected Write on line 0 (declaration); got {highlights:?}",
        );
        // Call site on line 1 should be Text (no read/write
        // semantics on command-invocation heads).
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Text),
            "expected Text on line 1 (call site); got {highlights:?}",
        );
    }

    #[test]
    fn document_highlights_empty_for_unknown_symbol() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(document_highlights(src, 0, 6, &analysis).is_empty());
    }

    // -- S-references-rich: resolved-qualified-name matching ---------

    #[test]
    fn resolved_qualified_name_matches_call_site_from_namespace() {
        // Source: a proc defined at the top level, called from
        // a namespace.  The call site's literal name (`greet`)
        // matches the proc name; the resolved qualified name
        // also matches.  We pin that the references provider
        // finds the call site.
        let src = "proc ::greet {} {}\nnamespace eval ::myns {\n    greet\n}\n";
        let analysis = analyse(src);
        // Cursor on the proc declaration.
        let refs = references(src, 0, 8, &analysis, true);
        // Should include the declaration and the call site.
        assert!(
            refs.len() >= 2,
            "expected proc decl + namespace call site; got {refs:?}",
        );
    }

    #[test]
    fn document_highlights_surfaces_var_reads_from_arg_positions() {
        // After the `record_arg_var_reads` follow-up, `$x`
        // reads in command arguments populate
        // `VarDef.references` and surface as `Read` spans in
        // the document-highlight provider.
        let src = "set x 1\nputs $x\nputs $x\n";
        let analysis = analyse(src);
        let highlights = document_highlights(src, 1, 6, &analysis);
        let reads: Vec<_> = highlights
            .iter()
            .filter(|(_, k)| *k == HighlightKind::Read)
            .collect();
        assert!(
            reads.len() >= 2,
            "expected >= 2 Read entries (for two `$x` sites); got {highlights:?}",
        );
        // The defining `set x` span is Write.
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
            "expected Write on line 0; got {highlights:?}",
        );
    }

    #[test]
    fn resolved_qualified_name_field_populated_for_simple_call() {
        // Verify that the analyser actually populates
        // `resolved_qualified_name` on
        // `command_invocations`.  At the top level a `greet`
        // call should resolve to `::greet`.
        let src = "greet hi\n";
        let analysis = analyse(src);
        let inv = analysis
            .command_invocations
            .iter()
            .find(|i| i.name == "greet")
            .expect("expected a `greet` invocation");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::greet"),
            "expected resolved name to be `::greet`; got {inv:?}",
        );
    }

    // -- S-references-rich: class-member references -----------------

    #[test]
    fn references_for_method_includes_decl_and_call_sites() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 11).
        let refs = references(src, 1, 11, &analysis, true);
        assert!(refs.len() >= 3, "expected ≥3 refs; got {refs:?}");
    }

    #[test]
    fn references_for_method_excludes_decl_when_requested() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        let refs = references(src, 1, 11, &analysis, false);
        // Only the two call sites — the declaration is
        // excluded when include_declaration=false.
        assert_eq!(refs.len(), 2, "{refs:?}");
    }

    #[test]
    fn document_highlights_for_method_marks_decl_write_calls_text() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        let h = document_highlights(src, 1, 11, &analysis);
        let writes: Vec<_> = h
            .iter()
            .filter(|(_, k)| *k == HighlightKind::Write)
            .collect();
        let texts: Vec<_> = h
            .iter()
            .filter(|(_, k)| *k == HighlightKind::Text)
            .collect();
        assert_eq!(writes.len(), 1, "{h:?}");
        assert_eq!(texts.len(), 2, "{h:?}");
    }

    // -- S-references-rich: external $obj method sites --------------

    #[test]
    fn references_from_external_obj_method_site() {
        // Declaration + 2 external call sites (`$d bark`,
        // `[$d bark]`).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        // Cursor on `bark` in `$d bark` (line 4, col 3).
        let refs = references(src, 4, 3, &analysis, true);
        // Declaration (line 1) + two external sites (lines 4, 5).
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "line-4 call missing: {refs:?}");
        assert!(lines.contains(&5), "line-5 call missing: {refs:?}");
    }

    #[test]
    fn references_from_inside_class_includes_external_sites() {
        // Cursor on the declaration; refs include the external
        // `$d bark` site as well as the declaration.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        let refs = references(src, 1, 11, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "external call missing: {refs:?}");
    }

    #[test]
    fn find_obj_method_call_sites_covers_top_level_and_subst() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, &analysis, "::Dog", "bark");
        // Two external sites: `$d bark` and `[$d bark]`.
        assert_eq!(sites.len(), 2, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_finds_calls_in_proc_body() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { $d bark }\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, &analysis, "::Dog", "bark");
        assert_eq!(sites.len(), 1, "{sites:?}");
    }
}
