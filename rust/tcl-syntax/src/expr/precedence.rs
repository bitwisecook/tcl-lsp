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

/// How strongly an unparenthesised grouping suggests a misreading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HazardTier {
    /// No shared reading convention covers this mix, and it is a common
    /// source of bugs: a comparison under `&`, a comparison chain, `!` before
    /// a comparison, `&&` inside `||`, arithmetic as a ternary condition, …
    Misreading,
    /// Any other grouping that changes the result if regrouped, including
    /// the conventional arithmetic hierarchy (`$a + $b * $c`), same-level
    /// chains (`$a - $b + $c`) and comparisons under `&&`.
    NonAssociative,
}

/// An unparenthesised operand whose other grouping gives a different result.
/// Only a repeated associative operator (`+`, `*`, `&`, `|`, `^`, `&&`,
/// `||` and the F5 `and`/`or`) is exempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedenceHazard {
    /// Whether the grouping is a likely misreading or merely non-associative.
    pub tier: HazardTier,
    /// The operator that takes the subexpression as an operand.
    pub outer: String,
    /// The operator of the unparenthesised operand.
    pub inner: String,
    /// The outer operator's token (`?` for a ternary).
    pub operator: Span,
    /// The whole subexpression the two groupings span.
    pub extent: Span,
    /// Why the parse groups this way.
    pub reason: String,
    /// The subexpression with the parse's grouping made explicit.
    pub as_parsed: String,
    /// The other grouping, or for a comparison chain the conjunction a
    /// reader most likely meant.
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
        let by_start = tokens
            .iter()
            .enumerate()
            .map(|(i, t)| (t.start, i))
            .collect();
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
            .filter(|(i, t)| {
                t.kind == ExprTokenType::ParenOpen && !geometry.call_parens.contains(i)
            })
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

    /// Every unparenthesised operand whose other grouping would give a
    /// different result, in source order of the outer operator.
    #[must_use]
    pub fn precedence_hazards(&self) -> Vec<PrecedenceHazard> {
        let Some(geometry) = Geometry::capture(self) else {
            return Vec::new();
        };
        let mut hazards = Vec::new();
        for node in self.root().evaluation_nodes() {
            hazards.extend(hazards_at(&geometry, node));
        }
        hazards.sort_by_key(|hazard| (hazard.operator.start(), hazard.extent.start()));
        hazards
    }

    /// Edits removing every parenthesis the parse does not need: each is a
    /// byte offset of a `(` or `)` and its replacement, a space where the
    /// nearest non-blank neighbours would otherwise fuse into one token, else
    /// nothing. The choice ignores surrounding whitespace, so the edits stay
    /// valid when that whitespace is compacted too. `None` when there is
    /// nothing to remove or the result would not re-parse to the same tree.
    #[must_use]
    pub fn redundant_paren_edits(&self) -> Option<Vec<(u32, &'static str)>> {
        let geometry = Geometry::capture(self)?;
        let mut offsets: Vec<u32> = geometry
            .parens
            .iter()
            .filter(|paren| !paren.needed)
            .flat_map(|paren| [paren.open, paren.close])
            .collect();
        if offsets.is_empty() {
            return None;
        }
        offsets.sort_unstable();
        let bytes = self.source().as_bytes();
        let removed = |i: usize| {
            offsets
                .binary_search(&u32::try_from(i).unwrap_or(u32::MAX))
                .is_ok()
        };
        let neighbour = |from: usize, forward: bool| {
            let mut i = from;
            loop {
                i = if forward {
                    i.checked_add(1)?
                } else {
                    i.checked_sub(1)?
                };
                let byte = *bytes.get(i)?;
                if !byte.is_ascii_whitespace() && !removed(i) {
                    return Some(byte);
                }
            }
        };
        let edits: Vec<(u32, &'static str)> = offsets
            .iter()
            .map(|&offset| {
                let at = offset as usize;
                let fuse = neighbour(at, false).is_some_and(joins)
                    && neighbour(at, true).is_some_and(joins);
                (offset, if fuse { " " } else { "" })
            })
            .collect();
        let mut out = String::with_capacity(bytes.len());
        let mut at = 0usize;
        for &(offset, replacement) in &edits {
            out.push_str(self.source().get(at..offset as usize)?);
            out.push_str(replacement);
            at = offset as usize + 1;
        }
        out.push_str(self.source().get(at..)?);
        self.same_parse(&out).then_some(edits)
    }

    /// The source with every parenthesis pair the parse does not need
    /// removed, trimmed. `None` when there is none, or when the result would
    /// not re-parse to the same tree.
    #[must_use]
    pub fn without_redundant_parens(&self) -> Option<String> {
        let edits = self.redundant_paren_edits()?;
        let mut out = String::with_capacity(self.source().len());
        let mut at = 0usize;
        for (offset, replacement) in edits {
            out.push_str(self.source().get(at..offset as usize)?);
            let spaced = out.ends_with(|c: char| c.is_ascii_whitespace())
                || self.source()[offset as usize + 1..]
                    .starts_with(|c: char| c.is_ascii_whitespace());
            if !spaced {
                out.push_str(replacement);
            }
            at = offset as usize + 1;
        }
        out.push_str(self.source().get(at..)?);
        let trimmed = out.trim();
        self.same_parse(trimmed).then(|| trimmed.to_owned())
    }

    /// Whether `candidate` parses, under this expression's grammar, to the
    /// same tree.
    #[must_use]
    pub fn same_parse_as(&self, candidate: &str) -> bool {
        self.same_parse(candidate)
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
        draw(&geometry, self.root(), "", None, None, &mut out, 0)?;
        Some(out)
    }

    /// [`Self::precedence_tree`] for a cursor at byte `offset`, marking the
    /// node whose operator or grouping parenthesis is there. `None` unless
    /// `offset` is on an operator, `?`, `:` or parenthesis token.
    #[must_use]
    pub fn precedence_tree_at(&self, offset: u32) -> Option<String> {
        let geometry = Geometry::capture(self)?;
        let token = geometry
            .tokens
            .iter()
            .position(|t| t.start <= offset && offset <= t.end)?;
        if !matches!(
            geometry.tokens[token].kind,
            ExprTokenType::Operator
                | ExprTokenType::ParenOpen
                | ExprTokenType::ParenClose
                | ExprTokenType::TernaryQ
                | ExprTokenType::TernaryC
        ) {
            return None;
        }
        let mut out = String::new();
        draw(&geometry, self.root(), "", None, Some(token), &mut out, 0)?;
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
    byte.is_ascii_alphanumeric()
        || matches!(byte, b'_' | b'$' | b'.' | b'"' | b'{' | b'}' | b'[' | b']')
}

/// Operator families for deciding whether a mix has a reading convention.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Arithmetic,
    Shift,
    Relational,
    Equality,
    Bitwise,
    LogicalAnd,
    LogicalOr,
}

