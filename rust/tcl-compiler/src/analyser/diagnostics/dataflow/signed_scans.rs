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
use crate::analyser::types::Severity;
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
    command: &'static str,
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
        let mut findings: Vec<(tcl_lexer::Span, String)> = Vec::new();
        let mut check = |expr: &ExprNode,
                         view: SsaSourceView<'_>,
                         exit: Option<&ExitVersions>,
                         (block, index, base): (crate::cfg::BlockId, usize, Option<u32>),
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
            };
            let locate = |variable: &ExprNode| match (base, variable) {
                (Some(base), ExprNode::Var { start, end, .. }) => {
                    tcl_lexer::Span::new(base + start, base + end + 1)
                }
                _ => fallback,
            };
            for (variable, composition) in compositions(expr, numbers) {
                if let Some(origin) = resolve(variable) {
                    findings.push((
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
                findings.push((fu.abs_span(span), message));
            }
        };
        for_each_expression(fu, self.file_lexer_config(), registry, &mut check);
        findings.extend(format_consumers(
            fu,
            registry,
            self.file_lexer_config(),
            (&signed, &unresolved),
        ));
        findings.sort_by_key(|(span, _)| (span.start(), span.end()));
        findings.dedup();
        for (span, message) in findings {
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W156,
                    span,
                    message,
                    Severity::Warning,
                ));
        }
    }
}

type ExitVersions = HashMap<crate::ssa::Symbol, crate::ssa::Version>;

