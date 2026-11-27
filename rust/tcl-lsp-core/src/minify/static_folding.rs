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

//! The aggressive tier's folding of quoted strings whose value is statically proven.

#[cfg(test)]
use super::Statement;
#[cfg(test)]
use super::TaintLattice;
#[cfg(test)]
use super::Version;
#[cfg(test)]
use super::command_rendering::utf8_len;
use super::{SourceEdit, apply_edits, original};
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;
use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::taint::TaintColour;
use tcl_lexer::LexerConfig;
#[cfg(test)]
use tcl_lexer::Span;
#[cfg(test)]
use tcl_lexer::close_quote_offset_in;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::NumericValue;

// Static-substring folding: the aggressive tier's pass between the
// optimiser rewrites and name compaction.

/// Taint colours that prove a fixed-form value, so folding stays
/// safe even when the underlying value is tainted.
fn safe_taint_colours() -> TaintColour {
    TaintColour::IP_ADDRESS
        | TaintColour::PORT
        | TaintColour::FQDN
        | TaintColour::LIST_CANONICAL
        | TaintColour::REGEX_LITERAL
}

/// Fold only original quoted words whose exact reads have independent erasure proof.
pub(super) fn fold_original_static_substrings(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> (String, usize, BTreeMap<String, String>) {
    let analysis = original::analysis_with_input(source, input);
    let context = input.context_registry();
    let registry = context.commands();
    let Some(unit) =
        crate::original_invocation::source_compilation_unit(source, &analysis, registry)
    else {
        return (source.to_owned(), 0, BTreeMap::new());
    };
    if unit.ir_module.source_metadata_input.as_ref() != Some(input) {
        return (source.to_owned(), 0, BTreeMap::new());
    }
    let mut edits = BTreeMap::new();
    let mut folds = BTreeMap::new();
    for function in unit.all_body_function_units() {
        if function
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .is_none()
        {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            if !function.sccp.executable_blocks.contains(&block) {
                continue;
            }
            for index in 0..body.statements.len() {
                let view =
                    tcl_compiler::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let Some(original) = tcl_compiler::ssa::original_analysis_words_for_statement(
                    &unit, function, block, index, registry,
                ) else {
                    continue;
                };
                let config = original.config();
                for word in tokens.words().iter().skip(1) {
                    let tcl_source::words::WordExpr::Template {
                        parts,
                        rejected: None,
                        ..
                    } = word
                    else {
                        continue;
                    };
                    let span = function.abs_span(word.source().span);
                    if !original
                        .words()
                        .iter()
                        .any(|source_word| source_word.span() == span)
                    {
                        continue;
                    }
                    let Some(inner_text) = source
                        .get(span.as_range())
                        .and_then(|source_text| source_text.strip_prefix('"'))
                        .and_then(|source_text| source_text.strip_suffix('"'))
                    else {
                        continue;
                    };
                    let Some(value) =
                        fold_original_word_parts(&unit, function, view, parts, registry, config)
                    else {
                        continue;
                    };
                    let Some(replacement) =
                        tcl_syntax::word_rules::lexical_unicode_source_word(&value, config)
                    else {
                        continue;
                    };
                    edits.insert((span.start(), span.end()), replacement);
                    folds.insert(inner_text.to_owned(), value);
                }
            }
        }
    }
    let count = edits.len();
    let edits: Vec<_> = edits
        .into_iter()
        .map(|((start, end), text)| SourceEdit {
            span: start as usize..end as usize,
            text,
        })
        .collect();
    apply_edits(source, &edits).map_or_else(
        |_| (source.to_owned(), 0, BTreeMap::new()),
        |result| (result, count, folds),
    )
}

fn fold_original_word_parts(
    unit: &CompilationUnit,
    function: &FunctionUnit,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    parts: &[tcl_source::words::WordPart],
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<String> {
    let mut value = String::new();
    let mut substituted = false;
    for part in parts {
        match part {
            tcl_source::words::WordPart::Text { text, .. } => {
                value.push_str(&tcl_lexer::backslash_subst_in(text, config.escapes));
            }
            tcl_source::words::WordPart::Variable { spelling, source } => {
                let physical = view.replaceable_read_at(source, spelling, registry)?;
                let version = physical.version?;
                let original = tcl_compiler::ssa::original_source_read_at_unit(
                    unit,
                    function.abs_span(source.span),
                    registry,
                )?;
                if !std::ptr::eq(original.function(), function)
                    || original.symbol() != physical.symbol
                    || original.version() != Some(version)
                {
                    return None;
                }
                let key = (physical.symbol, version);
                if function.taints.get(&key).is_some_and(|taint| {
                    taint.is_tainted() && !taint.colours.intersects(safe_taint_colours())
                }) {
                    return None;
                }
                let LatticeValue::Const(constant) = function.sccp.values.get(&key)? else {
                    return None;
                };
                value.push_str(&const_to_string(constant)?);
                substituted = true;
            }
            tcl_source::words::WordPart::CommandSubstitution { .. }
            | tcl_source::words::WordPart::Opaque { .. } => return None,
        }
    }
    substituted.then_some(value)
}

/// Replace dynamic quoted strings with their static values where the
/// compiler's SCCP pass proves every `$var` substitution is a
/// compile-time constant and no unsanitised tainted value is
/// involved.  Folds `$var` interpolations resolving to integer /
/// string constants; bails on command substitutions (`[…]`) and on
/// boolean / float constants.  Returns `(folded_source, fold_count,
/// fold_map)`.
#[cfg(test)]
pub(super) fn fold_static_substrings(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> (String, usize, BTreeMap<String, String>) {
    let cu = CompilationUnit::build_for_profile(source, registry, false, dialect);
    let mut edits: Vec<SourceEdit> = Vec::new();
    let mut fold_map: BTreeMap<String, String> = BTreeMap::new();

    let mut scopes: Vec<&FunctionUnit> = vec![&cu.top_level];
    scopes.extend(cu.procedures.values());
    for fu in scopes {
        let config = LexerConfig::for_profile(Some(dialect));
        let cx = FoldContext {
            fu,
            style: config.braced_var,
            config,
            view: None,
        };
        collect_folds_for_scope(source, cx, &mut edits, &mut fold_map);
    }
    if edits.is_empty() {
        return (source.to_owned(), 0, fold_map);
    }
    // Deduplicate by (offset, length).
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut unique: Vec<SourceEdit> = Vec::new();
    for e in edits {
        if seen.insert((e.span.start, e.span.end)) {
            unique.push(e);
        }
    }
    let fold_count = unique.len();
    apply_edits(source, &unique).map_or_else(
        |_| (source.to_owned(), 0, BTreeMap::new()),
        |result| (result, fold_count, fold_map),
    )
}

/// The read-only context a constant-fold decision needs: the scope's
/// analysed unit and the document's `${…}` close rule (which decides where
/// a `${…}` reference's name ends).
#[cfg(test)]
#[derive(Clone, Copy)]
struct FoldContext<'a> {
    fu: &'a FunctionUnit,
    style: tcl_dialect::BracedVarStyle,
    pub(super) config: LexerConfig,
    view: Option<tcl_compiler::ssa::SsaSourceView<'a>>,
}

/// Collect static-fold edits for one function scope.
#[cfg(test)]
fn collect_folds_for_scope(
    source: &str,
    cx: FoldContext<'_>,
    edits: &mut Vec<SourceEdit>,
    fold_map: &mut BTreeMap<String, String>,
) {
    let (fu, style) = (cx.fu, cx.style);
    for (block_id, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_id) else {
            continue;
        };
        if !fu.sccp.executable_blocks.contains(block_id) {
            continue;
        }
        let mut block_vars = ssa_block.entry_versions.clone();
        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            let Some(ssa_stmt) = ssa_block.statements.get(stmt_idx) else {
                continue;
            };
            let mut uses = block_vars.clone();
            uses.extend(
                ssa_stmt
                    .uses
                    .iter()
                    .map(|(&symbol, &version)| (symbol, version)),
            );
            let view = tcl_compiler::ssa::SsaSourceView::at_statement(&fu.ssa, *block_id, stmt_idx);
            let cx = FoldContext {
                view: Some(view),
                ..cx
            };
            match stmt {
                Statement::AssignValue {
                    span,
                    value,
                    value_needs_backsubst: true,
                    ..
                } if value.contains('$') || value.contains('[') => {
                    try_fold_region(source, *span, value, &uses, cx, edits, fold_map);
                }
                Statement::Call {
                    tokens: Some(toks), ..
                } => {
                    for i in 1..toks.argv.len() {
                        let arg_off = toks.argv[i].start() as usize;
                        if source.as_bytes().get(arg_off) != Some(&b'"') {
                            continue;
                        }
                        let content = &toks.argv_texts[i];
                        if !(content.contains('$') || content.contains('[')) {
                            continue;
                        }
                        if let Some(folded) =
                            fold_string_via_sccp(content, &uses, &fu.sccp.values, view, style)
                        {
                            if has_unsafe_tainted_inputs(content, &uses, &fu.taints, view, style) {
                                continue;
                            }
                            if let Some(close) = close_quote_offset_in(source, arg_off, cx.config)
                                && let Some(replacement) =
                                    tcl_syntax::word_rules::lexical_unicode_source_word(
                                        &folded, cx.config,
                                    )
                            {
                                edits.push(SourceEdit {
                                    span: arg_off..close + 1,
                                    text: replacement,
                                });
                                fold_map.insert(content.clone(), folded);
                            }
                        }
                    }
                }
                _ => {}
            }
            for (name, ver) in &ssa_stmt.defs {
                block_vars.insert(*name, *ver);
            }
        }
    }
}

