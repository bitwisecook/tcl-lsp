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

//! The shared `expr` evaluator — the one tree-walk over the [`ExprNode`] AST,
//! generic over a value type via the [`ExprOps`] trait. This is the evaluation
//! parallel of sharing the lexer/parser: the *structure* of `expr` semantics
//! (operator dispatch, short-circuit `&&`/`||`, `?:`, the numeric-vs-string
//! comparison rule, `eq`/`ne` always-string, `in`/`ni` membership) lives **once**
//! here, and each consumer plugs in its value operations:
//!
//! - the **runtime** over the full numeric tower (`i64`/bignum/`double` on
//!   `Tcl_Obj`),
//! - the **compiler's const-folder** over its `TclValue` (which bails — returns
//!   its "can't fold" — on anything it doesn't model).
//!
//! Only the value-type-specific bits (the actual arithmetic, value construction,
//! `$var`/`[cmd]` resolution, boolean coercion) are per consumer; the grammar of
//! evaluation is not re-derived.

use core::cmp::Ordering;

use crate::native_boolean_truth::NativeBooleanTruthPurpose;

use super::ast::{BinOp, ExprNode, ExprText, UnaryOp};

/// Outcome of a numeric comparison between two number-classified operands.
///
/// C Tcl's `==`/`<`/… are only *mostly* a three-way ordering: a NaN operand is
/// still a **number** (so the string-comparison fallback must not apply) but
/// orders against nothing — `tclExecute.c`'s rule is "NaN arg: NaN != to
/// everything, other compares are false". [`eval`] maps [`Self::Unordered`]
/// accordingly; a plain `Option<Ordering>` cannot carry that third state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericCompare {
    /// Both operands are ordered numbers.
    Ordered(Ordering),
    /// At least one operand is NaN.
    Unordered,
}

impl NumericCompare {
    /// Wrap an [`Ordering`], or [`Self::Unordered`] for `None` — the shape
    /// `f64::partial_cmp` hands back.
    #[must_use]
    pub fn from_partial(ord: Option<Ordering>) -> Self {
        ord.map_or(Self::Unordered, Self::Ordered)
    }
}

/// The value operations an `expr` consumer supplies. The shared [`eval`] walker
/// drives these; it owns the dispatch, short-circuit, ternary, and the
/// numeric-vs-string comparison rule.
pub trait ExprOps {
    /// The consumer's value type (a `Tcl_Obj` pointer, a const-fold value, …).
    type Value;
    /// The consumer's error type.
    type Error;

    /// Return an original value retained by a selected compiler for this exact
    /// tree. The default walks the tree normally; a mathematical value alone
    /// does not grant a compiled-object replacement.
    fn prepared_node<Text: ExprText>(
        &mut self,
        _node: &ExprNode<Text>,
    ) -> Option<Result<Self::Value, Self::Error>> {
        None
    }

    /// A numeric/boolean literal token (`42`, `0xff`, `1.5`, `true`).
    fn literal(&mut self, text: &str) -> Result<Self::Value, Self::Error>;
    /// A string operand, delimiters already stripped.
    ///
    /// `substitutes` says which spelling it was, because the stripped text no
    /// longer can: a `"…"` operand is a double-quoted word and substitutes
    /// `$var`, `[cmd]` and backslashes, while a `{…}` operand is literal. The
    /// two consumers that had to guess guessed in opposite directions — the
    /// const-folder read `expr {"pre$x"}` as the literal `pre$x`, and the VM's
    /// runtime `expr` ran the `[id 9]` in `expr {{[id 9]}}` — so the walk,
    /// which is the last place that sees the delimiter, now says (#2227).
    fn string(&mut self, inner: &str, substitutes: bool) -> Result<Self::Value, Self::Error>;
    /// Resolve a `$name` reference.
    fn var(&mut self, name: &str) -> Result<Self::Value, Self::Error>;
    /// Resolve an authored variable reference, retaining bracing and raw index
    /// substitution syntax. Engines override this using their dialect scanner.
    /// Pure consumers may keep the normalized, index-preserving lookup seam.
    fn variable_reference(&mut self, reference: &str) -> Result<Self::Value, Self::Error> {
        self.var(crate::naming::var_reference(reference))
    }
    /// Resolve a reached reference at its original parser offset. The default
    /// preserves the ordinary engine lookup; proof consumers retain this site.
    fn variable_reference_at(
        &mut self,
        reference: &str,
        _start: u32,
    ) -> Result<Self::Value, Self::Error> {
        self.variable_reference(reference)
    }
    /// Evaluate a `[script]` (brackets already stripped).
    fn command(&mut self, script: &str) -> Result<Self::Value, Self::Error>;
    /// A `func(args…)` math-function call (dispatched through `::tcl::mathfunc`).
    fn call(&mut self, function: &str, args: Vec<Self::Value>) -> Result<Self::Value, Self::Error>;
    /// Invoke a reached function with its exact offset in the parsed expression.
    /// Source-proof consumers use this position after argument evaluation; native
    /// engines may retain their ordinary dispatcher through the default adapter.
    fn call_at(
        &mut self,
        function: &str,
        args: Vec<Self::Value>,
        _start: u32,
    ) -> Result<Self::Value, Self::Error> {
        self.call(function, args)
    }

