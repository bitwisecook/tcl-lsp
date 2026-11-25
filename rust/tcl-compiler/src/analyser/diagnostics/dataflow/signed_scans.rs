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

//! W156: a value `binary scan` or `scan` stores as a signed integer, compared with a constant outside the signed range.
//!
//! Every release stores `binary scan` integer fields signed unless they carry
//! the `u` suffix, which Tcl 8.4 (and so iRules) lacks: `binary scan \x81 c b`
//! stores `-127`. Tcl 8.4, 9.0 and 9.1 also truncate `scan` integer
//! conversions without an `l` size to signed 32 bits (`scan ffffffff %x v`
//! stores `-1`); 8.5, 8.6 and Jim do not. A byte compared with `0x81`
//! therefore never matches. The check follows the scanned SSA version
//! (through phis whose every input is the same kind of scan) to an `==`,
//! `!=`, `<`, `<=`, `>`, `>=`, `eq` or `ne` against an integer literal.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity, SignedDataGroupComparison};
use crate::cfg::Terminator;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::ssa::{SsaSourceView, ValueKey};
use tcl_core_types::DiagCode;
use tcl_registry::ArgRole;

/// Where a signed integer came from.
#[derive(Clone, PartialEq, Eq)]
struct SignedOrigin {
    bits: u32,
    field: String,
    /// The scanning call as written: its command and, for an ensemble, the
    /// subcommand word (`binary scan`, `scan`).
    command: String,
    /// Whether the field is a `scan` conversion rather than a binary field.
    conversion: bool,
    /// The same field read unsigned or at full width, where the dialect has
    /// a spelling for it (`cu`, `%lx`).
    unsigned_field: Option<String>,
}

impl SignedOrigin {
    fn range(&self) -> (i64, i64) {
        let half = 1i64 << (self.bits - 1);
        (-half, half - 1)
    }
}

struct Comparison<'a> {
    op: BinOp,
    variable: &'a ExprNode,
    literal: i64,
    literal_text: &'a str,
    variable_left: bool,
}

impl Analyser {
    /// W156 over one function body.
    pub(in super::super) fn emit_signed_scan_comparisons(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        let context = self.analysis_context();
        let registry = context.commands();
        let signed = signed_scan_values(fu, registry, self.profile);
        let unresolved = unresolved_scans(fu, registry, self.profile);
        if signed.is_empty() && unresolved.is_empty() {
            return;
        }
        let numbers = crate::intervals::numbers_for_dialect(Some(self.profile));
        let mut findings = Vec::new();
        let mut check =
            |expr: &ExprNode,
             view: SsaSourceView<'_>,
             exit: Option<&ExitVersions>,
             (block, index, base, guard): (crate::cfg::BlockId, usize, Option<u32>, &[String]),
             fallback| {
                let used = |symbol: &crate::ssa::Symbol| {
                    fu.ssa
                        .blocks
                        .get(&block)
                        .and_then(|data| data.statements.get(index))
                        .and_then(|statement| statement.uses.get(symbol).copied())
                };
                let resolve = |variable: &ExprNode| {
                    view.read_expression_variable(variable, base)
                        .and_then(|read| {
                            let version = read
                                .version
                                .or_else(|| exit.and_then(|exit| exit.get(&read.symbol).copied()))
                                .or_else(|| used(&read.symbol))?;
                            signed.get(&(read.symbol, version))
                        })
                        .or_else(|| straight_line_scan(fu, &unresolved, variable, block, index))
                        .filter(|_| {
                            !matches!(variable, ExprNode::Var { name, .. }
                            if mentions_bare(guard, name))
                        })
                };
                let locate = |variable: &ExprNode| match (base, variable) {
                    (Some(base), ExprNode::Var { start, end, .. }) => {
                        tcl_lexer::Span::new(base + start, base + end + 1)
                    }
                    _ => fallback,
                };
                for (variable, composition) in compositions(expr, numbers) {
                    if let Some(origin) = resolve(variable) {
                        findings.push(signed_scan_warning(
                            fu.abs_span(locate(variable)),
                            composition.message(variable, origin),
                        ));
                    }
                }
                for comparison in comparisons(expr, numbers) {
                    let Some(origin) = resolve(comparison.variable) else {
                        continue;
                    };
                    let Some(message) = message(&comparison, origin) else {
                        continue;
                    };
                    let span = match (base, comparison.variable) {
                        (Some(base), ExprNode::Var { start, end, .. }) => {
                            tcl_lexer::Span::new(base + start, base + end + 1)
                        }
                        _ => fallback,
                    };
                    findings.push(signed_scan_warning(fu.abs_span(span), message));
                }
            };
        let parser_context = self.original_expression_parser_context();
        let parse = |text: &str| {
            let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
                text,
                parser_context.as_ref()?,
            );
            (!matches!(parsed, ExprNode::Raw { .. })).then_some(parsed)
        };
        for_each_expression(fu, self.file_lexer_config(), (registry, &parse), &mut check);
        findings.extend(call_consumers(
            fu,
            (registry, Some(context.as_ref().into())),
            self.file_lexer_config(),
            (&signed, &unresolved),
            &ConfigurationSource {
                source: &self.source,
                analysis: &self.result,
                context: context.as_ref(),
            },
        ));
        findings.sort_by_key(|finding| (finding.span.start(), finding.span.end()));
        findings.dedup();
        self.result.diagnostics.extend(findings);
    }
}

type ExitVersions = HashMap<crate::ssa::Symbol, crate::ssa::Version>;

/// Parses expression text under the document's grammar; `None` when it does
/// not parse.
type ExpressionParser<'a> = dyn Fn(&str) -> Option<ExprNode> + 'a;

