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

//! Interval-driven dynamic bounds checking.
//!
//! The syntactic bounds checks (`analyser::bounds_checks`) only fire when *both*
//! the container and the index are literals.  This module covers the **dynamic**
//! cases they skip: an index that is a plain `$var` whose [`crate::intervals`]
//! range — narrowed at the use site by the range refinements in force there —
//! *proves* the access is out of range,
//! against a container length we can establish statically (a literal list /
//! `[list …]` element count, propagated per SSA version).
//!
//! It is a *consumer* of the parallel interval analysis: it never perturbs SCCP
//! or any existing diagnostic, and it only emits on the dynamic shapes the
//! syntactic check leaves silent, so the two never double-fire.
//!
//! Soundness rule: an [`Interval`] over-approximates the runtime value, so a
//! finding is reported **only** when the *whole* interval lies outside the valid
//! range — never on "might be out of range".

use std::collections::HashMap;
use tcl_core_types::DiagCode;
use tcl_dialect::{NumberSyntax, StringCharacterModel};

use tcl_lexer::Span;
use tcl_syntax::expr::ast::ExprNode;

use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::intervals::{
    Interval, build_guard_index, compute_intervals_with, refine_interval_for_value,
};
use crate::ir::Statement;
use crate::registry_invocation::{
    InvocationMetadataContext, resolved_statement_invocation_with_metadata_context,
    resolved_tokens_invocation_with_metadata_context,
};
use crate::sccp::SccpResult;
use crate::ssa::{Phi, SsaFunction, SsaSourceView, Symbol, ValueKey, Version};
use crate::types::TypeLattice;
use tcl_registry::{CommandRegistry, IntrinsicId, SemanticOperationId};

/// `(name, version) → Phi` index over every block, for length resolution
/// through loop-header phis.
type PhiIndex<'a> = HashMap<ValueKey, &'a Phi>;

/// `(name, version) → defining statement` index, so length resolution can see
/// what produced a version it can't read directly from the length map (a
/// length-preserving `lset` vs a length-changing `lappend`/`concat`/…).
type DefIndex<'a> = HashMap<ValueKey, LengthDefinition<'a>>;

#[derive(Clone, Copy)]
struct LengthDefinition<'a> {
    statement: &'a crate::ssa::SsaStatement,
    source: SsaSourceView<'a>,
    declaration_input: Option<ValueKey>,
}

struct LengthProofs<'a, 'b> {
    phis: &'b PhiIndex<'a>,
    definitions: &'b DefIndex<'a>,
    lengths: &'b HashMap<ValueKey, i64>,
    intervals: &'b HashMap<ValueKey, Interval>,
    semantics: BoundsSemantics<'a>,
    conditional_declaration: bool,
}

/// A resolved list length in the merge lattice.
///
/// The join is over phi incomings; an incoming whose length can't be positively
/// established must *poison* the merge rather than be silently ignored — see
/// [`resolve_len`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Len {
    /// A concrete, proven element count.
    Known(i64),
    /// Contributes no length constraint: a length-*preserving* back-edge whose
    /// own length is pinned on the forward path (the loop-header `lset` case).
    Neutral,
    /// The length could be anything — a length-changing or opaque def, or a
    /// caller-supplied live-in. Poisons the whole merge.
    Unknown,
}

impl Len {
    /// Lattice meet of two incoming lengths.
    fn combine(self, other: Len) -> Len {
        match (self, other) {
            (Len::Unknown, _) | (_, Len::Unknown) => Len::Unknown,
            (Len::Neutral, x) | (x, Len::Neutral) => x,
            (Len::Known(a), Len::Known(b)) => {
                if a == b {
                    Len::Known(a)
                } else {
                    Len::Unknown // genuine disagreement
                }
            }
        }
    }
}

/// If `stmt` is a length-*preserving* `lset name idx value` on `sym`, the input
/// version of `sym` it mutates in place (its length is the output's length).
///
/// Only the element-indexing form preserves length. `lset name {} value` (empty
/// index) replaces the whole list, so its length is that of `value` — not
/// preserving; it returns `None` (→ `Unknown`, sound).
fn lset_length_change(
    definition: LengthDefinition<'_>,
    sym: Symbol,
    expected: Option<i64>,
    proofs: &LengthProofs<'_, '_>,
) -> Option<i64> {
    let stmt = definition.statement;
    let Some(invocation) =
        crate::registry_invocation::normal_statement_representation_with_metadata_context(
            proofs.semantics.registry,
            proofs.semantics.context,
            &stmt.statement,
        )
    else {
        if !proofs.conditional_declaration
            || definition
                .declaration_input
                .is_none_or(|(selected, _)| selected != sym)
        {
            return None;
        }
        let access =
            crate::registry_invocation::conditional_index_access_advice_with_metadata_context(
                proofs.semantics.registry,
                proofs.semantics.context,
                definition.source.source_tokens()?,
            )?;
        return (access.kind == crate::registry_invocation::NormalIndexAccessKind::ListWrite
            && access.list_set_bounds == Some(tcl_dialect::ListSetBounds::ExistingElement)
            && !access.index.is_empty())
        .then_some(0);
    };
    let access = invocation.index_access()?;
    if access.kind != crate::registry_invocation::NormalIndexAccessKind::ListWrite {
        return None;
    }
    let index = access.index_literal;
    if index.as_deref() == Some("") {
        return None;
    }
    if access.list_set_bounds == Some(tcl_dialect::ListSetBounds::ExistingElement) {
        return stmt.uses.contains_key(&sym).then_some(0);
    }
    // A successful modern lset can append. Retain length only when its index
    // is proved unable to address that slot; error-only paths do not grow it.
    let length = expected?;
    let interval = if let Some(index) = index {
        let length = usize::try_from(length).ok()?;
        crate::intervals::constant(tcl_cmd_core::index::resolve_opt_in(
            &index,
            length,
            access.index_syntax?,
        )?)
    } else {
        let read = definition.source.read_word(&access.index_word)?;
        *proofs.intervals.get(&(read.symbol, read.version?))?
    };
    if interval.is_bottom() {
        return None;
    }
    if interval.lo == Some(length) && interval.hi == Some(length) {
        return Some(1);
    }
    (interval.hi.is_some_and(|hi| hi < length) || interval.lo.is_some_and(|lo| lo > length))
        .then_some(0)
}

/// Resolve a length only when every incoming normal continuation preserves it.
/// An append-capable back edge must exclude the candidate length as an index.
fn resolve_len(
    sym: Symbol,
    version: Version,
    proofs: &LengthProofs<'_, '_>,
    visited: &mut std::collections::HashSet<ValueKey>,
    expected: Option<i64>,
) -> Len {
    let key = (sym, version);
    if let Some(&length) = proofs.lengths.get(&key) {
        return Len::Known(length);
    }
    if !visited.insert(key) {
        return Len::Neutral;
    }
    if let Some(phi) = proofs.phis.get(&key) {
        // Earlier definitions establish the forward candidate before cyclic
        // back edges are checked against it. Sorting also makes this stable.
        let mut incoming: Vec<_> = phi.incoming.values().copied().collect();
        incoming.sort_unstable();
        let mut acc = Len::Neutral;
        for version in incoming {
            let candidate = match acc {
                Len::Known(length) => Some(length),
                _ => expected,
            };
            acc = acc.combine(resolve_len(
                sym,
                version,
                proofs,
                &mut visited.clone(),
                candidate,
            ));
            if acc == Len::Unknown {
                return Len::Unknown;
            }
        }
        return acc;
    }
    if let Some(&definition) = proofs.definitions.get(&key) {
        let input = definition
            .statement
            .uses
            .get(&sym)
            .copied()
            .or_else(|| {
                proofs
                    .conditional_declaration
                    .then_some(definition.declaration_input)
                    .flatten()
                    .filter(|(selected, _)| *selected == sym)
                    .map(|(_, version)| version)
            })
            .unwrap_or(0);
        let input_length = resolve_len(sym, input, proofs, &mut visited.clone(), expected);
        let candidate = match input_length {
            Len::Known(length) => Some(length),
            _ => expected,
        };
        if let Some(change) = lset_length_change(definition, sym, candidate, proofs) {
            return match input_length {
                Len::Known(length) => length.checked_add(change).map_or(Len::Unknown, Len::Known),
                Len::Neutral if change == 0 => Len::Neutral,
                Len::Neutral | Len::Unknown => Len::Unknown,
            };
        }
    }
    Len::Unknown
}