    /// Construct a reached literal from original bytes. Native engines
    /// override byte leaves without projecting arbitrary storage to Unicode.
    fn literal_bytes(&mut self, text: &[u8]) -> Result<Self::Value, Self::Error> {
        let text = std::str::from_utf8(text)
            .map_err(|_| self.unsupported("native expression literal bytes"))?;
        self.literal(text)
    }
    /// Construct a literal at its original inclusive source extent.
    fn literal_bytes_at(
        &mut self,
        text: &[u8],
        _start: u32,
        _end: u32,
    ) -> Result<Self::Value, Self::Error> {
        self.literal_bytes(text)
    }
    /// Read an original quoted or braced operand, retaining native byte units.
    fn string_bytes(
        &mut self,
        inner: &[u8],
        substitutes: bool,
    ) -> Result<Self::Value, Self::Error> {
        let text = std::str::from_utf8(inner)
            .map_err(|_| self.unsupported("native expression string bytes"))?;
        self.string(text, substitutes)
    }
    /// Read a string using the original extent, including its delimiters.
    fn string_bytes_at(
        &mut self,
        inner: &[u8],
        substitutes: bool,
        _start: u32,
        _end: u32,
    ) -> Result<Self::Value, Self::Error> {
        self.string_bytes(inner, substitutes)
    }
    /// Resolve a reached original byte reference at its actual parser offset.
    fn variable_reference_bytes_at(
        &mut self,
        reference: &[u8],
        start: u32,
    ) -> Result<Self::Value, Self::Error> {
        let text = std::str::from_utf8(reference)
            .map_err(|_| self.unsupported("native expression variable bytes"))?;
        self.variable_reference_at(text, start)
    }
    /// Evaluate the original bracket body bytes in the current activation.
    fn command_bytes(&mut self, script: &[u8]) -> Result<Self::Value, Self::Error> {
        let text = std::str::from_utf8(script)
            .map_err(|_| self.unsupported("native expression command bytes"))?;
        self.command(text)
    }
    /// Evaluate a command using its original bracketed source extent.
    fn command_bytes_at(
        &mut self,
        script: &[u8],
        _start: u32,
        _end: u32,
    ) -> Result<Self::Value, Self::Error> {
        self.command_bytes(script)
    }
    /// Invoke a reached original function name after ordered argument evaluation.
    fn call_bytes_at(
        &mut self,
        function: &[u8],
        args: Vec<Self::Value>,
        start: u32,
    ) -> Result<Self::Value, Self::Error> {
        let text = std::str::from_utf8(function)
            .map_err(|_| self.unsupported("native expression function bytes"))?;
        self.call_at(text, args, start)
    }

    /// An arithmetic / bitwise / shift binary op (`op` is one of the arithmetic
    /// set — `Add`..`RShift`); the consumer applies its tower/fold semantics.
    fn arith(
        &mut self,
        op: BinOp,
        left: Self::Value,
        right: Self::Value,
    ) -> Result<Self::Value, Self::Error>;
    /// A unary op (`-`/`+`/`~`/`!`).
    fn unary(&mut self, op: UnaryOp, value: Self::Value) -> Result<Self::Value, Self::Error>;

    /// A binary op the shared core does not handle — the dialect operators
    /// (`contains`/`starts_with`/`matches_glob`/…). Standard consumers leave the
    /// default (`unsupported`); a dialect-aware consumer (the compiler's
    /// const-folder) overrides this. Operands are already evaluated.
    fn binary_other(
        &mut self,
        _op: BinOp,
        _left: Self::Value,
        _right: Self::Value,
    ) -> Result<Self::Value, Self::Error> {
        Err(self.unsupported("operator"))
    }

    /// Numeric comparison, or `None` when an operand is non-numeric (the
    /// walker then falls back to [`ExprOps::compare_string`] — the Tcl
    /// `==`/`<`… "numeric when both look numeric, else string" rule). A NaN
    /// operand is numeric but unordered: return
    /// `Some(NumericCompare::Unordered)`, **not** `None` — Tcl does not
    /// string-compare NaN, it applies the "`!=` true, everything else false"
    /// rule (which the walker owns).
    fn compare_numeric(
        &mut self,
        left: &Self::Value,
        right: &Self::Value,
    ) -> Option<NumericCompare>;
    /// String comparison (for `eq`/`ne`/`lt`… and the `==` string fallback).
    fn compare_string(
        &mut self,
        left: &Self::Value,
        right: &Self::Value,
    ) -> Result<Ordering, Self::Error>;
    /// String equality for `eq`/`ne`, separate from numeric-unit ordering.
    /// Consumers may preserve byte equality where the native operation does.
    fn equal_string(
        &mut self,
        left: &Self::Value,
        right: &Self::Value,
    ) -> Result<bool, Self::Error> {
        self.compare_string(left, right).map(Ordering::is_eq)
    }
    /// `needle in list` membership (string equality of elements).
    fn in_list(&mut self, needle: &Self::Value, list: &Self::Value) -> Result<bool, Self::Error>;

    /// The consumer's explicit mathematical or legacy Boolean projection.
    /// Actual native expression sites use the separately selected purpose seam.
    fn to_bool(&mut self, value: &Self::Value) -> Result<bool, Self::Error>;
    /// Convert the original operand at this reached expression instruction.
    /// The default declines: a mathematical projection cannot donate native
    /// cache, interpreter publication or result-production behaviour.
    fn to_bool_for_purpose(
        &mut self,
        _value: &Self::Value,
        _purpose: NativeBooleanTruthPurpose,
    ) -> Result<bool, Self::Error> {
        Err(self.unsupported("original Boolean operand purpose"))
    }
    /// Finish a reached logical right operand while retaining the original
    /// left operand. C8.4 reconverts both operands at LAND/LOR; other selected
    /// engines reach their own right-operand conversion.
    fn logical_right_truth(
        &mut self,
        _left: &Self::Value,
        _right: &Self::Value,
        _conjunction: bool,
    ) -> Result<bool, Self::Error> {
        Err(self.unsupported("original logical final operands"))
    }
    /// Construct a boolean result value (`0`/`1`).
    fn bool_value(&mut self, b: bool) -> Self::Value;

