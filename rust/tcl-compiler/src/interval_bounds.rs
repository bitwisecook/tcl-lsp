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
//! range — guard-narrowed at the use site — *proves* the access is out of range,
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
use crate::intervals::{Interval, build_guard_index, compute_intervals_with, refine_interval};
use crate::ir::Statement;
use crate::segmenter::segment_commands_with_offset_and_config;
use crate::ssa::{Phi, SsaFunction, Symbol, ValueKey, Version};

/// `(name, version) → Phi` index over every block, for length resolution
/// through loop-header phis.
type PhiIndex<'a> = HashMap<ValueKey, &'a Phi>;

/// `(name, version) → defining statement` index, so length resolution can see
/// what produced a version it can't read directly from the length map (a
/// length-preserving `lset` vs a length-changing `lappend`/`concat`/…).
type DefIndex<'a> = HashMap<ValueKey, &'a crate::ssa::SsaStatement>;

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
fn length_preserving_lset_input(stmt: &crate::ssa::SsaStatement, sym: Symbol) -> Option<Version> {
    let Statement::Call { command, args, .. } = &stmt.statement else {
        return None;
    };
    if command != LSET || args.len() != 3 {
        return None;
    }
    let idx = args[1].trim();
    if idx.is_empty() || idx == "{}" {
        return None; // whole-list replacement — length changes
    }
    stmt.uses.get(&sym).copied()
}

/// Resolve the list length of `(name, version)` in the merge lattice.
///
/// Reads a literal-assignment length directly, follows loop-header phis, and
/// sees *through* a length-preserving `lset` to its input list. Critically, an
/// incoming that is neither a literal, a resolvable phi, nor a length-preserving
/// `lset` — a `lappend`/`linsert`/`concat`/`split` result, or a caller-supplied
/// live-in — resolves to [`Len::Unknown`] and poisons the merge. The previous
/// code ignored such an incoming on the assumption it was always an `lset`
/// result, so a length-*growing* def in the loop (`lappend l x y; lset l 5 v`)
/// left the pre-loop length trusted and fired a false W231.
fn resolve_len(
    ssa: &SsaFunction,
    name: &str,
    version: Version,
    phi_index: &PhiIndex<'_>,
    defs: &DefIndex<'_>,
    lengths: &HashMap<ValueKey, i64>,
    visited: &mut std::collections::HashSet<ValueKey>,
) -> Len {
    let Some(sym) = ssa.var_symbol(name) else {
        return Len::Unknown;
    };
    let key = (sym, version);
    if let Some(&l) = lengths.get(&key) {
        return Len::Known(l);
    }
    if !visited.insert(key) {
        // Cycle via a loop back-edge. The edge that closed the loop is resolved
        // on its forward path; revisiting it adds no new constraint.
        return Len::Neutral;
    }
    if let Some(phi) = phi_index.get(&key) {
        let mut acc = Len::Neutral;
        for &inc in phi.incoming.values() {
            acc = acc.combine(resolve_len(
                ssa, name, inc, phi_index, defs, lengths, visited,
            ));
            if acc == Len::Unknown {
                return Len::Unknown; // early poison
            }
        }
        return acc;
    }
    // Non-phi, non-literal def: only a length-preserving `lset` can be seen
    // through — resolve to the version it mutated in place. Anything else
    // (including a version-0 live-in with no def) is an unknown length.
    if let Some(stmt) = defs.get(&key)
        && let Some(input) = length_preserving_lset_input(stmt, sym)
    {
        return resolve_len(ssa, name, input, phi_index, defs, lengths, visited);
    }
    Len::Unknown
}

