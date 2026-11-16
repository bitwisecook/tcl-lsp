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

//! Variable lookup at the cursor: scope extents and chains, declaration offsets, parameter lists and visible names.

use crate::original_context::CurrentSourcePosition;
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;

/// The extent (byte range) of the innermost proc / method body scope that
/// declares or contains variable `name` at `offset` — the binding-identity
/// key for per-object method matching.  Two uses of the same simple name
/// belong to the same binding iff their innermost enclosing proc/method
/// body is the same span (or both sit at the top level, extent `None`).
pub(super) fn variable_scope_extent(
    analysis: &AnalysisResult,
    _name: &str,
    offset: u32,
) -> Option<(u32, u32)> {
    source_frame_extent(analysis, offset)
}

/// Lexical source-frame boundary only; no entered-frame identity is issued.
pub(crate) fn source_frame_extent(analysis: &AnalysisResult, offset: u32) -> Option<(u32, u32)> {
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

/// Query declaration tables eligible under the checked Logical source owner.
pub(crate) fn lookup_var_in_scope_chain<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    byte_offset: u32,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    let config = analysis.body_lexer_config?;
    let advice = analysis.logical_variable_scope_advice(
        &tcl_lexer::SourceImage::document(source),
        config,
        byte_offset,
    )?;
    lookup_var_in_logical_scope(&advice, name)
}

pub(crate) fn lookup_var_in_logical_scope<'a>(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'a>,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    advice.variable_definition(name)
}

/// Current original source occurrences related to the actual declaration.
/// Conditional aliases remain readonly source candidates, with no successful
/// link, physical cell or editable-reference-set claim. Missing provenance is
/// definitive; reporting names and shared declaration spans are not identities.
#[must_use]
pub(crate) fn linked_var_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    var_def: &tcl_compiler::analyser::VarDef,
    cursor: u32,
) -> Option<Vec<tcl_lexer::Span>> {
    let declaration =
        crate::variable_symbol::declaration_at_cursor(source, analysis, var_def, cursor)?;
    Some(
        crate::variable_symbol::alias_template_reference_spans(
            source,
            analysis,
            declaration.symbol(),
            true,
        )
        .into_iter()
        .filter(|span| *span != declaration.declaration_span())
        .collect(),
    )
}

/// Logical namespace-variable coordinate for the cross-document tier.
/// Relative qualified occurrences require the same checked source namespace
/// as the in-document lookup; unknown callers supply no namespace.
/// Bare declarations use their independently retained declaration table.
#[must_use]
pub fn qualified_variable_cell_at(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<String> {
    if !analysis.allows_lexical_declaration_advice()
        || !std::ptr::eq(analysis.resolved_profile()?, dialect)
    {
        return None;
    }
    let cursor_off = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let scope = analysis.logical_variable_scope_advice(
        &tcl_lexer::SourceImage::document(source),
        analysis.body_lexer_config?,
        cursor_off,
    )?;
    if let Some(name) = substituting_var_at_position(source, analysis, line, character, cursor_off)
    {
        let base = name.as_str();
        if !tcl_syntax::naming::is_qualified(base.as_bytes()) {
            return None;
        }
        return scope.namespace_variable_coordinate(base);
    }
    var_def_at_declaration_offset(&analysis.global_scope, cursor_off)
        .and_then(|variable| {
            crate::variable_symbol::declaration_at_cursor(source, analysis, variable, cursor_off)
        })
        .and_then(|declaration| declaration.symbol().checked_c_namespace_key())
        .or_else(|| logical_variable_write_coordinate(source, analysis, cursor_off, &scope))
}

fn logical_variable_write_coordinate(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    scope: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
) -> Option<String> {
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut coordinate = None;
    for advice in &analysis.original_variable_write_advice {
        let span = advice.span();
        if !(span.start() <= cursor
            && cursor < span.end()
            && advice.matches_context(&context)
            && crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                advice.original_name_input(),
                span,
            ))
        {
            continue;
        }
        let input = advice.original_name_input();
        let protocol = input.policy().recipe();
        let selected = match advice.receiver_form().input_form(protocol, input.bytes())? {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let written = std::str::from_utf8(selected.root().selected()).ok()?;
        let current = scope.namespace_variable_coordinate(written)?;
        if coordinate
            .as_ref()
            .is_some_and(|previous| previous != &current)
        {
            return None;
        }
        coordinate = Some(current);
    }
    coordinate
}

/// Whether a retained original namespace-variable producer or conflict covers
/// this cursor, or whether exact current cursor ownership is unavailable.
/// This only withdraws legacy fallback; it issues no symbol.
#[must_use]
pub fn original_variable_cursor_retained(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> bool {
    let Some(position) = CurrentSourcePosition::capture_variable(source, analysis, line, character)
    else {
        return true;
    };
    let offset = position.offset();
    let contains = |span: tcl_lexer::Span| span.start() <= offset && offset < span.end();
    analysis
        .original_variable_symbols
        .iter()
        .any(|occurrence| contains(occurrence.span()))
        || analysis
            .original_variable_symbol_conflicts
            .iter()
            .copied()
            .any(contains)
        || analysis
            .qualified_var_refs
            .iter()
            .any(|reference| reference.original_name_input.is_some() && contains(reference.span))
}

/// Original namespace-variable occurrence at a cursor in the exact retained
/// document and grammar. Byte identity comes from the shared source issuer;
/// optional display keys supply no fallback for missing or stale receipts.
#[must_use]
pub fn original_variable_occurrence_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    line: u32,
    character: u32,
) -> Option<&'a tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
    let offset =
        CurrentSourcePosition::capture_variable(source, analysis, line, character)?.offset();
    analysis.original_variable_symbol_in_source(
        &tcl_lexer::SourceImage::document(source),
        analysis.body_lexer_config?,
        offset,
    )
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
/// the scope that owns it — go-to-definition / hover / find-references would
/// answer nothing for a cursor placed directly on a parameter's or a `catch`
/// result-var's own declaring token, even though every `$name` read of the
/// same variable resolves correctly. Instead this walks every scope in
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

/// A lexical `$name` read at the cursor, independently of the original
/// source syntax label. Scope lookup requires explicit lexical declaration
/// advice; Native providers consume their original variable naming owners.
pub(crate) fn lookup_var_read_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor_off: u32,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let var_def = lookup_var_in_scope_chain(analysis, source, cursor_off, name)?;
    (!offset_is_inert(source, analysis, cursor_off)?).then_some(var_def)
}

