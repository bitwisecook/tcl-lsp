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

//! Lowering expressions natively: conditions and truth, literals, binary and unary operators, math calls and merging arms.

use super::super::cells::variable_reference_place;
use super::super::ir::{CmpOp, CompareKind, IfElseResult, NativeOp, NativeType, NativeValueId};
use super::super::representation::{
    Representation, cmp_op, double_op, double_result_defined, exactly_representable_as_double,
    int_op, numeric_hint, proven_int_result, proven_neg_result,
};
use super::{ExprDecline, Lowerer};
use crate::intervals::Interval;
use crate::types::TypeShape;
use tcl_registry::native_boolean_truth::{
    NativeBooleanExpressionResultProduction, NativeBooleanTruthPurpose,
};
use tcl_syntax::expr::ast::render_expr;
use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp};
use tcl_syntax::number::Number;

impl Lowerer<'_> {
    // Expressions.

    /// Lower an expression, falling back to the runtime expression intrinsic
    /// over the rendered expression when it has no native shape.
    pub(super) fn lower_expression(&mut self, expr: &ExprNode) -> NativeValueId {
        let text = render_expr(expr);
        self.lower_expression_text(expr, &text)
    }

    pub(super) fn lower_expression_text(&mut self, expr: &ExprNode, text: &str) -> NativeValueId {
        if let Ok(value) = self.attempt(|this| this.lower_expr(expr)) {
            return value;
        }
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::ExprEval {
            dst,
            text: text.to_owned(),
        });
        self.clobber_shadows();
        self.note_observation();
        dst
    }

    /// A fallback owns raw expression evaluation, its result producer and truth
    /// in one runtime operation. `ExprEval` already produces a public result,
    /// so its object must not enter another public result producer here.
    pub(super) fn lower_condition_expression(
        &mut self,
        expr: &ExprNode,
        text: &str,
    ) -> NativeValueId {
        if let Ok(value) = self.attempt(|this| this.lower_boolean_operand(expr)) {
            return self.expression_result_truth(
                value,
                NativeBooleanExpressionResultProduction::InlineExpression,
            );
        }
        if self
            .boolean_dialect
            .and_then(|dialect| {
                dialect.native_boolean_expression_result_protocol(
                    NativeBooleanExpressionResultProduction::PublicExpressionApi,
                )
            })
            .is_none()
        {
            self.boolean_admission_missing = true;
        }
        let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
        self.emit(NativeOp::ExprBoolEval {
            dst,
            text: text.to_owned(),
        });
        self.clobber_shadows();
        self.note_observation();
        dst
    }

    /// Retain the object consumed by the reached truth stage. Mathematical
    /// numeric shadows and intermediate machine numbers carry no observable
    /// cache/conversion receipt; those expressions keep the runtime seam.
    fn lower_boolean_operand(&mut self, node: &ExprNode) -> Result<NativeValueId, ExprDecline> {
        match node {
            ExprNode::Var { text, .. } => {
                let place = variable_reference_place(text, self.lexer_config)
                    .map_err(|_| ExprDecline::DynamicVariable)?;
                Ok(self.read_original_cell(&place))
            }
            ExprNode::Literal { .. } | ExprNode::CompiledWord { .. } | ExprNode::String { .. } => {
                // Source bytes are not an original compiled literal pool/cache
                // receipt. Preserve the public expression runtime seam.
                Err(ExprDecline::OriginalBooleanObjectUnavailable)
            }
            _ => {
                let value = self.lower_expr(node)?;
                match self.ty(value) {
                    NativeType::Bool | NativeType::Obj => Ok(value),
                    NativeType::I64 | NativeType::F64 => {
                        Err(ExprDecline::OriginalBooleanObjectUnavailable)
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn lower_expr(&mut self, node: &ExprNode) -> Result<NativeValueId, ExprDecline> {
        match node {
            ExprNode::Literal { text, .. } => Ok(self.lower_literal(text)),
            // A word already reduced to its value: no delimiters to strip, and
            // a live substitution means the value is not known here.
            ExprNode::CompiledWord { text, braced } => {
                if *braced || !(text.contains("${") || text.contains('[')) {
                    Ok(self.lower_literal(text))
                } else {
                    // The same decline a substituted string operand takes.
                    Err(ExprDecline::SubstitutedString)
                }
            }
            ExprNode::String { text, .. } => {
                // A `"…"` operand is its text only when nothing in it
                // substitutes. A `{…}` one is literal but for its
                // backslash-newlines, which fold in Tcl and stay in Jim, so one
                // carrying them is declined rather than pushed raw (#2227).
                match tcl_syntax::expr::fixed_string_operand(text) {
                    Some(inner) => Ok(self.const_str(inner)),
                    None => Err(ExprDecline::SubstitutedString),
                }
            }
            ExprNode::Var { text, .. } => {
                let place = variable_reference_place(text, self.lexer_config)
                    .map_err(|_| ExprDecline::DynamicVariable)?;
                Ok(self.read_cell(&place))
            }
            ExprNode::Command { .. } => Err(ExprDecline::CommandSubstitution),
            ExprNode::Raw { .. } => Err(ExprDecline::Raw),
            ExprNode::Binary { op, left, right } => self.lower_binary(*op, left, right),
            ExprNode::Unary { op, operand } => self.lower_unary(*op, operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                let condition = self.lower_boolean_operand(condition)?;
                let condition = self.truth(condition, NativeBooleanTruthPurpose::ConditionalJump);
                // Exactly one arm runs: each lowers from the state the
                // condition left, and only what both agree on survives.
                let entry_shadows = self.shadows.clone();
                self.push_buffer();
                let then_value = self.lower_expr(true_branch);
                let mut then_ops = self.pop_buffer();
                let then_shadows = std::mem::replace(&mut self.shadows, entry_shadows);
                let then_value = then_value?;
                self.push_buffer();
                let else_value = self.lower_expr(false_branch);
                let mut else_ops = self.pop_buffer();
                self.shadows.intersect(&then_shadows);
                let else_value = else_value?;
                let result = self.merge_arms(&mut then_ops, then_value, &mut else_ops, else_value);
                self.emit(NativeOp::IfElse {
                    condition,
                    then_ops,
                    else_ops,
                    result: Some(result.clone()),
                });
                Ok(result.dst)
            }
            ExprNode::Call { function, args, .. } => self.lower_call(function, args),
        }
    }

    fn lower_literal(&mut self, text: &str) -> NativeValueId {
        if self.representation {
            match self.numbers.parse_whole(text) {
                Some(Number::Int(value)) => return self.const_int(value),
                Some(Number::Double(value)) if value.is_finite() => {
                    let dst = self.new_value(
                        NativeType::F64,
                        Representation::NativeDouble { finite: true },
                    );
                    self.emit(NativeOp::ConstDouble { dst, value });
                    return dst;
                }
                _ => {}
            }
        }
        self.const_str(text)
    }

    fn lower_binary(
        &mut self,
        op: BinOp,
        left: &ExprNode,
        right: &ExprNode,
    ) -> Result<NativeValueId, ExprDecline> {
        match op {
            BinOp::And | BinOp::Or => {
                let original_lhs = self.lower_boolean_operand(left)?;
                let conjunction = op == BinOp::And;
                let purpose = if conjunction {
                    NativeBooleanTruthPurpose::LogicalAnd
                } else {
                    NativeBooleanTruthPurpose::LogicalOr
                };
                let lhs = self.truth(original_lhs, purpose);
                // The right operand is only evaluated when the left does not
                // short-circuit, so its shadows hold on one path only.
                let entry_shadows = self.shadows.clone();
                self.push_buffer();
                let rhs = self.lower_logical_right(original_lhs, right, conjunction);
                let rhs_ops = self.pop_buffer();
                self.shadows.intersect(&entry_shadows);
                let rhs = rhs?;
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                let short = self.new_value(NativeType::Bool, Representation::NativeBool);
                let short_op = NativeOp::ConstBool {
                    dst: short,
                    value: op == BinOp::Or,
                };
                let (then_ops, else_ops, then_src, else_src) = if op == BinOp::And {
                    (rhs_ops, vec![short_op], rhs, short)
                } else {
                    (vec![short_op], rhs_ops, short, rhs)
                };
                self.emit(NativeOp::IfElse {
                    condition: lhs,
                    then_ops,
                    else_ops,
                    result: Some(IfElseResult {
                        dst,
                        then_src,
                        else_src,
                    }),
                });
                Ok(dst)
            }
            _ => {
                let lhs = self.lower_expr(left)?;
                let rhs = self.lower_expr(right)?;
                self.lower_binary_values(op, lhs, rhs)
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn lower_binary_values(
        &mut self,
        op: BinOp,
        lhs: NativeValueId,
        rhs: NativeValueId,
    ) -> Result<NativeValueId, ExprDecline> {
        let lhs = self.numeric_operand(lhs);
        let rhs = self.numeric_operand(rhs);
        let lrep = self.rep(lhs).clone();
        let rrep = self.rep(rhs).clone();
        if let Some(cmp) = cmp_op(op) {
            match (&lrep, &rrep) {
                (Representation::NativeInt(_), Representation::NativeInt(_)) => {
                    let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                    self.emit(NativeOp::Compare {
                        dst,
                        op: cmp,
                        kind: CompareKind::I64,
                        lhs,
                        rhs,
                    });
                    return Ok(dst);
                }
                // A mixed int/double comparison is only exact while the
                // integer side converts to `f64` without rounding; outside
                // that range the dynamic edge's exact comparator is the only
                // right answer.
                (l, r)
                    if l.is_native_numeric()
                        && r.is_native_numeric()
                        && mixed_compare_is_exact(l, r) =>
                {
                    let lhs = self.promote_double(lhs);
                    let rhs = self.promote_double(rhs);
                    let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                    self.emit(NativeOp::Compare {
                        dst,
                        op: cmp,
                        kind: CompareKind::F64,
                        lhs,
                        rhs,
                    });
                    return Ok(dst);
                }
                _ => {
                    let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                    let hint = numeric_hint(&lrep, &rrep);
                    self.emit(NativeOp::DynamicCompare {
                        dst,
                        op,
                        lhs,
                        rhs,
                        hint,
                    });
                    return Ok(dst);
                }
            }
        }
        if let Some(iop) = int_op(op) {
            if let (Representation::NativeInt(l), Representation::NativeInt(r)) = (&lrep, &rrep)
                && let Some(result) = proven_int_result(iop, *l, *r)
            {
                let dst = self.new_value(NativeType::I64, Representation::NativeInt(result));
                self.emit(NativeOp::IntBinary {
                    dst,
                    op: iop,
                    lhs,
                    rhs,
                });
                return Ok(dst);
            }
            if let Some(dop) = double_op(op)
                && double_result_defined(dop)
                && lrep.is_native_numeric()
                && rrep.is_native_numeric()
                && (matches!(lrep, Representation::NativeDouble { .. })
                    || matches!(rrep, Representation::NativeDouble { .. }))
                && self.finite(lhs)
                && self.finite(rhs)
            {
                let lhs = self.promote_double(lhs);
                let rhs = self.promote_double(rhs);
                let dst = self.new_value(
                    NativeType::F64,
                    Representation::NativeDouble { finite: false },
                );
                self.emit(NativeOp::DoubleBinary {
                    dst,
                    op: dop,
                    lhs,
                    rhs,
                });
                return Ok(dst);
            }
            let hint = numeric_hint(&lrep, &rrep);
            let dst = self.new_value(
                NativeType::Obj,
                Representation::Boxed(Some(TypeShape::Numeric)),
            );
            self.emit(NativeOp::DynamicBinary {
                dst,
                op,
                lhs,
                rhs,
                hint,
            });
            return Ok(dst);
        }
        if op.spec().mathop_shape.is_none() {
            return Err(ExprDecline::UnsupportedOperator);
        }
        let lhs = self.boxed(lhs);
        let rhs = self.boxed(rhs);
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::MathOp {
            dst,
            op: op.spec().spelling,
            args: vec![lhs, rhs],
        });
        Ok(dst)
    }

    /// Keep the original left operand until the selected final instruction.
    /// C8.4 reconverts left then right after evaluating the right expression;
    /// the later C and Jim short-circuit paths convert only the right operand.
    fn lower_logical_right(
        &mut self,
        original_lhs: NativeValueId,
        right: &ExprNode,
        conjunction: bool,
    ) -> Result<NativeValueId, ExprDecline> {
        let rhs = self.lower_boolean_operand(right)?;
        let Some(purpose) = self
            .boolean_dialect
            .and_then(|dialect| dialect.native_logical_final_operand_purpose(conjunction))
        else {
            self.boolean_admission_missing = true;
            return Err(ExprDecline::UnsupportedOperator);
        };
        if !matches!(
            purpose,
            NativeBooleanTruthPurpose::LogicalAndInstruction
                | NativeBooleanTruthPurpose::LogicalOrInstruction
        ) {
            return Ok(self.truth(rhs, purpose));
        }
        let lhs = self.truth(original_lhs, purpose);
        let rhs = self.truth(rhs, purpose);
        let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
        let fixed = self.new_value(NativeType::Bool, Representation::NativeBool);
        let constant = NativeOp::ConstBool {
            dst: fixed,
            value: !conjunction,
        };
        let (then_ops, else_ops, then_src, else_src) = if conjunction {
            (Vec::new(), vec![constant], rhs, fixed)
        } else {
            (vec![constant], Vec::new(), fixed, rhs)
        };
        self.emit(NativeOp::IfElse {
            condition: lhs,
            then_ops,
            else_ops,
            result: Some(IfElseResult {
                dst,
                then_src,
                else_src,
            }),
        });
        Ok(dst)
    }

    fn lower_unary(
        &mut self,
        op: UnaryOp,
        operand: &ExprNode,
    ) -> Result<NativeValueId, ExprDecline> {
        let value = if op == UnaryOp::Not {
            self.lower_boolean_operand(operand)?
        } else {
            self.lower_expr(operand)?
        };
        match op {
            UnaryOp::Not => {
                let truth = self.truth(value, NativeBooleanTruthPurpose::LogicalNot);
                let dst = self.new_value(NativeType::Bool, Representation::NativeBool);
                self.emit(NativeOp::NotBool { dst, src: truth });
                Ok(dst)
            }
            UnaryOp::WordNot => Err(ExprDecline::UnsupportedOperator),
            UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot => {
                let value = self.numeric_operand(value);
                let rep = self.rep(value).clone();
                match (op, &rep) {
                    (
                        UnaryOp::Pos,
                        Representation::NativeInt(_) | Representation::NativeDouble { .. },
                    ) => Ok(value),
                    (UnaryOp::Neg, Representation::NativeInt(interval)) => {
                        if let Some(result) = proven_neg_result(*interval) {
                            let dst =
                                self.new_value(NativeType::I64, Representation::NativeInt(result));
                            self.emit(NativeOp::IntNeg { dst, src: value });
                            return Ok(dst);
                        }
                        Ok(self.dynamic_unary(op, value))
                    }
                    (UnaryOp::Neg, Representation::NativeDouble { finite }) => {
                        let dst = self.new_value(
                            NativeType::F64,
                            Representation::NativeDouble { finite: *finite },
                        );
                        self.emit(NativeOp::DoubleNeg { dst, src: value });
                        Ok(dst)
                    }
                    (UnaryOp::BitNot, Representation::NativeInt(_)) => {
                        let dst = self.new_value(NativeType::I64, Representation::any_int());
                        self.emit(NativeOp::IntBitNot { dst, src: value });
                        Ok(dst)
                    }
                    _ => Ok(self.dynamic_unary(op, value)),
                }
            }
        }
    }

    fn dynamic_unary(&mut self, op: UnaryOp, value: NativeValueId) -> NativeValueId {
        let dst = self.new_value(
            NativeType::Obj,
            Representation::Boxed(Some(TypeShape::Numeric)),
        );
        self.emit(NativeOp::DynamicUnary {
            dst,
            op,
            src: value,
        });
        dst
    }

    #[allow(clippy::too_many_lines)]
    fn lower_call(
        &mut self,
        function: &str,
        args: &[ExprNode],
    ) -> Result<NativeValueId, ExprDecline> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.lower_expr(arg)?;
            values.push(self.numeric_operand(value));
        }
        if self.mathfunc_native {
            match (function, values.as_slice()) {
                ("double", [value]) => match self.rep(*value) {
                    Representation::NativeInt(_) => return Ok(self.int_to_double(*value)),
                    Representation::NativeDouble { .. } => return Ok(*value),
                    _ => {}
                },
                ("int" | "wide", [value]) => {
                    if matches!(self.rep(*value), Representation::NativeInt(_)) {
                        return Ok(*value);
                    }
                }
                ("abs", [value]) => {
                    if let Representation::NativeInt(interval) = self.rep(*value).clone()
                        && let Some(negated) = proven_neg_result(interval)
                    {
                        let zero = self.const_int(0);
                        let negative = self.new_value(NativeType::Bool, Representation::NativeBool);
                        self.emit(NativeOp::Compare {
                            dst: negative,
                            op: CmpOp::Lt,
                            kind: CompareKind::I64,
                            lhs: *value,
                            rhs: zero,
                        });
                        let neg =
                            self.new_value(NativeType::I64, Representation::NativeInt(negated));
                        let hull = hull(interval, negated);
                        let dst = self.new_value(NativeType::I64, Representation::NativeInt(hull));
                        self.emit(NativeOp::IfElse {
                            condition: negative,
                            then_ops: vec![NativeOp::IntNeg {
                                dst: neg,
                                src: *value,
                            }],
                            else_ops: Vec::new(),
                            result: Some(IfElseResult {
                                dst,
                                then_src: neg,
                                else_src: *value,
                            }),
                        });
                        return Ok(dst);
                    }
                }
                ("max" | "min", [first, second]) => {
                    let both_int = matches!(
                        (self.rep(*first), self.rep(*second)),
                        (Representation::NativeInt(_), Representation::NativeInt(_))
                    );
                    if both_int {
                        let (l, r) = (
                            self.rep(*first)
                                .interval()
                                .unwrap_or(Interval { lo: None, hi: None }),
                            self.rep(*second)
                                .interval()
                                .unwrap_or(Interval { lo: None, hi: None }),
                        );
                        let pick_first =
                            self.new_value(NativeType::Bool, Representation::NativeBool);
                        self.emit(NativeOp::Compare {
                            dst: pick_first,
                            op: if function == "max" {
                                CmpOp::Ge
                            } else {
                                CmpOp::Le
                            },
                            kind: CompareKind::I64,
                            lhs: *first,
                            rhs: *second,
                        });
                        let dst =
                            self.new_value(NativeType::I64, Representation::NativeInt(hull(l, r)));
                        self.emit(NativeOp::IfElse {
                            condition: pick_first,
                            then_ops: Vec::new(),
                            else_ops: Vec::new(),
                            result: Some(IfElseResult {
                                dst,
                                then_src: *first,
                                else_src: *second,
                            }),
                        });
                        return Ok(dst);
                    }
                }
                _ => {}
            }
        }
        let args = values
            .into_iter()
            .map(|value| self.boxed(value))
            .collect::<Vec<_>>();
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::MathFunc {
            dst,
            name: function.to_owned(),
            args,
        });
        // The math function resolves through the command table, so it may be
        // user code that observes or writes any cell.
        self.clobber_shadows();
        self.note_observation();
        Ok(dst)
    }

    /// A boolean operand of arithmetic reads as the integer `0`/`1`.
    fn numeric_operand(&mut self, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::Bool {
            self.bool_to_int(value)
        } else {
            value
        }
    }

    fn promote_double(&mut self, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::I64 {
            self.int_to_double(value)
        } else {
            value
        }
    }

    pub(super) fn finite(&self, value: NativeValueId) -> bool {
        match self.rep(value) {
            Representation::NativeInt(_) => true,
            Representation::NativeDouble { finite } => *finite,
            _ => false,
        }
    }

    /// Merge the values two arms produce into one, boxing both when their
    /// native types differ.
    fn merge_arms(
        &mut self,
        then_ops: &mut Vec<NativeOp>,
        then_value: NativeValueId,
        else_ops: &mut Vec<NativeOp>,
        else_value: NativeValueId,
    ) -> IfElseResult {
        let then_ty = self.ty(then_value);
        let else_ty = self.ty(else_value);
        if then_ty == else_ty {
            let rep = match (self.rep(then_value).clone(), self.rep(else_value).clone()) {
                (Representation::NativeInt(l), Representation::NativeInt(r)) => {
                    Representation::NativeInt(hull(l, r))
                }
                (
                    Representation::NativeDouble { finite: l },
                    Representation::NativeDouble { finite: r },
                ) => Representation::NativeDouble { finite: l && r },
                (Representation::NativeBool, Representation::NativeBool) => {
                    Representation::NativeBool
                }
                (Representation::Boxed(l), Representation::Boxed(r)) if l == r => {
                    Representation::Boxed(l)
                }
                _ => Representation::Boxed(None),
            };
            let dst = self.new_value(then_ty, rep);
            return IfElseResult {
                dst,
                then_src: then_value,
                else_src: else_value,
            };
        }
        let then_boxed = self.box_into(then_ops, then_value);
        let else_boxed = self.box_into(else_ops, else_value);
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        IfElseResult {
            dst,
            then_src: then_boxed,
            else_src: else_boxed,
        }
    }

    fn box_into(&mut self, ops: &mut Vec<NativeOp>, value: NativeValueId) -> NativeValueId {
        if self.ty(value) == NativeType::Obj {
            return value;
        }
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        ops.push(NativeOp::Box { dst, src: value });
        dst
    }
}

fn hull(a: Interval, b: Interval) -> Interval {
    let lo = match (a.lo, b.lo) {
        (Some(x), Some(y)) => Some(x.min(y)),
        _ => None,
    };
    let hi = match (a.hi, b.hi) {
        (Some(x), Some(y)) => Some(x.max(y)),
        _ => None,
    };
    Interval { lo, hi }
}

/// Whether comparing these two native numeric representations through `f64`
/// gives Tcl's answer. Two doubles always do; a mixed pair only while the
/// integer side is exactly representable (see
/// [`exactly_representable_as_double`]). Two integers never reach here.
fn mixed_compare_is_exact(l: &Representation, r: &Representation) -> bool {
    match (l, r) {
        (Representation::NativeInt(i), Representation::NativeDouble { .. })
        | (Representation::NativeDouble { .. }, Representation::NativeInt(i)) => {
            exactly_representable_as_double(*i)
        }
        _ => true,
    }
}