/// Every expression the body evaluates: lowered `expr`/`return` operands,
/// `[expr …]` substitutions inside other commands, and branch conditions,
/// each with its SSA view, the block's exit versions for a condition, its
/// source base and a fallback span.
fn for_each_expression(
    fu: &crate::compilation_unit::FunctionUnit,
    config: tcl_lexer::LexerConfig,
    (registry, parse): (&tcl_registry::CommandRegistry, &ExpressionParser<'_>),
    visit: &mut impl FnMut(
        &ExprNode,
        SsaSourceView<'_>,
        Option<&ExitVersions>,
        (crate::cfg::BlockId, usize, Option<u32>, &[String]),
        tcl_lexer::Span,
    ),
) {
    for (&block, data) in &fu.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&fu.ssa, block, index);
            match &statement.statement {
                Statement::AssignExpr {
                    expr,
                    expr_base,
                    span,
                    ..
                }
                | Statement::ExprEval {
                    expr,
                    expr_base,
                    span,
                    ..
                }
                | Statement::Return {
                    expr: Some(expr),
                    expr_base,
                    span,
                    ..
                } => visit(expr, view, None, (block, index, *expr_base, &[]), *span),
                _ => {
                    let entered = crate::word_subst::entered_expression_evaluations(
                        view.source_tokens(),
                        config,
                        registry,
                    );
                    if entered.is_empty() {
                        for (expr, base, span, guard) in
                            unlowered_expressions(&statement.statement, config, (registry, parse))
                        {
                            visit(&expr, view, None, (block, index, Some(base), &guard), span);
                        }
                    }
                    for evaluation in entered {
                        visit(
                            evaluation.expression(),
                            view,
                            None,
                            (block, index, Some(evaluation.expression_base()), &[]),
                            evaluation.span(),
                        );
                    }
                }
            }
        }
        if let Some(Terminator::Branch {
            condition,
            span: Some(span),
            condition_base,
            ..
        }) = fu
            .cfg
            .blocks
            .get(&block)
            .and_then(|b| b.terminator.as_ref())
        {
            visit(
                condition,
                SsaSourceView::at_terminator(&fu.ssa, block),
                Some(&data.exit_versions),
                (block, data.statements.len(), *condition_base, &[]),
                *span,
            );
        }
    }
}

/// Braced expression arguments of a call the module did not lower — in a
/// procedure body, a command after one the module cannot prove (it may have
/// renamed `if`, `while` or `expr`) stays a plain call — and of every `[…]`
/// call its words make, parsed under the document's expression grammar, with
/// each expression's absolute source base. The registry's `Expr` role picks
/// the arguments; W156 then reads them as the commands they normally are.
fn unlowered_expressions(
    statement: &Statement,
    config: tcl_lexer::LexerConfig,
    (registry, parse): (&tcl_registry::CommandRegistry, &ExpressionParser<'_>),
) -> Vec<(ExprNode, u32, tcl_lexer::Span, Vec<String>)> {
    let Statement::Call {
        command,
        canonical_command,
        args,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        return Vec::new();
    };
    let mut calls: Vec<(String, Vec<String>, Vec<tcl_lexer::Span>)> = vec![(
        canonical_command.as_deref().unwrap_or(command).to_owned(),
        args.clone(),
        tokens.argv.iter().skip(1).copied().collect(),
    )];
    for lifted in crate::word_subst::lifted_calls(Some(tokens), config) {
        calls.push((lifted.command, lifted.args, lifted.arg_spans));
    }
    let mut out = Vec::new();
    let mut walk = UnloweredWalk {
        config,
        registry,
        parse,
        guard: Vec::new(),
        out: &mut out,
    };
    for (name, args, spans) in &calls {
        walk.call(name, args, spans, 0);
    }
    out
}

/// The content of a braced word and the offset of its first content byte.
/// A statement's IR keeps a braced word's content, whose span runs from `{`
/// to the last inner character (or past the closer); a lifted or segmented
/// word keeps the braces and spans them whole.
fn braced_content(word: &str, span: tcl_lexer::Span) -> Option<(&str, u32)> {
    let length = usize::try_from(span.end() - span.start()).ok();
    match word.strip_prefix('{').and_then(|w| w.strip_suffix('}')) {
        Some(inner) if length == Some(word.len()) => Some((inner, span.start() + 1)),
        _ if length == Some(word.len() + 1) || length == Some(word.len() + 2) => {
            Some((word, span.start() + 1))
        }
        _ => None,
    }
}

/// Walks unlowered calls and the bodies they hold, collecting their braced
/// expressions. `guard` holds the words of every earlier command in the
/// bodies entered, so a name one of them writes is not trusted after it.
struct UnloweredWalk<'a, 'o> {
    config: tcl_lexer::LexerConfig,
    registry: &'a tcl_registry::CommandRegistry,
    parse: &'a ExpressionParser<'a>,
    guard: Vec<String>,
    out: &'o mut Vec<(ExprNode, u32, tcl_lexer::Span, Vec<String>)>,
}

impl UnloweredWalk<'_, '_> {
    const MAX_DEPTH: u32 = 16;

    fn call(&mut self, name: &str, args: &[String], spans: &[tcl_lexer::Span], depth: u32) {
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        for index in self
            .registry
            .arg_indices_for_role(name, &words, ArgRole::Expr)
        {
            let (Some(word), Some(&span)) = (args.get(index), spans.get(index)) else {
                continue;
            };
            let Some((content, base)) = braced_content(word, span) else {
                continue;
            };
            if let Some(expr) = (self.parse)(content) {
                self.out.push((expr, base, span, self.guard.clone()));
            }
        }
        if depth >= Self::MAX_DEPTH {
            return;
        }
        for index in self
            .registry
            .arg_indices_for_role(name, &words, ArgRole::Body)
        {
            let (Some(word), Some(&span)) = (args.get(index), spans.get(index)) else {
                continue;
            };
            let Some((content, base)) = braced_content(word, span) else {
                continue;
            };
            let mark = self.guard.len();
            for command in crate::segmenter::segment_commands_with_offset_and_config(
                content,
                base,
                self.config,
            ) {
                if command.is_partial {
                    break;
                }
                let Some((head, rest)) = command.texts.split_first() else {
                    continue;
                };
                let spans: Vec<tcl_lexer::Span> = command
                    .argv
                    .iter()
                    .skip(1)
                    .map(|token| token.span)
                    .collect();
                self.call(head, rest, &spans, depth + 1);
                self.guard.extend(command.texts.iter().cloned());
            }
            self.guard.truncate(mark);
        }
    }
}

/// Every SSA version that holds a signed integer straight from a scan.
fn signed_scan_values(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    profile: &tcl_dialect::DialectProfile,
) -> HashMap<ValueKey, SignedOrigin> {
    let mut signed = HashMap::new();
    for (&block, data) in &fu.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &statement.statement
            else {
                continue;
            };
            let name = canonical_command.as_deref().unwrap_or(command);
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            let fields = scanned_fields(registry, name, &words, profile);
            if fields.is_empty() {
                continue;
            }
            let clobbered = beside_marker(fu, block, &data.statements, index);
            for (variable, origin) in fields {
                let named = |symbol: &crate::ssa::Symbol| fu.ssa.var_name(*symbol) == variable;
                let mut defined = statement
                    .defs
                    .iter()
                    .filter(|(symbol, _)| named(symbol))
                    .map(|(&symbol, &version)| (symbol, version))
                    .chain(
                        clobbered
                            .into_iter()
                            .flatten()
                            .filter(|(symbol, _)| named(symbol))
                            .map(|(&symbol, &(_, fresh))| (symbol, fresh)),
                    );
                if let (Some(key), None) = (defined.next(), defined.next()) {
                    signed.insert(key, origin);
                }
            }
        }
    }
    extend_through_phis(fu, &mut signed);
    signed
}