/// Original comment/data source classification shared by every variable
/// cursor and lexical caller-reference gate. Unknown ownership stays unknown.
#[must_use]
pub(crate) fn offset_is_inert(
    source: &str,
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<bool> {
    if crate::inert_text::offset_in_physical_comment_in_analysis(source, analysis, cursor_off)? {
        Some(true)
    } else {
        crate::inert_text::offset_in_data_brace_in_analysis(source, analysis, cursor_off)
    }
}

/// Shared lexical variable-cursor entry. The actual analysis, selected
/// grammar, complete original words and comment/data applicability issue the
/// syntax label. Scope resolution remains a separate Logical-only operation;
/// unknown source applicability declines rather than borrowing nominal roles.
#[must_use]
pub(crate) fn substituting_var_at_position(
    source: &str,
    analysis: &AnalysisResult,
    _line: u32,
    _character: u32,
    cursor_off: u32,
) -> Option<String> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let reference =
        crate::source_structure::original_variable_reference_at(source, analysis, cursor_off)?;
    Some(reference.root.to_owned())
}

/// Whether `off` sits inside a **literal** parameter-list word of a proc or
/// method — the word between its name token and its body.
///
/// Every word inside a literal parameter list is pure data: a parameter's own
/// name, or a default value.  Neither is a command reference, so bareword
/// command resolution must not run on it: `proc ::tk::RestoreFocusGrab {grab
/// focus {destroy destroy}}` would otherwise have both the parameter name
/// `destroy` and the default-value literal `destroy` resolve to Tk's `destroy`
/// **command**, where tclsh proves the bare word in a parameter list never
/// invokes anything.  A cursor on the parameter
/// *name* is answered earlier, by [`var_def_at_declaration_offset`]; this
/// guard is what stops the remaining data words falling through.
///
/// **Literal** is the load-bearing word.  `proc`'s parameter-list argument is
/// an ordinary Tcl word, so it may be *computed*, and then it holds live code
/// that must stay navigable.  Verified on tclsh 9.0.4 and 8.6.16:
///
/// ```tcl
/// proc makeargs {} { return {a b} }
/// proc p [makeargs] { return "p got $a $b" }
/// puts [p 1 2]          ;# p got 1 2
/// puts [info args p]    ;# a b
/// set params {x y}
/// proc q $params { … }  ;# works the same way
/// proc r "m n" { … }    ;# and so does a quoted one
/// ```
///
/// `[makeargs]` really is a call, made at definition time, so go-to-definition
/// and hover on it must work.  Only a braced word (`{a {b 1} args}` —
/// substitution suppressed outright) or a plain bareword with no substitution
/// syntax is treated as data; anything containing `$`, `[`, `"`, or a
/// backslash escape stays navigable.
pub(crate) fn offset_is_in_parameter_list(
    analysis: &AnalysisResult,
    source: &str,
    off: u32,
) -> bool {
    parameter_list_position_at(analysis, source, off) == ParamListPosition::LiteralData
}

