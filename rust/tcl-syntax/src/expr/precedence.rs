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

//! Operator-precedence views of an original source expression: which grouping
//! parentheses the parse needs, which unparenthesised groupings are likely
//! mistakes, and the fully- and minimally-parenthesised spellings.
//!
//! Everything works on an [`OriginalSourceExpression`], so the dialect's own
//! expression grammar — including the F5 word operators (`contains`,
//! `starts_with`, `and`, `not`, …) — decides the tree. Each rewrite is
//! re-parsed under the same grammar and returned only when it yields the same
//! tree, so a rewrite never changes how Tcl groups the expression.

use std::collections::HashMap;

use tcl_lexer::{ExprToken, ExprTokenType, Span};

use super::ast::{BinOp, ChildSlot, ExprNode, UnaryOp, render_expr};
use super::substitution::OriginalSourceExpression;

/// Deepest tree the views walk; deeper expressions get no view.
const MAX_DEPTH: u32 = 256;

/// One pair of grouping parentheses (not a math-function call's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupingParen {
    /// Byte offset of `(`.
    pub open: u32,
    /// Byte offset of `)`.
    pub close: u32,
    /// Whether removing this pair would change the parse.
    pub needed: bool,
}

/// An unparenthesised grouping that commonly reads differently from how Tcl
/// parses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedenceHazard {
    /// The operator that takes the tighter-bound subexpression as an operand.
    pub outer: String,
    /// The tighter-binding operator nested under it.
    pub inner: String,
    /// The outer operator's token.
    pub operator: Span,
    /// The whole hazardous subexpression.
    pub extent: Span,
    /// The subexpression with the parse's grouping made explicit.
    pub as_parsed: String,
    /// The grouping a reader most likely intended.
    pub likely_intended: String,
}

/// A node's source extent, including any parentheses wrapping it.
#[derive(Clone, Copy)]
struct Extent {
    start: usize,
    end: usize,
    wrapped: bool,
}

struct Geometry<'a> {
    source: &'a str,
    tokens: Vec<ExprToken<Vec<u8>>>,
    by_start: HashMap<u32, usize>,
    by_end: HashMap<u32, usize>,
    partner: HashMap<usize, usize>,
    call_parens: Vec<usize>,
    extents: HashMap<*const ExprNode, Extent>,
    parens: Vec<GroupingParen>,
}