const fn family(op: BinOp) -> Family {
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
            Family::Arithmetic
        }
        BinOp::LShift | BinOp::RShift => Family::Shift,
        BinOp::Lt
        | BinOp::Le
        | BinOp::Gt
        | BinOp::Ge
        | BinOp::StrLt
        | BinOp::StrLe
        | BinOp::StrGt
        | BinOp::StrGe => Family::Relational,
        BinOp::Eq
        | BinOp::Ne
        | BinOp::StrEq
        | BinOp::StrNe
        | BinOp::In
        | BinOp::Ni
        | BinOp::Contains
        | BinOp::StartsWith
        | BinOp::EndsWith
        | BinOp::StrEquals
        | BinOp::Matches
        | BinOp::MatchesGlob
        | BinOp::MatchesRegex => Family::Equality,
        BinOp::BitAnd | BinOp::BitXor | BinOp::BitOr => Family::Bitwise,
        BinOp::And | BinOp::WordAnd => Family::LogicalAnd,
        BinOp::Or | BinOp::WordOr => Family::LogicalOr,
    }
}

const fn is_comparison(family: Family) -> bool {
    matches!(family, Family::Relational | Family::Equality)
}

/// Whether `(a op b) op c` and `a op (b op c)` always agree.
fn associative(outer: BinOp, inner: BinOp) -> bool {
    let same = outer == inner
        || (family(outer) == Family::LogicalAnd && family(inner) == Family::LogicalAnd)
        || (family(outer) == Family::LogicalOr && family(inner) == Family::LogicalOr);
    same && matches!(
        outer,
        BinOp::Add
            | BinOp::Mul
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::And
            | BinOp::WordAnd
            | BinOp::Or
            | BinOp::WordOr
    )
}