    /// Build the error for an unsupported construct / `Raw` (unparseable) node.
    fn unsupported(&mut self, what: &str) -> Self::Error;
}

/// Evaluate `node` against the consumer's [`ExprOps`].
pub fn eval<O: ExprOps, Text: ExprText>(
    node: &ExprNode<Text>,
    ops: &mut O,
) -> Result<O::Value, O::Error> {
    let mut state = ExprEvalState::new(node.clone());
    loop {
        match state.advance(ops)? {
            ExprEvalStep::Complete(value) => return Ok(value),
            ExprEvalStep::Request(request) => {
                let value = match request {
                    ExprEvalRequest::SubstitutedString { text, start, end } => {
                        ops.string_bytes_at(text.bytes(), true, start, end)?
                    }
                    ExprEvalRequest::Variable { reference, start } => {
                        ops.variable_reference_bytes_at(reference.bytes(), start)?
                    }
                    ExprEvalRequest::Command { text, start, end } => {
                        ops.command_bytes_at(text.bytes(), start, end)?
                    }
                    ExprEvalRequest::Call {
                        function,
                        args,
                        start,
                    } => ops.call_bytes_at(function.bytes(), args, start)?,
                };
                state.resume(value);
            }
        }
    }
}

/// An engine operation which may suspend without losing expression operands.
#[derive(Debug)]
pub enum ExprEvalRequest<V, Text = String> {
    /// Substitute a double-quoted operand with the engine's word evaluator.
    SubstitutedString {
        /// Original quoted operand body.
        text: Text,
        /// First byte of the original quoted operand.
        start: u32,
        /// Last byte of the original quoted operand, inclusive.
        end: u32,
    },
    /// Read an authored variable reference, including its leading `$`, bracing,
    /// raw array-index substitution syntax and read traces.
    Variable {
        /// Original spelling, including bracing and array-index syntax.
        reference: Text,
        /// Original offset of the reference in the parsed expression.
        start: u32,
    },
    /// Evaluate a bracketed script in the current activation.
    Command {
        /// Original command body, without brackets.
        text: Text,
        /// First byte of the original bracketed operand.
        start: u32,
        /// Last byte of the original bracketed operand, inclusive.
        end: u32,
    },
    /// Invoke a math function after evaluating its arguments from left to right.
    Call {
        /// Unqualified math-function name.
        function: Text,
        /// Evaluated argument values.
        args: Vec<V>,
        /// Exact offset of the function name in the parsed expression.
        start: u32,
    },
}

/// One result of advancing the expression continuation.
#[derive(Debug)]
pub enum ExprEvalStep<V, Text = String> {
    /// The expression has finished.
    Complete(V),
    /// Perform this engine operation, then call [`ExprEvalState::resume`].
    Request(ExprEvalRequest<V, Text>),
}

#[derive(Debug)]
enum ExprTask<V, Text> {
    Node(ExprNode<Text>),
    Unary(UnaryOp),
    Binary(BinOp),
    Logical {
        op: BinOp,
        right: ExprNode<Text>,
    },
    Boolean {
        left: V,
        conjunction: bool,
    },
    Ternary {
        when_true: ExprNode<Text>,
        when_false: ExprNode<Text>,
    },
    Call {
        function: Text,
        count: usize,
        start: u32,
    },
}

/// An explicit operand/task stack shared by blocking and resumable engines.
///
/// Requests are emitted only when execution reaches them: short-circuited
/// branches never substitute, and completed requests are never replayed.
/// Engines propagate abrupt completion by dropping this state. This API never
/// dispatches the public, shadowable `expr` command.
#[derive(Debug)]
pub struct ExprEvalState<V, Text = String> {
    tasks: Vec<ExprTask<V, Text>>,
    values: Vec<V>,
    awaiting: bool,
}

impl<V, Text: ExprText> ExprEvalState<V, Text> {
    /// Start an expression using its already parsed dialect-specific AST.
    #[must_use]
    pub fn new(node: ExprNode<Text>) -> Self {
        Self {
            tasks: vec![ExprTask::Node(node)],
            values: Vec::new(),
            awaiting: false,
        }
    }

    /// Supply the successful value of the most recently emitted request.
    pub fn resume(&mut self, value: V) {
        assert!(
            self.awaiting,
            "expression continuation has no pending request"
        );
        self.awaiting = false;
        self.values.push(value);
    }