/// Scans whose outputs the SSA leaves unnamed, by block: in a procedure
/// body a head the module cannot prove (it may have been renamed before the
/// procedure runs) is an unseen call, after which the frame's names have no
/// version. `(statement index, variable, origin)`.
/// Whether any word names `name` bare — `set t 0`, `upvar 1 t x`,
/// `uplevel {incr t}` — rather than reading it as `$t` or `${t}`.
fn mentions_bare(args: &[String], name: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    args.iter().any(|arg| {
        arg.match_indices(name).any(|(at, _)| {
            let before = arg[..at].chars().next_back();
            let after = arg[at + name.len()..].chars().next();
            let read = arg[..at].ends_with('$') || arg[..at].ends_with("${");
            !read && !before.is_some_and(ident) && !after.is_some_and(ident)
        })
    })
}

#[derive(Default)]
struct UnresolvedScans {
    scans: HashMap<crate::cfg::BlockId, Vec<(usize, String, SignedOrigin)>>,
    /// Unseen-call markers that stand beside a call to a command the
    /// registry ships, with that call's argument words: such a call writes a
    /// name only by mentioning it (`upvar 1 t x`, `uplevel {set t 5}`).
    known_markers: HashMap<(crate::cfg::BlockId, usize), Vec<String>>,
}

impl UnresolvedScans {
    fn is_empty(&self) -> bool {
        self.scans.is_empty()
    }

    fn get(&self, block: crate::cfg::BlockId) -> Option<&Vec<(usize, String, SignedOrigin)>> {
        self.scans.get(&block)
    }

    /// Whether statement `at` of `block` may write `name`.
    fn writes(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        (block, at): (crate::cfg::BlockId, usize),
        statement: &crate::ssa::SsaStatement,
        name: &str,
    ) -> bool {
        let mentions = |args: &[String]| mentions_bare(args, name);
        if crate::ssa::is_unseen_call_marker(&statement.statement) {
            return self
                .known_markers
                .get(&(block, at))
                .is_none_or(|args| mentions(args));
        }
        statement
            .defs
            .keys()
            .any(|symbol| fu.ssa.var_name(*symbol) == name)
            || matches!(&statement.statement, Statement::Call { args, .. } if mentions(args))
    }
}

fn unresolved_scans(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    profile: &tcl_dialect::DialectProfile,
) -> UnresolvedScans {
    let mut out = UnresolvedScans::default();
    for (&block, data) in &fu.ssa.blocks {
        let mut beside: Option<Vec<String>> = None;
        for (index, statement) in data.statements.iter().enumerate() {
            if crate::ssa::is_unseen_call_marker(&statement.statement) {
                if let Some(args) = beside.take() {
                    out.known_markers.insert((block, index), args);
                }
                continue;
            }
            let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &statement.statement
            else {
                if !matches!(statement.statement, Statement::Barrier { .. }) {
                    beside = None;
                }
                continue;
            };
            let name = canonical_command.as_deref().unwrap_or(command);
            beside = registry.get(name).map(|_| args.clone());
            if !statement.defs.is_empty() {
                continue;
            }
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            for (variable, origin) in scanned_fields(registry, name, &words, profile) {
                out.scans
                    .entry(block)
                    .or_default()
                    .push((index, variable, origin));
            }
        }
    }
    out
}

/// A scan earlier in the same block that wrote the plain variable `read`
/// names, with nothing between that could write it again: no statement that
/// defines or names it, and no unseen call other than the scan's own marker.
/// The read must be a plain local name; a qualified or array name abstains.
fn straight_line_scan<'a>(
    fu: &crate::compilation_unit::FunctionUnit,
    unresolved: &'a UnresolvedScans,
    read: &ExprNode,
    block: crate::cfg::BlockId,
    index: usize,
) -> Option<&'a SignedOrigin> {
    let ExprNode::Var { text, name, .. } = read else {
        return None;
    };
    if text.as_str() != format!("${name}") {
        return None;
    }
    straight_line_scan_named(fu, unresolved, name, block, index)
}

/// [`straight_line_scan`] for a plain local name.
fn straight_line_scan_named<'a>(
    fu: &crate::compilation_unit::FunctionUnit,
    unresolved: &'a UnresolvedScans,
    name: &str,
    block: crate::cfg::BlockId,
    index: usize,
) -> Option<&'a SignedOrigin> {
    if name.contains(['$', '(', ':', '[', '\\', '{', '"']) {
        return None;
    }
    let writes = |at: crate::cfg::BlockId, index: usize, s: &crate::ssa::SsaStatement| {
        unresolved.writes(fu, (at, index), s, name)
    };
    let latest = |at: crate::cfg::BlockId, before: usize| {
        unresolved
            .get(at)?
            .iter()
            .filter(|(scan, variable, _)| *scan < before && variable == name)
            .max_by_key(|(scan, ..)| *scan)
    };
    // The statements after a scan, past the marker standing beside it.
    let after_scan = |at: crate::cfg::BlockId, scan: usize| -> bool {
        let Some(data) = fu.ssa.blocks.get(&at) else {
            return false;
        };
        let mut rest = data.statements.iter().enumerate().skip(scan + 1).peekable();
        while rest.peek().is_some_and(|(_, s)| {
            matches!(s.statement, Statement::Barrier { .. })
                || crate::ssa::is_unseen_call_marker(&s.statement)
        }) {
            rest.next();
        }
        !rest.any(|(i, s)| writes(at, i, s))
    };
    let statements = &fu.ssa.blocks.get(&block)?.statements;
    if let Some((scan, _, origin)) = latest(block, index) {
        let mut between = statements
            .iter()
            .enumerate()
            .take(index)
            .skip(scan + 1)
            .peekable();
        while between.peek().is_some_and(|(_, s)| {
            matches!(s.statement, Statement::Barrier { .. })
                || crate::ssa::is_unseen_call_marker(&s.statement)
        }) {
            between.next();
        }
        return (!between.any(|(i, s)| writes(block, i, s))).then_some(origin);
    }
    // The nearest dominating block that scans the name, with no write on any
    // path from its scan to this read.
    let mut dominator = fu.ssa.idom.get(&block).copied().flatten();
    while let Some(at) = dominator {
        if let Some((scan, _, origin)) = latest(at, usize::MAX) {
            let (forward, backward) = paths_between(fu, at, block);
            let cyclic = backward.contains(&block);
            let clean_between = forward
                .intersection(&backward)
                .filter(|&&between| between != block)
                .all(|between| {
                    fu.ssa.blocks.get(between).is_some_and(|data| {
                        !data
                            .statements
                            .iter()
                            .enumerate()
                            .any(|(i, s)| writes(*between, i, s))
                    })
                });
            let reach = if cyclic { statements.len() } else { index };
            let clean_here = !statements
                .iter()
                .enumerate()
                .take(reach)
                .any(|(i, s)| writes(block, i, s));
            return (clean_between && clean_here && after_scan(at, *scan)).then_some(origin);
        }
        dominator = fu.ssa.idom.get(&at).copied().flatten();
    }
    None
}