impl<'a> Geometry<'a> {
    fn capture(expression: &'a OriginalSourceExpression) -> Option<Self> {
        let context = expression.context();
        let (tokens, failures) = tcl_lexer::tokenise_expr_bytes_with_config_and_failures(
            expression.source().as_bytes(),
            &context.lexer_grammar,
            context.expr_grammar_base,
            context.f5_word_grammar,
            expression.config(),
        );
        if !failures.is_empty() {
            return None;
        }
        let tokens: Vec<_> = tokens
            .into_iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    ExprTokenType::Whitespace | ExprTokenType::Comment | ExprTokenType::Eof
                )
            })
            .collect();
        let mut partner = HashMap::new();
        let mut call_parens = Vec::new();
        let mut stack = Vec::new();
        for (index, token) in tokens.iter().enumerate() {
            match token.kind {
                ExprTokenType::ParenOpen => stack.push(index),
                ExprTokenType::ParenClose => {
                    let open = stack.pop()?;
                    partner.insert(open, index);
                    partner.insert(index, open);
                    if open > 0 && tokens[open - 1].kind == ExprTokenType::Function {
                        call_parens.push(open);
                    }
                }
                _ => {}
            }
        }
        if !stack.is_empty() {
            return None;
        }
        let by_start = tokens.iter().enumerate().map(|(i, t)| (t.start, i)).collect();
        let by_end = tokens.iter().enumerate().map(|(i, t)| (t.end, i)).collect();
        let mut geometry = Self {
            source: expression.source(),
            tokens,
            by_start,
            by_end,
            partner,
            call_parens,
            extents: HashMap::new(),
            parens: Vec::new(),
        };
        geometry.measure(expression.root(), None, 0)?;
        let grouping = geometry
            .tokens
            .iter()
            .enumerate()
            .filter(|(i, t)| t.kind == ExprTokenType::ParenOpen && !geometry.call_parens.contains(i))
            .count();
        (grouping == geometry.parens.len()).then_some(geometry)
    }

    /// Token indices of `node`, absorbing grouping parentheses around it.
    fn measure(&mut self, node: &ExprNode, slot: Option<ChildSlot>, depth: u32) -> Option<Extent> {
        if depth > MAX_DEPTH {
            return None;
        }
        let (mut start, mut end) = match node {
            ExprNode::Literal { start, end, .. }
            | ExprNode::String { start, end, .. }
            | ExprNode::Var { start, end, .. }
            | ExprNode::Command { start, end, .. } => {
                (*self.by_start.get(start)?, *self.by_end.get(end)?)
            }
            ExprNode::Call {
                args, start, end, ..
            } => {
                for arg in args {
                    self.measure(arg, Some(ChildSlot::CallArgument), depth + 1)?;
                }
                (*self.by_start.get(start)?, *self.by_end.get(end)?)
            }
            ExprNode::Binary { op, left, right } => {
                let l = self.measure(left, Some(ChildSlot::BinaryLeft(*op)), depth + 1)?;
                let r = self.measure(right, Some(ChildSlot::BinaryRight(*op)), depth + 1)?;
                (l.start, r.end)
            }
            ExprNode::Unary { op, operand } => {
                let inner = self.measure(operand, Some(ChildSlot::Unary), depth + 1)?;
                let at = inner.start.checked_sub(1)?;
                let token = &self.tokens[at];
                if token.kind != ExprTokenType::Operator || token.text != op.as_str().as_bytes() {
                    return None;
                }
                (at, inner.end)
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                let c = self.measure(condition, Some(ChildSlot::TernaryCondition), depth + 1)?;
                self.measure(true_branch, Some(ChildSlot::TernaryBranch), depth + 1)?;
                let f = self.measure(false_branch, Some(ChildSlot::TernaryBranch), depth + 1)?;
                (c.start, f.end)
            }
            ExprNode::CompiledWord { .. } | ExprNode::Raw { .. } => return None,
        };
        let mut wrapped = false;
        while start > 0
            && self.tokens[start - 1].kind == ExprTokenType::ParenOpen
            && !self.call_parens.contains(&(start - 1))
            && self.partner.get(&(start - 1)) == Some(&(end + 1))
        {
            let needed = !wrapped && slot.is_some_and(|slot| slot.needs_parens(node));
            self.parens.push(GroupingParen {
                open: self.tokens[start - 1].start,
                close: self.tokens[end + 1].start,
                needed,
            });
            wrapped = true;
            start -= 1;
            end += 1;
        }
        let extent = Extent {
            start,
            end,
            wrapped,
        };
        self.extents.insert(std::ptr::from_ref(node), extent);
        Some(extent)
    }

    fn extent(&self, node: &ExprNode) -> Option<Extent> {
        self.extents.get(&std::ptr::from_ref(node)).copied()
    }

    fn text(&self, extent: Extent) -> Option<&'a str> {
        let start = self.tokens[extent.start].start as usize;
        let end = self.tokens[extent.end].end as usize + 1;
        self.source.get(start..end)
    }

    fn span(&self, extent: Extent) -> Span {
        Span::new(
            self.tokens[extent.start].start,
            self.tokens[extent.end].end + 1,
        )
    }

    /// The operator token between a binary node's operands.
    fn operator_span(&self, left: &ExprNode, right: &ExprNode) -> Option<Span> {
        let (l, r) = (self.extent(left)?, self.extent(right)?);
        (r.start == l.end + 2).then(|| {
            let token = &self.tokens[l.end + 1];
            Span::new(token.start, token.end + 1)
        })
    }
}

impl OriginalSourceExpression {
    /// Every grouping parenthesis pair, with whether the parse needs it.
    /// `None` when the source's parentheses cannot be matched to the tree.
    #[must_use]
    pub fn grouping_parens(&self) -> Option<Vec<GroupingParen>> {
        let mut parens = Geometry::capture(self)?.parens;
        parens.sort_by_key(|paren| paren.open);
        Some(parens)
    }

    /// Unparenthesised groupings that are probably not what the author meant:
    /// a comparison as an operand of `&`, `|` or `^` (`$x & 0xff == 1` is
    /// `$x & (0xff == 1)`), `+`/`-` under a shift (`1 << $n - 1` is
    /// `1 << ($n - 1)`), and a logical NOT as a comparison's left operand
    /// (`!$x == 1` is `(!$x) == 1`).
    #[must_use]
    pub fn precedence_hazards(&self) -> Vec<PrecedenceHazard> {
        let Some(geometry) = Geometry::capture(self) else {
            return Vec::new();
        };
        let mut hazards = Vec::new();
        for node in self.root().evaluation_nodes() {
            if let Some(hazard) = hazard_at(&geometry, node) {
                hazards.push(hazard);
            }
        }
        hazards.sort_by_key(|hazard| hazard.operator.start());
        hazards
    }