    fn advance_node<O: ExprOps<Value = V>>(
        &mut self,
        node: ExprNode<Text>,
        ops: &mut O,
    ) -> Result<Option<ExprEvalRequest<V, Text>>, O::Error> {
        if let Some(value) = ops.prepared_node(&node) {
            self.values.push(value?);
            return Ok(None);
        }
        Ok(match node {
            ExprNode::Literal { text, start, end } => {
                self.values
                    .push(ops.literal_bytes_at(text.bytes(), start, end)?);
                None
            }
            ExprNode::String { text, start, end } => {
                if text.bytes().first() == Some(&b'"') {
                    Some(ExprEvalRequest::SubstitutedString {
                        text: Text::from_source_bytes(strip_delims_bytes(text.bytes())),
                        start,
                        end,
                    })
                } else {
                    self.values.push(ops.string_bytes_at(
                        strip_delims_bytes(text.bytes()),
                        false,
                        start,
                        end,
                    )?);
                    None
                }
            }
            ExprNode::CompiledWord { text, braced } => {
                if !braced
                    && (text.bytes().windows(2).any(|part| part == b"${")
                        || text.bytes().contains(&b'['))
                {
                    return Err(ops.unsupported("compiled word with a live substitution"));
                }
                self.values.push(ops.string_bytes(text.bytes(), false)?);
                None
            }
            ExprNode::Var { text, start, .. } => Some(ExprEvalRequest::Variable {
                reference: text,
                start,
            }),
            ExprNode::Command { text, start, end } => Some(ExprEvalRequest::Command {
                text: Text::from_source_bytes(strip_brackets_bytes(text.bytes())),
                start,
                end,
            }),
            ExprNode::Unary { op, operand } => {
                self.tasks.push(ExprTask::Unary(op));
                self.tasks.push(ExprTask::Node(*operand));
                None
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                self.tasks.push(ExprTask::Ternary {
                    when_true: *true_branch,
                    when_false: *false_branch,
                });
                self.tasks.push(ExprTask::Node(*condition));
                None
            }
            ExprNode::Call {
                function,
                args,
                start,
                ..
            } => {
                self.tasks.push(ExprTask::Call {
                    function,
                    count: args.len(),
                    start,
                });
                self.tasks
                    .extend(args.into_iter().rev().map(ExprTask::Node));
                None
            }
            ExprNode::Binary { op, left, right } => {
                if matches!(op, BinOp::And | BinOp::WordAnd | BinOp::Or | BinOp::WordOr) {
                    self.tasks.push(ExprTask::Logical { op, right: *right });
                } else {
                    self.tasks.push(ExprTask::Binary(op));
                    self.tasks.push(ExprTask::Node(*right));
                }
                self.tasks.push(ExprTask::Node(*left));
                None
            }
            ExprNode::Raw { .. } => {
                return Err(ops.unsupported("syntax error in expression"));
            }
        })
    }

    /// Evaluate local value operations until completion or an engine request.
    pub fn advance<O: ExprOps<Value = V>>(
        &mut self,
        ops: &mut O,
    ) -> Result<ExprEvalStep<V, Text>, O::Error> {
        assert!(
            !self.awaiting,
            "expression request must be resumed before advancing"
        );
        while let Some(task) = self.tasks.pop() {
            let request = match task {
                ExprTask::Node(node) => self.advance_node(node, ops)?,
                ExprTask::Unary(op) => {
                    let value = self.values.pop().expect("unary operand");
                    self.values.push(ops.unary(op, value)?);
                    None
                }
                ExprTask::Binary(op) => {
                    let right = self.values.pop().expect("right operand");
                    let left = self.values.pop().expect("left operand");
                    self.values.push(apply_binary(op, left, right, ops)?);
                    None
                }
                ExprTask::Logical { op, right } => {
                    let left = self.values.pop().expect("logical operand");
                    let conjunction = matches!(op, BinOp::And | BinOp::WordAnd);
                    let purpose = if conjunction {
                        NativeBooleanTruthPurpose::LogicalAnd
                    } else {
                        NativeBooleanTruthPurpose::LogicalOr
                    };
                    let truth = ops.to_bool_for_purpose(&left, purpose)?;
                    if truth == conjunction {
                        self.tasks.push(ExprTask::Boolean { left, conjunction });
                        self.tasks.push(ExprTask::Node(right));
                    } else {
                        self.values.push(ops.bool_value(truth));
                    }
                    None
                }
                ExprTask::Boolean { left, conjunction } => {
                    let value = self.values.pop().expect("boolean operand");
                    let truth = ops.logical_right_truth(&left, &value, conjunction)?;
                    self.values.push(ops.bool_value(truth));
                    None
                }
                ExprTask::Ternary {
                    when_true,
                    when_false,
                } => {
                    let condition = self.values.pop().expect("conditional operand");
                    self.tasks.push(ExprTask::Node(
                        if ops.to_bool_for_purpose(
                            &condition,
                            NativeBooleanTruthPurpose::ConditionalJump,
                        )? {
                            when_true
                        } else {
                            when_false
                        },
                    ));
                    None
                }
                ExprTask::Call {
                    function,
                    count,
                    start,
                } => {
                    let args = self.values.split_off(self.values.len() - count);
                    Some(ExprEvalRequest::Call {
                        function,
                        args,
                        start,
                    })
                }
            };
            if let Some(request) = request {
                self.awaiting = true;
                return Ok(ExprEvalStep::Request(request));
            }
        }
        Ok(ExprEvalStep::Complete(
            self.values.pop().expect("completed expression value"),
        ))
    }
}

