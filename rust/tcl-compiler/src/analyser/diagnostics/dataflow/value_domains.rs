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

//! Diagnostics on the value a definition holds: a non-channel channel argument (W126), a zero divisor (W233), an index outside its interval and a malformed IP literal (W124).

use super::super::helpers::{find_dotted_quads, is_word_byte, source_slice};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
#[cfg(test)]
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
#[cfg(test)]
use crate::expr_ast::ExprNode;
use rustc_hash::FxHashSet;
use std::collections::HashSet;
use tcl_core_types::DiagCode;

impl Analyser {
    /// W126 — channel-argument validation.
    ///
    /// Walks every
    /// SSA-annotated `Call` statement for commands that declare
    /// `ArgRole::Channel` arguments; for each channel-position
    /// argument, checks the SSA type lattice to determine whether
    /// the value is genuinely a channel.  Two failure modes:
    ///
    /// - **`$var` reference** with `TypeKind::Known` and a non-
    ///   `TclType::Channel` type — emits "passed as channel … has
    ///   type X, not CHANNEL".
    /// - **String literal** that isn't `stdin` / `stdout` /
    ///   `stderr` and contains no substitutions — emits
    ///   "String literal 'X' used as channel argument".
    ///
    /// The standard channels (`stdin`, `stdout`, `stderr`) are
    /// always accepted.  Unknown / overdefined types skip the
    /// check (could be anything).
    pub(in super::super) fn emit_channel_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        let context = self.analysis_context();
        for (&bn, block) in &fu.ssa.blocks {
            for (statement_index, ssa_stmt) in block.statements.iter().enumerate() {
                let crate::ir::Statement::Call { command, .. } = &ssa_stmt.statement else {
                    continue;
                };
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, bn, statement_index)
                        .source_tokens()
                else {
                    continue;
                };
                let arguments = super::super::original_roles::channel_arguments(
                    &self.source,
                    &self.result,
                    tokens,
                    std::sync::Arc::clone(&context),
                );
                for argument in arguments {
                    let message = if let Some((name, reference_span)) =
                        super::super::original_roles::scalar_variable_reference(&argument.word)
                    {
                        let mut reads = tokens
                            .variable_accesses
                            .iter()
                            .filter(|access| access.source.span == reference_span)
                            .filter_map(|access| {
                                fu.ssa.read_reference_at(
                                    bn,
                                    statement_index,
                                    &access.source,
                                    &access.original_spelling,
                                )
                            });
                        let Some(read) = reads.next() else {
                            continue;
                        };
                        if reads.any(|other| other != read) {
                            continue;
                        }
                        let sym = read.symbol;
                        let Some(version) = read.version else {
                            continue;
                        };
                        let Some(var_type) = fu.types.get(&(sym, version)) else {
                            continue;
                        };
                        let Some(type_label) = non_channel_union_label(var_type) else {
                            continue;
                        };
                        format!(
                            "Variable '${name}' passed as channel to '{command}' has type {type_label}, not CHANNEL."
                        )
                    } else {
                        let Some(literal) = argument.literal else {
                            continue;
                        };
                        if matches!(literal.as_slice(), b"stdin" | b"stdout" | b"stderr") {
                            continue;
                        }
                        let Ok(literal) = std::str::from_utf8(&literal) else {
                            continue;
                        };
                        format!(
                            "String literal '{literal}' used as channel argument to '{command}' — expected a channel from open/socket/chan create."
                        )
                    };
                    self.result.diagnostics.push(
                        crate::analyser::types::Diagnostic::new(
                            DiagCode::W126,
                            argument.word.span(),
                            message,
                            Severity::Warning,
                        )
                        .with_subject(argument.subject),
                    );
                }
            }
        }
    }

    /// W124 — invalid IP address literal.
    ///
    /// Walks every
    /// SSA-tracked constant string in the function's SCCP
    /// values; regex-searches for IPv4 dotted-quad and IPv6
    /// candidates and validates each.
    ///
    /// **Validation:**
    /// - **IPv4** — each octet must be 0..255; leading-zero
    ///   octets emit a Warning (interpreted as octal in some
    ///   contexts); over-255 octets emit an Error.  Patterns
    ///   preceded by ``/`` (CIDR / version-number context) are
    ///   skipped.
    /// - **IPv6** — parsed via [`std::net::Ipv6Addr`]; failure
    ///   emits an Error.
    ///
    /// Diagnostic anchors at the SSA def site (the assignment
    /// statement's span); seen-offsets dedup avoids duplicate
    /// emissions when multiple SSA versions share a def.
    /// **W233.** Division / modulo by a provably-zero divisor — raises
    /// "divide by zero" at runtime.  Delegates to the canonical
    /// interval-bounds analysis [`crate::interval_bounds::find_divide_by_zero`]
    /// (the single source of truth, shared with the interval-bounds index
    /// checks): a `/` or `%` whose divisor's interval — guard-narrowed at the
    /// use site and seeded from the SCCP lattice — is exactly `[0, 0]`, on the
    /// always-evaluated spine of an executable expression.
    ///
    /// (Verified against tclsh 8.4–9.0: integer `1/0` and `5%0` raise "divide
    /// by zero"; float division such as `1.0/0` yields `Inf` and does not
    /// error. The interval domain is integer, matching that boundary for the
    /// common cases.)
    pub(in super::super) fn emit_w233_divide_by_zero(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        // The block set SCCP proved reachable; fall back to every SSA block
        // when SCCP produced nothing (e.g. a trivial function) so the check
        // still runs.
        let executable: HashSet<crate::cfg::BlockId> =
            if fu.diagnostic_value_facts().executable_blocks().is_empty() {
                fu.ssa.blocks.keys().copied().collect()
            } else {
                fu.diagnostic_value_facts().executable_blocks().clone()
            };
        let context = self.analysis_context();
        let registry = context.commands();
        for finding in crate::interval_bounds::find_divide_by_zero_with_entered_operands(
            &fu.cfg,
            &fu.ssa,
            (fu.diagnostic_value_facts().values(), (&fu.sccp, &fu.types)),
            &executable,
            // The document's own numeral grammar: a divisor literal means what
            // this dialect says it means (`0755` is 493 up to 8.6, 755 from
            // 9.0), and this process analyses documents of several dialects.
            crate::intervals::numbers_for_dialect(Some(self.profile)),
            crate::interval_bounds::BoundsSemantics {
                registry,
                context: Some(context.as_ref().into()),
                grammar: self.grammar(),
            },
        ) {
            let span = fu.abs_span(finding.span);
            if span.is_empty() {
                continue;
            }
            let verb = if finding.op == "/" {
                "Division"
            } else {
                "Modulo"
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W233,
                    span,
                    format!(
                        "{verb} by a provably-zero divisor — raises 'divide by zero' at runtime."
                    ),
                    Severity::Warning,
                ));
        }
    }

    pub(in super::super) fn emit_interval_bounds_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        let executable: HashSet<crate::cfg::BlockId> =
            if fu.diagnostic_value_facts().executable_blocks().is_empty() {
                fu.ssa.blocks.keys().copied().collect()
            } else {
                fu.diagnostic_value_facts()
                    .executable_blocks()
                    .iter()
                    .copied()
                    .collect()
            };
        let context = self.analysis_context();
        let registry = context.commands();
        let findings = crate::interval_bounds::find_interval_bounds_in_release(
            &fu.cfg,
            &fu.ssa,
            (fu.diagnostic_value_facts().values(), (&fu.sccp, &fu.types)),
            &executable,
            // The document's own character model, numeral grammar and `lset`
            // append boundary — dialect-derived facts, threaded rather than
            // ambient.
            self.bounds_release(),
            crate::interval_bounds::BoundsSemantics {
                registry,
                context: Some(context.as_ref().into()),
                grammar: self.grammar(),
            },
        );
        for f in findings {
            if f.span.is_empty() {
                continue;
            }
            let bound = if f.reason == "negative" {
                "below 0".to_string()
            } else {
                format!("past the end ({})", f.length)
            };
            let rng = if f.reason == "negative" {
                "negative".to_string()
            } else if f.index_interval.lo == f.index_interval.hi {
                format!("is {}", f.index_interval.lo.unwrap_or(0))
            } else {
                let lo = f
                    .index_interval
                    .lo
                    .map_or("-inf".to_string(), |l| l.to_string());
                let hi = f
                    .index_interval
                    .hi
                    .map_or("+inf".to_string(), |h| h.to_string());
                format!("is in [{lo}, {hi}]")
            };
            let outcome = if f.code == DiagCode::W231 {
                "raises 'index out of range' at runtime"
            } else {
                "silently returns the empty string"
            };
            self.interval_index_sites
                .insert((f.code, fu.abs_span(f.span), f.index_var.clone()));
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    f.code,
                    fu.abs_span(f.span),
                    format!(
                        "{}: index ${} {rng}, {bound} \u{2014} {outcome}.",
                        f.command, f.index_var
                    ),
                    Severity::Warning,
                ));
        }
    }

    pub(in super::super) fn emit_invalid_ip_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let mut seen_offsets: FxHashSet<u32> = FxHashSet::default();
        for (key, lv) in fu.diagnostic_value_facts().values() {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // IPv4 candidates.
            for quad in find_dotted_quads(text, 4) {
                let bytes = text.as_bytes();
                if quad.start > 0 && bytes[quad.start - 1] == b'/' {
                    continue;
                }
                // Skip OID-like patterns: the matched quad is a slice of a
                // longer dotted-digit chain (LDAP/SNMP OIDs like
                // ``1.3.6.1.4.1.4203.1.11.3``).  Detect a ``digit.<quad>``
                // before or a ``<quad>.digit`` after.
                let before_dot_digit = quad.start >= 2
                    && bytes[quad.start - 1] == b'.'
                    && bytes[quad.start - 2].is_ascii_digit();
                let after_dot_digit = quad.end + 1 < bytes.len()
                    && bytes[quad.end] == b'.'
                    && bytes[quad.end + 1].is_ascii_digit();
                if before_dot_digit || after_dot_digit {
                    continue;
                }
                let octets = quad.octets;
                let mut diag: Option<(String, Severity)> = None;
                for (i, octet) in octets.iter().enumerate() {
                    let v: u32 = octet.parse().unwrap_or(0);
                    if v > 255 {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) exceeds 255 — this is not a valid IP address.",
                                i + 1,
                                octet,
                            ),
                            Severity::Error,
                        ));
                        break;
                    }
                    if octet.len() > 1
                        && octet.starts_with('0')
                        && octet.bytes().all(|b| (b'0'..=b'7').contains(&b))
                    {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) has a leading zero — may be interpreted as octal in some contexts.",
                                i + 1,
                                octet,
                            ),
                            Severity::Warning,
                        ));
                        break;
                    }
                }
                if let Some((msg, sev)) = diag {
                    let literal = &text[quad.start..quad.end];
                    self.emit_ip_diag_at_def(fu, *key, &msg, sev, Some(literal), &mut seen_offsets);
                    break;
                }
            }

            // IPv6 candidates.
            for candidate in find_ipv6_candidates(text) {
                if Ipv6Addr::from_str(candidate).is_err() && !time_or_mac(candidate) {
                    let msg = format!("Invalid IPv6 address '{candidate}'.");
                    self.emit_ip_diag_at_def(
                        fu,
                        *key,
                        &msg,
                        Severity::Error,
                        Some(candidate),
                        &mut seen_offsets,
                    );
                    break;
                }
            }
        }
    }

    /// Helper for [`Self::emit_invalid_ip_diagnostics`].
    ///
    /// Anchors on the offending IP `literal` within the def statement when
    /// it appears verbatim in the source (with non-address bytes on both
    /// sides), falling back to the whole statement span when the constant
    /// was folded from parts the source never spells out.
    fn emit_ip_diag_at_def(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        key: crate::ssa::ValueKey,
        message: &str,
        severity: Severity,
        literal: Option<&str>,
        seen_offsets: &mut FxHashSet<u32>,
    ) {
        let (sym, version) = key;
        if fu.def_use.symbol(&fu.ssa, sym).is_none() {
            return;
        }
        let Some(chain) = fu.def_use.chain_for(sym, version) else {
            return;
        };
        let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
            return;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            return;
        };
        let Some(stmt) = block.statements.get(idx) else {
            return;
        };
        let stmt_span = fu.abs_span(stmt.span());
        if stmt_span.is_empty() {
            return;
        }
        if !seen_offsets.insert(stmt_span.start()) {
            return;
        }
        // Tight anchor: the literal's own bytes inside the statement, when
        // the source spells it out directly.
        let span = literal
            .and_then(|lit| {
                let slice = source_slice(&self.source, stmt_span)?;
                let is_addr_byte = |b: u8| b.is_ascii_hexdigit() || b == b'.' || b == b':';
                let mut from = 0;
                while let Some(off) = slice[from..].find(lit) {
                    let start = from + off;
                    let end = start + lit.len();
                    let left_ok = start == 0 || !is_addr_byte(slice.as_bytes()[start - 1]);
                    let right_ok = end >= slice.len() || !is_addr_byte(slice.as_bytes()[end]);
                    if left_ok && right_ok {
                        return Some(tcl_lexer::Span::new(
                            stmt_span.start() + u32::try_from(start).ok()?,
                            stmt_span.start() + u32::try_from(end).ok()?,
                        ));
                    }
                    from = start + 1;
                }
                None
            })
            .unwrap_or(stmt_span);
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W124,
                span,
                message.to_string(),
                severity,
            ));
    }
}