    /// The source with every parenthesis pair the parse does not need
    /// removed. `None` when there is none, or when the result would not
    /// re-parse to the same tree.
    #[must_use]
    pub fn without_redundant_parens(&self) -> Option<String> {
        let geometry = Geometry::capture(self)?;
        let mut removals: Vec<u32> = geometry
            .parens
            .iter()
            .filter(|paren| !paren.needed)
            .flat_map(|paren| [paren.open, paren.close])
            .collect();
        if removals.is_empty() {
            return None;
        }
        removals.sort_unstable();
        let bytes = self.source().as_bytes();
        let mut out = String::with_capacity(bytes.len());
        let mut at = 0usize;
        for offset in removals {
            let offset = offset as usize;
            out.push_str(self.source().get(at..offset)?);
            let before = out.as_bytes().last().copied();
            let after = bytes.get(offset + 1).copied();
            if before.is_some_and(joins) && after.is_some_and(joins) {
                out.push(' ');
            }
            at = offset + 1;
        }
        out.push_str(self.source().get(at..)?);
        let trimmed = out.trim();
        self.same_parse(trimmed).then(|| trimmed.to_owned())
    }

    /// The expression with every operator's operands explicitly grouped.
    /// `None` when it is already fully grouped or would not re-parse to the
    /// same tree.
    #[must_use]
    pub fn fully_parenthesised(&self) -> Option<String> {
        let geometry = Geometry::capture(self)?;
        let rendered = render_grouped(&geometry, self.root(), 0)?;
        (rendered != self.source().trim() && self.same_parse(&rendered)).then_some(rendered)
    }

    /// The parse as an indented tree drawn with box-drawing connectors, one
    /// node per line: operators with their precedence level (higher binds
    /// tighter), operands as written.
    #[must_use]
    pub fn precedence_tree(&self) -> Option<String> {
        let geometry = Geometry::capture(self)?;
        let mut out = String::new();
        draw(&geometry, self.root(), "", None, &mut out, 0)?;
        Some(out)
    }

    fn same_parse(&self, candidate: &str) -> bool {
        OriginalSourceExpression::capture(candidate, self.context(), self.config())
            .is_some_and(|other| render_expr(other.root()) == render_expr(self.root()))
    }
}

/// Bytes that would fuse with a neighbour if the parenthesis between them
/// were deleted outright.
fn joins(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.' | b'"' | b'{' | b'}' | b']')
}

const fn is_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::StrEq
            | BinOp::StrNe
            | BinOp::StrLt
            | BinOp::StrLe
            | BinOp::StrGt
            | BinOp::StrGe
            | BinOp::In
            | BinOp::Ni
            | BinOp::Contains
            | BinOp::StartsWith
            | BinOp::EndsWith
            | BinOp::StrEquals
            | BinOp::Matches
            | BinOp::MatchesGlob
            | BinOp::MatchesRegex
    )
}

const fn suspicious_child(outer: BinOp, inner: BinOp) -> bool {
    match outer {
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => is_comparison(inner),
        BinOp::LShift | BinOp::RShift => matches!(inner, BinOp::Add | BinOp::Sub),
        _ => false,
    }
}

