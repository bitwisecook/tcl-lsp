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
//! range — never on "might be out of range".  Port of the bounds-finding portion
//! of `compiler/interval_bounds.py::find_interval_bounds`.

use std::collections::HashMap;

use tcl_lexer::Span;
use tcl_syntax::expr::ast::ExprNode;

use crate::analyses::LatticeValue;
use crate::cfg::{Function as CfgFunction, Terminator};
use crate::intervals::{Interval, build_guard_index, compute_intervals, refine_interval};
use crate::ir::Statement;
use crate::segmenter::segment_commands;
use crate::ssa::{Phi, SsaFunction, ValueKey, Version};

/// `(name, version) → Phi` index over every block, for length resolution
/// through loop-header phis.
type PhiIndex<'a> = HashMap<ValueKey, &'a Phi>;

/// Resolve the list length of `(name, version)`, following phis when the
/// version itself was not seeded by a literal-list assignment.
///
/// Rust's SSA inserts a loop-header phi for a list `l` that `lset` mutates
/// (`l_h = phi(l_entry, l_body)`); the body's `lset` then reads `l_h`, which is
/// not in the length map.  Python's pruned SSA never inserts that phi (its
/// `lset` does not *read* `l`, so `l` is not live across the back-edge), leaving
/// `l` at its literal-assignment version.  To match Python's *result*, resolve a
/// phi to the length its known incomings agree on, ignoring an unknown back-edge
/// incoming (the `lset` result — `lset` preserves length under the
/// assume-no-error model the diagnostic already encodes).  A genuine
/// disagreement (two known but different lengths) yields `None` (sound).
fn resolve_list_length(
    name: &str,
    version: Version,
    phi_index: &PhiIndex<'_>,
    lengths: &HashMap<ValueKey, i64>,
    visited: &mut std::collections::HashSet<ValueKey>,
) -> Option<i64> {
    let key = (name.to_owned(), version);
    if let Some(&l) = lengths.get(&key) {
        return Some(l);
    }
    if !visited.insert(key.clone()) {
        return None; // cycle (loop back-edge) — give up this path
    }
    let phi = phi_index.get(&key)?;
    let mut found: Option<i64> = None;
    for &inc in phi.incoming.values() {
        if inc == 0 {
            continue;
        }
        if let Some(l) = resolve_list_length(name, inc, phi_index, lengths, visited) {
            match found {
                None => found = Some(l),
                Some(f) if f == l => {}
                Some(_) => return None, // disagreement → unknown
            }
        }
    }
    found
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
    pub code: String,
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
/// `None` for `end`, `end-1`, `$arr(i)`, `[expr …]`, composites.  Mirrors
/// `_plain_var_name`.
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
/// Mirrors `_literal_list_length`.
fn literal_list_length(text: &str) -> Option<i64> {
    if text.contains('$') || text.contains('[') {
        return None;
    }
    i64::try_from(crate::tcl_expr_eval::split_tcl_list(text).len()).ok()
}

/// If a value word is exactly `[list a b c]` with no substitution / expansion,
/// its element count, else `None`.  Replaces Python's intent-driven `[list …]`
/// detection in `_list_length_map`.
fn list_command_length(value: &str) -> Option<i64> {
    let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?;
    let cmds = segment_commands(inner);
    let cmd = cmds.first()?;
    if cmds.len() != 1 || cmd.name() != "list" {
        return None;
    }
    let args = cmd.args();
    if args
        .iter()
        .any(|a| a.contains('$') || a.contains('[') || a.starts_with("{*}"))
    {
        return None;
    }
    i64::try_from(args.len()).ok()
}

/// Length of list-valued SSA versions established from literal-list assignments.
/// Mirrors `_list_length_map`.
fn list_length_map(ssa: &SsaFunction) -> HashMap<ValueKey, i64> {
    let mut lengths = HashMap::new();
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            let n = match &s.statement {
                Statement::AssignConst { value, .. } => literal_list_length(value),
                Statement::AssignValue { value, .. } => list_command_length(value),
                _ => None,
            };
            if let Some(n) = n {
                for (name, &ver) in &s.defs {
                    lengths.insert((name.clone(), ver), n);
                }
            }
        }
    }
    lengths
}

/// Character length of string-valued SSA versions from literal assignments.
/// Mirrors `_string_length_map`.
fn string_length_map(ssa: &SsaFunction) -> HashMap<ValueKey, i64> {
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
            if let Ok(len) = i64::try_from(resolved.chars().count()) {
                for (name, &ver) in &s.defs {
                    lengths.insert((name.clone(), ver), len);
                }
            }
        }
    }
    lengths
}