/// Fold the quoted string inside the `span` region of an
/// `AssignValue` (`set x "…"`).
#[cfg(test)]
fn try_fold_region(
    source: &str,
    span: Span,
    value: &str,
    uses: &HashMap<tcl_compiler::ssa::Symbol, Version>,
    cx: FoldContext<'_>,
    edits: &mut Vec<SourceEdit>,
    fold_map: &mut BTreeMap<String, String>,
) {
    let (fu, style) = (cx.fu, cx.style);
    let Some(view) = cx.view else {
        return;
    };
    let Some(folded) = fold_string_via_sccp(value, uses, &fu.sccp.values, view, style) else {
        return;
    };
    if has_unsafe_tainted_inputs(value, uses, &fu.taints, view, style) {
        return;
    }
    let (start, end) = (span.start() as usize, span.end() as usize);
    let region = &source[start..end.min(source.len())];
    let Some(q) = region.find('"') else {
        return;
    };
    let abs_start = start + q;
    let Some(close) = close_quote_offset_in(source, abs_start, cx.config) else {
        return;
    };
    let inner = &source[abs_start + 1..close];
    if !(inner.contains('$') || inner.contains('[')) {
        return;
    }
    let Some(replacement) = tcl_syntax::word_rules::lexical_unicode_source_word(&folded, cx.config)
    else {
        return;
    };
    edits.push(SourceEdit {
        span: abs_start..close + 1,
        text: replacement,
    });
    fold_map.insert(inner.to_owned(), folded);
}