fn is_boolean_literal(node: &ExprNode) -> bool {
    matches!(node, ExprNode::Literal { text, .. }
        if matches!(text.trim(), "0" | "1") || crate::boolean::parse_boolean_word(text.trim()).is_some())
}

/// The tier of a binary operand nested unparenthesised under a binary
/// operator. `other` is the outer operator's other operand.
fn binary_tier(outer: BinOp, inner: BinOp, other: &ExprNode) -> HazardTier {
    let (o, i) = (family(outer), family(inner));
    let misreading = match o {
        Family::Bitwise => i != Family::LogicalAnd && i != Family::LogicalOr,
        Family::Shift => i == Family::Arithmetic,
        Family::Relational | Family::Equality => {
            is_comparison(i) && !(o == Family::Equality && is_boolean_literal(other))
        }
        Family::LogicalOr => i == Family::LogicalAnd,
        _ => false,
    };
    if misreading {
        HazardTier::Misreading
    } else {
        HazardTier::NonAssociative
    }
}

fn is_not(node: &ExprNode) -> bool {
    matches!(
        node,
        ExprNode::Unary {
            op: UnaryOp::Not | UnaryOp::WordNot,
            ..
        }
    )
}

fn unary_text(op: UnaryOp, operand: &str) -> String {
    if op == UnaryOp::WordNot {
        format!("{op} {operand}")
    } else {
        format!("{op}{operand}")
    }
}

fn precedence_reason(outer: BinOp, inner: BinOp, inner_on_left: bool) -> String {
    let (o, i) = (outer.precedence(), inner.precedence());
    if is_comparison(family(outer)) && is_comparison(family(inner)) && o == i {
        format!("comparisons do not chain: '{outer}' compares the result of '{inner}'")
    } else if o == i {
        let direction = if inner_on_left {
            "left to right"
        } else {
            "right to left"
        };
        format!("'{inner}' and '{outer}' share a precedence level and group {direction}")
    } else {
        format!("'{inner}' binds tighter than '{outer}'")
    }
}

struct HazardSite<'g, 'a> {
    geometry: &'g Geometry<'a>,
    operator: Span,
    extent: Span,
}

impl HazardSite<'_, '_> {
    fn text(&self, node: &ExprNode) -> Option<String> {
        let extent = self.geometry.extent(node)?;
        self.geometry.text(extent).map(str::to_owned)
    }

    fn wrapped(&self, node: &ExprNode) -> bool {
        self.geometry.extent(node).is_none_or(|e| e.wrapped)
    }

    fn hazard(
        &self,
        tier: HazardTier,
        outer: &str,
        inner: &str,
        reason: String,
        as_parsed: String,
        likely_intended: String,
    ) -> PrecedenceHazard {
        PrecedenceHazard {
            tier,
            outer: outer.to_owned(),
            inner: inner.to_owned(),
            operator: self.operator,
            extent: self.extent,
            reason,
            as_parsed,
            likely_intended,
        }
    }
}

fn hazards_at(geometry: &Geometry<'_>, node: &ExprNode) -> Vec<PrecedenceHazard> {
    match node {
        ExprNode::Binary { op, left, right } => binary_hazards(geometry, node, *op, left, right),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ternary_hazards(geometry, node, condition, true_branch, false_branch),
        _ => None,
    }
    .unwrap_or_default()
}