/// Versions of each name reaching statement index `upto` within a block.  Used
/// for `lset`, whose target list is a *def* (not a use).  Mirrors
/// `_reaching_versions`.
fn reaching_versions(
    entry: &HashMap<String, Version>,
    stmts: &[crate::ssa::SsaStatement],
    upto: usize,
) -> HashMap<String, Version> {
    let mut cur = entry.clone();
    for s in stmts.iter().take(upto) {
        for (name, &ver) in &s.defs {
            cur.insert(name.clone(), ver);
        }
    }
    cur
}

/// Reason string if `index` is *wholly* out of range for `length`, else `None`.
/// `lset` permits the append slot (`index == length`); `lindex` does not.
/// Mirrors `_classify`.
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
/// `None`.  `lset` is excluded (its first arg is a var *name*).  Mirrors
/// `_parse_index_sub`.
fn parse_index_sub(text: &str) -> Option<Candidate> {
    let s = text.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let cmds = segment_commands(inner);
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
/// non-selected ternary arms are skipped).  Mirrors `_index_subs_in_expr` over
/// `_walk_eager`.
fn index_subs_in_expr(expr: &ExprNode) -> Vec<Candidate> {
    let mut out = Vec::new();
    walk_eager(expr, &mut |e| {
        if let ExprNode::Command { text, .. } = e
            && let Some(c) = parse_index_sub(text)
        {
            out.push(c);
        }
    });
    out
}

/// Constant truthiness of a literal expression node (`Some(true/false)`), else
/// `None`.  Mirrors `_const_bool` for the eager walk's guard resolution.
fn const_bool(expr: &ExprNode) -> Option<bool> {
    use tcl_syntax::expr::ast::UnaryOp;
    // Fold the boolean-relevant unaries over a constant operand, matching
    // Python's `expr_ast._const_bool`: `+`/`-` preserve zero-ness (`-1` is
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
/// arm run only when forced by a *constant* guard.  Mirrors `_walk_eager`.
fn walk_eager(expr: &ExprNode, visit: &mut impl FnMut(&ExprNode)) {
    use tcl_syntax::expr::ast::BinOp;
    visit(expr);
    match expr {
        ExprNode::Binary { op, left, right } => {
            walk_eager(left, visit);
            let lazy = matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr);
            if !lazy {
                walk_eager(right, visit);
                return;
            }
            let Some(guard) = const_bool(left) else {
                return; // maybe-dead RHS — leave it skipped
            };
            let is_and = matches!(op, BinOp::And | BinOp::WordAnd);
            let forced = if is_and { guard } else { !guard };
            if forced {
                walk_eager(right, visit);
            }
        }
        ExprNode::Unary { operand, .. } => walk_eager(operand, visit),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_eager(condition, visit);
            match const_bool(condition) {
                Some(true) => walk_eager(true_branch, visit),
                Some(false) => walk_eager(false_branch, visit),
                None => {}
            }
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                walk_eager(a, visit);
            }
        }
        _ => {}
    }
}

/// All index accesses a statement performs.  Mirrors `_statement_candidates`.
fn statement_candidates(stmt: &Statement) -> Vec<Candidate> {
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
                if let Some(c) = parse_index_sub(a) {
                    out.push(c);
                }
            }
        }
        Statement::AssignValue { value, .. } => {
            if let Some(c) = parse_index_sub(value) {
                out.push(c);
            }
        }
        Statement::Barrier { args, .. } => {
            for a in args {
                if let Some(c) = parse_index_sub(a) {
                    out.push(c);
                }
            }
        }
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            out.extend(index_subs_in_expr(expr));
        }
        _ => {}
    }
    if let Statement::Return {
        expr: Some(expr), ..
    } = stmt
    {
        out.extend(index_subs_in_expr(expr));
    }
    out
}

/// Cheap pre-scan: any index access with a plain `$var` index?  Mirrors
/// `_has_candidate`.
fn has_candidate(cfg: &CfgFunction, ssa: &SsaFunction) -> bool {
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            for c in statement_candidates(&s.statement) {
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
                    && let Some(c) = parse_index_sub(v)
                {
                    cands.push(c);
                }
                if let Some(e) = expr {
                    cands.extend(index_subs_in_expr(e));
                }
            }
            Some(Terminator::Branch { condition, .. }) => {
                cands.extend(index_subs_in_expr(condition));
            }
            _ => {}
        }
        if cands.iter().any(|c| plain_var_name(&c.index_arg).is_some()) {
            return true;
        }
    }
    false
}