/// Resolve a quoted-string body to a static value via SCCP, or
/// `None` when any substitution is non-constant.  `$var` only;
/// command substitutions (`[…]`) bail out.
#[cfg(test)]
fn fold_string_via_sccp(
    content: &str,
    uses: &HashMap<tcl_compiler::ssa::Symbol, Version>,
    values: &HashMap<(tcl_compiler::ssa::Symbol, Version), LatticeValue>,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    style: tcl_dialect::BracedVarStyle,
) -> Option<String> {
    let bytes = content.as_bytes();
    let n = bytes.len();
    let mut out = String::new();
    let mut has_dynamic = false;
    let mut pos = 0;
    while pos < n {
        match bytes[pos] {
            b'$' => {
                let (end, name) = parse_var_ref(content, pos, style);
                let name = name?;
                let symbol = view.symbol(name)?;
                let ver = uses.get(&symbol).copied().unwrap_or(0);
                if ver == 0 {
                    return None;
                }
                let lv = values.get(&(symbol, ver))?;
                let LatticeValue::Const(cv) = lv else {
                    return None;
                };
                out.push_str(&const_to_string(cv)?);
                has_dynamic = true;
                pos = end;
            }
            b'[' => return None, // command substitution — not handled.
            b'\\' if pos + 1 < n => {
                match bytes[pos + 1] {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    _ => out.push_str(&content[pos + 1..pos + 1 + utf8_len(bytes[pos + 1])]),
                }
                pos += 1 + utf8_len(bytes[pos + 1]);
            }
            _ => {
                let len = utf8_len(bytes[pos]);
                out.push_str(&content[pos..pos + len]);
                pos += len;
            }
        }
    }
    if has_dynamic { Some(out) } else { None }
}