fn apply_binary<O: ExprOps>(
    op: BinOp,
    l: O::Value,
    r: O::Value,
    ops: &mut O,
) -> Result<O::Value, O::Error> {
    // Arithmetic / bitwise / shift → the consumer's value ops.
    if matches!(
        op,
        BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Pow
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::LShift
            | BinOp::RShift
    ) {
        return ops.arith(op, l, r);
    }

    // Otherwise it's a comparison / membership → a boolean. Compute the boolean
    // first (releasing the `ops` borrow) before constructing the result value.
    let b = match op {
        // `==`/`!=`/`<`… numeric when both look numeric, else string; a NaN
        // operand is unordered — unequal to everything, ordered before/after
        // nothing (`tclExecute.c`: "NaN arg: NaN != to everything, other
        // compares are false").
        BinOp::Eq => matches!(num_or_str(ops, &l, &r)?, NumericCompare::Ordered(o) if o.is_eq()),
        BinOp::Ne => !matches!(num_or_str(ops, &l, &r)?, NumericCompare::Ordered(o) if o.is_eq()),
        BinOp::Lt => matches!(num_or_str(ops, &l, &r)?, NumericCompare::Ordered(o) if o.is_lt()),
        BinOp::Le => matches!(num_or_str(ops, &l, &r)?, NumericCompare::Ordered(o) if o.is_le()),
        BinOp::Gt => matches!(num_or_str(ops, &l, &r)?, NumericCompare::Ordered(o) if o.is_gt()),
        BinOp::Ge => matches!(num_or_str(ops, &l, &r)?, NumericCompare::Ordered(o) if o.is_ge()),
        // `eq`/`ne`/`lt`… always string-compare.
        BinOp::StrEq | BinOp::StrEquals => ops.equal_string(&l, &r)?,
        BinOp::StrNe => !ops.equal_string(&l, &r)?,
        BinOp::StrLt => ops.compare_string(&l, &r)?.is_lt(),
        BinOp::StrLe => ops.compare_string(&l, &r)?.is_le(),
        BinOp::StrGt => ops.compare_string(&l, &r)?.is_gt(),
        BinOp::StrGe => ops.compare_string(&l, &r)?.is_ge(),
        // List membership.
        BinOp::In => ops.in_list(&l, &r)?,
        BinOp::Ni => !ops.in_list(&l, &r)?,
        // `&&`/`||` handled above; the remaining (dialect) operators go to the
        // consumer's `binary_other` hook — which returns a value directly, so
        // short-circuit out of the boolean path here.
        _ => return ops.binary_other(op, l, r),
    };
    Ok(ops.bool_value(b))
}

/// The numeric-or-string comparison rule: numeric when both operands compare
/// numerically (possibly unordered, for NaN), else a string comparison.
fn num_or_str<O: ExprOps>(
    ops: &mut O,
    l: &O::Value,
    r: &O::Value,
) -> Result<NumericCompare, O::Error> {
    match ops.compare_numeric(l, r) {
        Some(outcome) => Ok(outcome),
        None => ops.compare_string(l, r).map(NumericCompare::Ordered),
    }
}