fn resolve_list_length(
    symbol: Symbol,
    version: Version,
    proofs: &LengthProofs<'_, '_>,
) -> Option<i64> {
    match resolve_len(
        symbol,
        version,
        proofs,
        &mut std::collections::HashSet::new(),
        None,
    ) {
        Len::Known(length) => Some(length),
        Len::Neutral | Len::Unknown => None,
    }
}

const LINDEX: &str = "lindex";
const LSET: &str = "lset";
const STRING_INDEX: &str = "string index";

/// A proven out-of-range dynamic index access.
#[derive(Debug, Clone)]
pub struct BoundsFinding {
    /// Source span to anchor the diagnostic on.
    pub span: Span,
    /// `"W230"` (lindex) / `"W231"` (lset) / `"W232"` (string index).
    pub code: DiagCode,
    /// `"lindex"` / `"lset"` / `"string index"` (display).
    pub command: String,
    /// The `$var` index name (display only).
    pub index_var: String,
    /// The proven index interval.
    pub index_interval: Interval,
    /// The container length proven OOR against.
    pub length: i64,
    /// `"negative"` | `"past_end"` | `"past_append"`.
    pub reason: String,
}

/// One index access: `(command, list_arg, index_arg, is_lset)`.
#[derive(Debug, Clone)]
struct Candidate {
    command: &'static str,
    list_arg: String,
    list_literal: Option<String>,
    index_arg: String,
    list_word: Option<crate::ir::WordExpr>,
    index_word: Option<crate::ir::WordExpr>,
    is_lset: bool,
    set_bounds: Option<tcl_dialect::ListSetBounds>,
    conditional_handler: bool,
}

/// The scalar variable name if `arg` is exactly `$name` / `${name}`.  Returns
/// `None` for `end`, `end-1`, `$arr(i)`, `[expr …]`, composites.
pub(crate) fn plain_var_name(arg: &str) -> Option<String> {
    let s = arg.trim();
    let mut s = s.strip_prefix('$')?;
    if let Some(inner) = s.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        s = inner;
    }
    if s.is_empty() {
        return None;
    }
    if s.bytes()
        .any(|b| matches!(b, b'(' | b'[' | b'$' | b' ' | b'\t' | b')' | b']'))
    {
        return None;
    }
    Some(s.to_owned())
}

/// Element count of a static Tcl list literal, or `None` if not literal.
fn literal_list_length(text: &str, rules: tcl_syntax::word_rules::WordValueRules) -> Option<i64> {
    i64::try_from(rules.split_list(text).ok()?.len()).ok()
}

/// If a value word is exactly `[list a b c]` with no substitution / expansion,
/// its element count, else `None`.
fn list_command_length(stmt: &Statement, semantics: BoundsSemantics<'_>) -> Option<i64> {
    let Statement::AssignValue {
        tokens: Some(parent),
        ..
    } = stmt
    else {
        return None;
    };
    let outer = resolved_statement_invocation_with_metadata_context(
        semantics.registry,
        semantics.context,
        stmt,
    )?;
    let word = outer.effective.words.get(2)?;
    let mut nested = crate::word_subst::whole_word_command_tokens(
        word,
        tcl_lexer::LexerConfig::from_grammar(semantics.grammar),
    )?;
    nested.inherit_nested_bindings(parent);
    let invocation = resolved_tokens_invocation_with_metadata_context(
        semantics.registry,
        semantics.context,
        &nested,
    )?;
    if invocation.facts.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListConstruct)
        || invocation
            .effective
            .words
            .iter()
            .any(|word| matches!(word, crate::ir::WordExpr::Expand { .. }))
    {
        return None;
    }
    i64::try_from(invocation.arguments.len()).ok()
}

/// Length of list-valued SSA versions established from literal-list assignments.
fn list_length_map(ssa: &SsaFunction, semantics: BoundsSemantics<'_>) -> HashMap<ValueKey, i64> {
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&semantics.grammar);
    let mut lengths = HashMap::new();
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            let n = match &s.statement {
                Statement::AssignConst { value, .. } => literal_list_length(value, rules),
                Statement::AssignValue { .. } => {
                    resolved_statement_invocation_with_metadata_context(
                        semantics.registry,
                        semantics.context,
                        &s.statement,
                    )
                    .and_then(|invocation| invocation.argument_literal(1))
                    .and_then(|value| literal_list_length(&value, rules))
                    .or_else(|| list_command_length(&s.statement, semantics))
                }
                _ => normal_stored_literal(&s.statement, semantics)
                    .and_then(|value| literal_list_length(&value, rules)),
            };
            if let Some(n) = n {
                for (&sym, &ver) in &s.defs {
                    if !s.may_defs.contains(&sym) {
                        lengths.insert((sym, ver), n);
                    }
                }
            }
        }
    }
    lengths
}

/// Actual normal setter passthrough, independent of whether the original
/// compiler selected a specialised assignment. Missing current address or
/// observer closure supplies no literal-length evidence.
fn normal_stored_literal(statement: &Statement, semantics: BoundsSemantics<'_>) -> Option<String> {
    let tokens = statement.tokens()?;
    let binding = tokens.source_binding.as_ref()?;
    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        semantics.registry,
        semantics.context,
        tokens,
    )?
    .stored_value_literal(&binding.variable_context, semantics.registry)
}

/// Character length of string-valued SSA versions from literal assignments,
/// counted under the selected dialect's character model.
///
/// `None` means no runtime release was selected: a string the two models count
/// identically still contributes its length, while a supplementary character
/// leaves the width ambiguous and contributes no fact at all.
fn string_length_map(
    ssa: &SsaFunction,
    characters: Option<StringCharacterModel>,
    semantics: BoundsSemantics<'_>,
) -> HashMap<ValueKey, i64> {
    let mut lengths = HashMap::new();
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            let value = match &s.statement {
                Statement::AssignConst { value, .. } => Some(value.clone()),
                Statement::AssignValue { .. } => {
                    resolved_statement_invocation_with_metadata_context(
                        semantics.registry,
                        semantics.context,
                        &s.statement,
                    )
                    .and_then(|invocation| invocation.argument_literal(1))
                }
                _ => normal_stored_literal(&s.statement, semantics),
            };
            let Some(resolved) = value else { continue };
            if let Some(count) = StringCharacterModel::count_for(characters, &resolved)
                && let Ok(len) = i64::try_from(count)
            {
                for (&sym, &ver) in &s.defs {
                    if !s.may_defs.contains(&sym) {
                        lengths.insert((sym, ver), len);
                    }
                }
            }
        }
    }
    lengths
}

/// Versions of each name reaching statement index `upto` within a block.  Used
/// for `lset`, whose target list is a *def* (not a use).
fn reaching_versions(
    entry: &HashMap<Symbol, Version>,
    stmts: &[crate::ssa::SsaStatement],
    upto: usize,
) -> HashMap<Symbol, Version> {
    let mut cur = entry.clone();
    for s in stmts.iter().take(upto) {
        for (&sym, &ver) in &s.defs {
            cur.insert(sym, ver);
        }
    }
    cur
}