fn binary_hazards(
    geometry: &Geometry<'_>,
    node: &ExprNode,
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
) -> Option<Vec<PrecedenceHazard>> {
    let site = HazardSite {
        geometry,
        operator: geometry.operator_span(left, right)?,
        extent: geometry.span(geometry.extent(node)?),
    };
    let mut out = Vec::new();
    if let ExprNode::Binary {
        op: inner,
        left: b,
        right: c,
    } = right
        && !site.wrapped(right)
        && !associative(op, *inner)
    {
        let (a, b, c) = (site.text(left)?, site.text(b)?, site.text(c)?);
        let tier = binary_tier(op, *inner, left);
        let chain = tier == HazardTier::Misreading
            && is_comparison(family(op))
            && is_comparison(family(*inner));
        out.push(site.hazard(
            tier,
            op.as_str(),
            inner.as_str(),
            precedence_reason(op, *inner, false),
            format!("{a} {op} ({b} {inner} {c})"),
            if chain {
                format!("{a} {op} {b} && {b} {inner} {c}")
            } else {
                format!("({a} {op} {b}) {inner} {c}")
            },
        ));
    }
    match left {
        ExprNode::Binary {
            op: inner,
            left: a,
            right: b,
        } if !site.wrapped(left) && !associative(op, *inner) => {
            let (a, b, c) = (site.text(a)?, site.text(b)?, site.text(right)?);
            let tier = binary_tier(op, *inner, right);
            let chain = tier == HazardTier::Misreading
                && is_comparison(family(op))
                && is_comparison(family(*inner));
            out.push(site.hazard(
                tier,
                op.as_str(),
                inner.as_str(),
                precedence_reason(op, *inner, true),
                format!("({a} {inner} {b}) {op} {c}"),
                if chain {
                    format!("{a} {inner} {b} && {b} {op} {c}")
                } else {
                    format!("{a} {inner} ({b} {op} {c})")
                },
            ));
        }
        ExprNode::Unary { op: unary, operand }
            if !site.wrapped(left) && !unary_associative(*unary, op) =>
        {
            let (x, c) = (site.text(operand)?, site.text(right)?);
            let family = family(op);
            let misreading = (is_not(left)
                && (family == Family::Bitwise || (is_comparison(family) && !is_not(right))))
                || (matches!(unary, UnaryOp::Neg) && op == BinOp::Pow);
            out.push(site.hazard(
                if misreading {
                    HazardTier::Misreading
                } else {
                    HazardTier::NonAssociative
                },
                op.as_str(),
                unary.as_str(),
                format!("unary '{unary}' binds tighter than '{op}'"),
                format!("({}) {op} {c}", unary_text(*unary, &x)),
                unary_text(*unary, &format!("({x} {op} {c})")),
            ));
        }
        _ => {}
    }
    Some(out)
}

/// Whether a unary operator distributes over this binary operator, so that
/// `(-a) op b` and `-(a op b)` always agree.
fn unary_associative(unary: UnaryOp, op: BinOp) -> bool {
    match unary {
        UnaryOp::Pos => op != BinOp::Pow,
        UnaryOp::Neg => op == BinOp::Mul,
        _ => false,
    }
}