/// Collect the bracketed text of every `[…]` command-substitution node in
/// an `expr` AST (recursing operands but stopping at the substitution
/// boundary). Used to recover
/// variable reads hidden inside `if`/`while` conditions and `expr` values.
#[cfg(test)]
pub(super) fn collect_expr_command_texts(node: &ExprNode, out: &mut Vec<String>) {
    // Entry point: the top of an expression tree is nesting depth 0 (the
    // recursion cap lives in [`collect_expr_command_texts_at`]).
    collect_expr_command_texts_at(node, out, 0);
}

#[cfg(test)]
fn collect_expr_command_texts_at(node: &ExprNode, out: &mut Vec<String>, depth: u32) {
    // Native-stack safety net: walks the `ExprNode` tree, one native frame
    // per level. Past the cap, stop descending — a collector
    // that returns the command texts gathered so far is the safe fallback
    // (substitutions buried deeper than the cap are not collected; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, .. } => out.push(text.clone()),
        ExprNode::Binary { left, right, .. } => {
            collect_expr_command_texts_at(left, out, depth + 1);
            collect_expr_command_texts_at(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => collect_expr_command_texts_at(operand, out, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_command_texts_at(condition, out, depth + 1);
            collect_expr_command_texts_at(true_branch, out, depth + 1);
            collect_expr_command_texts_at(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_command_texts_at(arg, out, depth + 1);
            }
        }
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => {}
    }
}

fn time_or_mac(candidate: &str) -> bool {
    let parts: Vec<_> = candidate.split(':').collect();
    if parts.iter().any(|part| part.len() != 2) {
        return false;
    }
    match parts.as_slice() {
        [hours, minutes, seconds] => {
            hours.parse::<u8>().is_ok_and(|value| value < 24)
                && minutes.parse::<u8>().is_ok_and(|value| value < 60)
                && seconds.parse::<u8>().is_ok_and(|value| value <= 60)
        }
        [_, _, _, _, _, _] => parts
            .iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_hexdigit())),
        _ => false,
    }
}