/// Reason string if `index` is *wholly* out of range for `length`, else `None`.
/// `lset` permits the append slot (`index == length`); `lindex` does not.
fn classify(
    index: Interval,
    length: i64,
    is_lset: bool,
    set_bounds: Option<tcl_dialect::ListSetBounds>,
) -> Option<&'static str> {
    // Provably negative: the whole interval is below 0.
    if let Some(hi) = index.hi
        && hi < 0
    {
        return Some("negative");
    }
    // Provably past the end.
    if let Some(lo) = index.lo {
        if is_lset {
            if lo > length
                || (lo == length && set_bounds == Some(tcl_dialect::ListSetBounds::ExistingElement))
            {
                return Some("past_append");
            }
        } else if lo >= length {
            return Some("past_end");
        }
    }
    None
}

/// Select bounds operands from the live implementation's semantic identity.
fn invocation_candidate(
    invocation: &crate::registry_invocation::NormalRepresentationInvocation,
) -> Option<Candidate> {
    use crate::registry_invocation::NormalIndexAccessKind as Kind;
    let access = invocation.index_access()?;
    let (command, is_lset) = match access.kind {
        Kind::ListRead => (LINDEX, false),
        Kind::ListWrite => (LSET, true),
        Kind::StringRead => (STRING_INDEX, false),
    };
    Some(Candidate {
        command,
        list_literal: access.container_literal,
        list_arg: access.container,
        index_arg: access.index,
        list_word: Some(access.container_word),
        index_word: Some(access.index_word),
        is_lset,
        set_bounds: access.list_set_bounds,
        conditional_handler: false,
    })
}

/// Constant truthiness of a literal expression node (`Some(true/false)`), else
/// `None`.
fn const_bool(expr: &ExprNode) -> Option<bool> {
    use tcl_syntax::expr::ast::UnaryOp;
    // Fold the boolean-relevant unaries over a constant operand:
    // `+`/`-` preserve zero-ness (`-1` is
    // true, `-0` false), `!`/`not` invert it. `~` needs the integer value (a
    // bitwise-not guard is rare) so it stays conservative. Verified against
    // tclsh 8.4–9.0: `-1 && 1/0`, `!0 && 1/0` evaluate the RHS (a forced
    // arm → divide-by-zero), while `+0 && 1/0` short-circuits.
    if let ExprNode::Unary { op, operand } = expr {
        return match op {
            UnaryOp::Neg | UnaryOp::Pos => const_bool(operand),
            UnaryOp::Not | UnaryOp::WordNot => const_bool(operand).map(|b| !b),
            UnaryOp::BitNot => None,
        };
    }
    let ExprNode::Literal { text, .. } = expr else {
        return None;
    };
    let t = text.trim();
    if let Ok(n) = t.parse::<i64>() {
        return Some(n != 0);
    }
    if let Ok(f) = t.parse::<f64>() {
        return Some(f != 0.0);
    }
    match t.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Visit `expr` and every **guaranteed-to-evaluate** sub-expression.  The
/// short-circuit operand of `&&`/`||`/`and`/`or` and the non-selected ternary
/// arm run only when forced by a *constant* guard.
fn walk_eager(expr: &ExprNode, visit: &mut impl FnMut(&ExprNode)) {
    // Public entry: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`walk_eager_at`].
    walk_eager_at(expr, visit, 0);
}

fn walk_eager_at(expr: &ExprNode, visit: &mut impl FnMut(&ExprNode), depth: u32) {
    use tcl_syntax::expr::ast::BinOp;
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — the visitor
    // simply isn't invoked on sub-expressions buried deeper than the cap
    // (a conservative under-visit only reachable past 256 levels of
    // expression nesting); never a crash.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    visit(expr);
    match expr {
        ExprNode::Binary { op, left, right } => {
            walk_eager_at(left, visit, depth + 1);
            let lazy = matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr);
            if !lazy {
                walk_eager_at(right, visit, depth + 1);
                return;
            }
            let Some(guard) = const_bool(left) else {
                return; // maybe-dead RHS — leave it skipped
            };
            let is_and = matches!(op, BinOp::And | BinOp::WordAnd);
            let forced = if is_and { guard } else { !guard };
            if forced {
                walk_eager_at(right, visit, depth + 1);
            }
        }
        ExprNode::Unary { operand, .. } => walk_eager_at(operand, visit, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_eager_at(condition, visit, depth + 1);
            match const_bool(condition) {
                Some(true) => walk_eager_at(true_branch, visit, depth + 1),
                Some(false) => walk_eager_at(false_branch, visit, depth + 1),
                None => {}
            }
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                walk_eager_at(a, visit, depth + 1);
            }
        }
        _ => {}
    }
}

/// All index accesses a statement performs, retaining exact nested dispatch proofs.
fn statement_candidates(
    stmt: &Statement,
    source: SsaSourceView<'_>,
    semantics: BoundsSemantics<'_>,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    if let Some(invocation) =
        crate::registry_invocation::normal_statement_representation_with_metadata_context(
            semantics.registry,
            semantics.context,
            stmt,
        )
        && let Some(candidate) = invocation_candidate(&invocation)
    {
        out.push(candidate);
    }
    if out.is_empty()
        && let Some(access) = stmt.tokens().and_then(|tokens| {
            crate::registry_invocation::conditional_index_access_advice_with_metadata_context(
                semantics.registry,
                semantics.context,
                tokens,
            )
        })
    {
        use crate::registry_invocation::NormalIndexAccessKind as Kind;
        let (command, is_lset) = match access.kind {
            Kind::ListRead => (LINDEX, false),
            Kind::ListWrite => (LSET, true),
            Kind::StringRead => (STRING_INDEX, false),
        };
        out.push(Candidate {
            command,
            is_lset,
            list_arg: access.container,
            list_literal: access.container_literal,
            index_arg: access.index,
            list_word: Some(access.container_word),
            index_word: Some(access.index_word),
            set_bounds: access.list_set_bounds,
            conditional_handler: true,
        });
    }
    for nested in crate::word_subst::lifted_calls(
        source.source_tokens().or_else(|| stmt.tokens()),
        tcl_lexer::LexerConfig::from_grammar(semantics.grammar),
    ) {
        if let Some(tokens) = nested.tokens.as_ref()
            && let Some(invocation) =
                crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                    semantics.registry,
                    semantics.context,
                    tokens,
                )
            && let Some(candidate) = invocation_candidate(&invocation)
        {
            out.push(candidate);
        }
    }
    out
}