/// What a cursor offset is, with respect to the parameter-list word of the
/// routine that encloses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamListPosition {
    /// Not inside any routine's parameter-list word.
    Outside,
    /// Inside a **literal** parameter list — pure data, no references.
    LiteralData,
    /// Inside a **computed** one (`proc p [makeargs] …`, `proc q $params …`,
    /// `proc r "m n" …`): live code that must stay navigable, and *not* a
    /// place a parameter declaration can be trusted either — the analyser
    /// records a stub `VarDef` named after the whole word (`"[makeargs]"`),
    /// which would otherwise make go-to-definition point at the cursor's own
    /// token instead of resolving the call.
    Computed,
}

/// Classify `off` against the parameter-list word of whichever proc or method
/// encloses it — see [`ParamListPosition`] and
/// [`offset_is_in_parameter_list`].
pub(crate) fn parameter_list_position_at(
    analysis: &AnalysisResult,
    source: &str,
    off: u32,
) -> ParamListPosition {
    // The parameter-list word divides under the document's own grammar, the
    // same one every other provider in this module reads off the analysis.
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return ParamListPosition::Outside;
    };
    let config = current.config();
    let classify = |name_span: tcl_lexer::Span, body_span: tcl_lexer::Span| {
        // A token content span can omit its real name-word closing delimiter.
        let name = tcl_lexer::word_span_at(source, name_span);
        param_word_position(source, name.end(), body_span.start(), off, config)
    };
    let routines = analysis
        .all_procs
        .values()
        .map(|p| (p.name_span, p.body_span));
    let members = analysis.all_classes.values().flat_map(|c| {
        c.methods
            .values()
            .chain(c.class_methods.values())
            .map(|m| (m.name_span, m.body_span))
    });
    routines
        .chain(members)
        .map(|(name_span, body_span)| classify(name_span, body_span))
        .find(|p| *p != ParamListPosition::Outside)
        .unwrap_or(ParamListPosition::Outside)
}

/// Classify `off` against the parameter-list word sitting in
/// `[region_start, region_end)` — a routine's name-token end to its body
/// start.  See [`offset_is_in_parameter_list`] for the oracle behind the
/// literal test.
fn param_word_position(
    source: &str,
    region_start: u32,
    region_end: u32,
    off: u32,
    config: tcl_lexer::LexerConfig,
) -> ParamListPosition {
    let (start, end, off) = (region_start as usize, region_end as usize, off as usize);
    if start > end || off < start || off >= end || end > source.len() {
        return ParamListPosition::Outside;
    }
    let image = tcl_lexer::SourceImage::document(source);
    // Lex the original tail, not a truncated prefix ending at the body opener.
    // The first grouped word is the candidate; no closer or word is invented.
    let Some(source_end) = u32::try_from(source.len()).ok() else {
        return ParamListPosition::Outside;
    };
    let Ok(tokens) = tcl_lexer::source_region_tokens_in(
        &image,
        tcl_lexer::Span::new(region_start, source_end),
        config,
    ) else {
        return ParamListPosition::Outside;
    };
    let commands = tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config);
    let Some(word) = commands.first().and_then(|command| command.words.first()) else {
        return ParamListPosition::Outside;
    };
    let Ok(span) =
        tcl_lexer::NativeWord::written_span_from_group(image.bytes(), config, &tokens, word)
    else {
        return ParamListPosition::Outside;
    };
    if span.start() < region_start
        || span.end() > region_end
        || off < span.start() as usize
        || off >= span.end() as usize
    {
        return ParamListPosition::Outside;
    }
    let Some(fragments) = tokens.get(word.tokens.clone()) else {
        return ParamListPosition::Outside;
    };
    let literal = !word.expand
        && fragments.len() == 1
        && fragments.first().is_some_and(|token| {
            tcl_compiler::signature_scan::params::param_word_is_literal(token.kind, true)
        });
    if literal {
        ParamListPosition::LiteralData
    } else {
        ParamListPosition::Computed
    }
}