/// The proven list length of `(name, version)`, or `None` when it can't be
/// positively established (unknown / disagreeing).
fn resolve_list_length(
    ssa: &SsaFunction,
    name: &str,
    version: Version,
    phi_index: &PhiIndex<'_>,
    defs: &DefIndex<'_>,
    lengths: &HashMap<ValueKey, i64>,
    visited: &mut std::collections::HashSet<ValueKey>,
) -> Option<i64> {
    match resolve_len(ssa, name, version, phi_index, defs, lengths, visited) {
        Len::Known(l) => Some(l),
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
    index_arg: String,
    is_lset: bool,
}

/// The scalar variable name if `arg` is exactly `$name` / `${name}`.  Returns
/// `None` for `end`, `end-1`, `$arr(i)`, `[expr …]`, composites.
fn plain_var_name(arg: &str) -> Option<String> {
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
    if text.contains('$') || text.contains('[') {
        return None;
    }
    i64::try_from(crate::tcl_expr_eval::split_tcl_list(text, rules).len()).ok()
}

/// If a value word is exactly `[list a b c]` with no substitution / expansion,
/// its element count, else `None`.
fn list_command_length(value: &str, grammar: tcl_dialect::LexerGrammar) -> Option<i64> {
    let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?;
    // `{*}` argument expansion makes the element count unknown at this layer:
    // the segmenter strips the `{*}` prefix, so `[list {*}{a b}]` looks like a
    // single arg `"a b"` but expands to N elements (tclsh: `llength` == 2, not
    // 1). Bail rather than under-count and fire a false out-of-range warning.
    if inner.contains("{*}") {
        return None;
    }
    let cmds = segment_commands_with_offset_and_config(
        inner,
        0,
        tcl_lexer::LexerConfig::from_grammar(grammar),
    );
    let cmd = cmds.first()?;
    if cmds.len() != 1 || cmd.name() != "list" {
        return None;
    }
    let args = cmd.args();
    if args.iter().any(|a| a.contains('$') || a.contains('[')) {
        return None;
    }
    i64::try_from(args.len()).ok()
}

/// Length of list-valued SSA versions established from literal-list assignments.
fn list_length_map(
    ssa: &SsaFunction,
    grammar: tcl_dialect::LexerGrammar,
) -> HashMap<ValueKey, i64> {
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar);
    let mut lengths = HashMap::new();
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            let n = match &s.statement {
                Statement::AssignConst { value, .. } => literal_list_length(value, rules),
                Statement::AssignValue { value, .. } => list_command_length(value, grammar),
                _ => None,
            };
            if let Some(n) = n {
                for (&sym, &ver) in &s.defs {
                    lengths.insert((sym, ver), n);
                }
            }
        }
    }
    lengths
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
) -> HashMap<ValueKey, i64> {
    let mut lengths = HashMap::new();
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            let (value, needs_backsubst) = match &s.statement {
                Statement::AssignConst { value, .. } => (value, false),
                Statement::AssignValue {
                    value,
                    value_needs_backsubst,
                    ..
                } => (value, *value_needs_backsubst),
                _ => continue,
            };
            if value.contains('$') || value.contains('[') {
                continue;
            }
            let resolved = if needs_backsubst && value.contains('\\') {
                tcl_lexer::backslash_subst(value).into_owned()
            } else {
                value.clone()
            };
            if let Some(count) = StringCharacterModel::count_for(characters, &resolved)
                && let Ok(len) = i64::try_from(count)
            {
                for (&sym, &ver) in &s.defs {
                    lengths.insert((sym, ver), len);
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
fn classify(index: Interval, length: i64, is_lset: bool) -> Option<&'static str> {
    // Provably negative: the whole interval is below 0.
    if let Some(hi) = index.hi
        && hi < 0
    {
        return Some("negative");
    }
    // Provably past the end.
    if let Some(lo) = index.lo {
        if is_lset {
            if lo > length {
                return Some("past_append");
            }
        } else if lo >= length {
            return Some("past_end");
        }
    }
    None
}

/// If `text` is exactly `[lindex …]` / `[string index …]`, its candidate; else
/// `None`.  `lset` is excluded (its first arg is a var *name*).
fn parse_index_sub(text: &str, grammar: tcl_dialect::LexerGrammar) -> Option<Candidate> {
    let s = text.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let cmds = segment_commands_with_offset_and_config(
        inner,
        0,
        tcl_lexer::LexerConfig::from_grammar(grammar),
    );
    let cmd = cmds.first()?;
    let args = cmd.args();
    match cmd.name() {
        "lindex" if args.len() == 2 => Some(Candidate {
            command: LINDEX,
            list_arg: args[0].clone(),
            index_arg: args[1].clone(),
            is_lset: false,
        }),
        "string" if args.len() == 3 && args[0] == "index" => Some(Candidate {
            command: STRING_INDEX,
            list_arg: args[1].clone(),
            index_arg: args[2].clone(),
            is_lset: false,
        }),
        _ => None,
    }
}

/// Index accesses embedded as `[…]` command substitutions inside an expression,
/// restricted to *guaranteed-to-evaluate* positions (short-circuit operands and
/// non-selected ternary arms are skipped).
fn index_subs_in_expr(expr: &ExprNode, grammar: tcl_dialect::LexerGrammar) -> Vec<Candidate> {
    let mut out = Vec::new();
    walk_eager(expr, &mut |e| {
        if let ExprNode::Command { text, .. } = e
            && let Some(c) = parse_index_sub(text, grammar)
        {
            out.push(c);
        }
    });
    out
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
    // Public entry: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`walk_eager_at`]).
    walk_eager_at(expr, visit, 0);
}

fn walk_eager_at(expr: &ExprNode, visit: &mut impl FnMut(&ExprNode), depth: u32) {
    use tcl_syntax::expr::ast::BinOp;
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
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

/// All index accesses a statement performs.
fn statement_candidates(stmt: &Statement, grammar: tcl_dialect::LexerGrammar) -> Vec<Candidate> {
    let mut out = Vec::new();
    match stmt {
        Statement::Call { command, args, .. } => {
            if command == LINDEX && args.len() == 2 {
                out.push(Candidate {
                    command: LINDEX,
                    list_arg: args[0].clone(),
                    index_arg: args[1].clone(),
                    is_lset: false,
                });
            } else if command == LSET && args.len() == 3 {
                out.push(Candidate {
                    command: LSET,
                    list_arg: args[0].clone(),
                    index_arg: args[1].clone(),
                    is_lset: true,
                });
            }
            for a in args {
                if let Some(c) = parse_index_sub(a, grammar) {
                    out.push(c);
                }
            }
        }
        Statement::AssignValue { value, .. } => {
            if let Some(c) = parse_index_sub(value, grammar) {
                out.push(c);
            }
        }
        Statement::Barrier { args, .. } => {
            for a in args {
                if let Some(c) = parse_index_sub(a, grammar) {
                    out.push(c);
                }
            }
        }
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            out.extend(index_subs_in_expr(expr, grammar));
        }
        _ => {}
    }
    if let Statement::Return {
        expr: Some(expr), ..
    } = stmt
    {
        out.extend(index_subs_in_expr(expr, grammar));
    }
    out
}

/// Cheap pre-scan: any index access with a plain `$var` index?
fn has_candidate(cfg: &CfgFunction, ssa: &SsaFunction, grammar: tcl_dialect::LexerGrammar) -> bool {
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            for c in statement_candidates(&s.statement, grammar) {
                if plain_var_name(&c.index_arg).is_some() {
                    return true;
                }
            }
        }
    }
    for block in cfg.blocks.values() {
        let mut cands: Vec<Candidate> = Vec::new();
        match &block.terminator {
            Some(Terminator::Return { value, expr, .. }) => {
                if let Some(v) = value
                    && let Some(c) = parse_index_sub(v, grammar)
                {
                    cands.push(c);
                }
                if let Some(e) = expr {
                    cands.extend(index_subs_in_expr(e, grammar));
                }
            }
            Some(Terminator::Branch { condition, .. }) => {
                cands.extend(index_subs_in_expr(condition, grammar));
            }
            _ => {}
        }
        if cands.iter().any(|c| plain_var_name(&c.index_arg).is_some()) {
            return true;
        }
    }
    false
}