/// Failure-only owner diagnostics retain the original candidate and read gates.
#[cfg(test)]
pub(crate) fn report_interval_operand_gates(
    fu: &crate::compilation_unit::FunctionUnit,
    semantics: BoundsSemantics<'_>,
) {
    let lengths = list_length_map(&fu.ssa, semantics);
    eprintln!("bounds lengths={lengths:?}");
    let conditional = crate::intervals::compute_declaration_intervals_with(
        &fu.cfg,
        &fu.ssa,
        fu.diagnostic_value_facts().values(),
        semantics.grammar.numbers,
        semantics.registry,
    );
    eprintln!("bounds conditional_intervals={:?}", conditional.values());
    for (block, body) in &fu.ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&fu.ssa, *block, index);
            let normal =
                crate::registry_invocation::normal_statement_representation_with_metadata_context(
                    semantics.registry,
                    semantics.context,
                    &statement.statement,
                );
            let candidates = statement_candidates(&statement.statement, view, semantics);
            if let Some(tokens) = statement.statement.tokens() {
                eprintln!(
                    "bounds block={block:?} index={index} head={:?} normal={} index_access={} candidates={} runtime_unknown={:?}",
                    tokens.argv_texts.first(),
                    normal.is_some(),
                    normal
                        .as_ref()
                        .is_some_and(|normal| normal.index_access().is_some()),
                    candidates.len(),
                    tokens
                        .source_binding
                        .as_ref()
                        .map(crate::command_binding::SourceInvocationBinding::execution_is_unknown)
                );
            }
            for candidate in candidates {
                eprintln!(
                    "bounds command={} index_read={:?} list_read={:?}",
                    candidate.command,
                    candidate
                        .index_word
                        .as_ref()
                        .and_then(|word| view.read_word(word)),
                    candidate
                        .list_word
                        .as_ref()
                        .and_then(|word| view.read_word(word))
                );
                if candidate.conditional_handler
                    && let Some(tokens) = view.source_tokens()
                {
                    let reads = tokens.source_binding.as_ref().and_then(|binding| {
                        binding.declaration_read_occurrences(semantics.registry, tokens)
                    });
                    eprintln!(
                        "bounds declaration_reads={:?}",
                        reads.as_ref().map(|reads| {
                            reads
                                .iter()
                                .map(|read| {
                                    (
                                        read.name(),
                                        read.spelling(),
                                        read.diagnostic_version(
                                            &fu.ssa,
                                            *block,
                                            index,
                                            semantics.registry,
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    );
                }
            }
        }
    }
}

/// Interpreter and registry evidence used by interval bounds queries.
#[derive(Clone, Copy)]
pub struct BoundsSemantics<'a> {
    /// Actual command registry for this compilation unit.
    pub registry: &'a CommandRegistry,
    /// Complete supplied availability generation. Actual contexts retain their
    /// command-store identity; explicit standalone callers may supply `None`.
    /// Metadata grants no Native lookup, cell, normal effect or compiler proof.
    pub context: Option<InvocationMetadataContext<'a>>,
    /// Document word grammar.
    pub grammar: tcl_dialect::LexerGrammar,
}

/// Dynamic out-of-range findings for this function (empty if none), over
/// the solver's result `sccp` and the type lattice `types`: the lattice seeds
/// the intervals, and its range refinements narrow an index the types prove
/// an integer where it is read ([`crate::intervals::refine_interval`]). `executable` restricts
/// to SCCP-reachable blocks.
///
/// `numbers` is the target release's numeric-literal grammar, threaded from the
/// analyser's dialect alongside `characters` (the same shape of dialect-derived
/// fact): it decides what an index literal in the guarded expression *is* —
/// `0755` is 493 up to 8.6 and 755 from 9.0 — so a version-blind read can prove
/// a range that reality never has.
#[must_use]
pub fn find_interval_bounds_with<S, T>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (sccp, types): (&SccpResult, &HashMap<ValueKey, TypeLattice, T>),
    executable: &std::collections::HashSet<BlockId, S>,
    characters: Option<StringCharacterModel>,
    numbers: NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
) -> Vec<BoundsFinding>
where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher,
{
    find_interval_bounds_resolved(
        cfg,
        ssa,
        (&sccp.values, (sccp, types)),
        executable,
        characters,
        numbers,
        BoundsSemantics {
            registry: tcl_registry::model::ingress::static_context_for("tcl9.0").commands(),
            context: None,
            grammar,
        },
    )
}

/// Find dynamic bounds failures through point-resolved invocation facts.
#[must_use]
pub fn find_interval_bounds_resolved<S1, S2, S3>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (values, (sccp, types)): (
        &HashMap<ValueKey, LatticeValue, S1>,
        (&SccpResult, &HashMap<ValueKey, TypeLattice, S3>),
    ),
    executable: &std::collections::HashSet<BlockId, S2>,
    characters: Option<StringCharacterModel>,
    numbers: NumberSyntax,
    semantics: BoundsSemantics<'_>,
) -> Vec<BoundsFinding>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
    S3: std::hash::BuildHasher,
{
    let grammar = semantics.grammar;
    let ctx = BoundsCtx {
        cfg,
        ssa,
        sccp,
        types,
        numbers,
        characters,
        intervals: compute_intervals_with(cfg, ssa, values, numbers),
        declaration_intervals: conditional_index_layout_exists(ssa, semantics).then(|| {
            crate::intervals::compute_declaration_intervals_with(
                cfg,
                ssa,
                values,
                numbers,
                semantics.registry,
            )
        }),
        guard_index: build_guard_index(cfg, ssa, grammar),
        pred_counts: cfg
            .predecessors()
            .into_iter()
            .map(|(bid, preds)| (bid, preds.len()))
            .collect(),
        lengths: list_length_map(ssa, semantics),
        grammar,
        semantics,
        str_lengths: string_length_map(ssa, characters, semantics),
        phi_index: ssa
            .blocks
            .values()
            .flat_map(|sb| sb.phis.iter())
            .map(|p| ((p.name, p.version), p))
            .collect(),
        defs: ssa
            .blocks
            .iter()
            .flat_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .flat_map(move |(index, statement)| {
                        statement.defs.iter().map(move |(&symbol, &version)| {
                            (
                                (symbol, version),
                                LengthDefinition {
                                    statement,
                                    source: SsaSourceView::at_statement(ssa, block, index),
                                    declaration_input: declaration_lset_input(
                                        ssa, block, index, semantics,
                                    ),
                                },
                            )
                        })
                    })
            })
            .collect(),
    };
    let mut findings = Vec::new();

    for (bid, sb) in &ssa.blocks {
        if !executable.contains(bid) {
            continue;
        }
        let bn = *bid;
        for (idx, s) in sb.statements.iter().enumerate() {
            let span = statement_span(&s.statement);
            for cand in statement_candidates(
                &s.statement,
                SsaSourceView::at_statement(ssa, bn, idx),
                semantics,
            ) {
                if let Some(span) = span {
                    let site = CandidateSite {
                        bn,
                        span,
                        entry_versions: &sb.entry_versions,
                        block_stmts: &sb.statements,
                        stmt_idx: idx,
                    };
                    ctx.process(&cand, &site, &mut findings);
                }
            }
        }
        // Index accesses in a `return [...]` value / branch condition: the read
        // versions are the block's exit versions; anchor on the terminator.
        let Some(block) = cfg.blocks.get(bid) else {
            continue;
        };
        ctx.process_terminator(block, sb, bn, &mut findings);
    }
    findings
}

fn conditional_index_layout_exists(ssa: &SsaFunction, semantics: BoundsSemantics<'_>) -> bool {
    ssa.blocks.iter().any(|(&block, body)| {
        body.statements.iter().enumerate().any(|(index, _)| {
            SsaSourceView::at_statement(ssa, block, index)
                .source_tokens()
                .is_some_and(|tokens| {
                    crate::registry_invocation::conditional_index_access_advice_with_metadata_context(
                        semantics.registry,
                        semantics.context,
                        tokens,
                    )
                    .is_some()
                })
        })
    })
}

fn declaration_lset_input(
    ssa: &SsaFunction,
    block: BlockId,
    index: usize,
    semantics: BoundsSemantics<'_>,
) -> Option<ValueKey> {
    let view = SsaSourceView::at_statement(ssa, block, index);
    let tokens = view.source_tokens()?;
    let access = crate::registry_invocation::conditional_index_access_advice_with_metadata_context(
        semantics.registry,
        semantics.context,
        tokens,
    )?;
    if access.kind != crate::registry_invocation::NormalIndexAccessKind::ListWrite {
        return None;
    }
    tokens
        .source_binding
        .as_ref()?
        .declaration_variable_operand_advice(semantics.registry, tokens, &access.container_word)?
        .diagnostic_version(ssa, block, index, semantics.registry)
}