/// Source declaration candidates from the shared Logical scope owner.
pub(crate) fn visible_variable_names(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
) -> Vec<String> {
    let mut names = Vec::new();
    for scope in advice.eligible_scopes().rev() {
        for name in scope.variables.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    if advice.root_names_require_qualification()
        || (advice.is_namespace_frame() && advice.namespace_global_fallback())
    {
        for name in advice.root().variables.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names
}

/// Independently declared root candidates requiring rooted completion insertion.
pub(crate) fn global_vars_needing_qualification(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
) -> Option<FxHashSet<String>> {
    if !advice.root_names_require_qualification() {
        return None;
    }
    let mut globals = advice
        .root()
        .variables
        .keys()
        .cloned()
        .collect::<FxHashSet<_>>();
    globals.extend(
        advice
            .readonly_root_variable_names()
            .into_iter()
            .map(str::to_owned),
    );
    for name in advice.readonly_variable_names() {
        globals.remove(name);
    }
    for scope in advice.eligible_scopes() {
        if std::ptr::eq(scope, advice.root()) {
            continue;
        }
        for name in scope.variables.keys() {
            globals.remove(name);
        }
    }
    Some(globals)
}

/// Test-only display coordinate for isolated scope-walk geometry controls.
/// Production insertion uses current Logical source namespace advice.
#[cfg(test)]
fn qualified_var_name(scope: &tcl_compiler::analyser::Scope, var: &str) -> String {
    use tcl_compiler::analyser::ScopeKind;
    if tcl_syntax::naming::unroot_rooted_key(var).is_some() {
        var.to_string()
    } else if scope.kind == ScopeKind::Global {
        tcl_syntax::naming::root_unrooted_key(var)
    } else {
        // Join constructed Logical display coordinates without reparsing
        // literal colon components as a written name.
        format!("{}::{var}", scope.name)
    }
}

/// Walk the scope tree and return the fully-qualified names of every
/// namespace / global variable whose enclosing namespace is not in the
/// cursor's lexical `chain` (proc locals excluded).
#[cfg(test)]
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

/// Current readonly scan geometry. Document visibility is explicit, rather
/// than inferred from an empty or unavailable collection of body spans.
pub(crate) enum SourceScopeBodySpans {
    Document(tcl_lexer::Span),
    Bodies(Vec<tcl_lexer::Span>),
}

/// Join the actual Logical scope owner to genuine original script interiors.
/// Missing/foreign body geometry withdraws the scan, never widens it to a file.
pub(crate) fn scope_body_spans_at(
    position: &CurrentSourcePosition<'_>,
) -> Option<SourceScopeBodySpans> {
    use tcl_compiler::analyser::ScopeKind;
    let source = position.source();
    let analysis = position.analysis();
    let offset = position.offset();
    let image = tcl_lexer::SourceImage::document(source);
    let advice =
        analysis.logical_variable_scope_advice(&image, position.context().config(), offset)?;
    let root = advice.root();
    if !std::ptr::eq(root, &raw const analysis.global_scope)
        || root.kind != ScopeKind::Global
        || root.body_span.is_some()
    {
        return None;
    }
    let structure = crate::source_structure::SourceStructure::capture(
        source,
        Some(analysis),
        position.context().config(),
    )?;
    let mut pending = root
        .children
        .iter()
        .map(|scope| (scope, 1))
        .collect::<Vec<_>>();
    while let Some((scope, depth)) = pending.pop() {
        if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
            return None;
        }
        let body = scope.body_span?;
        source.get(body.as_range())?;
        if !structure
            .scripts
            .iter()
            .any(|(span, nesting)| *nesting > 0 && *span == body)
        {
            return None;
        }
        pending.extend(scope.children.iter().map(|child| (child, depth + 1)));
    }
    let mut selected = root;
    let mut spans = Vec::new();
    loop {
        let mut children = selected
            .children
            .iter()
            .filter_map(|child| {
                let body = child.body_span?;
                (body.start() <= offset && offset < body.end()).then_some((child, body))
            })
            .collect::<Vec<_>>();
        children.sort_by_key(|(_, body)| body.len());
        let Some((child, body)) = children.first().copied() else {
            break;
        };
        if children
            .get(1)
            .is_some_and(|(_, other)| other.len() == body.len())
        {
            return None;
        }
        spans.push(body);
        selected = child;
    }
    if let Some(body) = structure
        .caller_context_unavailable_bodies
        .iter()
        .filter(|body| body.start() <= offset && offset < body.end())
        .min_by_key(|body| body.len())
        && !spans.contains(body)
    {
        return None;
    }
    if spans.is_empty() {
        Some(SourceScopeBodySpans::Document(tcl_lexer::Span::new(
            0,
            u32::try_from(source.len()).ok()?,
        )))
    } else {
        spans.reverse();
        Some(SourceScopeBodySpans::Bodies(spans))
    }
}