fn hazard_at(geometry: &Geometry<'_>, node: &ExprNode) -> Option<PrecedenceHazard> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let operator = geometry.operator_span(left, right)?;
    let whole = geometry.span(geometry.extent(node)?);
    let text = |n: &ExprNode| geometry.extent(n).and_then(|e| geometry.text(e));
    if let ExprNode::Binary {
        op: inner,
        left: inner_left,
        right: inner_right,
    } = right.as_ref()
        && suspicious_child(*op, *inner)
        && !geometry.extent(right)?.wrapped
    {
        let (a, b, c) = (text(left)?, text(inner_left)?, text(inner_right)?);
        return Some(PrecedenceHazard {
            outer: op.as_str().to_owned(),
            inner: inner.as_str().to_owned(),
            operator,
            extent: whole,
            as_parsed: format!("{a} {op} ({b} {inner} {c})"),
            likely_intended: format!("({a} {op} {b}) {inner} {c}"),
        });
    }
    if let ExprNode::Binary {
        op: inner,
        left: inner_left,
        right: inner_right,
    } = left.as_ref()
        && suspicious_child(*op, *inner)
        && !geometry.extent(left)?.wrapped
    {
        let (a, b, c) = (text(inner_left)?, text(inner_right)?, text(right)?);
        return Some(PrecedenceHazard {
            outer: op.as_str().to_owned(),
            inner: inner.as_str().to_owned(),
            operator,
            extent: whole,
            as_parsed: format!("({a} {inner} {b}) {op} {c}"),
            likely_intended: format!("{a} {inner} ({b} {op} {c})"),
        });
    }
    let is_not = |n: &ExprNode| {
        matches!(
            n,
            ExprNode::Unary {
                op: UnaryOp::Not | UnaryOp::WordNot,
                ..
            }
        )
    };
    if is_comparison(*op)
        && let ExprNode::Unary {
            op: not,
            operand,
        } = left.as_ref()
        && is_not(left)
        && !is_not(right)
        && !geometry.extent(left)?.wrapped
        && !matches!(operand.as_ref(), ExprNode::Binary { .. } | ExprNode::Ternary { .. })
    {
        let (a, c) = (text(operand)?, text(right)?);
        let space = if *not == UnaryOp::WordNot { " " } else { "" };
        return Some(PrecedenceHazard {
            outer: op.as_str().to_owned(),
            inner: not.as_str().to_owned(),
            operator,
            extent: whole,
            as_parsed: format!("({not}{space}{a}) {op} {c}"),
            likely_intended: format!("{not}{space}({a} {op} {c})"),
        });
    }
    None
}

fn render_grouped(geometry: &Geometry<'_>, node: &ExprNode, depth: u32) -> Option<String> {
    if depth > MAX_DEPTH {
        return None;
    }
    let child = |slot: ChildSlot, n: &ExprNode| -> Option<String> {
        let text = render_grouped(geometry, n, depth + 1)?;
        let composite = matches!(n, ExprNode::Binary { .. } | ExprNode::Ternary { .. });
        Some(
            if composite && !matches!(slot, ChildSlot::CallArgument) {
                format!("({text})")
            } else {
                text
            },
        )
    };
    Some(match node {
        ExprNode::Binary { op, left, right } => format!(
            "{} {op} {}",
            child(ChildSlot::BinaryLeft(*op), left)?,
            child(ChildSlot::BinaryRight(*op), right)?
        ),
        ExprNode::Unary { op, operand } => {
            let space = if *op == UnaryOp::WordNot { " " } else { "" };
            format!("{op}{space}{}", child(ChildSlot::Unary, operand)?)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => format!(
            "{} ? {} : {}",
            child(ChildSlot::TernaryCondition, condition)?,
            child(ChildSlot::TernaryBranch, true_branch)?,
            child(ChildSlot::TernaryBranch, false_branch)?
        ),
        ExprNode::Call { function, args, .. } => {
            let args = args
                .iter()
                .map(|arg| child(ChildSlot::CallArgument, arg))
                .collect::<Option<Vec<_>>>()?;
            format!("{function}({})", args.join(", "))
        }
        leaf => {
            let mut extent = geometry.extent(leaf)?;
            while extent.wrapped && extent.start < extent.end {
                extent.start += 1;
                extent.end -= 1;
                extent.wrapped = geometry.tokens[extent.start].kind == ExprTokenType::ParenOpen
                    && geometry.partner.get(&extent.start) == Some(&extent.end);
            }
            geometry.text(extent)?.to_owned()
        }
    })
}

fn draw(
    geometry: &Geometry<'_>,
    node: &ExprNode,
    prefix: &str,
    last: Option<bool>,
    out: &mut String,
    depth: u32,
) -> Option<()> {
    if depth > MAX_DEPTH {
        return None;
    }
    let (connector, child_prefix) = match last {
        None => ("", String::new()),
        Some(true) => ("└─ ", format!("{prefix}   ")),
        Some(false) => ("├─ ", format!("{prefix}│  ")),
    };
    let label = match node {
        ExprNode::Binary { op, .. } => format!("{op}  (precedence {})", op.precedence()),
        ExprNode::Unary { op, .. } => format!("{op}  (unary)"),
        ExprNode::Ternary { .. } => "?:  (ternary)".to_owned(),
        ExprNode::Call { function, .. } => format!("{function}()"),
        leaf => render_grouped(geometry, leaf, depth)?,
    };
    out.push_str(prefix);
    out.push_str(connector);
    out.push_str(&label);
    out.push('\n');
    let children = node.slotted_children();
    let count = children.len();
    for (index, (_, child)) in children.into_iter().enumerate() {
        draw(geometry, child, &child_prefix, Some(index + 1 == count), out, depth + 1)?;
    }
    Some(())
}

#[cfg(test)]
mod tests;