/// Read-only analysis state shared by the per-candidate bounds checks,
/// borrowed for the duration of [`find_interval_bounds_with`].
struct BoundsCtx<'a, T> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    sccp: &'a SccpResult,
    types: &'a HashMap<ValueKey, TypeLattice, T>,
    /// The target release's numeric-literal grammar — carried here so the
    /// conditional source-guard tables read a branch's
    /// constant bounds for the right dialect.
    numbers: NumberSyntax,
    characters: Option<StringCharacterModel>,
    intervals: HashMap<ValueKey, Interval>,
    declaration_intervals: Option<crate::intervals::DeclarationIntervals>,
    guard_index: HashMap<ValueKey, Vec<BlockId>>,
    /// Predecessor count per block — used to require a guarded branch target
    /// have a single entry edge before its constraint is applied.
    pred_counts: HashMap<BlockId, usize>,
    lengths: HashMap<ValueKey, i64>,
    str_lengths: HashMap<ValueKey, i64>,
    phi_index: PhiIndex<'a>,
    defs: DefIndex<'a>,
    /// The document dialect's lexer grammar — how a literal list word divides
    /// and where a `[…]` substitution's word boundaries fall, so a length
    /// proof matches the document's own parser.
    grammar: tcl_dialect::LexerGrammar,
    semantics: BoundsSemantics<'a>,
}

/// The single index-access call site `process` evaluates: the versions
/// reaching it and where in the block it sits.
struct CandidateSite<'a> {
    bn: crate::cfg::BlockId,
    span: Span,
    entry_versions: &'a HashMap<Symbol, Version>,
    block_stmts: &'a [crate::ssa::SsaStatement],
    stmt_idx: usize,
}

impl CandidateSite<'_> {
    fn source_view<'a>(&self, ssa: &'a SsaFunction) -> SsaSourceView<'a> {
        if self.stmt_idx == self.block_stmts.len() {
            SsaSourceView::at_terminator(ssa, self.bn)
        } else {
            SsaSourceView::at_statement(ssa, self.bn, self.stmt_idx)
        }
    }
}

impl<T: std::hash::BuildHasher> BoundsCtx<'_, T> {
    fn interval_values(&self, conditional: bool) -> &HashMap<ValueKey, Interval> {
        if conditional && let Some(intervals) = &self.declaration_intervals {
            intervals.values()
        } else {
            &self.intervals
        }
    }

    /// Resolve the list length backing `cand` at one call site, if known.
    fn length_for_list(&self, cand: &Candidate, site: &CandidateSite) -> Option<i64> {
        let ssa = self.ssa;
        let proofs = LengthProofs {
            phis: &self.phi_index,
            definitions: &self.defs,
            lengths: &self.lengths,
            intervals: self.interval_values(cand.conditional_handler),
            semantics: self.semantics,
            conditional_declaration: cand.conditional_handler,
        };
        if cand.is_lset {
            // `lset`'s first arg is a variable *name*, recorded as a def —
            // use the version reaching this statement.
            let lname = if cand.conditional_handler {
                cand.list_arg.as_str()
            } else {
                cand.list_arg.trim()
            };
            if !cand.conditional_handler && (lname.contains('$') || lname.contains('[')) {
                return None;
            }
            let reaching = reaching_versions(site.entry_versions, site.block_stmts, site.stmt_idx);
            let selected = site
                .source_view(ssa)
                .symbol(lname)
                .and_then(|symbol| {
                    reaching
                        .get(&symbol)
                        .copied()
                        .map(|version| (symbol, version))
                })
                .or_else(|| {
                    if !cand.conditional_handler {
                        return None;
                    }
                    let view = site.source_view(ssa);
                    let tokens = view.source_tokens()?;
                    let operand = tokens
                        .source_binding
                        .as_ref()?
                        .declaration_variable_operand_advice(
                            self.semantics.registry,
                            tokens,
                            cand.list_word.as_ref()?,
                        )?;
                    if operand.name() != lname {
                        return None;
                    }
                    operand.diagnostic_version(ssa, site.bn, site.stmt_idx, self.semantics.registry)
                })?;
            return resolve_list_length(selected.0, selected.1, &proofs);
        }
        // A *value* arg: literal list, or `$l`.
        if let Some(value) = &cand.list_literal {
            return literal_list_length(
                value,
                tcl_syntax::word_rules::WordValueRules::from_grammar(&self.grammar),
            );
        }
        let view = site.source_view(ssa);
        let read = view.read_word(cand.list_word.as_ref()?)?;
        resolve_list_length(read.symbol, read.version?, &proofs)
    }

    /// Evaluate one candidate index access; push a finding when the index
    /// interval is provably out of range for the resolved length.
    fn process(&self, cand: &Candidate, site: &CandidateSite, findings: &mut Vec<BoundsFinding>) {
        let ssa = self.ssa;
        let Some(index_var) = plain_var_name(&cand.index_arg) else {
            return;
        };
        let read = cand
            .index_word
            .as_ref()
            .and_then(|word| site.source_view(ssa).read_word(word));
        let selected = read
            .and_then(|read| read.version.map(|version| (read.symbol, version)))
            .or_else(|| {
                if !cand.conditional_handler {
                    return None;
                }
                let (spelling, source) = cand.index_word.as_ref()?.sole_variable_substitution()?;
                let tokens = site.source_view(ssa).source_tokens()?;
                let occurrences = tokens
                    .source_binding
                    .as_ref()?
                    .declaration_read_occurrences(self.semantics.registry, tokens)?;
                let occurrence = occurrences
                    .iter()
                    .find(|read| read.source() == source && read.spelling() == spelling)?;
                occurrence.diagnostic_version(
                    ssa,
                    site.bn,
                    if site.stmt_idx == site.block_stmts.len() {
                        usize::MAX
                    } else {
                        site.stmt_idx
                    },
                    self.semantics.registry,
                )
            });
        let Some((index_sym, index_version)) = selected else {
            return;
        };
        if index_version == 0 {
            return;
        }
        let length = if cand.command == STRING_INDEX {
            cand.list_literal
                .as_ref()
                .and_then(|value| StringCharacterModel::count_for(self.characters, value))
                .and_then(|length| i64::try_from(length).ok())
                .or_else(|| {
                    let read = site.source_view(ssa).read_word(cand.list_word.as_ref()?)?;
                    self.str_lengths.get(&(read.symbol, read.version?)).copied()
                })
        } else {
            self.length_for_list(cand, site)
        };
        let Some(length) = length else {
            return;
        };
        let iv = if cand.conditional_handler {
            let Some(intervals) = &self.declaration_intervals else {
                return;
            };
            crate::intervals::refine_declaration_interval_for_value(
                intervals,
                self.cfg,
                ssa,
                site.bn,
                index_sym,
                index_version,
                crate::intervals::DeclarationGuardTables {
                    guard_index: &self.guard_index,
                    pred_counts: &self.pred_counts,
                    numbers: self.numbers,
                },
            )
        } else {
            refine_interval_for_value(
                &self.intervals,
                (self.sccp, self.types),
                site.bn,
                index_sym,
                index_version,
            )
        };
        if iv.is_top() || iv.is_bottom() {
            return;
        }
        let Some(reason) = classify(iv, length, cand.is_lset, cand.set_bounds) else {
            return;
        };
        let code = if cand.is_lset {
            DiagCode::W231
        } else if cand.command == STRING_INDEX {
            DiagCode::W232
        } else {
            DiagCode::W230
        };
        findings.push(BoundsFinding {
            span: site.span,
            code,
            command: cand.command.to_owned(),
            index_var,
            index_interval: iv,
            length,
            reason: if cand.conditional_handler {
                format!(
                    "{reason} if the original native handlers and declaration-local values are used"
                )
            } else {
                reason.to_owned()
            },
        });
    }

    /// Index accesses in a `return [...]` value / branch condition: the read
    /// versions are the block's exit versions; anchor on the terminator.
    fn process_terminator(
        &self,
        block: &crate::cfg::Block,
        sb: &crate::ssa::SsaBlock,
        bn: crate::cfg::BlockId,
        findings: &mut Vec<BoundsFinding>,
    ) {
        let exit_site = |span: Span| CandidateSite {
            bn,
            span,
            entry_versions: &sb.exit_versions,
            block_stmts: &sb.statements,
            stmt_idx: sb.statements.len(),
        };
        if let Some(tokens) = SsaSourceView::at_terminator(self.ssa, bn).source_tokens()
            && let Some(span) = block.terminator.as_ref().and_then(Terminator::span)
        {
            for nested in crate::word_subst::lifted_calls(
                Some(tokens),
                tcl_lexer::LexerConfig::from_grammar(self.grammar),
            ) {
                if let Some(tokens) = nested.tokens.as_ref()
                    && let Some(invocation) =
                        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                            self.semantics.registry,
                            self.semantics.context,
                            tokens,
                        )
                    && let Some(candidate) = invocation_candidate(&invocation)
                {
                    self.process(&candidate, &exit_site(span), findings);
                }
            }
        }
    }
}