fn strip_delims_bytes(text: &[u8]) -> &[u8] {
    if text.len() >= 2 && matches!((text[0], text[text.len() - 1]), (b'{', b'}') | (b'"', b'"')) {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

fn strip_brackets_bytes(text: &[u8]) -> &[u8] {
    text.strip_prefix(b"[")
        .and_then(|inner| inner.strip_suffix(b"]"))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::parser::parse_expr;

    /// A minimal two-rung value: integers compare numerically, everything else
    /// string-compares — enough to drive every branch of the shared walker.
    #[derive(Debug, Clone, PartialEq)]
    enum V {
        Num(i64),
        Str(String),
    }
    impl V {
        fn as_string(&self) -> String {
            match self {
                V::Num(n) => n.to_string(),
                V::Str(s) => s.clone(),
            }
        }
    }

    /// Records which seam methods fired so the tests can assert dispatch, not
    /// just the final value.
    #[derive(Default)]
    struct Ops {
        commands: Vec<String>,
        calls: Vec<String>,
        /// When set, every quoted operand is recorded here and read back
        /// marked, so a test can tell which hook the walker called.
        quoted: Option<Vec<String>>,
        purposes: Vec<(NativeBooleanTruthPurpose, V)>,
        final_operands: Vec<(V, V, bool)>,
    }

    impl ExprOps for Ops {
        type Value = V;
        type Error = String;

        fn literal(&mut self, text: &str) -> Result<V, String> {
            text.parse::<i64>()
                .map(V::Num)
                .or_else(|_| Ok(V::Str(text.to_string())))
        }
        fn string(&mut self, inner: &str, substitutes: bool) -> Result<V, String> {
            match &mut self.quoted {
                Some(seen) if substitutes => {
                    seen.push(inner.to_string());
                    Ok(V::Str(format!("quoted:{inner}")))
                }
                _ => Ok(V::Str(inner.to_string())),
            }
        }
        fn var(&mut self, name: &str) -> Result<V, String> {
            // `x` → 10, `y` → 0; any other name (an `arr(idx)` reference
            // included) echoes itself as a string.
            Ok(match name {
                "x" => V::Num(10),
                "y" => V::Num(0),
                other => V::Str(other.to_string()),
            })
        }
        fn command(&mut self, script: &str) -> Result<V, String> {
            self.commands.push(script.to_string());
            Ok(V::Num(7))
        }
        fn call(&mut self, function: &str, args: Vec<V>) -> Result<V, String> {
            self.calls.push(format!("{function}/{}", args.len()));
            // `max` of two numbers; otherwise echo the arg count.
            if function == "max"
                && args.len() == 2
                && let (V::Num(a), V::Num(b)) = (&args[0], &args[1])
            {
                return Ok(V::Num((*a).max(*b)));
            }
            Ok(V::Num(i64::try_from(args.len()).unwrap_or(i64::MAX)))
        }
        fn arith(&mut self, op: BinOp, left: V, right: V) -> Result<V, String> {
            let (V::Num(a), V::Num(b)) = (&left, &right) else {
                return Err(self.unsupported("non-numeric arith"));
            };
            let (a, b) = (*a, *b);
            Ok(V::Num(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b,
                BinOp::Mod => a % b,
                BinOp::Pow => a.pow(u32::try_from(b).unwrap_or(0)),
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::LShift => a << b,
                BinOp::RShift => a >> b,
                _other => return Err(self.unsupported("bad arith op")),
            }))
        }
        fn unary(&mut self, op: UnaryOp, value: V) -> Result<V, String> {
            let V::Num(n) = value else {
                return Err(self.unsupported("non-numeric unary"));
            };
            Ok(match op {
                UnaryOp::Neg => V::Num(-n),
                UnaryOp::Pos => V::Num(n),
                UnaryOp::BitNot => V::Num(!n),
                UnaryOp::Not | UnaryOp::WordNot => V::Num(i64::from(n == 0)),
            })
        }
        fn compare_numeric(&mut self, left: &V, right: &V) -> Option<NumericCompare> {
            match (left, right) {
                (V::Num(a), V::Num(b)) => Some(NumericCompare::Ordered(a.cmp(b))),
                _ => None,
            }
        }
        fn compare_string(&mut self, left: &V, right: &V) -> Result<Ordering, String> {
            Ok(left.as_string().cmp(&right.as_string()))
        }
        fn in_list(&mut self, needle: &V, list: &V) -> Result<bool, String> {
            let n = needle.as_string();
            Ok(list.as_string().split_whitespace().any(|e| e == n))
        }
        fn to_bool(&mut self, value: &V) -> Result<bool, String> {
            Ok(match value {
                V::Num(n) => *n != 0,
                V::Str(s) => s == "1" || s == "true",
            })
        }
        // This adapter models mathematical values, not native headers or effects.
        fn to_bool_for_purpose(
            &mut self,
            value: &V,
            purpose: NativeBooleanTruthPurpose,
        ) -> Result<bool, String> {
            self.purposes.push((purpose, value.clone()));
            self.to_bool(value)
        }
        fn logical_right_truth(
            &mut self,
            left: &V,
            right: &V,
            conjunction: bool,
        ) -> Result<bool, String> {
            self.final_operands
                .push((left.clone(), right.clone(), conjunction));
            self.to_bool(right)
        }
        fn bool_value(&mut self, b: bool) -> V {
            V::Num(i64::from(b))
        }
        fn unsupported(&mut self, what: &str) -> String {
            format!("unsupported: {what}")
        }
        // NB: `binary_other` intentionally left as the trait default so the
        // default (`Err(unsupported)`) body is exercised.
    }

    #[test]
    fn reached_truth_purposes_retain_original_left_and_skip_unreached_right() {
        // Software walker contract: this observes routing and retained values,
        // independently of any native getter/cache or provider equivalence.
        let mut ops = Ops::default();
        assert_eq!(eval(&parse_expr("17 && 23", None), &mut ops), Ok(V::Num(1)));
        assert_eq!(
            ops.purposes,
            [(NativeBooleanTruthPurpose::LogicalAnd, V::Num(17))]
        );
        assert_eq!(ops.final_operands, [(V::Num(17), V::Num(23), true)]);
        ops.purposes.clear();
        ops.final_operands.clear();
        assert_eq!(
            eval(&parse_expr("17 || [unreached]", None), &mut ops),
            Ok(V::Num(1))
        );
        assert_eq!(
            ops.purposes,
            [(NativeBooleanTruthPurpose::LogicalOr, V::Num(17))]
        );
        assert!(ops.final_operands.is_empty());
        assert!(ops.commands.is_empty());
        ops.purposes.clear();
        assert_eq!(
            eval(&parse_expr("17 ? 31 : [unreached]", None), &mut ops),
            Ok(V::Num(31))
        );
        assert_eq!(
            ops.purposes,
            [(NativeBooleanTruthPurpose::ConditionalJump, V::Num(17))]
        );
        assert!(ops.commands.is_empty());
    }

    fn eval_str(src: &str) -> Result<V, String> {
        let node = parse_expr(src, None);
        let mut ops = Ops::default();
        eval(&node, &mut ops)
    }

    // The `Ops` mock's numeric/comparison/membership results below were
    // cross-checked against `tclsh8.6` and `tclsh9.0` so the walker's dispatch
    // is exercised against C-Tcl-accurate expectations:
    //   expr {9/2}==4  {2**3}==8  {5^1}==4  {16>>2}==4  {6&3}==2  {4|1}==5
    //   {1<<3}==8  {9%2}==1  {-5}==-5  {~0}==-1  {!0}==1  {"abc"<"abd"}==1
    //   {2 in {1 2 3}}==1  {5 ni {1 2 3}}==1  {1?20:30}==20  {0?20:30}==30
    //   {max(3,9)}==9. NB `lt`/`le`/`gt`/`ge` are 9.0-only operators (error in
    //   8.6); the Rust parser models them under the default/latest dialect.

    #[test]
    fn literal_string_var_command() {
        assert_eq!(eval_str("42").unwrap(), V::Num(42));
        assert_eq!(eval_str("{hello}").unwrap(), V::Str("hello".into()));
        assert_eq!(eval_str("\"hi\"").unwrap(), V::Str("hi".into()));
        assert_eq!(eval_str("$x").unwrap(), V::Num(10));
        // Command substitution routes through `command` (brackets stripped).
        let node = parse_expr("[foo bar]", None);
        let mut ops = Ops::default();
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(7));
        assert_eq!(ops.commands, vec!["foo bar".to_string()]);
    }

    #[test]
    fn unary_ops() {
        assert_eq!(eval_str("-5").unwrap(), V::Num(-5));
        assert_eq!(eval_str("+5").unwrap(), V::Num(5));
        assert_eq!(eval_str("~0").unwrap(), V::Num(-1));
        assert_eq!(eval_str("!0").unwrap(), V::Num(1));
        assert_eq!(eval_str("!1").unwrap(), V::Num(0));
    }

    #[test]
    fn ternary_both_branches() {
        assert_eq!(eval_str("1 ? 20 : 30").unwrap(), V::Num(20));
        assert_eq!(eval_str("0 ? 20 : 30").unwrap(), V::Num(30));
    }

    #[test]
    fn call_dispatch() {
        assert_eq!(eval_str("max(3, 9)").unwrap(), V::Num(9));
        let node = parse_expr("max(1, 2)", None);
        let mut ops = Ops::default();
        eval(&node, &mut ops).unwrap();
        assert_eq!(ops.calls, vec!["max/2".to_string()]);
    }

    #[test]
    fn arithmetic_ops() {
        for (src, want) in [
            ("1 + 2", 3),
            ("5 - 3", 2),
            ("4 * 3", 12),
            ("9 / 2", 4),
            ("9 % 2", 1),
            ("2 ** 3", 8),
            ("6 & 3", 2),
            ("4 | 1", 5),
            ("5 ^ 1", 4),
            ("1 << 3", 8),
            ("16 >> 2", 4),
        ] {
            assert_eq!(eval_str(src).unwrap(), V::Num(want), "{src}");
        }
    }

    #[test]
    fn numeric_comparisons() {
        assert_eq!(eval_str("1 == 1").unwrap(), V::Num(1));
        assert_eq!(eval_str("1 != 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("1 < 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("2 <= 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("3 > 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("2 >= 3").unwrap(), V::Num(0));
    }

    #[test]
    fn string_fallback_comparisons() {
        // Both operands non-numeric → `num_or_str` falls back to compare_string.
        assert_eq!(eval_str("\"abc\" == \"abc\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"abc\" < \"abd\"").unwrap(), V::Num(1));
        // `eq`/`ne`/`lt`… always string-compare.
        assert_eq!(eval_str("\"a\" eq \"a\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"a\" ne \"b\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"a\" lt \"b\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"a\" le \"a\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"b\" gt \"a\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"b\" ge \"b\"").unwrap(), V::Num(1));
    }

    #[test]
    fn membership_ops() {
        assert_eq!(eval_str("2 in {1 2 3}").unwrap(), V::Num(1));
        assert_eq!(eval_str("5 in {1 2 3}").unwrap(), V::Num(0));
        assert_eq!(eval_str("5 ni {1 2 3}").unwrap(), V::Num(1));
        assert_eq!(eval_str("2 ni {1 2 3}").unwrap(), V::Num(0));
    }

    /// Only the delimiter says whether `expr` substitutes inside a string
    /// operand, so the walker hands a `"…"` operand to `string` as one that
    /// substitutes and a `{…}` one as one that does not.
    #[test]
    fn a_quoted_operand_reaches_its_own_hook() {
        let node = parse_expr(r#""a" eq {a}"#, None);
        let mut ops = Ops {
            quoted: Some(Vec::new()),
            ..Ops::default()
        };
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(0));
        assert_eq!(ops.quoted, Some(vec!["a".to_owned()]));
        // A consumer that does not tell them apart reads both alike.
        assert_eq!(eval_str(r#""a" eq {a}"#).unwrap(), V::Num(1));
    }

    #[test]
    fn continuation_retains_operands_and_requests_each_effect_once() {
        let mut ops = Ops::default();
        let mut state = ExprEvalState::new(parse_expr("max([first], $x + [second])", None));
        assert!(
            matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Request(ExprEvalRequest::Command { text: script, .. }) if script == "first")
        );
        state.resume(V::Num(3));
        assert!(
            matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Request(ExprEvalRequest::Variable { reference, .. }) if reference == "$x")
        );
        state.resume(V::Num(10));
        assert!(
            matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Request(ExprEvalRequest::Command { text: script, .. }) if script == "second")
        );
        state.resume(V::Num(2));
        let ExprEvalStep::Request(ExprEvalRequest::Call { function, args, .. }) =
            state.advance(&mut ops).unwrap()
        else {
            panic!("expected math-function request")
        };
        assert_eq!(function, "max");
        assert_eq!(args, vec![V::Num(3), V::Num(12)]);
        state.resume(V::Num(12));
        assert!(matches!(
            state.advance(&mut ops).unwrap(),
            ExprEvalStep::Complete(V::Num(12))
        ));
        assert_eq!(ops.commands, [] as [String; 0]);
        assert_eq!(ops.calls, [] as [String; 0]);
    }

    #[test]
    fn continuation_keeps_distinct_original_extents_for_equal_term_text() {
        let mut ops = Ops::default();
        let mut state = ExprEvalState::new(parse_expr("[same] + [same]", None));
        assert!(matches!(state.advance(&mut ops).unwrap(),
            ExprEvalStep::Request(ExprEvalRequest::Command { text, start: 0, end: 5 })
                if text == "same"));
        state.resume(V::Num(1));
        assert!(matches!(state.advance(&mut ops).unwrap(),
            ExprEvalStep::Request(ExprEvalRequest::Command { text, start: 9, end: 14 })
                if text == "same"));
        state.resume(V::Num(2));
        assert!(matches!(
            state.advance(&mut ops).unwrap(),
            ExprEvalStep::Complete(V::Num(3))
        ));

        let mut state = ExprEvalState::new(parse_expr("\"same\" eq \"same\"", None));
        assert!(matches!(state.advance(&mut ops).unwrap(),
            ExprEvalStep::Request(ExprEvalRequest::SubstitutedString { text, start: 0, end: 5 })
                if text == "same"));
        state.resume(V::Str("same".into()));
        assert!(matches!(state.advance(&mut ops).unwrap(),
            ExprEvalStep::Request(ExprEvalRequest::SubstitutedString { text, start: 10, end: 15 })
                if text == "same"));
    }

    #[test]
    fn continuation_variable_requests_preserve_authored_index_grammar() {
        for reference in ["$arr([step])", "${arr([step])}", "${plain:name}"] {
            let mut ops = Ops::default();
            let mut state = ExprEvalState::new(parse_expr(reference, None));
            assert!(
                matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Request(ExprEvalRequest::Variable { reference: source, .. }) if source == reference)
            );
            state.resume(V::Num(7));
            assert!(matches!(
                state.advance(&mut ops).unwrap(),
                ExprEvalStep::Complete(V::Num(7))
            ));
        }
    }

    #[test]
    fn continuation_short_circuits_after_resumed_conditions() {
        for (source, resumed, expected) in [
            ("[condition] && [skipped]", 0, 0),
            ("[condition] || [skipped]", 1, 1),
            ("[condition] ? 7 : [skipped]", 1, 7),
            ("[condition] ? [skipped] : 9", 0, 9),
        ] {
            let mut ops = Ops::default();
            let mut state = ExprEvalState::new(parse_expr(source, None));
            assert!(
                matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Request(ExprEvalRequest::Command { text: script, .. }) if script == "condition")
            );
            state.resume(V::Num(resumed));
            assert!(
                matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Complete(V::Num(value)) if value == expected)
            );
        }
        let mut ops = Ops::default();
        let mut state = ExprEvalState::new(parse_expr("\"prefix[step]\" eq {prefixDONE}", None));
        assert!(
            matches!(state.advance(&mut ops).unwrap(), ExprEvalStep::Request(ExprEvalRequest::SubstitutedString { text, .. }) if text == "prefix[step]")
        );
        state.resume(V::Str("prefixDONE".to_owned()));
        assert!(matches!(
            state.advance(&mut ops).unwrap(),
            ExprEvalStep::Complete(V::Num(1))
        ));
    }

    #[test]
    fn short_circuit_logical() {
        // `&&`: when left is false the right is NOT evaluated (records nothing).
        let node = parse_expr("0 && [boom]", None);
        let mut ops = Ops::default();
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(0));
        assert!(ops.commands.is_empty(), "right side must be skipped");
        // `&&` true path evaluates the right operand.
        assert_eq!(eval_str("1 && 1").unwrap(), V::Num(1));
        assert_eq!(eval_str("1 && 0").unwrap(), V::Num(0));
        // `||`: when left is true the right is NOT evaluated.
        let node = parse_expr("1 || [boom]", None);
        let mut ops = Ops::default();
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(1));
        assert!(ops.commands.is_empty(), "right side must be skipped");
        // `||` false path evaluates the right operand.
        assert_eq!(eval_str("0 || 1").unwrap(), V::Num(1));
        assert_eq!(eval_str("0 || 0").unwrap(), V::Num(0));
    }

    #[test]
    fn dialect_operator_routes_to_binary_other_default() {
        // A dialect operator (`Contains`) the shared core doesn't handle hits the
        // trait-default `binary_other`, which returns `unsupported`.
        let node = ExprNode::Binary {
            op: BinOp::Contains,
            left: Box::new(parse_expr("1", None)),
            right: Box::new(parse_expr("2", None)),
        };
        let mut ops = Ops::default();
        let err = eval(&node, &mut ops).unwrap_err();
        assert_eq!(err, "unsupported: operator");
    }

    #[test]
    fn raw_node_is_unsupported() {
        let node = ExprNode::Raw {
            text: "@#%".to_string(),
        };
        let mut ops = Ops::default();
        let err = eval(&node, &mut ops).unwrap_err();
        assert_eq!(err, "unsupported: syntax error in expression");
    }

    #[test]
    fn error_propagates_from_operand_seams() {
        // A non-numeric arithmetic operand surfaces the consumer's error
        // (the `?` propagation through eval_binary's arith arm).
        assert!(eval_str("\"abc\" + 1").is_err());
        // Non-numeric unary operand likewise propagates.
        assert!(eval_str("-\"abc\"").is_err());
        // Error from a nested operand short-circuits the whole walk.
        assert!(eval_str("(\"x\" + 1) * 2").is_err());
    }

    #[test]
    fn strip_delims_variants() {
        assert_eq!(strip_delims_bytes(b"{abc}"), b"abc");
        assert_eq!(strip_delims_bytes(b"\"abc\""), b"abc");
        assert_eq!(strip_delims_bytes(b"abc"), b"abc");
        assert_eq!(strip_delims_bytes(b"x"), b"x"); // too short to strip
    }

    #[test]
    fn strip_brackets_variants() {
        assert_eq!(strip_brackets_bytes(b"[cmd]"), b"cmd");
        assert_eq!(strip_brackets_bytes(b"cmd"), b"cmd");
    }
}