/// Blocks reachable from `from`'s successors and blocks that reach `to`
/// through its predecessors, neither passing back through `from`.
fn paths_between(
    fu: &crate::compilation_unit::FunctionUnit,
    from: crate::cfg::BlockId,
    to: crate::cfg::BlockId,
) -> (
    std::collections::HashSet<crate::cfg::BlockId>,
    std::collections::HashSet<crate::cfg::BlockId>,
) {
    let walk = |start: Vec<crate::cfg::BlockId>,
                next: &dyn Fn(crate::cfg::BlockId) -> Vec<crate::cfg::BlockId>| {
        let mut seen = std::collections::HashSet::new();
        let mut pending = start;
        while let Some(at) = pending.pop() {
            if at == from || !seen.insert(at) {
                continue;
            }
            pending.extend(next(at));
        }
        seen
    };
    let successors = |at: crate::cfg::BlockId| {
        fu.cfg
            .blocks
            .get(&at)
            .map_or_else(Vec::new, crate::cfg::Block::successors)
    };
    let predecessors = fu.cfg.predecessors();
    let preds = |at: crate::cfg::BlockId| {
        predecessors
            .get(&at)
            .map_or_else(Vec::new, |set| set.iter().copied().collect())
    };
    (walk(successors(from), &successors), walk(preds(to), &preds))
}

/// The fresh versions an unseen-call marker beside the call at `index`
/// leaves. Where the module cannot prove the head still names the shipped
/// command (a procedure body can run after a rename), the call's writes land
/// there instead of as defs; the check reads them as the scan's output, the
/// same name-binding assumption every registry-driven check makes.
fn beside_marker<'a>(
    fu: &'a crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    statements: &[crate::ssa::SsaStatement],
    index: usize,
) -> Option<&'a HashMap<crate::ssa::Symbol, (crate::ssa::Version, crate::ssa::Version)>> {
    let marker = statements
        .iter()
        .enumerate()
        .skip(index + 1)
        .find(|(_, s)| !matches!(s.statement, Statement::Barrier { .. }))
        .filter(|(_, s)| crate::ssa::is_unseen_call_marker(&s.statement))?
        .0;
    fu.ssa.value_clobbers.get(&block)?.get(&marker)
}

/// A phi whose every input is the same signed scan holds that scan's value.
fn extend_through_phis(
    fu: &crate::compilation_unit::FunctionUnit,
    signed: &mut HashMap<ValueKey, SignedOrigin>,
) {
    loop {
        let mut added = false;
        for data in fu.ssa.blocks.values() {
            for phi in &data.phis {
                let key = (phi.name, phi.version);
                if signed.contains_key(&key) {
                    continue;
                }
                let mut origin: Option<&SignedOrigin> = None;
                let all = phi.incoming.values().all(|&version| {
                    let Some(input) = signed.get(&(phi.name, version)) else {
                        return false;
                    };
                    origin.is_none_or(|seen| seen == input) && {
                        origin = Some(input);
                        true
                    }
                });
                if all && let Some(origin) = origin.cloned() {
                    signed.insert(key, origin);
                    added = true;
                }
            }
        }
        if !added {
            return;
        }
    }
}

/// `(variable, origin)` for each signed integer a literal scan template stores.
fn scanned_fields(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    words: &[&str],
    profile: &tcl_dialect::DialectProfile,
) -> Vec<(String, SignedOrigin)> {
    let formats = registry.format_string_args(command, words);
    let [format] = formats.as_slice() else {
        return Vec::new();
    };
    let Some(template) = words
        .get(format.index)
        .filter(|t| !t.contains(['$', '[', '\\']))
    else {
        return Vec::new();
    };
    let variables: Vec<&str> = registry
        .arg_indices_for_role(command, words, ArgRole::VarWrite)
        .into_iter()
        .filter(|&index| index > format.index)
        .filter_map(|index| words.get(index).copied())
        .collect();
    if !format.scan {
        return Vec::new();
    }
    let widths: Vec<Option<(u32, String, Option<String>)>> = match format.kind {
        tcl_registry::patterns::FormatType::Binary => {
            let unsigned = tcl_cmd_core::binary::signedness_available(profile);
            tcl_cmd_core::binary::specifiers(template.as_bytes(), unsigned)
                .iter()
                .filter(|field| tcl_cmd_core::binary::scan_field_assigns(field.letter))
                .map(|field| {
                    tcl_cmd_core::binary::signed_scan_width(field).map(|bits| {
                        let written = template[field.letter_start..field.end].to_owned();
                        let suffixed = unsigned.then(|| format!("{written}u"));
                        (bits, written, suffixed)
                    })
                })
                .collect()
        }
        tcl_registry::patterns::FormatType::Sprintf => {
            let fields = tcl_syntax::scan::conversion_spans(template, profile);
            if fields.iter().any(|field| field.spec.xpg_index.is_some()) {
                return Vec::new();
            }
            fields
                .iter()
                .filter(|field| field.spec.assigns())
                .map(|field| {
                    field.spec.truncates_to_int32(profile).then(|| {
                        let written = &template[field.percent.start..field.conversion.end];
                        let wide = format!(
                            "{}l{}",
                            &template[field.percent.start..field.size.start],
                            &template[field.conversion.clone()]
                        );
                        let full_width =
                            tcl_syntax::scan::ScanConversion::long_size_reads_full_width(profile)
                                && field.spec.size != Some(tcl_syntax::scan::SizeModifier::Long);
                        (32, written.to_owned(), full_width.then_some(wide))
                    })
                })
                .collect()
        }
        _ => return Vec::new(),
    };
    let conversion = format.kind == tcl_registry::patterns::FormatType::Sprintf;
    let ensemble = registry
        .get(command)
        .is_some_and(|spec| !spec.subcommands.is_empty());
    let command = match words.first() {
        Some(word) if ensemble => format!("{command} {word}"),
        _ => command.to_owned(),
    };
    widths
        .into_iter()
        .zip(variables)
        .filter_map(|(width, variable)| {
            let (bits, field, unsigned_field) = width?;
            let plain = !variable.contains(['$', '[', '(', '\\', '{', '"']);
            plain.then(|| {
                (
                    variable.to_owned(),
                    SignedOrigin {
                        bits,
                        field,
                        command: command.clone(),
                        conversion,
                        unsigned_field,
                    },
                )
            })
        })
        .collect()
}