/// A divide-by-zero finding: a `/` or `%` whose divisor is provably the
/// single point `[0, 0]` on the always-evaluated spine of an executable
/// expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivZeroFinding {
    /// Source span of the enclosing statement / terminator.
    pub span: Span,
    /// The offending operator: `"/"` (divide) or `"%"` (modulo).
    pub op: &'static str,
}

/// The expression a flat IR statement evaluates, if any.
fn statement_expr(stmt: &Statement) -> Option<&ExprNode> {
    match stmt {
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => Some(expr),
        Statement::Return { expr, .. } => expr.as_ref(),
        _ => None,
    }
}

/// Does `expr` contain a `/` or `%` operator anywhere (eager or not)?
/// Cheap pre-scan helper.
fn expr_has_divisor(expr: &ExprNode) -> bool {
    use tcl_syntax::expr::ast::BinOp;
    let mut found = false;
    walk_eager(expr, &mut |e| {
        if let ExprNode::Binary { op, .. } = e
            && matches!(op, BinOp::Div | BinOp::Mod)
        {
            found = true;
        }
    });
    found
}

/// Push a [`DivZeroFinding`] for each unconditionally-evaluated `/` / `%`
/// whose divisor abstract-evaluates to exactly `[0, 0]`. Short-circuited
/// `&&`/`||` operands and dead ternary arms are skipped by [`walk_eager`],
/// so a guarded `1/$d` never yields a finding. Only owned `Copy`/`'static`
/// data escapes the walk closure.
fn collect_divzero(
    expr: &ExprNode,
    span: Span,
    read: &impl Fn(&ExprNode) -> Interval,
    numbers: NumberSyntax,
    out: &mut Vec<DivZeroFinding>,
) {
    use tcl_syntax::expr::ast::BinOp;
    walk_eager(expr, &mut |e| {
        if let ExprNode::Binary { op, right, .. } = e {
            let op = match op {
                BinOp::Div => "/",
                BinOp::Mod => "%",
                _ => return,
            };
            let iv = crate::intervals::eval_expr_with_reads(right, read, numbers);
            if iv.lo == Some(0) && iv.hi == Some(0) {
                out.push(DivZeroFinding { span, op });
            }
        }
    });
}

/// Cheap pre-scan: does any reachable expression contain a `/` or `%`?
fn has_division(cfg: &CfgFunction, ssa: &SsaFunction) -> bool {
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            if statement_expr(&s.statement).is_some_and(expr_has_divisor) {
                return true;
            }
        }
    }
    for block in cfg.blocks.values() {
        let has = match &block.terminator {
            Some(Terminator::Branch { condition, .. }) => expr_has_divisor(condition),
            Some(Terminator::Return { expr: Some(e), .. }) => expr_has_divisor(e),
            _ => false,
        };
        if has {
            return true;
        }
    }
    false
}

/// Divisions / modulo whose divisor is provably `[0, 0]` (a runtime error).
///
/// Sound: the divisor's interval (narrowed at the use site by the range
/// refinements of the solver's result `sccp` in force there, for a divisor
/// the type lattice `types` proves an integer) must be
/// exactly `[0, 0]`, and the block must be SCCP-executable. Shares the same
/// interval machinery (`compute_intervals_with` / `refine_interval` /
/// `eval_expr_with_reads`) as [`find_interval_bounds_with`], including its `numbers`
/// numeral grammar — a divisor literal is read for the target release, so a
/// spelling that is not a numeral there (`0o0` under 8.4) proves nothing.
/// Findings are returned in source-span order for deterministic output.
#[must_use]
pub fn find_divide_by_zero_with<S, T>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (sccp, types): (&SccpResult, &HashMap<ValueKey, TypeLattice, T>),
    executable: &std::collections::HashSet<BlockId, S>,
    numbers: NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
) -> Vec<DivZeroFinding>
where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher,
{
    find_divide_by_zero_impl(
        cfg,
        ssa,
        (&sccp.values, (sccp, types)),
        executable,
        numbers,
        grammar,
        None,
    )
}

/// Include original entered operand evaluators without granting parent dispatch.
#[must_use]
pub fn find_divide_by_zero_with_entered_operands<S1, S2, S3>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (values, facts): (
        &HashMap<ValueKey, LatticeValue, S1>,
        (&SccpResult, &HashMap<ValueKey, TypeLattice, S3>),
    ),
    executable: &std::collections::HashSet<BlockId, S2>,
    numbers: NumberSyntax,
    semantics: BoundsSemantics<'_>,
) -> Vec<DivZeroFinding>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
    S3: std::hash::BuildHasher,
{
    find_divide_by_zero_impl(
        cfg,
        ssa,
        (values, facts),
        executable,
        numbers,
        semantics.grammar,
        Some(semantics.registry),
    )
}

fn entered_division_operands<S>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &std::collections::HashSet<BlockId, S>,
    grammar: tcl_dialect::LexerGrammar,
    registry: Option<&CommandRegistry>,
) -> Vec<(
    BlockId,
    usize,
    crate::word_subst::EnteredExpressionEvaluation,
)>
where
    S: std::hash::BuildHasher,
{
    let mut entered = Vec::new();
    if let Some(registry) = registry {
        for (&block, data) in &ssa.blocks {
            if !executable.contains(&block) {
                continue;
            }
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                if index != usize::MAX
                    && statement_expr(&data.statements[index].statement).is_some()
                {
                    continue;
                }
                if index == usize::MAX
                    && cfg.blocks.get(&block).is_some_and(|block| {
                        matches!(
                            block.terminator.as_ref(),
                            Some(Terminator::Return { expr: Some(_), .. })
                        )
                    })
                {
                    continue;
                }
                let view = if index == usize::MAX {
                    SsaSourceView::at_terminator(ssa, block)
                } else {
                    SsaSourceView::at_statement(ssa, block, index)
                };
                for evaluation in crate::word_subst::entered_expression_evaluations(
                    view.source_tokens(),
                    tcl_lexer::LexerConfig::from_grammar(grammar),
                    registry,
                ) {
                    entered.push((block, index, evaluation));
                }
            }
        }
    }
    entered
}

fn collect_lowered_divzero<S>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &std::collections::HashSet<BlockId, S>,
    numbers: NumberSyntax,
    read: &impl Fn(&ExprNode, BlockId, usize, Option<u32>) -> crate::intervals::Interval,
    findings: &mut Vec<DivZeroFinding>,
) where
    S: std::hash::BuildHasher,
{
    for (bid, sb) in &ssa.blocks {
        if !executable.contains(bid) {
            continue;
        }
        let bn = *bid;
        for (index, s) in sb.statements.iter().enumerate() {
            if let Some(expr) = statement_expr(&s.statement) {
                let span = statement_span(&s.statement).unwrap_or_else(|| Span::new(0, 0));
                collect_divzero(
                    expr,
                    span,
                    &|node| {
                        let base = match &s.statement {
                            Statement::AssignExpr { expr_base, .. }
                            | Statement::ExprEval { expr_base, .. }
                            | Statement::Return { expr_base, .. } => *expr_base,
                            _ => None,
                        };
                        read(node, bn, index, base)
                    },
                    numbers,
                    findings,
                );
            }
        }
        let Some(block) = cfg.blocks.get(bid) else {
            continue;
        };
        match &block.terminator {
            Some(Terminator::Branch {
                condition,
                span: Some(span),
                condition_base,
                ..
            }) => collect_divzero(
                condition,
                *span,
                &|node| read(node, bn, usize::MAX, *condition_base),
                numbers,
                findings,
            ),
            Some(Terminator::Return {
                expr: Some(e),
                span: Some(span),
                expr_base,
                ..
            }) => collect_divzero(
                e,
                *span,
                &|node| read(node, bn, usize::MAX, *expr_base),
                numbers,
                findings,
            ),
            _ => {}
        }
    }
}