/// Dynamic out-of-range findings for this function (empty if none).  Mirrors
/// `find_interval_bounds`; `executable` restricts to SCCP-reachable blocks.
#[must_use]
#[allow(clippy::too_many_lines, clippy::similar_names, clippy::implicit_hasher)]
pub fn find_interval_bounds(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable: &std::collections::HashSet<String>,
) -> Vec<BoundsFinding> {
    if !has_candidate(cfg, ssa) {
        return Vec::new();
    }
    let intervals = compute_intervals(cfg, ssa, values);
    let guard_index = build_guard_index(cfg, ssa);
    let lengths = list_length_map(ssa);
    let str_lengths = string_length_map(ssa);
    let phi_index: PhiIndex = ssa
        .blocks
        .values()
        .flat_map(|sb| sb.phis.iter())
        .map(|p| ((p.name.clone(), p.version), p))
        .collect();
    let mut findings = Vec::new();

    let length_for_list = |cand: &Candidate,
                           version_map: &HashMap<String, Version>,
                           entry_versions: &HashMap<String, Version>,
                           block_stmts: &[crate::ssa::SsaStatement],
                           stmt_idx: usize|
     -> Option<i64> {
        let mut visited = std::collections::HashSet::new();
        if cand.is_lset {
            // `lset`'s first arg is a variable *name*, recorded as a def —
            // use the version reaching this statement.
            let lname = cand.list_arg.trim();
            if lname.contains('$') || lname.contains('[') {
                return None;
            }
            let reaching = reaching_versions(entry_versions, block_stmts, stmt_idx);
            let lver = *reaching.get(lname)?;
            return resolve_list_length(lname, lver, &phi_index, &lengths, &mut visited);
        }
        // A *value* arg: literal list, or `$l`.
        if let Some(lit) = literal_list_length(&cand.list_arg) {
            return Some(lit);
        }
        let lvar = plain_var_name(&cand.list_arg)?;
        let lver = *version_map.get(&lvar)?;
        resolve_list_length(&lvar, lver, &phi_index, &lengths, &mut visited)
    };

    let process = |cand: &Candidate,
                   bn: &str,
                   span: Span,
                   version_map: &HashMap<String, Version>,
                   entry_versions: &HashMap<String, Version>,
                   block_stmts: &[crate::ssa::SsaStatement],
                   stmt_idx: usize,
                   findings: &mut Vec<BoundsFinding>| {
        let Some(ivar) = plain_var_name(&cand.index_arg) else {
            return;
        };
        let Some(&iver) = version_map.get(&ivar) else {
            return;
        };
        if iver == 0 {
            return;
        }
        let length = if cand.command == STRING_INDEX {
            let svar = plain_var_name(&cand.list_arg);
            svar.and_then(|sv| {
                version_map
                    .get(&sv)
                    .and_then(|&v| str_lengths.get(&(sv.clone(), v)).copied())
            })
        } else {
            length_for_list(cand, version_map, entry_versions, block_stmts, stmt_idx)
        };
        let Some(length) = length else {
            return;
        };
        let iv = refine_interval(&intervals, cfg, ssa, bn, &ivar, iver, &guard_index);
        if iv.is_top() || iv.is_bottom() {
            return;
        }
        let Some(reason) = classify(iv, length, cand.is_lset) else {
            return;
        };
        let code = if cand.is_lset {
            "W231"
        } else if cand.command == STRING_INDEX {
            "W232"
        } else {
            "W230"
        };
        findings.push(BoundsFinding {
            span,
            code: code.to_owned(),
            command: cand.command.to_owned(),
            index_var: ivar,
            index_interval: iv,
            length,
            reason: reason.to_owned(),
        });
    };

    for (bn, sb) in &ssa.blocks {
        if !executable.contains(bn) {
            continue;
        }
        for (idx, s) in sb.statements.iter().enumerate() {
            let span = statement_span(&s.statement);
            for cand in statement_candidates(&s.statement) {
                if let Some(span) = span {
                    process(
                        &cand,
                        bn,
                        span,
                        &s.uses,
                        &sb.entry_versions,
                        &sb.statements,
                        idx,
                        &mut findings,
                    );
                }
            }
        }
        // Index accesses in a `return [...]` value / branch condition: the read
        // versions are the block's exit versions; anchor on the terminator.
        let Some(block) = cfg.blocks.get(bn) else {
            continue;
        };
        match &block.terminator {
            Some(Terminator::Return {
                value, expr, span, ..
            }) => {
                let Some(span) = span else { continue };
                if let Some(v) = value
                    && let Some(cand) = parse_index_sub(v)
                {
                    process(
                        &cand,
                        bn,
                        *span,
                        &sb.exit_versions,
                        &sb.exit_versions,
                        &sb.statements,
                        sb.statements.len(),
                        &mut findings,
                    );
                }
                if let Some(e) = expr {
                    for cand in index_subs_in_expr(e) {
                        process(
                            &cand,
                            bn,
                            *span,
                            &sb.exit_versions,
                            &sb.exit_versions,
                            &sb.statements,
                            sb.statements.len(),
                            &mut findings,
                        );
                    }
                }
            }
            Some(Terminator::Branch {
                condition, span, ..
            }) => {
                let Some(span) = span else { continue };
                for cand in index_subs_in_expr(condition) {
                    process(
                        &cand,
                        bn,
                        *span,
                        &sb.exit_versions,
                        &sb.exit_versions,
                        &sb.statements,
                        sb.statements.len(),
                        &mut findings,
                    );
                }
            }
            _ => {}
        }
    }
    findings
}