fn comparisons(expr: &ExprNode, numbers: tcl_dialect::NumberSyntax) -> Vec<Comparison<'_>> {
    expr.evaluation_nodes()
        .into_iter()
        .filter_map(|node| {
            let ExprNode::Binary { op, left, right } = node else {
                return None;
            };
            if !matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::StrEq
                    | BinOp::StrNe
            ) {
                return None;
            }
            let (variable, constant, variable_left) = match (left.as_ref(), right.as_ref()) {
                (var @ ExprNode::Var { .. }, lit @ ExprNode::Literal { .. }) => (var, lit, true),
                (lit @ ExprNode::Literal { .. }, var @ ExprNode::Var { .. }) => (var, lit, false),
                _ => return None,
            };
            let ExprNode::Literal { text, .. } = constant else {
                return None;
            };
            let value = crate::intervals::literal_int(constant, numbers)?;
            // A string comparison sees the literal's spelling, so only a
            // canonical decimal literal is the same number to `eq`.
            if matches!(op, BinOp::StrEq | BinOp::StrNe) && value.to_string() != text.trim() {
                return None;
            }
            Some(Comparison {
                op: *op,
                variable,
                literal: value,
                literal_text: text.trim(),
                variable_left,
            })
        })
        .collect()
}

fn message(comparison: &Comparison<'_>, origin: &SignedOrigin) -> Option<String> {
    let (lo, hi) = origin.range();
    let k = comparison.literal;
    // Normalise to `variable OP literal`.
    let op = if comparison.variable_left {
        comparison.op
    } else {
        match comparison.op {
            BinOp::Lt => BinOp::Gt,
            BinOp::Le => BinOp::Ge,
            BinOp::Gt => BinOp::Lt,
            BinOp::Ge => BinOp::Le,
            other => other,
        }
    };
    // The comparison's outcome when it is the same for every value in
    // `lo..=hi`: `$byte > 127` is always false for a signed byte.
    let outside = !(lo..=hi).contains(&k);
    let always = match op {
        BinOp::Eq | BinOp::StrEq if outside => false,
        BinOp::Ne | BinOp::StrNe if outside => true,
        BinOp::Lt if k > hi || k <= lo => k > hi,
        BinOp::Le if k >= hi || k < lo => k >= hi,
        BinOp::Gt if k < lo || k >= hi => k < lo,
        BinOp::Ge if k <= lo || k > hi => k <= lo,
        _ => return None,
    };
    let ExprNode::Var { text: var, .. } = comparison.variable else {
        return None;
    };
    let bits = origin.bits;
    let mask = format!("0x{:x}", (1u64 << bits) - 1);
    let modulus = 1i64 << bits;
    let equality = matches!(op, BinOp::Eq | BinOp::Ne | BinOp::StrEq | BinOp::StrNe);
    let signed_equivalent = (equality && k > hi && k < modulus).then(|| k - modulus);
    let mut text = format!(
        "{var} holds a signed {bits}-bit value from `{} … {}` ({lo}..{hi}), so comparing it with {} is always {}. ",
        origin.command,
        origin.field,
        comparison.literal_text,
        if always { "true" } else { "false" },
    );
    if let Some(unsigned) = &origin.unsigned_field {
        let how = if origin.conversion {
            "at full width"
        } else {
            "unsigned"
        };
        let _ = write!(text, "Read it {how} with `{unsigned}`, mask it as ");
    } else {
        text.push_str("Mask it as ");
    }
    let _ = write!(
        text,
        "`({var} & {mask}) {} {}`",
        op.as_str(),
        comparison.literal_text
    );
    let _ = match signed_equivalent {
        Some(signed) => write!(text, ", or compare with the signed value {signed}."),
        None => write!(text, "."),
    };
    Some(text)
}

/// How an unmasked signed operand corrupts a byte composition.
#[derive(Clone, Copy)]
enum Composition {
    /// The low part of `hi << k | lo`: a negative value sign-extends over
    /// every bit above it.
    Low { op: BinOp },
    /// The shifted high part: a negative value makes the whole result
    /// negative.
    High,
}

impl Composition {
    fn message(self, variable: &ExprNode, origin: &SignedOrigin) -> String {
        let ExprNode::Var { text: var, .. } = variable else {
            return String::new();
        };
        let (lo, hi) = origin.range();
        let mask = format!("0x{:x}", (1u64 << origin.bits) - 1);
        let effect = match self {
            Self::Low { op } => {
                format!("combining it with '{op}' sign-extends a negative value over the high bits")
            }
            Self::High => "shifting a negative value keeps the combined result negative".to_owned(),
        };
        format!(
            "{var} holds a signed {}-bit value from `{} … {}` ({lo}..{hi}); {effect}. Mask it first: `({var} & {mask})`.",
            origin.bits, origin.command, origin.field
        )
    }
}

/// `k` when `node` is `base << k` or `base * 2**k` with a literal `k`.
fn shifted_base(node: &ExprNode, numbers: tcl_dialect::NumberSyntax) -> Option<&ExprNode> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let amount = crate::intervals::literal_int(right, numbers)?;
    match op {
        BinOp::LShift if amount > 0 => Some(left),
        BinOp::Mul if amount > 1 && (amount & (amount - 1)) == 0 => Some(left),
        _ => None,
    }
}

/// Plain variables used as parts of a byte composition: `(hi << k) | lo`,
/// `hi * 256 + lo`, `^` likewise. Measured on Tcl 8.4-9.1 and Jim
/// (`evidence/tcl/signed-scan-downstream-consumers.md`): with signed bytes
/// `0xc0 0xff` both give a negative result instead of `49407`.
fn compositions(
    expr: &ExprNode,
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<(&ExprNode, Composition)> {
    let mut out = Vec::new();
    for node in expr.evaluation_nodes() {
        let ExprNode::Binary { op, left, right } = node else {
            continue;
        };
        if !matches!(op, BinOp::BitOr | BinOp::BitXor | BinOp::Add) {
            continue;
        }
        for (shifted, other) in [(left, right), (right, left)] {
            let Some(base) = shifted_base(shifted, numbers) else {
                continue;
            };
            if matches!(base, ExprNode::Var { .. }) {
                out.push((base, Composition::High));
            }
            if matches!(other.as_ref(), ExprNode::Var { .. }) {
                out.push((other.as_ref(), Composition::Low { op: *op }));
            }
        }
    }
    out
}

/// What `format` does with a negative value in one conversion.
fn format_effect(verb: u8, dotted: bool) -> Option<&'static str> {
    match verb {
        b'x' | b'X' | b'o' | b'b' => Some(
            "renders a negative value sign-extended (`format %02x -127` is `ffffffffffffff81`, or `ffffff81` on Tcl 9)",
        ),
        b'u' => {
            Some("renders a negative value as a negative or wrapped number, not the unsigned one")
        }
        b'c' => Some("renders a negative value as some other character, not the byte"),
        b'd' | b'i' if dotted => {
            Some("builds a dotted address with a negative octet, such as `192.-64.-1.1`")
        }
        _ => None,
    }
}