/// Find every IPv6 *candidate* substring — `\b[hex]{1,4}(:[hex]{0,4}){2,7}\b`
/// — in `text` (the caller validates each via `Ipv6Addr::from_str`).
/// Replaces the regex; each candidate begins at a word boundary, has a
/// 1-4 hex-digit first group, 2-7 following `:`-groups (each 0-4 hex),
/// and ends on a hex digit at a trailing word boundary.
pub(in super::super) fn find_ipv6_candidates(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before
            && bytes[i].is_ascii_hexdigit()
            && let Some(end) = match_ipv6_candidate(bytes, i)
        {
            out.push(&text[i..end]);
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Read up to `max` contiguous hex-digit bytes from `start`, returning
/// the count.
fn hex_run_len(bytes: &[u8], start: usize, max: usize) -> usize {
    let mut k = 0;
    while k < max && start + k < bytes.len() && bytes[start + k].is_ascii_hexdigit() {
        k += 1;
    }
    k
}

/// Match an IPv6 candidate starting at `start`, returning the end offset
/// of the longest `hex(:hex?){2,7}` run that ends on a hex digit and is
/// followed by a word boundary, or `None`.
fn match_ipv6_candidate(bytes: &[u8], start: usize) -> Option<usize> {
    let first = hex_run_len(bytes, start, 4);
    if first == 0 {
        return None;
    }
    let mut pos = start + first;
    let mut groups = 0usize;
    let mut best: Option<usize> = None;
    while groups < 7 && bytes.get(pos) == Some(&b':') {
        let after_colon = pos + 1;
        let h = hex_run_len(bytes, after_colon, 4);
        pos = after_colon + h;
        groups += 1;
        // A valid `\b`-terminated end: ≥2 groups, ends on a hex digit,
        // and is followed by a non-word byte (or end of input).
        if groups >= 2 && h >= 1 && (pos >= bytes.len() || !is_word_byte(bytes[pos])) {
            best = Some(pos);
        }
    }
    best
}

/// Must-policy over a channel argument's type union: `Some(label)` when
/// EVERY member is a non-channel — whatever path ran, the value cannot be a
/// channel — with the members rendered `"INT | STRING"` for the message. A
/// union with any Channel member, or an Unknown / Overdefined node, returns
/// `None`: some path may be fine.
fn non_channel_union_label(var_type: &crate::types::TypeLattice) -> Option<String> {
    use crate::types::{TypeKind, TypeShape};
    if !matches!(var_type.kind(), TypeKind::Known | TypeKind::Shimmered) {
        return None;
    }
    let member_types: Vec<tcl_registry::TclType> =
        var_type.shapes().iter().map(TypeShape::coarse).collect();
    if member_types.is_empty()
        || member_types
            .iter()
            .any(|t| matches!(t, tcl_registry::TclType::Channel))
    {
        return None;
    }
    Some(
        member_types
            .iter()
            .map(|t| format!("{t:?}").to_uppercase())
            .collect::<Vec<_>>()
            .join(" | "),
    )
}