fn find_divide_by_zero_impl<S1, S2, S3>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (values, (sccp, types)): (
        &HashMap<ValueKey, LatticeValue, S1>,
        (&SccpResult, &HashMap<ValueKey, TypeLattice, S3>),
    ),
    executable: &std::collections::HashSet<BlockId, S2>,
    numbers: NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
    registry: Option<&CommandRegistry>,
) -> Vec<DivZeroFinding>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
    S3: std::hash::BuildHasher,
{
    let entered = entered_division_operands(cfg, ssa, executable, grammar, registry);
    if !has_division(cfg, ssa)
        && !entered
            .iter()
            .any(|(_, _, evaluation)| expr_has_divisor(evaluation.expression()))
    {
        return Vec::new();
    }
    let intervals = compute_intervals_with(cfg, ssa, values, numbers);

    let interval_for = |node: &ExprNode, bn: BlockId, index: usize, base: Option<u32>| {
        let view = if index == usize::MAX {
            SsaSourceView::at_terminator(ssa, bn)
        } else {
            SsaSourceView::at_statement(ssa, bn, index)
        };
        let Some(read) = view.read_expression_variable(node, base) else {
            return crate::intervals::TOP;
        };
        let Some(version) = read.version.filter(|version| *version > 0) else {
            return crate::intervals::TOP;
        };
        refine_interval_for_value(&intervals, (sccp, types), bn, read.symbol, version)
    };

    let mut findings: Vec<DivZeroFinding> = Vec::new();
    collect_lowered_divzero(cfg, ssa, executable, numbers, &interval_for, &mut findings);
    for (block, index, evaluation) in entered {
        collect_divzero(
            evaluation.expression(),
            evaluation.span(),
            &|node| {
                if evaluation.numbers() == numbers && evaluation.grammar() == grammar {
                    interval_for(node, block, index, Some(evaluation.expression_base()))
                } else {
                    crate::intervals::TOP
                }
            },
            evaluation.numbers(),
            &mut findings,
        );
    }
    // Deterministic, source-order output (HashMap block iteration is not).
    findings.sort_by_key(|f| (f.span.start(), f.span.end(), f.op));
    findings
}