/// One call a statement makes, itself or in a `[…]` word: its command, its
/// post-head words with their spans, and which of them are indices.
struct ConsumerCall {
    head: Option<u32>,
    name: String,
    args: Vec<String>,
    spans: Vec<tcl_lexer::Span>,
    indices: Vec<usize>,
}

struct ConfigurationSource<'a> {
    source: &'a str,
    analysis: &'a crate::analyser::AnalysisResult,
    context: &'a tcl_registry::model::ContextRegistry,
}

fn signed_scan_warning(span: tcl_lexer::Span, message: String) -> Diagnostic {
    Diagnostic::new(DiagCode::W156, span, message, Severity::Warning)
}

/// Signed scan values passed, unmasked, to a command that mishandles them:
/// a `format` conversion ([`format_consumers`]) or an argument with a
/// non-negative domain or index role ([`argument_consumers`]), in every call
/// a statement makes.
fn call_consumers(
    fu: &crate::compilation_unit::FunctionUnit,
    (registry, context): (
        &tcl_registry::CommandRegistry,
        Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ),
    config: tcl_lexer::LexerConfig,
    (signed, unresolved): (&HashMap<ValueKey, SignedOrigin>, &UnresolvedScans),
    configuration: &ConfigurationSource<'_>,
) -> Vec<Diagnostic> {
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let lift = |lifted: crate::word_subst::LiftedCall| ConsumerCall {
        head: lifted.words.as_ref().and_then(consumer_head),
        indices: index_positions(registry, context, lifted.tokens.as_ref(), &lifted.args),
        name: lifted.command,
        args: lifted.args,
        spans: lifted.arg_spans,
    };
    let mut out = Vec::new();
    for (&block, data) in &fu.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let tokens = statement.statement.tokens();
            let mut calls: Vec<ConsumerCall> = Vec::new();
            if let (
                Statement::Call {
                    command,
                    canonical_command,
                    args,
                    ..
                },
                Some(tokens),
            ) = (&statement.statement, tokens)
            {
                calls.push(ConsumerCall {
                    head: consumer_head(tokens),
                    name: canonical_command.as_deref().unwrap_or(command).to_owned(),
                    args: args.clone(),
                    spans: tokens.argv.iter().skip(1).copied().collect(),
                    indices: index_positions(registry, context, Some(tokens), args),
                });
            }
            if tokens.is_some() {
                calls.extend(
                    crate::word_subst::lifted_calls(tokens, config)
                        .into_iter()
                        .map(lift),
                );
            }
            if let Some((expr, base)) = statement_expression(&statement.statement) {
                calls.extend(
                    crate::word_subst::lifted_calls_in_expr(expr, base, config, &surface)
                        .into_iter()
                        .map(lift),
                );
            }
            let site = ReadSite {
                fu,
                block,
                index,
                versions: &statement.uses,
                accesses: tokens.map_or(&[], |tokens| &tokens.variable_accesses[..]),
                signed,
                unresolved,
            };
            consume_calls(registry, configuration, &calls, &site, &mut out);
        }
        if let Some(Terminator::Branch {
            condition,
            condition_base,
            ..
        }) = fu
            .cfg
            .blocks
            .get(&block)
            .and_then(|b| b.terminator.as_ref())
        {
            let calls: Vec<ConsumerCall> = crate::word_subst::lifted_calls_in_expr(
                condition,
                *condition_base,
                config,
                &surface,
            )
            .into_iter()
            .map(lift)
            .collect();
            let site = ReadSite {
                fu,
                block,
                index: data.statements.len(),
                versions: &data.exit_versions,
                accesses: &[],
                signed,
                unresolved,
            };
            consume_calls(registry, configuration, &calls, &site, &mut out);
        }
    }
    out
}

fn consumer_head(tokens: &crate::ir::CommandTokens) -> Option<u32> {
    let head = tokens.words().first()?.source();
    (head.provenance == crate::ir::Provenance::Source).then_some(head.span.start())
}

/// The expression a statement evaluates, with its source base.
fn statement_expression(statement: &Statement) -> Option<(&ExprNode, Option<u32>)> {
    match statement {
        Statement::AssignExpr {
            expr, expr_base, ..
        }
        | Statement::ExprEval {
            expr, expr_base, ..
        }
        | Statement::Return {
            expr: Some(expr),
            expr_base,
            ..
        } => Some((expr, *expr_base)),
        _ => None,
    }
}

/// Where a call's variable words are read: the SSA versions in force there
/// and the retained source accesses, if any.
struct ReadSite<'a> {
    fu: &'a crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    versions: &'a HashMap<crate::ssa::Symbol, crate::ssa::Version>,
    accesses: &'a [crate::command_binding::SourceVariableAccess],
    signed: &'a HashMap<ValueKey, SignedOrigin>,
    unresolved: &'a UnresolvedScans,
}

impl<'a> ReadSite<'a> {
    /// The signed scan a read of `variable` at `span` sees: through its
    /// retained access, else by name among the versions in force, else along
    /// a straight run of code from an unproven scan.
    fn read(&self, span: tcl_lexer::Span, variable: &str) -> Option<&'a SignedOrigin> {
        let ssa = &self.fu.ssa;
        self.accesses
            .iter()
            .filter(|access| access.source.span == span)
            .find_map(|access| {
                let read = ssa.read_reference_at(
                    self.block,
                    self.index,
                    &access.source,
                    &access.original_spelling,
                )?;
                let version = read
                    .version
                    .or_else(|| self.versions.get(&read.symbol).copied())?;
                self.signed.get(&(read.symbol, version))
            })
            .or_else(|| {
                self.versions
                    .iter()
                    .filter(|(symbol, _)| ssa.var_name(**symbol) == variable)
                    .find_map(|(&symbol, &version)| self.signed.get(&(symbol, version)))
            })
            .or_else(|| {
                straight_line_scan_named(self.fu, self.unresolved, variable, self.block, self.index)
            })
    }
}