/// [`find_interval_bounds_with`] under the Tcl 9.0 numeral grammar.
///
/// **A dialect-blind entry point**, kept for out-of-crate consumers that do not
/// thread a dialect yet (`tcl-explorer`'s bounds view). Every in-crate caller
/// passes the document's own grammar — see
/// [`crate::intervals::compute_intervals`] for why `Tcl90` is the fallback and
/// what it costs on an 8.x target.
/// Dynamic out-of-range findings for this function (empty if none).
/// `executable` restricts to SCCP-reachable blocks.
///
/// `numbers` is the target release's numeric-literal grammar, threaded from the
/// analyser's dialect alongside `characters` (the same shape of dialect-derived
/// fact): it decides what an index literal in the guarded expression *is* —
/// `0755` is 493 up to 8.6 and 755 from 9.0 — so a version-blind read can prove
/// a range that reality never has.
#[must_use]
pub fn find_interval_bounds_with<S1, S2>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue, S1>,
    executable: &std::collections::HashSet<BlockId, S2>,
    characters: Option<StringCharacterModel>,
    numbers: NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
) -> Vec<BoundsFinding>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    if !has_candidate(cfg, ssa, grammar) {
        return Vec::new();
    }
    let ctx = BoundsCtx {
        cfg,
        ssa,
        numbers,
        intervals: compute_intervals_with(cfg, ssa, values, numbers),
        guard_index: build_guard_index(cfg, ssa, grammar),
        pred_counts: cfg
            .predecessors()
            .into_iter()
            .map(|(bid, preds)| (bid, preds.len()))
            .collect(),
        lengths: list_length_map(ssa, grammar),
        grammar,
        str_lengths: string_length_map(ssa, characters),
        phi_index: ssa
            .blocks
            .values()
            .flat_map(|sb| sb.phis.iter())
            .map(|p| ((p.name, p.version), p))
            .collect(),
        defs: ssa
            .blocks
            .values()
            .flat_map(|sb| sb.statements.iter())
            .flat_map(|s| s.defs.iter().map(move |(&sym, &ver)| ((sym, ver), s)))
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
            for cand in statement_candidates(&s.statement, grammar) {
                if let Some(span) = span {
                    let site = CandidateSite {
                        bn,
                        span,
                        version_map: &s.uses,
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

/// Read-only analysis state shared by the per-candidate bounds checks,
/// borrowed for the duration of [`find_interval_bounds`].
struct BoundsCtx<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    /// The target release's numeric-literal grammar — carried here so the
    /// guard-narrowing tables it hands [`refine_interval`] read a branch's
    /// constant bounds for the right dialect.
    numbers: NumberSyntax,
    intervals: HashMap<ValueKey, Interval>,
    guard_index: HashMap<ValueKey, Vec<BlockId>>,
    /// Predecessor count per block — used to require a guarded branch target
    /// have a single entry edge before its constraint is applied (issue 148).
    pred_counts: HashMap<BlockId, usize>,
    lengths: HashMap<ValueKey, i64>,
    str_lengths: HashMap<ValueKey, i64>,
    phi_index: PhiIndex<'a>,
    defs: DefIndex<'a>,
    /// The document dialect's lexer grammar — how a literal list word divides
    /// and where a `[…]` substitution's word boundaries fall, so a length
    /// proof matches the document's own parser.
    grammar: tcl_dialect::LexerGrammar,
}

/// The single index-access call site `process` evaluates: the versions
/// reaching it and where in the block it sits.
struct CandidateSite<'a> {
    bn: crate::cfg::BlockId,
    span: Span,
    version_map: &'a HashMap<Symbol, Version>,
    entry_versions: &'a HashMap<Symbol, Version>,
    block_stmts: &'a [crate::ssa::SsaStatement],
    stmt_idx: usize,
}

impl BoundsCtx<'_> {
    /// Resolve the list length backing `cand` at one call site, if known.
    fn length_for_list(&self, cand: &Candidate, site: &CandidateSite) -> Option<i64> {
        let ssa = self.ssa;
        let mut visited = std::collections::HashSet::new();
        if cand.is_lset {
            // `lset`'s first arg is a variable *name*, recorded as a def —
            // use the version reaching this statement.
            let lname = cand.list_arg.trim();
            if lname.contains('$') || lname.contains('[') {
                return None;
            }
            let reaching = reaching_versions(site.entry_versions, site.block_stmts, site.stmt_idx);
            let lver = *reaching.get(&ssa.var_symbol(lname)?)?;
            return resolve_list_length(
                ssa,
                lname,
                lver,
                &self.phi_index,
                &self.defs,
                &self.lengths,
                &mut visited,
            );
        }
        // A *value* arg: literal list, or `$l`.
        if let Some(lit) = literal_list_length(
            &cand.list_arg,
            tcl_syntax::word_rules::WordValueRules::from_grammar(&self.grammar),
        ) {
            return Some(lit);
        }
        let list_name = plain_var_name(&cand.list_arg)?;
        let list_version = *site.version_map.get(&ssa.var_symbol(&list_name)?)?;
        resolve_list_length(
            ssa,
            &list_name,
            list_version,
            &self.phi_index,
            &self.defs,
            &self.lengths,
            &mut visited,
        )
    }

    /// Evaluate one candidate index access; push a finding when the index
    /// interval is provably out of range for the resolved length.
    fn process(&self, cand: &Candidate, site: &CandidateSite, findings: &mut Vec<BoundsFinding>) {
        let ssa = self.ssa;
        let Some(index_var) = plain_var_name(&cand.index_arg) else {
            return;
        };
        let Some(index_sym) = ssa.var_symbol(&index_var) else {
            return;
        };
        let Some(&index_version) = site.version_map.get(&index_sym) else {
            return;
        };
        if index_version == 0 {
            return;
        }
        let length = if cand.command == STRING_INDEX {
            let str_var = plain_var_name(&cand.list_arg);
            str_var.and_then(|sv| {
                ssa.var_symbol(&sv)
                    .and_then(|str_sym| site.version_map.get(&str_sym).map(|&v| (str_sym, v)))
                    .and_then(|(str_sym, v)| self.str_lengths.get(&(str_sym, v)).copied())
            })
        } else {
            self.length_for_list(cand, site)
        };
        let Some(length) = length else {
            return;
        };
        let iv = refine_interval(
            &self.intervals,
            self.cfg,
            ssa,
            site.bn,
            &index_var,
            index_version,
            crate::intervals::GuardTables {
                guard_index: &self.guard_index,
                pred_counts: &self.pred_counts,
                numbers: self.numbers,
            },
        );
        if iv.is_top() || iv.is_bottom() {
            return;
        }
        let Some(reason) = classify(iv, length, cand.is_lset) else {
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
            reason: reason.to_owned(),
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
            version_map: &sb.exit_versions,
            entry_versions: &sb.exit_versions,
            block_stmts: &sb.statements,
            stmt_idx: sb.statements.len(),
        };
        match &block.terminator {
            Some(Terminator::Return {
                value, expr, span, ..
            }) => {
                let Some(span) = span else { return };
                if let Some(v) = value
                    && let Some(cand) = parse_index_sub(v, self.grammar)
                {
                    self.process(&cand, &exit_site(*span), findings);
                }
                if let Some(e) = expr {
                    for cand in index_subs_in_expr(e, self.grammar) {
                        self.process(&cand, &exit_site(*span), findings);
                    }
                }
            }
            Some(Terminator::Branch {
                condition, span, ..
            }) => {
                let Some(span) = span else { return };
                for cand in index_subs_in_expr(condition, self.grammar) {
                    self.process(&cand, &exit_site(*span), findings);
                }
            }
            _ => {}
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
    env: &HashMap<String, Interval>,
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
            let iv = crate::intervals::eval_expr(right, env, numbers);
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

/// [`find_divide_by_zero_with`] under the Tcl 9.0 numeral grammar.
///
/// **A dialect-blind entry point**, kept for out-of-crate consumers that do not
/// thread a dialect yet (`tcl-explorer`'s divide-by-zero view). Every in-crate
/// caller passes the document's own grammar — see
/// [`crate::intervals::compute_intervals`] for why `Tcl90` is the fallback.
/// Divisions / modulo whose divisor is provably `[0, 0]` (a runtime error).
///
/// Sound: the divisor's interval (guard-narrowed at the use site) must be
/// exactly `[0, 0]`, and the block must be SCCP-executable. Shares the same
/// interval machinery (`compute_intervals_with` / `refine_interval` /
/// `eval_expr`) as [`find_interval_bounds_with`], including its `numbers`
/// numeral grammar — a divisor literal is read for the target release, so a
/// spelling that is not a numeral there (`0o0` under 8.4) proves nothing.
/// Findings are returned in source-span order for deterministic output.
#[must_use]
pub fn find_divide_by_zero_with<S1, S2>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue, S1>,
    executable: &std::collections::HashSet<BlockId, S2>,
    numbers: NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
) -> Vec<DivZeroFinding>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    if !has_division(cfg, ssa) {
        return Vec::new();
    }
    let intervals = compute_intervals_with(cfg, ssa, values, numbers);
    let guard_index = build_guard_index(cfg, ssa, grammar);
    let pred_counts: HashMap<BlockId, usize> = cfg
        .predecessors()
        .into_iter()
        .map(|(bid, preds)| (bid, preds.len()))
        .collect();

    let env_for =
        |uses: &HashMap<Symbol, Version>, bn: crate::cfg::BlockId| -> HashMap<String, Interval> {
            uses.iter()
                .filter(|&(_, &ver)| ver > 0)
                .map(|(&sym, &ver)| {
                    let name = ssa.var_name(sym);
                    (
                        name.to_owned(),
                        refine_interval(
                            &intervals,
                            cfg,
                            ssa,
                            bn,
                            name,
                            ver,
                            crate::intervals::GuardTables {
                                guard_index: &guard_index,
                                pred_counts: &pred_counts,
                                numbers,
                            },
                        ),
                    )
                })
                .collect()
        };

    let mut findings: Vec<DivZeroFinding> = Vec::new();
    for (bid, sb) in &ssa.blocks {
        if !executable.contains(bid) {
            continue;
        }
        let bn = *bid;
        for s in &sb.statements {
            if let Some(expr) = statement_expr(&s.statement) {
                let span = statement_span(&s.statement).unwrap_or_else(|| Span::new(0, 0));
                collect_divzero(expr, span, &env_for(&s.uses, bn), numbers, &mut findings);
            }
        }
        let Some(block) = cfg.blocks.get(bid) else {
            continue;
        };
        match &block.terminator {
            Some(Terminator::Branch {
                condition,
                span: Some(span),
                ..
            }) => collect_divzero(
                condition,
                *span,
                &env_for(&sb.exit_versions, bn),
                numbers,
                &mut findings,
            ),
            Some(Terminator::Return {
                expr: Some(e),
                span: Some(span),
                ..
            }) => collect_divzero(
                e,
                *span,
                &env_for(&sb.exit_versions, bn),
                numbers,
                &mut findings,
            ),
            _ => {}
        }
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

    /// Regression coverage for issue #996: `walk_eager` recurses once per
    /// `ExprNode` level with no depth cap before this fix. A tree built
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
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = &cu.top_level;
        super::find_divide_by_zero_with(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &fu.sccp.executable_blocks,
            tcl_dialect::NumberSyntax::default(),
            tcl_dialect::LexerGrammar::default(),
        )
        .iter()
        .map(|d| d.op)
        .collect()
    }

    #[test]
    fn provably_zero_divisor_fires_w233() {
        // `$d` is the SCCP/interval constant 0 → `1 / $d` is a runtime error.
        assert_eq!(divzero("set d 0\nset x [expr {1 / $d}]"), vec!["/"]);
        assert_eq!(divzero("set d 0\nexpr {1 / $d}"), vec!["/"]);
        assert_eq!(divzero("set d 0\nexpr {5 % $d}"), vec!["%"]);
    }

    #[test]
    fn nonzero_or_guarded_divisor_is_clean() {
        // A non-zero divisor: no finding.
        assert!(divzero("set d 3\nset x [expr {1 / $d}]").is_empty());
        // Guarded by `$d != 0`: SCCP marks the division block unreachable.
        assert!(divzero("set d 0\nif {$d != 0} { expr {1 / $d} }").is_empty());
        // No division at all.
        assert!(divzero("set x 1\nset y 2").is_empty());
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
        assert_eq!(
            bounds("proc f {} {\n    set s \"hello\"\n    set i 10\n    return [string index $s $i]\n}\n")
                .iter()
                .map(|(c, _)| c.clone())
                .collect::<Vec<_>>(),
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
        assert!(
            bounds("proc f {} { set s \"hello\"\n set i 2\n return [string index $s $i] }")
                .is_empty()
        );
        // Unknown string + unknown index (both params) — not provable.
        assert!(bounds("proc f {s i} { return [string index $s $i] }").is_empty());
        // The legal append slot (`index == length`) for `lset` is silent.
        assert!(bounds("proc f {v} { set l {a b c}\n set j 3\n lset l $j $v }").is_empty());
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
        assert!(
            bounds(
                "proc f {v} { set l {a b c}\n foreach i {1} { lappend l x y\n lset l 5 $v }\n}",
            )
            .is_empty(),
            "a length-growing op in the loop must poison the length, not trust the pre-loop value",
        );
        // `concat`/reassignment in the loop is equally opaque — no false W231.
        assert!(
            bounds(
                "proc f {v} { set l {a b c}\n foreach i {1} { set l [concat $l x y z]\n lset l 5 $v }\n}",
            )
            .is_empty(),
        );
    }
}