fn ternary_hazards(
    geometry: &Geometry<'_>,
    node: &ExprNode,
    condition: &ExprNode,
    true_branch: &ExprNode,
    false_branch: &ExprNode,
) -> Option<Vec<PrecedenceHazard>> {
    let question = geometry.extent(condition)?.end + 1;
    let token = &geometry.tokens[question];
    let site = HazardSite {
        geometry,
        operator: Span::new(token.start, token.end + 1),
        extent: geometry.span(geometry.extent(node)?),
    };
    let (then, otherwise) = (site.text(true_branch)?, site.text(false_branch)?);
    let mut out = Vec::new();
    if !site.wrapped(condition) {
        match condition {
            ExprNode::Binary { op, left, right } => {
                let (a, b) = (site.text(left)?, site.text(right)?);
                let misreading = matches!(
                    family(*op),
                    Family::Arithmetic | Family::Shift | Family::Bitwise
                );
                out.push(site.hazard(
                    if misreading {
                        HazardTier::Misreading
                    } else {
                        HazardTier::NonAssociative
                    },
                    "?:",
                    op.as_str(),
                    format!("'{op}' binds tighter than '?:'"),
                    format!("({a} {op} {b}) ? {then} : {otherwise}"),
                    format!("{a} {op} ({b} ? {then} : {otherwise})"),
                ));
            }
            ExprNode::Unary { op, operand } => {
                let x = site.text(operand)?;
                out.push(site.hazard(
                    HazardTier::NonAssociative,
                    "?:",
                    op.as_str(),
                    format!("unary '{op}' binds tighter than '?:'"),
                    format!("({}) ? {then} : {otherwise}", unary_text(*op, &x)),
                    unary_text(*op, &format!("({x} ? {then} : {otherwise})")),
                ));
            }
            _ => {}
        }
    }
    if !site.wrapped(false_branch) {
        let test = site.text(condition)?;
        let split = match false_branch {
            ExprNode::Binary { op, left, right } => Some((
                op.as_str().to_owned(),
                site.text(left)?,
                format!("{op} {}", site.text(right)?),
            )),
            ExprNode::Ternary {
                condition: inner,
                true_branch: inner_then,
                false_branch: inner_else,
            } => Some((
                "?:".to_owned(),
                site.text(inner)?,
                format!("? {} : {}", site.text(inner_then)?, site.text(inner_else)?),
            )),
            _ => None,
        };
        if let Some((inner, head, tail)) = split {
            out.push(site.hazard(
                HazardTier::NonAssociative,
                "?:",
                &inner,
                "the else branch of '?:' extends to the end of the expression".to_owned(),
                format!("{test} ? {then} : ({head} {tail})"),
                format!("({test} ? {then} : {head}) {tail}"),
            ));
        }
    }
    Some(out)
}

fn render_grouped(geometry: &Geometry<'_>, node: &ExprNode, depth: u32) -> Option<String> {
    if depth > MAX_DEPTH {
        return None;
    }
    let child = |slot: ChildSlot, n: &ExprNode| -> Option<String> {
        let text = render_grouped(geometry, n, depth + 1)?;
        let composite = matches!(n, ExprNode::Binary { .. } | ExprNode::Ternary { .. });
        Some(if composite && !matches!(slot, ChildSlot::CallArgument) {
            format!("({text})")
        } else {
            text
        })
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

/// Whether the token at `focus` belongs to `node` itself: its operator, its
/// ternary punctuation or a parenthesis wrapping it.
fn owns_token(geometry: &Geometry<'_>, node: &ExprNode, focus: usize) -> bool {
    let Some(extent) = geometry.extent(node) else {
        return false;
    };
    let own = match node {
        ExprNode::Binary { left, right, .. } => {
            geometry.extent(left).is_some_and(|l| l.end + 1 == focus)
                && geometry.extent(right).is_some_and(|r| r.start == focus + 1)
        }
        ExprNode::Unary { operand, .. } => geometry
            .extent(operand)
            .is_some_and(|o| o.start == focus + 1),
        ExprNode::Ternary {
            true_branch,
            false_branch,
            ..
        } => {
            geometry
                .extent(true_branch)
                .is_some_and(|t| t.start == focus + 1)
                || geometry
                    .extent(false_branch)
                    .is_some_and(|f| f.start == focus + 1)
        }
        _ => false,
    };
    own || (extent.wrapped && (focus == extent.start || focus == extent.end))
}

fn draw(
    geometry: &Geometry<'_>,
    node: &ExprNode,
    prefix: &str,
    last: Option<bool>,
    focus: Option<usize>,
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
    if focus.is_some_and(|focus| owns_token(geometry, node, focus)) {
        out.push_str("  ◀");
    }
    out.push('\n');
    let children = node.slotted_children();
    let count = children.len();
    for (index, (_, child)) in children.into_iter().enumerate() {
        draw(
            geometry,
            child,
            &child_prefix,
            Some(index + 1 == count),
            focus,
            out,
            depth + 1,
        )?;
    }
    Some(())
}

#[cfg(test)]
mod tests;