fn consume_calls(
    registry: &tcl_registry::CommandRegistry,
    configuration: &ConfigurationSource<'_>,
    calls: &[ConsumerCall],
    site: &ReadSite<'_>,
    out: &mut Vec<Diagnostic>,
) {
    let read = |span: tcl_lexer::Span, variable: &str| site.read(span, variable);
    for call in calls {
        let words: Vec<&str> = call.args.iter().map(String::as_str).collect();
        let found = argument_consumers(
            registry,
            &call.name,
            (&words, &call.indices),
            &call.spans,
            &read,
        )
        .into_iter()
        .chain(dotted_word_consumers(
            &words,
            &call.spans,
            site.accesses,
            &read,
        ))
        .chain(format_consumers(
            registry,
            (&call.name, &words),
            &call.spans,
            &read,
        ));
        out.extend(
            found.map(|(span, message)| signed_scan_warning(site.fu.abs_span(span), message)),
        );
        out.extend(data_group_comparison(configuration, call, site));
    }
}

/// A data-group equality comparison with an item written by a signed scan.
/// A source comparison alone does not establish which keys the group holds.
fn data_group_comparison(
    configuration: &ConfigurationSource<'_>,
    call: &ConsumerCall,
    site: &ReadSite<'_>,
) -> Option<Diagnostic> {
    // Only calls with a candidate descriptor need the complete source query.
    let spec = configuration.context.commands().get(&call.name)?;
    spec.resolve_subcommand(call.args.first()?)?
        .source_configuration_reference?;
    let head = site.fu.abs_pos(call.head?);
    let selected = crate::registry_invocation::source_structure::source_registry_words_at(
        configuration.source,
        configuration.analysis,
        head,
    )?;
    let reference = selected.with_source_schema(configuration.context, |invocation| {
        invocation.authored_source_configuration_reference_operand()
    })??;
    let item = reference.comparison_item?;
    let operator = reference.argument.min(item).checked_add(1)?;
    let literal =
        |index: usize| std::str::from_utf8(selected.arguments().get(index)?.literal_bytes()?).ok();
    if !literal(operator)?.eq_ignore_ascii_case(crate::expr_ast::BinOp::StrEquals.as_str()) {
        return None;
    }
    let group = literal(reference.argument)?;
    let word = selected.original_argument_word(item)?;
    let (variable, span) = super::super::original_roles::scalar_variable_reference(word)?;
    let relative_span = call
        .spans
        .iter()
        .copied()
        .find(|&arg| site.fu.abs_span(arg) == word.span())?;
    let origin = site.read(relative_span, &variable)?;
    let (lo, hi) = origin.range();
    let mask = (1u64 << origin.bits) - 1;
    let mut finding = Diagnostic::new(
        DiagCode::W156,
        span,
        format!(
            "${variable} holds a signed {}-bit value from `{} … {}` ({lo}..{hi}); it may not match unsigned keys in data group `{group}`. For unsigned keys, mask it as `[expr {{${variable} & 0x{mask:x}}}]`.",
            origin.bits, origin.command, origin.field,
        ),
        Severity::Hint,
    );
    finding.signed_data_group_comparison = Some(Box::new(SignedDataGroupComparison {
        group: group.to_owned(),
        unsigned_keys: (1u64 << (origin.bits - 1))..=mask,
    }));
    Some(finding)
}

/// `$name` arguments of a `format` call whose conversion renders a signed
/// scan value wrongly: `%x %X %o %b %u %c`, and `%d`/`%i` in a dotted run
/// such as `%d.%d.%d.%d`. Measured on Tcl 8.4-9.1, Jim and TMM
/// (`evidence/tcl/signed-scan-downstream-consumers.md`,
/// `evidence/tcl/bigip-tmm-signed-scan-and-format.md`).
fn format_consumers<'a>(
    registry: &tcl_registry::CommandRegistry,
    (name, words): (&str, &[&str]),
    spans: &[tcl_lexer::Span],
    read: &impl Fn(tcl_lexer::Span, &str) -> Option<&'a SignedOrigin>,
) -> Vec<(tcl_lexer::Span, String)> {
    let mut out = Vec::new();
    let formats = registry.format_string_args(name, words);
    let [format] = formats.as_slice() else {
        return out;
    };
    if format.scan || format.kind != tcl_registry::patterns::FormatType::Sprintf {
        return out;
    }
    let Some(template) = words
        .get(format.index)
        .filter(|t| !t.contains(['$', '[', '\\']))
    else {
        return out;
    };
    for (argument, verb, dotted) in format_arguments(template.as_bytes(), format.index) {
        let Some(effect) = format_effect(verb, dotted) else {
            continue;
        };
        let (Some(word), Some(&span)) = (words.get(argument), spans.get(argument)) else {
            continue;
        };
        let Some(variable) = plain_variable(word) else {
            continue;
        };
        let Some(origin) = read(span, variable) else {
            continue;
        };
        let word = format!("${variable}");
        let (lo, hi) = origin.range();
        let mask = format!("0x{:x}", (1u64 << origin.bits) - 1);
        out.push((
            span,
            format!(
                "{word} holds a signed {}-bit value from `{} … {}` ({lo}..{hi}); `format %{}` {effect}. Mask it first: `[expr {{{word} & {mask}}}]`.",
                origin.bits,
                origin.command,
                origin.field,
                char::from(verb)
            ),
        ));
    }
    out
}