/// Render an integer / string SCCP constant; `None` for boolean /
/// float (rendering is ambiguous, so folding bails for safety).
fn const_to_string(cv: &ConstValue) -> Option<String> {
    match cv {
        ConstValue::Number(NumericValue::Int(n)) => Some(n.to_string()),
        ConstValue::String(s) => Some(s.as_str().to_owned()),
        ConstValue::Number(NumericValue::Bool(_) | NumericValue::Float(_)) => None,
    }
}

/// Whether any `$var` in `content` is tainted without a
/// fixed-form mitigation colour.
#[cfg(test)]
fn has_unsafe_tainted_inputs(
    content: &str,
    uses: &HashMap<tcl_compiler::ssa::Symbol, Version>,
    taints: &HashMap<(tcl_compiler::ssa::Symbol, Version), TaintLattice>,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    style: tcl_dialect::BracedVarStyle,
) -> bool {
    let safe = safe_taint_colours();
    let mut pos = 0;
    let bytes = content.as_bytes();
    while pos < bytes.len() {
        if bytes[pos] == b'$' {
            let (end, name) = parse_var_ref(content, pos, style);
            if let Some(name) = name {
                let Some(symbol) = view.symbol(name) else {
                    return true;
                };
                let ver = uses.get(&symbol).copied().unwrap_or(0);
                if ver > 0
                    && let Some(t) = taints.get(&(symbol, ver))
                    && t.is_tainted()
                    && !t.colours.intersects(safe)
                {
                    return true;
                }
                pos = end;
            } else {
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
    false
}

/// Fixture scalar eligibility uses the shared reference extent and grammar.
#[cfg(test)]
fn scalar_reference_at(text: &str, pos: usize, config: LexerConfig) -> (usize, Option<&str>) {
    if text.as_bytes().get(pos) != Some(&b'$') {
        return (pos.saturating_add(1), None);
    }
    let Ok(Some(reference)) = tcl_lexer::scan_var_ref(text.as_bytes(), pos, config) else {
        return (pos.saturating_add(1), None);
    };
    if reference.index.is_some() {
        return (reference.next, None);
    }
    (reference.next, std::str::from_utf8(reference.name).ok())
}

#[cfg(test)]
pub(super) fn parse_var_ref(
    text: &str,
    pos: usize,
    style: tcl_dialect::BracedVarStyle,
) -> (usize, Option<&str>) {
    scalar_reference_at(
        text,
        pos,
        LexerConfig {
            braced_var: style,
            ..LexerConfig::default()
        },
    )
}