/// The source span of an IR statement, for anchoring a finding.  Only the flat
/// statement shapes that appear in CFG-lowered SSA blocks carry an index access;
/// structured statements (`If`/`For`/…) are gone by this point, so they need no
/// span here.
fn statement_span(stmt: &Statement) -> Option<Span> {
    match stmt {
        Statement::AssignConst { span, .. }
        | Statement::AssignExpr { span, .. }
        | Statement::AssignValue { span, .. }
        | Statement::Incr { span, .. }
        | Statement::ExprEval { span, .. }
        | Statement::Call { span, .. }
        | Statement::Return { span, .. }
        | Statement::Barrier { span, .. } => Some(*span),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    fn context_test_unit(
        source: &str,
        context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> crate::compilation_unit::CompilationUnit {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            std::sync::Arc::clone(context),
        )
    }

    fn context_test_candidates(
        function: &crate::compilation_unit::FunctionUnit,
        semantics: super::BoundsSemantics<'_>,
    ) -> Vec<super::Candidate> {
        function
            .ssa
            .blocks
            .iter()
            .flat_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .flat_map(move |(index, row)| {
                        super::statement_candidates(
                            &row.statement,
                            crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index),
                            semantics,
                        )
                    })
            })
            .collect()
    }

    #[test]
    fn retained_bounds_context_keeps_availability_source_operands_and_store_identity() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // A custom gated source descriptor; no stock release-availability or Native-entry claim.
        use super::*;
        use std::sync::Arc;
        let source = "proc f {i} {lset values $i NEW}";
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let mut registry = CommandRegistry::build_default();
        let mut gated = registry.get("lset").unwrap().clone();
        gated.surface = registry.get("dict").unwrap().surface;
        registry.insert(gated);
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(Arc::new(registry)),
        );
        let unit = context_test_unit(source, &context);
        let function = &unit.procedures["::f"];
        let original = function
            .ssa
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements.iter().enumerate().find_map(|(index, _)| {
                    SsaSourceView::at_statement(&function.ssa, block, index).source_tokens()
                })
            })
            .unwrap();
        assert!(
            original
                .source_binding
                .as_ref()
                .unwrap()
                .execution_is_unknown()
        );
        let selected = BoundsSemantics {
            registry: context.commands(),
            context: Some(context.as_ref().into()),
            grammar: profile.grammar,
        };
        let positive = context_test_candidates(function, selected);
        assert_eq!(positive.len(), 1);
        assert!(
            positive[0].conditional_handler,
            "source layout cannot create a normal native handler"
        );
        assert_eq!(positive[0].index_arg, "$i");
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands()));
        assert!(Arc::ptr_eq(older.commands(), context.commands()));
        let unavailable = BoundsSemantics {
            context: Some((&older).into()),
            ..selected
        };
        assert!(context_test_candidates(function, unavailable).is_empty());
        let mut foreign = CommandRegistry::build_default();
        foreign.insert(tcl_registry::CommandSpec {
            name: "foreign-marker",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let foreign_context = context.with_command_store(Arc::new(foreign));
        let mismatched = BoundsSemantics {
            context: Some((&foreign_context).into()),
            ..selected
        };
        assert!(context_test_candidates(function, mismatched).is_empty());
        let replaced = context_test_unit(
            "proc lset {args} {}; proc f {i} {lset values $i NEW}",
            &context,
        );
        assert!(context_test_candidates(&replaced.procedures["::f"], selected).is_empty());
        let mut missing = original.clone();
        missing.source_binding = None;
        assert!(
            crate::registry_invocation::conditional_index_access_advice_with_metadata_context(
                selected.registry,
                selected.context,
                &missing
            )
            .is_none()
        );
    }

    #[test]
    fn bounds_follow_live_command_identity_and_literal_body_quoting() {
        assert_eq!(
            bounds(
                "proc lindex {args} {return custom}; proc f {} {set l {a b}; set i 9; return [lindex $l $i]}"
            ),
            [] as [(String, String); 0]
        );
        assert_eq!(
            bounds("proc f {} {set l {a b}; set i 9; puts {[lindex $l $i]}"),
            [] as [(String, String); 0]
        );
        let renamed =
            bounds("rename lindex pick; proc f {} {set l {a b}; set i 9; return [pick $l $i]}");
        assert_eq!(renamed.len(), 1, "{renamed:?}");
        let prefixed =
            bounds("interp alias {} pick {} lindex {a b}; proc f {} {set i 9; return [pick $i]}");
        assert_eq!(prefixed.len(), 1, "{prefixed:?}");
    }

    #[test]
    fn lset_append_updates_following_length_without_an_old_length_warning() {
        assert_eq!(
            bounds("proc f {} {set l {a}; lset l 1 b; set i 1; return [lindex $l $i]}"),
            [] as [(String, String); 0]
        );
        let past = bounds("proc f {} {set l {a}; lset l 1 b; set i 2; return [lindex $l $i]}");
        assert_eq!(past.len(), 1, "{past:?}");
    }

    #[test]
    fn append_slot_bounds_follow_each_c_runtime_release() {
        let source = "proc f {} {set l {a}; set i 1; lset l $i b}";
        for (dialect, expected) in [
            ("tcl8.4", 1),
            ("tcl8.5", 1),
            ("tcl8.6", 0),
            ("tcl9.0", 0),
            ("tcl9.1", 0),
        ] {
            let diagnostics = Analyser::new().analyse(source, dialect).diagnostics;
            assert_eq!(
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.code.as_str() == "W231")
                    .count(),
                expected,
                "{dialect}: {diagnostics:?}"
            );
        }
    }

    /// `walk_eager` recurses once per
    /// `ExprNode` level, so it needs a depth cap. A tree built
    /// directly is unbounded (the Pratt parser caps its own output at 256)
    /// and empirically overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread. 3000 is past that crash range
    /// and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that it returns
    /// at all.
    #[test]
    fn deeply_nested_walk_eager_survives() {
        use crate::expr_ast::{ExprNode, UnaryOp};
        let mut node = ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(node),
            };
        }
        let mut count = 0usize;
        super::walk_eager(&node, &mut |_| count += 1);
        // The visitor ran without overflowing the native stack; it stops
        // descending at the cap, so it visits at most ~257 nodes here.
        assert!(count >= 1);
    }

    /// The `op` of every W233 divide-by-zero finding for `src`'s top level.
    fn divzero(src: &str) -> Vec<&'static str> {
        use crate::compilation_unit::CompilationUnit;
        let context = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let profile = context.commands().profile().expect("actual C9 profile");
        let cu = CompilationUnit::build_for_profile(src, context.commands(), false, profile);
        let fu = &cu.top_level;
        super::find_divide_by_zero_with(
            &fu.cfg,
            &fu.ssa,
            (&fu.sccp, &fu.types),
            &fu.sccp.executable_blocks,
            tcl_dialect::NumberSyntax::of_profile(Some(profile)),
            profile.grammar,
        )
        .iter()
        .map(|d| d.op)
        .collect()
    }

    #[test]
    fn entered_operand_divisions_survive_parent_failure_and_respect_reachability() {
        for (source, expected) in [
            ("return [expr {1 / 0}]", 1),
            ("set d 0; return [expr {1 / $d}]", 1),
            ("error STOP; return [expr {1 / 0}]", 0),
            ("return [error STOP] [expr {1 / 0}]", 0),
            ("return {[expr {1 / 0}]}", 0),
            (
                "rename expr saved; proc expr args {return CUSTOM}; return [expr {1 / 0}]",
                0,
            ),
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            assert_eq!(
                result
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.code.as_str() == "W233")
                    .count(),
                expected,
                "{source}: {:?}",
                result.diagnostics,
            );
        }
        assert!(
            !Analyser::new()
                .analyse("return [expr {1 / 0o0}]", "tcl8.4")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == "W233")
        );
        assert!(
            Analyser::new()
                .analyse("return [expr {1 / 0o0}]", "tcl9.1")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == "W233")
        );
    }

    #[test]
    fn provably_zero_divisor_fires_w233() {
        // `$d` is the SCCP/interval constant 0 → `1 / $d` is a runtime error.
        assert_eq!(divzero("set d 0\nset x [expr {1 / $d}]"), vec!["/"]);
        assert_eq!(divzero("set d 0\nexpr {1 / $d}"), vec!["/"]);
        assert_eq!(divzero("set d 0\nexpr {5 % $d}"), vec!["%"]);
    }

    #[test]
    fn divisor_ranges_keep_literal_sigils_and_array_elements_separate() {
        assert_eq!(divzero("set {$d} 0; set d 3; expr {1 / ${$d}}"), vec!["/"]);
        assert_eq!(
            divzero("set {$d} 3; set d 0; expr {1 / ${$d}}"),
            [] as [&str; 0]
        );
        // A root and another element cannot donate a zero divisor range.
        assert_eq!(
            divzero("set a(k) 3; set a(j) 0; expr {1 / $a(k)}"),
            [] as [&str; 0]
        );
    }

    #[test]
    fn nonzero_or_guarded_divisor_is_clean() {
        // A non-zero divisor: no finding.
        assert_eq!(divzero("set d 3\nset x [expr {1 / $d}]"), [] as [&str; 0]);
        // Guarded by `$d != 0`: SCCP marks the division block unreachable.
        assert_eq!(
            divzero("set d 0\nif {$d != 0} { expr {1 / $d} }"),
            [] as [&str; 0]
        );
        // No division at all.
        assert_eq!(divzero("set x 1\nset y 2"), [] as [&str; 0]);
    }

    fn bounds(src: &str) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W230" | "W231" | "W232"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    #[test]
    fn lset_loop_counter_past_append_fires_w231() {
        // `$j ∈ [4, 8]` against a length-3 list — the interval domain proves
        // every iteration is past the append slot.  The list length is
        // recovered through the loop-header phi `lset` induces.
        let v = bounds(
            "proc f {v} {\n    set l {a b c}\n    for {set j 4} {$j < 9} {incr j} { lset l $j $v }\n}\n",
        );
        assert_eq!(v.len(), 1, "{v:?}");
        assert_eq!(v[0].0, "W231");
        assert!(v[0].1.contains("$j"), "{v:?}");
    }

    #[test]
    fn string_index_const_var_past_end_fires_w232() {
        // `$i` is the SCCP constant 10 against a 5-char string — past end.
        let source =
            "proc f {} {\n    set s \"hello\"\n    set i 10\n    return [string index $s $i]\n}\n";
        let result = bounds(source);
        assert_eq!(
            result.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>(),
            vec!["W232"]
        );
        // `$i == length` is also out of range for `string index`.
        assert_eq!(
            bounds("proc f {} { set s \"hello\"\n set i 5\n return [string index $s $i] }").len(),
            1
        );
    }

    #[test]
    fn dynamic_bounds_silent_when_not_provable() {
        // In-range index — no diagnostic.
        assert_eq!(
            bounds("proc f {} { set s \"hello\"\n set i 2\n return [string index $s $i] }").len(),
            0
        );
        // Unknown string + unknown index (both params) — not provable.
        assert_eq!(
            bounds("proc f {s i} { return [string index $s $i] }"),
            [] as [(std::string::String, std::string::String); 0]
        );
        // The legal append slot (`index == length`) for `lset` is silent.
        assert_eq!(
            bounds("proc f {v} { set l {a b c}\n set j 3\n lset l $j $v }"),
            [] as [(std::string::String, std::string::String); 0]
        );
    }

    #[test]
    fn lset_preserves_length_through_linear_chain_fires_w231() {
        // (precision gain): a length-preserving `lset` is now
        // seen through to its input, so the length survives a linear `lset`
        // chain. `lset l 99` on the (still length-3) list is a real Tcl error
        // ("list index out of range") — W231 correctly fires.
        let v = bounds("proc f {v w} { set l {a b c}\n lset l 0 $v\n set j 99\n lset l $j $w }");
        assert_eq!(v.len(), 1, "{v:?}");
        assert_eq!(v[0].0, "W231");
    }

    #[test]
    fn lset_after_length_growing_op_is_silent() {
        // (false-positive fix): a length-*growing* def
        // (`lappend`) in the loop body makes the list length unknown at the
        // `lset`, so no bound can be proven. The pre-loop length of 3 must NOT
        // be trusted — `lset l 5` is the legal append slot after two lappends.
        assert_eq!(
            bounds(
                "proc f {v} { set l {a b c}\n foreach i {1} { lappend l x y\n lset l 5 $v }\n}",
            ).len(), 0,
            "a length-growing op in the loop must poison the length, not trust the pre-loop value",
        );
        // `concat`/reassignment in the loop is equally opaque — no false W231.
        assert_eq!(
            bounds(
                "proc f {v} { set l {a b c}\n foreach i {1} { set l [concat $l x y z]\n lset l 5 $v }\n}",
            ).len(), 0,
        );
    }
}