/// Octets of an address built by interpolation (`"$a.$b.$c.$d"`): a word of
/// four `.`-separated parts, each a variable or a decimal octet. A signed
/// 8-bit scan value there renders negative (`192.-64.-1.1`), which TMM's
/// `IP::addr` rejects at run time
/// (`evidence/tcl/bigip-tmm-signed-scan-and-format.md`).
fn dotted_word_consumers<'a>(
    words: &[&str],
    spans: &[tcl_lexer::Span],
    accesses: &[crate::command_binding::SourceVariableAccess],
    read: &impl Fn(tcl_lexer::Span, &str) -> Option<&'a SignedOrigin>,
) -> Vec<(tcl_lexer::Span, String)> {
    let mut out = Vec::new();
    for (word, span) in words.iter().zip(spans) {
        let text = word.trim_matches('"');
        let parts: Vec<&str> = text.split('.').collect();
        if parts.len() != 4
            || !parts.iter().all(|part| {
                plain_variable(part).is_some_and(|v| !v.is_empty())
                    || (!part.is_empty()
                        && part.len() <= 3
                        && part.bytes().all(|b| b.is_ascii_digit()))
            })
        {
            continue;
        }
        let inside: Vec<(tcl_lexer::Span, &str)> = accesses
            .iter()
            .filter(|a| span.start() <= a.source.span.start() && a.source.span.end() <= span.end())
            .filter_map(|a| Some((a.source.span, plain_variable(&a.original_spelling)?)))
            .collect();
        let reads = if inside.is_empty() {
            // A lifted expression call keeps its source spelling, so a word
            // as long as its span locates each part within it.
            let spelled = usize::try_from(span.end() - span.start()).ok() == Some(word.len());
            let mut cursor = 0;
            parts
                .iter()
                .filter_map(|part| {
                    let at = word[cursor..].find(part).map(|found| cursor + found)?;
                    cursor = at + part.len();
                    let variable = plain_variable(part)?;
                    let start = span.start() + u32::try_from(at).ok()?;
                    let located =
                        tcl_lexer::Span::new(start, start + u32::try_from(part.len()).ok()?);
                    Some((if spelled { located } else { *span }, variable))
                })
                .collect()
        } else {
            inside
        };
        for (at, variable) in reads {
            let Some(origin) = read(at, variable).filter(|o| o.bits == 8) else {
                continue;
            };
            out.push((
                at,
                format!(
                    "${variable} holds a signed 8-bit value from `{} … {}` (-128..127); interpolated into a dotted address it renders a negative octet, such as `192.-64.-1.1`. Mask it first: `[expr {{${variable} & 0xff}}]`.",
                    origin.command, origin.field
                ),
            ));
        }
    }
    out
}

/// The name a `$name` or `${name}` word reads.
fn plain_variable(word: &str) -> Option<&str> {
    word.strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
        .or_else(|| word.strip_prefix('$'))
}

/// Post-head argument positions that the call's index relationship reads as
/// indices, from its declared or operation-implied
/// [`tcl_registry::SourceIndexBounds`].
fn index_positions(
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: Option<&crate::ir::CommandTokens>,
    words: &[String],
) -> Vec<usize> {
    let Some(resolved) = tokens.and_then(|tokens| {
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            registry, context, tokens,
        )
    }) else {
        return Vec::new();
    };
    let Some(spec) = registry.get(&resolved.facts.canonical_command) else {
        return Vec::new();
    };
    let sub = (!spec.subcommands.is_empty())
        .then(|| words.first().and_then(|word| spec.resolve_subcommand(word)))
        .flatten();
    let declared = sub.map_or(spec.source_index_bounds, |sub| sub.source_index_bounds);
    let Some(bounds) =
        tcl_registry::SourceIndexBounds::selected(declared, resolved.facts.operation)
    else {
        return Vec::new();
    };
    let first = usize::from(sub.is_some());
    bounds
        .index_operands(words.len().saturating_sub(first))
        .map(|at| at + first)
        .collect()
}

/// `$name` arguments whose registry facts say a negative value is refused or
/// mishandled: a declared non-negative [`tcl_registry::hover::IntegerDomain`]
/// (a length, offset or port measured on BIG-IP 21.1, a `string repeat`
/// count), or an index (`lindex`, `string index`, `string range`, …), where a
/// negative value selects nothing on Tcl 8.4-9.1 and Jim.
fn argument_consumers<'a>(
    registry: &tcl_registry::CommandRegistry,
    name: &str,
    (words, indices): (&[&str], &[usize]),
    spans: &[tcl_lexer::Span],
    read: &impl Fn(tcl_lexer::Span, &str) -> Option<&'a SignedOrigin>,
) -> Vec<(tcl_lexer::Span, String)> {
    use tcl_registry::hover::IntegerDomain;
    let mut out = Vec::new();
    for (argument, word) in words.iter().enumerate() {
        let Some(variable) = plain_variable(word) else {
            continue;
        };
        let word = format!("${variable}");
        let effect = match registry.arg_domain(name, words, argument) {
            Some(IntegerDomain::Port) => Some("takes a port (0..65535) here".to_owned()),
            Some(IntegerDomain::Range(lo, hi)) if lo >= 0 => Some(if hi == i64::MAX {
                "takes a non-negative value here".to_owned()
            } else {
                format!("takes a value in {lo}..{hi} here")
            }),
            _ if indices.contains(&argument) => {
                Some("selects nothing for a negative index".to_owned())
            }
            _ => None,
        };
        let (Some(effect), Some(&span)) = (effect, spans.get(argument)) else {
            continue;
        };
        let Some(origin) = read(span, variable) else {
            continue;
        };
        let (lo, hi) = origin.range();
        let mask = format!("0x{:x}", (1u64 << origin.bits) - 1);
        let head = words.first().filter(|_| {
            registry
                .get(name)
                .is_some_and(|spec| !spec.subcommands.is_empty())
        });
        let shown = head.map_or_else(|| name.to_owned(), |sub| format!("{name} {sub}"));
        out.push((
            span,
            format!(
                "{word} holds a signed {}-bit value from `{} … {}` ({lo}..{hi}); `{shown}` {effect}. Mask it first: `[expr {{{word} & {mask}}}]`.",
                origin.bits, origin.command, origin.field
            ),
        ));
    }
    out
}

/// `(argument index, verb, in a dotted run)` for each value a sequential
/// `format` template consumes; a positional (`%n$`) template yields nothing.
fn format_arguments(template: &[u8], format_index: usize) -> Vec<(usize, u8, bool)> {
    let mut conversions = Vec::new();
    let mut i = 0;
    let mut argument = format_index + 1;
    while i < template.len() {
        if template[i] != b'%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if template.get(i) == Some(&b'%') {
            i += 1;
            continue;
        }
        let Some(spec) = tcl_syntax::format::parse_spec(template, &mut i) else {
            return Vec::new();
        };
        if spec.arg_index.is_some() {
            return Vec::new();
        }
        argument += usize::from(spec.width_star) + usize::from(spec.precision_star);
        conversions.push((argument, spec.verb, start, i));
        argument += 1;
    }
    let integer = |verb: u8| matches!(verb, b'd' | b'i' | b'u');
    (0..conversions.len())
        .map(|n| {
            let (argument, verb, start, end) = conversions[n];
            let joined = |other: Option<&(usize, u8, usize, usize)>, dot: usize| {
                other.is_some_and(|o| integer(o.1)) && template.get(dot) == Some(&b'.')
            };
            let before = n.checked_sub(1).and_then(|p| conversions.get(p));
            let after = conversions.get(n + 1);
            let dotted = integer(verb)
                && (before.is_some_and(|b| b.3 + 1 == start)
                    && joined(before, start.wrapping_sub(1))
                    || after.is_some_and(|a| end + 1 == a.2) && joined(after, end));
            (argument, verb, dotted)
        })
        .collect()
}

#[cfg(test)]
mod tests;