/// A divide-by-zero finding: a `/` or `%` whose divisor is provably the
/// single point `[0, 0]` on the always-evaluated spine of an executable
/// expression. Mirrors `interval_bounds.DivZeroFinding`.
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
/// Cheap pre-scan helper; mirrors the body of Python's `_divisors` test.
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
/// so a guarded `1/$d` never yields a finding (mirrors Python's
/// `_divisors` + `check`). Only owned `Copy`/`'static` data escapes the
/// walk closure.
fn collect_divzero(
    expr: &ExprNode,
    span: Span,
    env: &HashMap<String, Interval>,
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
            let iv = crate::intervals::eval_expr(right, env);
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
/// Sound: the divisor's interval (guard-narrowed at the use site) must be
/// exactly `[0, 0]`, and the block must be SCCP-executable. Shares the same
/// interval machinery (`compute_intervals` / `refine_interval` / `eval_expr`)
/// as [`find_interval_bounds`]. Findings are returned in source-span order
/// for deterministic output.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn find_divide_by_zero(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable: &std::collections::HashSet<String>,
) -> Vec<DivZeroFinding> {
    if !has_division(cfg, ssa) {
        return Vec::new();
    }
    let intervals = compute_intervals(cfg, ssa, values);
    let guard_index = build_guard_index(cfg, ssa);

    let env_for = |uses: &HashMap<String, Version>, bn: &str| -> HashMap<String, Interval> {
        uses.iter()
            .filter(|&(_, &ver)| ver > 0)
            .map(|(name, &ver)| {
                (
                    name.clone(),
                    refine_interval(&intervals, cfg, ssa, bn, name, ver, &guard_index),
                )
            })
            .collect()
    };

    let mut findings: Vec<DivZeroFinding> = Vec::new();
    for (bn, sb) in &ssa.blocks {
        if !executable.contains(bn) {
            continue;
        }
        for s in &sb.statements {
            if let Some(expr) = statement_expr(&s.statement) {
                let span = statement_span(&s.statement).unwrap_or_else(|| Span::new(0, 0));
                collect_divzero(expr, span, &env_for(&s.uses, bn), &mut findings);
            }
        }
        let Some(block) = cfg.blocks.get(bn) else {
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
                &mut findings,
            ),
            Some(Terminator::Return {
                expr: Some(e),
                span: Some(span),
                ..
            }) => collect_divzero(e, *span, &env_for(&sb.exit_versions, bn), &mut findings),
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

    /// The `op` of every W233 divide-by-zero finding for `src`'s top level.
    fn divzero(src: &str) -> Vec<&'static str> {
        use crate::compilation_unit::CompilationUnit;
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = &cu.top_level;
        super::find_divide_by_zero(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &fu.sccp.executable_blocks,
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
            .map(|d| (d.code.clone(), d.message.clone()))
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
        // A second `lset` reads a non-phi mutated version — length not
        // recovered (matches Python's pruned SSA), so no false positive.
        assert!(
            bounds("proc f {v w} { set l {a b c}\n lset l 0 $v\n set j 99\n lset l $j $w }")
                .is_empty()
        );
    }
}