/// Every expression the body evaluates: lowered `expr`/`return` operands,
/// `[expr …]` substitutions inside other commands, and branch conditions,
/// each with its SSA view, the block's exit versions for a condition, its
/// source base and a fallback span.
fn for_each_expression(
    fu: &crate::compilation_unit::FunctionUnit,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    visit: &mut impl FnMut(
        &ExprNode,
        SsaSourceView<'_>,
        Option<&ExitVersions>,
        (crate::cfg::BlockId, usize, Option<u32>),
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
                } => visit(expr, view, None, (block, index, *expr_base), *span),
                _ => {
                    for evaluation in crate::word_subst::entered_expression_evaluations(
                        view.source_tokens(),
                        config,
                        registry,
                    ) {
                        visit(
                            evaluation.expression(),
                            view,
                            None,
                            (block, index, Some(evaluation.expression_base())),
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
                (block, data.statements.len(), *condition_base),
                *span,
            );
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
type UnresolvedScans = HashMap<crate::cfg::BlockId, Vec<(usize, String, SignedOrigin)>>;

fn unresolved_scans(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    profile: &tcl_dialect::DialectProfile,
) -> UnresolvedScans {
    let mut out: UnresolvedScans = HashMap::new();
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
            if !statement.defs.is_empty() {
                continue;
            }
            let name = canonical_command.as_deref().unwrap_or(command);
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            for (variable, origin) in scanned_fields(registry, name, &words, profile) {
                out.entry(block)
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
    let statements = &fu.ssa.blocks.get(&block)?.statements;
    let (scan, _, origin) = unresolved
        .get(&block)?
        .iter()
        .filter(|(at, variable, _)| *at < index && variable == name)
        .max_by_key(|(at, ..)| *at)?;
    let mut between = statements.iter().take(index).skip(scan + 1).peekable();
    while between.peek().is_some_and(|s| {
        matches!(s.statement, Statement::Barrier { .. })
            || crate::ssa::is_unseen_call_marker(&s.statement)
    }) {
        between.next();
    }
    let writes = |s: &crate::ssa::SsaStatement| {
        crate::ssa::is_unseen_call_marker(&s.statement)
            || s.defs.keys().any(|symbol| fu.ssa.var_name(*symbol) == name)
            || matches!(&s.statement, Statement::Call { args, .. }
                if args.iter().any(|arg| arg == name))
    };
    (!between.any(writes)).then_some(origin)
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
    let command = match format.kind {
        tcl_registry::patterns::FormatType::Binary => "binary scan",
        _ => "scan",
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
                        command,
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
    if (lo..=hi).contains(&k) {
        return None;
    }
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
    let always = match op {
        BinOp::Eq | BinOp::StrEq => false,
        BinOp::Ne | BinOp::StrNe => true,
        BinOp::Lt | BinOp::Le => k > hi,
        BinOp::Gt | BinOp::Ge => k < lo,
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
        let how = if origin.command == "scan" {
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

/// `$name` arguments of a `format` call whose conversion renders a signed
/// scan value wrongly: `%x %X %o %b %u %c`, and `%d`/`%i` in a dotted run
/// such as `%d.%d.%d.%d`. Measured on Tcl 8.4-9.1, Jim and TMM
/// (`evidence/tcl/signed-scan-downstream-consumers.md`,
/// `evidence/tcl/bigip-tmm-signed-scan-and-format.md`).
fn format_consumers(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    (signed, unresolved): (&HashMap<ValueKey, SignedOrigin>, &UnresolvedScans),
) -> Vec<(tcl_lexer::Span, String)> {
    let mut out = Vec::new();
    for (&block, data) in &fu.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let Some(tokens) = statement.statement.tokens() else {
                continue;
            };
            // The statement's own call and every `[…]` call its words hold.
            let mut calls: Vec<(String, Vec<String>, Vec<tcl_lexer::Span>)> = Vec::new();
            if let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &statement.statement
            {
                calls.push((
                    canonical_command.as_deref().unwrap_or(command).to_owned(),
                    args.clone(),
                    tokens.argv.iter().skip(1).copied().collect(),
                ));
            }
            for lifted in crate::word_subst::lifted_calls(Some(tokens), config) {
                calls.push((lifted.command, lifted.args, lifted.arg_spans));
            }
            let read = |span: tcl_lexer::Span, variable: &str| {
                tokens
                    .variable_accesses
                    .iter()
                    .filter(|access| access.source.span == span)
                    .find_map(|access| {
                        let read = fu.ssa.read_reference_at(
                            block,
                            index,
                            &access.source,
                            &access.original_spelling,
                        )?;
                        let version = read
                            .version
                            .or_else(|| statement.uses.get(&read.symbol).copied())?;
                        signed.get(&(read.symbol, version))
                    })
                    .or_else(|| straight_line_scan_named(fu, unresolved, variable, block, index))
            };
            for (name, args, spans) in &calls {
                let words: Vec<&str> = args.iter().map(String::as_str).collect();
                let formats = registry.format_string_args(name, &words);
                let [format] = formats.as_slice() else {
                    continue;
                };
                if format.scan || format.kind != tcl_registry::patterns::FormatType::Sprintf {
                    continue;
                }
                let Some(template) = words
                    .get(format.index)
                    .filter(|t| !t.contains(['$', '[', '\\']))
                else {
                    continue;
                };
                for (argument, verb, dotted) in format_arguments(template.as_bytes(), format.index)
                {
                    let Some(effect) = format_effect(verb, dotted) else {
                        continue;
                    };
                    let (Some(word), Some(&span)) = (words.get(argument), spans.get(argument))
                    else {
                        continue;
                    };
                    let Some(origin) = word.strip_prefix('$').and_then(|v| read(span, v)) else {
                        continue;
                    };
                    let (lo, hi) = origin.range();
                    let mask = format!("0x{:x}", (1u64 << origin.bits) - 1);
                    out.push((
                        fu.abs_span(span),
                        format!(
                            "{word} holds a signed {}-bit value from `{} … {}` ({lo}..{hi}); `format %{}` {effect}. Mask it first: `[expr {{{word} & {mask}}}]`.",
                            origin.bits,
                            origin.command,
                            origin.field,
                            char::from(verb)
                        ),
                    ));
                }
            }
        }
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
