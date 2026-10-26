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

//! The runtime's `expr` value operations — a thin [`tcl_syntax::expr::ExprOps`]
//! implementation over the numeric tower ([`crate::bignum`]). The evaluation
//! *walk* (operator dispatch, short-circuit `&&`/`||`, `?:`, the
//! numeric-vs-string comparison rule, `eq`/`ne`/`in`/`ni`) is **shared** with the
//! compiler via `tcl-syntax` (the same way the lexer/parser are shared); only the
//! value-type-specific bits live here — the tower arithmetic, `Tcl_Obj`
//! construction, `$var`/`[cmd]` resolution, and boolean coercion.
//!
//! `$var`/`[cmd]` resolve through caller closures (the interp wires its frame +
//! eval machinery; tests use mocks). Refcounts are managed by the [`Owned`] RAII
//! guard so every early return in the shared walk releases cleanly.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod native_math;
mod native_numeric;
pub(crate) use native_math::MathOperation;
mod native_values;
use native_numeric::NumericOperation;

use core::cmp::Ordering;

use std::rc::Rc;

use crate::bignum::{self, ArithError};
use crate::obj::{self, TclObj, TclObjType};

// Compatibility re-exports; object lifetime and error transport do not need the engine.
pub use crate::expr_error::ExprError;
pub use crate::obj::Owned;
use tcl_syntax::expr::errors::{OperandDesc, OperandSide};
use tcl_syntax::expr::mathfunc::MathFuncError;
use tcl_syntax::expr::{eval, BinOp, ExprNode, ExprOps, NumericCompare, UnaryOp};

/// Immutable original expression backing. Jim terms retain their own objects,
/// including unvisited lazy branches and original command Source descriptors.
struct CachedExpr {
    parser_policy: Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )>,
    node: Option<Rc<tcl_syntax::expr::NativeExprNode>>,
    dialect: tcl_registry::InvocationDialect,
    jim: Option<Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Owned>>>,
    source_origin: Option<Rc<crate::native_source::NativeJimSourceOrigin>>,
}

pub(crate) fn native_jim_source_origin(
    value: *mut TclObj,
    context: &Rc<crate::native_source::NativeJimObjectContext>,
) -> Result<Option<Rc<crate::native_source::NativeJimSourceOrigin>>, tcl_syntax::value::ValueError>
{
    let Some(backing) = expression_backing(value) else {
        return Ok(None);
    };
    let origin = backing
        .source_origin
        .as_ref()
        .filter(|origin| origin.belongs_to(value))
        .cloned();
    if origin.is_some() && !Rc::ptr_eq(&crate::native_source::context(value)?, context) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "Jim Expression original interpreter",
        ));
    }
    Ok(origin)
}

/// C expression source cache; its resident spelling is never regenerated.
pub static TCL_EXPR_TYPE: TclObjType = TclObjType {
    name: c"expr".as_ptr(),
    free_int_rep_proc: Some(expr_free),
    dup_int_rep_proc: Some(expr_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};

/// Jim expression tree, including a prepared rejected Expression(NULL).
pub(crate) static JIM_EXPR_TYPE: TclObjType = TclObjType {
    name: c"expression".as_ptr(),
    free_int_rep_proc: Some(expr_free),
    dup_int_rep_proc: Some(jim_expr_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};

fn expression_type(dialect: tcl_registry::InvocationDialect) -> &'static TclObjType {
    if dialect.native_string_protocol()
        == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
    {
        &JIM_EXPR_TYPE
    } else {
        &TCL_EXPR_TYPE
    }
}

fn expression_backing(value: *mut TclObj) -> Option<Rc<CachedExpr>> {
    if obj::obj_type_ptr(value) != &TCL_EXPR_TYPE && obj::obj_type_ptr(value) != &JIM_EXPR_TYPE {
        return None;
    }
    // SAFETY: both exact descriptors own a boxed Rc to the immutable backing.
    Some(unsafe { Rc::clone(&*(obj::internal_rep(value) as usize as *const Rc<CachedExpr>)) })
}

fn install_expression(value: *mut TclObj, backing: Rc<CachedExpr>) {
    let descriptor = expression_type(backing.dialect);
    obj::change_type(
        value,
        descriptor,
        Box::into_raw(Box::new(backing)) as usize as u64,
    );
}

extern "C" fn expr_free(value: *mut TclObj) {
    // SAFETY: the exact expression descriptor owns this boxed Rc.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut Rc<CachedExpr>
        ))
    };
}

extern "C" fn expr_dup(original: *mut TclObj, duplicate: *mut TclObj) {
    install_expression(
        duplicate,
        expression_backing(original).expect("live C expression backing"),
    );
}

extern "C" fn jim_expr_dup(_original: *mut TclObj, _duplicate: *mut TclObj) {
    // Jim deliberately leaves the new header untyped, retaining only bytes.
}

/// Original matching expression tree, without treating a rejected tree as absent.
pub(crate) fn cached_expr(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    parser_policy: Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )>,
) -> Option<Rc<tcl_syntax::expr::NativeExprNode>> {
    let cached = expression_backing(value)?;
    (cached.dialect == dialect && cached.parser_policy == parser_policy)
        .then(|| cached.node.clone())
        .flatten()
}

pub(crate) fn expression_rejected(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    parser_policy: Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )>,
) -> bool {
    expression_backing(value).is_some_and(|cached| {
        cached.dialect == dialect && cached.parser_policy == parser_policy && cached.node.is_none()
    })
}

pub(crate) fn native_jim_expression_objects(
    value: *mut TclObj,
) -> Option<Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Owned>>> {
    expression_backing(value)?.jim.clone()
}

/// Preserve the reached rejected primary after the original string is resident.
pub(crate) fn prepare_expr_cache(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    parser_policy: Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )>,
) {
    if obj::has_string_rep(value) {
        install_expression(
            value,
            Rc::new(CachedExpr {
                node: None,
                dialect,
                parser_policy,
                jim: None,
                source_origin: None,
            }),
        );
    }
}

/// Cache the C tree; a genuine Jim backing retains its existing original terms.
pub(crate) fn cache_expr(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    parser_policy: Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )>,
    node: &Rc<tcl_syntax::expr::NativeExprNode>,
) {
    if !obj::has_string_rep(value)
        || (native_jim_expression_objects(value).is_some()
            && cached_expr(value, dialect, parser_policy).is_some())
    {
        return;
    }
    install_expression(
        value,
        Rc::new(CachedExpr {
            node: Some(Rc::clone(node)),
            dialect,
            parser_policy,
            jim: None,
            source_origin: None,
        }),
    );
}

pub(crate) struct JimExpressionInstall<'a> {
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) parser_policy: Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )>,
    pub(crate) node: &'a tcl_syntax::expr::NativeExprNode,
    pub(crate) source: &'a [u8],
    pub(crate) preparation: &'a tcl_syntax::expr::native_objects::JimExpressionPreparation,
    pub(crate) info: &'a crate::native_source::NativeJimSourceInfo,
}

pub(crate) fn install_jim_expression(
    value: *mut TclObj,
    input: JimExpressionInstall<'_>,
) -> Result<(), tcl_syntax::value::ValueError> {
    use tcl_syntax::expr::native_objects::{JimExpressionObjects, JimExpressionTermValue};
    let context = crate::native_source::context(value)?;
    let source_origin = crate::native_source::original_source_origin(value, &context)?;
    let objects =
        JimExpressionObjects::prepare(input.source, input.preparation, |term, payload| {
            let value = Owned::fresh(match payload {
                JimExpressionTermValue::Number(
                    tcl_syntax::scalar_getter::JimExpressionNumber::Integer(number),
                ) => obj::new_wide_int_obj(number),
                JimExpressionTermValue::Number(
                    tcl_syntax::scalar_getter::JimExpressionNumber::Double(number),
                ) => obj::new_double_obj(number),
                JimExpressionTermValue::String(bytes) => obj::new_string_bytes(bytes),
            });
            crate::native_source::bind_context(value.as_ptr(), &context)?;
            if term.kind == tcl_lexer::ExprTermKind::Command {
                crate::native_source::install_source_from(
                    value.as_ptr(),
                    crate::native_source::NativeJimSourceInfo {
                        filename: input.info.filename.clone(),
                        line: input.info.line.wrapping_add_unsigned(term.line_delta),
                    },
                    &context,
                    source_origin.as_ref().map(|origin| {
                        (
                            origin,
                            tcl_syntax::jim_script_objects::JimScriptObjectKind::Source(
                                tcl_lexer::JimScriptTokenKind::Command,
                            ),
                        )
                    }),
                )?;
            }
            Ok(value)
        })?;
    install_expression(
        value,
        Rc::new(CachedExpr {
            dialect: input.dialect,
            parser_policy: input.parser_policy,
            node: Some(Rc::new(input.node.clone())),
            jim: Some(Rc::new(objects)),
            source_origin,
        }),
    );
    Ok(())
}

/// Keeps the original parent and actual tree alive during evaluation, then
/// reinstalls the same backing even when a reached operation shimmered it.
pub(crate) struct ExpressionLease {
    original: Owned,
    backing: Rc<CachedExpr>,
}
impl Drop for ExpressionLease {
    fn drop(&mut self) {
        install_expression(self.original.as_ptr(), Rc::clone(&self.backing));
    }
}
pub(crate) fn retain_expression_primary(value: *mut TclObj) -> Option<ExpressionLease> {
    let backing = expression_backing(value)?;
    backing.jim.as_ref()?;
    Some(ExpressionLease {
        original: Owned::retain(value),
        backing,
    })
}

#[cfg(test)]
thread_local! {
    /// Test hook: how many expression *parses* have run since the last reset.
    /// The cache's whole job is to keep this from growing per evaluation.
    static EXPR_PARSE_COUNT: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Test hook: record that the parser ran.
#[cfg(test)]
pub(crate) fn note_expr_parse() {
    EXPR_PARSE_COUNT.with(|c| c.set(c.get() + 1));
}

/// Test hook: reset the parse counter and read it back.
#[cfg(test)]
pub(crate) fn reset_expr_parse_count() {
    EXPR_PARSE_COUNT.with(|c| c.set(0));
}

/// Test hook: expression parses since [`reset_expr_parse_count`].
#[cfg(test)]
pub(crate) fn expr_parse_count() -> u64 {
    EXPR_PARSE_COUNT.with(core::cell::Cell::get)
}

pub(crate) fn arith_err(e: ArithError) -> ExprError {
    use tcl_syntax::expr::errors::NativeArithmeticFailure as Failure;
    let failure = match e {
        ArithError::DivideByZero => Some(Failure::DivideByZero),
        ArithError::ZeroToNegativePower => Some(Failure::ZeroToNegativePower),
        ArithError::NanResult => Some(Failure::NanResult),
        ArithError::NegativeShift => Some(Failure::NegativeShift),
        _ => None,
    };
    if let Some(failure) = failure {
        let (message, code) = failure.diagnostic();
        return code.map_or_else(
            || ExprError::msg(message.as_bytes()),
            |code| ExprError::with_code(message.as_bytes(), code.as_bytes()),
        );
    }
    match e {
        ArithError::NonNumeric => {
            ExprError::msg(b"can't use non-numeric string as operand of arithmetic")
        }
        ArithError::NonInteger => {
            ExprError::msg(b"can't use floating-point value as operand of bitwise op")
        }
        ArithError::DivideByZero
        | ArithError::ZeroToNegativePower
        | ArithError::NanResult
        | ArithError::NegativeShift => {
            unreachable!("shared native arithmetic diagnostic handled above")
        }
        ArithError::ExponentTooLarge => ExprError::msg(b"exponent too large"),
        ArithError::TooLargeToRepresent => ExprError::msg(b"integer value too large to represent"),
        ArithError::Alloc => ExprError::msg(b"out of memory"),
    }
}

/// C's `IllegalExprOperandType` (`tclExecute.c`), through the shared owner
/// [`tcl_syntax::expr::errors`]: the *wording* is a release axis (9.0 names
/// the value and the side, 8.4-8.6 name neither and have no list branch),
/// while the `-errorcode ARITH DOMAIN <description>` is invariant.
fn operand_type_err(
    desc: OperandDesc,
    value: &[u8],
    side: OperandSide,
    op: &[u8],
    dialect: tcl_registry::InvocationDialect,
    stage: tcl_registry::native_numeric_error::NativeExpressionOperandStage,
) -> ExprError {
    let release = dialect
        .native_string_protocol()
        .and_then(|protocol| protocol.tcl_version())
        .or_else(|| {
            dialect.byte_array_string_recipe(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )).and_then(|recipe| recipe.protocol().tcl_version())
        });
    let Some(release) = release else {
        return ExprError::host_refusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "expression operand diagnostic",
            ),
        );
    };
    let message =
        tcl_syntax::expr::errors::illegal_operand_message_bytes(desc, value, side, op, release);
    let code = tcl_syntax::expr::errors::illegal_operand_error_code(
        desc,
        &String::from_utf8_lossy(value),
        release,
    );
    let error = ExprError::from_parts(message, code.into_bytes());
    if desc == OperandDesc::NonNumericFloatingPointValue {
        error
    } else {
        error.with_invalid_type_stage(dialect, stage)
    }
}

/// How C describes `o` when an operator cannot use it: a NaN is a
/// "non-numeric floating-point value", a well-formed multi-element list is
/// 9.0's list branch, a double handed to an integer-only operator is a
/// "floating-point value", and anything else is a "non-numeric string".
fn operand_desc(
    o: *mut TclObj,
    float_operand: bool,
    bytes: &[u8],
    dialect: tcl_registry::InvocationDialect,
) -> OperandDesc {
    if float_operand {
        return OperandDesc::FloatingPointValue;
    }
    if obj::obj_type_ptr(o) == &obj::TCL_DOUBLE_TYPE && obj::double_of(o).is_nan() {
        return OperandDesc::NonNumericFloatingPointValue;
    }
    if dialect
        .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ))
        .is_some_and(|recipe| {
            tcl_syntax::list::split_native_list_elements(bytes, recipe.protocol())
                .is_ok_and(|members| members.len() > 1)
        })
    {
        return OperandDesc::List;
    }
    OperandDesc::NonNumericString
}

/// Map a binary operator to its source symbol (for operand-type errors).
fn binop_sym(op: BinOp) -> &'static [u8] {
    match op {
        BinOp::Add => b"+",
        BinOp::Sub => b"-",
        BinOp::Mul => b"*",
        BinOp::Div => b"/",
        BinOp::Mod => b"%",
        BinOp::Pow => b"**",
        BinOp::BitAnd => b"&",
        BinOp::BitOr => b"|",
        BinOp::BitXor => b"^",
        BinOp::LShift => b"<<",
        BinOp::RShift => b">>",
        _ => b"?",
    }
}

fn original_operand_bytes(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<Vec<u8>, ExprError> {
    let current = || {
        currency.map_or(Ok(()), |currency| {
            currency
                .ensure_current_or_refuse()
                .map_err(ExprError::from_execution_refusal)
        })
    };
    current()?;
    let recipe = dialect
        .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ))
        .ok_or_else(|| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            )
        })?;
    let result = crate::dict::native_object_bytes(value, recipe.protocol());
    current()?;
    result.map_err(|error| ExprError::from_cmd_error(error.into()))
}

fn selected_unary_error(
    error: ArithError,
    op: UnaryOp,
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> ExprError {
    use tcl_registry::native_numeric_error::NativeExpressionOperandStage as Stage;
    if !matches!(error, ArithError::NonNumeric | ArithError::NonInteger) {
        return arith_err(error);
    }
    let current = || {
        currency.map_or(Ok(()), |currency| {
            currency
                .ensure_current_or_refuse()
                .map_err(ExprError::from_execution_refusal)
        })
    };
    if let Err(error) = current() {
        return error;
    }
    if let Some(presentation) = dialect.expression_operand_error_presentation() {
        if let Some(message) = presentation.non_numeric_unary_message(op) {
            // JimExprOpNumUnary reaches the neutral public Boolean primitive
            // solely to classify rejected numeric operands, never logical truth.
            let result = crate::typed_value::native_scalar_probe_with_environment_and_currency(
                value,
                dialect,
                tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean,
                environment,
                currency,
            );
            if let Err(error) = current() {
                return error;
            }
            match result {
                Ok(Ok(_)) => return ExprError::from_bytes(message),
                Ok(Err(_)) => {}
                Err(cause) => return ExprError::from_cmd_error(cause.into()),
            }
        }
    }
    let bytes = match original_operand_bytes(value, dialect, currency) {
        Ok(bytes) => bytes,
        Err(error) => return error,
    };
    let stage = if op == UnaryOp::BitNot {
        Stage::Integer
    } else {
        Stage::Boolean
    };
    if let Some(presentation) = dialect.expression_operand_error_presentation() {
        return ExprError::from_bytes(presentation.message(stage, &bytes));
    }
    let symbol: &[u8] = match op {
        UnaryOp::Pos => b"+",
        UnaryOp::Neg => b"-",
        UnaryOp::BitNot => b"~",
        _ => b"?",
    };
    operand_type_err(
        operand_desc(value, error == ArithError::NonInteger, &bytes, dialect),
        &bytes,
        OperandSide::Unary,
        symbol,
        dialect,
        if op == UnaryOp::BitNot {
            Stage::Integer
        } else {
            Stage::FloatingPoint
        },
    )
}

/// The evaluation context the tower `ExprOps` resolves `$var`/`[cmd]` through —
/// one `&mut` borrow (the interp implements this; tests use a mock). A single
/// trait (vs two closures) avoids double-borrowing the interp for var-read +
/// command-eval.
pub trait ExprCtx {
    /// Actual entered Host capability, independently of native grammar.
    fn numeric_host(&self) -> Option<Rc<dyn tcl_platform::Host>> {
        None
    }
    /// The genuine active interpreter that owns reached Boolean publication.
    /// A neutral mathematical/C-only context supplies no Jim reporting owner.
    fn boolean_interpreter(&mut self) -> Option<&mut crate::interp::Interp> {
        None
    }

    /// Reach the original selected operand conversion, retaining first Host.
    fn original_boolean(
        &mut self,
        value: *mut TclObj,
        purpose: tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose,
    ) -> Result<bool, ExprError> {
        let dialect = self.invocation_dialect();
        let host = self.numeric_host();
        let result = if let Some(interp) = self.boolean_interpreter() {
            crate::typed_value::native_boolean_for_interp(interp, value, purpose)
        } else {
            crate::typed_value::native_boolean_in(
                value,
                dialect,
                purpose,
                host.as_ref().and_then(|host| host.numeric_environment()),
            )
        };
        result.map_err(ExprError::from_cmd_error)
    }

    /// Retain the original lhs for the selected final logical conversion.
    fn original_logical_right(
        &mut self,
        left: *mut TclObj,
        right: *mut TclObj,
        conjunction: bool,
    ) -> Result<bool, ExprError> {
        let dialect = self.invocation_dialect();
        if let Some(interp) = self.boolean_interpreter() {
            return crate::typed_value::native_logical_right_for_interp(
                interp,
                left,
                right,
                conjunction,
            )
            .map_err(ExprError::from_cmd_error);
        }
        let purpose = dialect
            .native_logical_final_operand_purpose(conjunction)
            .ok_or_else(|| {
                ExprError::from_cmd_error(
                    tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into(),
                )
            })?;
        let legacy = matches!(purpose, tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::LogicalAndInstruction | tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::LogicalOrInstruction);
        let left = if legacy {
            Some(self.original_boolean(left, purpose)?)
        } else {
            None
        };
        let right = self.original_boolean(right, purpose)?;
        Ok(left.map_or(right, |left| {
            if conjunction {
                left && right
            } else {
                left || right
            }
        }))
    }

    /// Actually perform the result producer before granting its owned operand.
    fn original_expression_result(
        &mut self,
        value: *mut TclObj,
        production: tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction,
    ) -> Result<Owned, ExprError> {
        let dialect = self.invocation_dialect();
        let host = self.numeric_host();
        let result = if let Some(interp) = self.boolean_interpreter() {
            crate::typed_value::normalize_boolean_result_for_interp(interp, value, production)
        } else {
            crate::typed_value::normalize_boolean_result_in(
                value,
                dialect,
                production,
                host.as_ref().and_then(|host| host.numeric_environment()),
            )
        };
        result.map_err(ExprError::from_cmd_error)
    }
    /// Exact native grammar and arithmetic policy for this evaluation.
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect;
    /// Independently installed authored F5 operator provider. Native dialects
    /// and contexts without the explicit parser retain no such capability.
    fn f5_string_predicate_provider(
        &self,
    ) -> Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider> {
        None
    }
    /// Whether the admitted compiler retained results for complete subtrees.
    fn has_compiled_nodes(&self) -> bool {
        false
    }
    /// Reuse a result belonging to this exact checked tree before visiting its leaves.
    fn compiled_node(
        &mut self,
        _node: &tcl_syntax::expr::NativeExprNode,
    ) -> Option<Result<Owned, ExprError>> {
        None
    }
    /// Original literal from an admitted compiled expression's literal bank.
    /// Ordinary expression evaluation does not supply this capability.
    fn compiled_literal(&mut self, _start: u32, _end: u32) -> Option<Owned> {
        None
    }
    /// Resolve a retained compiled variable operand at its original offset.
    fn compiled_variable(&mut self, reference: &[u8], _start: u32) -> Result<Owned, ExprError> {
        self.read_variable_reference_bytes(reference)
    }
    /// Execute an original bracket program belonging to the same compiled unit.
    fn compiled_command(
        &mut self,
        script: &[u8],
        _start: u32,
        _end: u32,
    ) -> Result<Owned, ExprError> {
        self.eval_command_bytes(script)
    }
    /// Substitute an original quoted operand through its retained arena.
    fn compiled_string(
        &mut self,
        inner: &[u8],
        _start: u32,
        _end: u32,
    ) -> Result<Owned, ExprError> {
        self.subst_string_bytes(inner)
    }
    /// Invoke the original compiled function-head operand after its arguments.
    fn compiled_call(
        &mut self,
        name: &str,
        args: &[Owned],
        _start: u32,
    ) -> Result<Owned, ExprError> {
        self.call_function(name, args)
    }
    /// Resolve a `$name` reference to an owned value, or `Err` (`can't read …`).
    fn read_var(&mut self, name: &str) -> Result<Owned, ExprError>;
    /// Resolve authored variable syntax without losing braced-name semantics.
    fn read_variable_reference(&mut self, reference: &str) -> Result<Owned, ExprError> {
        self.read_var(tcl_syntax::naming::var_reference(reference))
    }
    /// Evaluate a `[script]` (brackets stripped) to an owned result.
    fn eval_command(&mut self, script: &str) -> Result<Owned, ExprError>;
    /// Substitute the raw contents of a `"…"` operand — `$var`, `${var}`,
    /// `[cmd]`, and backslashes — exactly as a double-quoted word (C's
    /// expr parser quotes the operand and substitutes it). The default treats
    /// the contents literally (the standalone evaluator has no interp).
    fn subst_string(&mut self, inner: &str) -> Result<Owned, ExprError> {
        Ok(Owned::fresh(obj::new_string_bytes(inner.as_bytes())))
    }
    /// Resolve original byte variable syntax without a Unicode name projection.
    fn read_variable_reference_bytes(&mut self, reference: &[u8]) -> Result<Owned, ExprError> {
        let text = core::str::from_utf8(reference).map_err(|_| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression variable bytes",
                ),
            )
        })?;
        self.read_variable_reference(text)
    }
    /// Evaluate original command source bytes in the current activation.
    fn eval_command_bytes(&mut self, script: &[u8]) -> Result<Owned, ExprError> {
        let text = core::str::from_utf8(script).map_err(|_| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression command bytes",
                ),
            )
        })?;
        self.eval_command(text)
    }
    /// Evaluate the same original command token object. Implementations with
    /// an interpreter preserve its Source primary instead of rebuilding text.
    fn eval_command_object(&mut self, original: &Owned) -> Result<Owned, ExprError> {
        let _ = original;
        Err(ExprError::host_refusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original expression command object",
            ),
        ))
    }

    /// Substitute a native expression quoted operand's exact byte content.
    fn subst_string_bytes(&mut self, inner: &[u8]) -> Result<Owned, ExprError> {
        let text = core::str::from_utf8(inner).map_err(|_| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression quote bytes",
                ),
            )
        })?;
        self.subst_string(text)
    }

    /// Evaluate a `func(args…)` math-function call. The interp routes this
    /// through the command table (`::tcl::mathfunc::func`, so user overrides
    /// win — the A3 contract); the standalone evaluator falls back to the shared
    /// [`dispatch_shared`] built-in dispatch.
    fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError>;
}

/// The shared built-in math-function dispatch over the tower
/// ([`tcl_syntax::expr::mathfunc`]) — the fallback when a function isn't an
/// overridable command. `args` are the already-evaluated operands.
pub fn dispatch_shared(name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
    dispatch_shared_in(
        name,
        args,
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
    )
}

pub(crate) fn dispatch_shared_in(
    name: &str,
    args: &[Owned],
    dialect: tcl_registry::InvocationDialect,
) -> Result<Owned, ExprError> {
    struct StandaloneMath(tcl_registry::InvocationDialect);
    impl ExprCtx for StandaloneMath {
        fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
            self.0
        }
        fn read_var(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"standalone math variable"))
        }
        fn eval_command(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"standalone math command"))
        }
        fn call_function(&mut self, _: &str, _: &[Owned]) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"standalone math function"))
        }
    }
    // This explicit standalone context supplies no interpreter or native
    // reporting owner. Actual commands enter through InterpExprCtx instead.
    MathOperation::enter(&mut StandaloneMath(dialect))?.dispatch(name, args)
}

/// Select math integer conversion independently of the host runtime release.
pub(crate) fn int_width_for_dialect(
    dialect: tcl_registry::InvocationDialect,
) -> tcl_syntax::expr::mathfunc::IntWidth {
    use tcl_syntax::expr::mathfunc::IntWidth;
    dialect
        .tcl_version
        .map(IntWidth::for_tcl_version)
        .or_else(|| dialect.arithmetic().map(IntWidth::for_native_arithmetic))
        .unwrap_or(IntWidth::Unresolved)
}

/// Fixed-width functions cannot accidentally produce an arbitrary-precision
/// result just because the adapter links a bignum backend.
pub(crate) fn native_math_result(
    value: tcl_syntax::expr::mathfunc::NumValue<bignum::TowerMp>,
    dialect: tcl_registry::InvocationDialect,
) -> Result<*mut TclObj, ExprError> {
    use tcl_syntax::expr::mathfunc::NumValue;
    if let NumValue::Big(integer) = &value {
        if let Some(policy) = dialect
            .arithmetic()
            .filter(|policy| *policy != tcl_dialect::NativeArithmetic::TclBignum)
        {
            let integer = tcl_syntax::expr::wide::literal(policy, integer)
                .map_err(|error| wide_error(error, policy))?;
            return Ok(obj::new_wide_int_obj(integer));
        }
    }
    Ok(bignum::math_num_to_obj(value))
}

/// A shared math-function refusal as this engine's error: C's verbatim
/// message and `-errorcode`. `Abstain` cannot occur here — the
/// runtime's backend has an arbitrary-precision rung and its release is
/// resolved — so it falls back to the generic domain error.
pub(crate) fn math_func_err(e: MathFuncError) -> ExprError {
    let message = e.message();
    if message.is_empty() {
        return ExprError::with_code(
            tcl_syntax::expr::errors::DOMAIN_MESSAGE.as_bytes(),
            tcl_syntax::expr::errors::DOMAIN_CODE.as_bytes(),
        );
    }
    ExprError::with_code(message.as_bytes(), e.error_code().as_bytes())
}

/// Evaluate selected owned operands with the same original
/// arithmetic ingress as the expression walker; this performs no source eval.
pub(crate) fn eval_binary_operands(
    op: BinOp,
    left: Owned,
    right: Owned,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, ExprError> {
    native_numeric::binary(ctx, op, left, right)
}

/// The tower [`ExprOps`] over an [`ExprCtx`].
struct TowerOps<'a> {
    ctx: &'a mut dyn ExprCtx,
    jim: Option<Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Owned>>>,
    safe: bool,
}
impl TowerOps<'_> {
    fn original_term(
        &self,
        start: u32,
        end: Option<u32>,
    ) -> Result<Option<(tcl_lexer::ExprTermKind, Owned)>, ExprError> {
        let Some(objects) = &self.jim else {
            return Ok(None);
        };
        let (term, value) = objects.at(start, end).ok_or_else(|| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original Jim expression term extent",
                ),
            )
        })?;
        Ok(Some((term.kind, value.clone())))
    }
}

impl ExprOps for TowerOps<'_> {
    type Value = Owned;
    type Error = ExprError;

    fn prepared_node<Text: tcl_syntax::expr::ExprText>(
        &mut self,
        node: &ExprNode<Text>,
    ) -> Option<Result<Owned, ExprError>> {
        if !self.ctx.has_compiled_nodes() {
            return None;
        }
        let original = node.clone().map_text(|text| text.bytes().to_vec());
        self.ctx.compiled_node(&original)
    }

    fn literal_bytes_at(&mut self, text: &[u8], start: u32, end: u32) -> Result<Owned, ExprError> {
        if let Some(original) = self.ctx.compiled_literal(start, end) {
            return Ok(original);
        }
        match self.original_term(start, Some(end))? {
            Some((_, original)) => Ok(original),
            None => self.literal_bytes(text),
        }
    }
    fn string_bytes_at(
        &mut self,
        inner: &[u8],
        substitutes: bool,
        start: u32,
        end: u32,
    ) -> Result<Owned, ExprError> {
        if let Some(original) = self.ctx.compiled_literal(start, end) {
            return Ok(original);
        }
        if let Some((kind, original)) = self.original_term(start, Some(end))? {
            if kind == tcl_lexer::ExprTermKind::String {
                return Ok(original);
            }
        }
        if self.safe {
            return Err(ExprError::msg(b""));
        }
        if substitutes {
            self.ctx.compiled_string(inner, start, end)
        } else {
            self.string_bytes(inner, false)
        }
    }
    fn command_bytes_at(
        &mut self,
        script: &[u8],
        start: u32,
        end: u32,
    ) -> Result<Owned, ExprError> {
        let original = self.original_term(start, Some(end))?;
        if self.safe {
            return Err(ExprError::msg(b""));
        }
        match original {
            Some((_, original)) => self.ctx.eval_command_object(&original),
            None => self.ctx.compiled_command(script, start, end),
        }
    }
    fn literal_bytes(&mut self, text: &[u8]) -> Result<Owned, ExprError> {
        let text = core::str::from_utf8(text).map_err(|_| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression literal grammar",
                ),
            )
        })?;
        self.literal(text)
    }
    fn string_bytes(&mut self, inner: &[u8], substitutes: bool) -> Result<Owned, ExprError> {
        if !substitutes {
            let bytes = tcl_syntax::backslash::collapse_brace_continuations_for(
                inner,
                self.ctx.invocation_dialect().word_values.brace,
            );
            return Ok(Owned::fresh(obj::new_string_bytes(&bytes)));
        }
        self.ctx.subst_string_bytes(inner)
    }
    fn variable_reference_bytes_at(
        &mut self,
        reference: &[u8],
        start: u32,
    ) -> Result<Owned, ExprError> {
        if self.safe {
            return self.original_term(start, None)?.map(|(_, original)| original).ok_or_else(|| ExprError::host_refusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("original Jim safe variable term")));
        }
        self.ctx.compiled_variable(reference, start)
    }
    fn command_bytes(&mut self, script: &[u8]) -> Result<Owned, ExprError> {
        self.ctx.eval_command_bytes(script)
    }
    fn call_bytes_at(
        &mut self,
        function: &[u8],
        args: Vec<Owned>,
        start: u32,
    ) -> Result<Owned, ExprError> {
        let function = core::str::from_utf8(function).map_err(|_| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native math function grammar",
                ),
            )
        })?;
        self.ctx.compiled_call(function, &args, start)
    }

    fn literal(&mut self, text: &str) -> Result<Owned, ExprError> {
        make_literal(text, self.ctx.invocation_dialect())
    }
    fn string(&mut self, inner: &str, substitutes: bool) -> Result<Owned, ExprError> {
        // Only a `"…"` operand substitutes; a `{…}` one is its text with its
        // backslash-newlines folded, as they are even inside braces (#2227).
        if !substitutes {
            let text = tcl_syntax::backslash::collapse_brace_continuations(inner.as_bytes());
            return Ok(Owned::fresh(obj::new_string_bytes(&text)));
        }
        self.ctx.subst_string(inner)
    }
    fn var(&mut self, name: &str) -> Result<Owned, ExprError> {
        self.ctx.read_var(name)
    }
    fn variable_reference(&mut self, reference: &str) -> Result<Owned, ExprError> {
        self.ctx.read_variable_reference(reference)
    }
    fn command(&mut self, script: &str) -> Result<Owned, ExprError> {
        self.ctx.eval_command(script)
    }
    fn call(&mut self, function: &str, args: Vec<Owned>) -> Result<Owned, ExprError> {
        // Resolve `func(…)` through the context: the interp routes it to the
        // command table (`::tcl::mathfunc::func`, so overrides/renames win — A3),
        // falling back to the shared built-in dispatch for the standalone case.
        float_result(self.ctx.call_function(function, &args)?)
    }

    fn arith(&mut self, op: BinOp, left: Owned, right: Owned) -> Result<Owned, ExprError> {
        // 8.4 refuses a double quotient by zero (`divide by zero`) where it
        // computes one, and any non-finite result (`TclExprFloatError`).
        let divisor = (op == BinOp::Div).then(|| right.clone());
        let value = native_numeric::binary(self.ctx, op, left, right)?;
        if let Some(divisor) = divisor {
            if tcl_syntax::expr::errors::ambient_release().raises_float_range_errors()
                && !bignum::is_integer(value.ptr())
                && bignum::read_double(value.ptr()).is_some_and(|f| !f.is_finite())
                && bignum::read_double(divisor.ptr()) == Some(0.0)
            {
                return Err(arith_err(ArithError::DivideByZero));
            }
        }
        float_result(value)
    }

    fn unary(&mut self, op: UnaryOp, value: Owned) -> Result<Owned, ExprError> {
        match op {
            UnaryOp::Pos | UnaryOp::Neg | UnaryOp::BitNot => {
                native_numeric::unary(self.ctx, op, value)
            }
            UnaryOp::Not => {
                let dialect = self.ctx.invocation_dialect();
                let truth = self.ctx.original_boolean(
                    value.ptr(),
                    tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::LogicalNot,
                )?;
                native_integer_result(dialect, i64::from(!truth), &[])
            }
            UnaryOp::WordNot => Err(ExprError::msg(b"unsupported operator")),
        }
    }

    fn compare_numeric(&mut self, _left: &Owned, _right: &Owned) -> Option<NumericCompare> {
        // Executable object classification requires the fallible owner below.
        None
    }

    fn compare_numeric_checked(
        &mut self,
        left: &Owned,
        right: &Owned,
    ) -> Result<Option<NumericCompare>, ExprError> {
        let operation = NumericOperation::enter(self.ctx)?;
        let result = operation.with_environment(|environment| {
            let dialect = operation.dialect();
            let fixed = dialect
                .arithmetic()
                .is_some_and(|policy| policy != tcl_dialect::NativeArithmetic::TclBignum);
            let classify = |value: &Owned| -> Result<Option<Owned>, ExprError> {
                if fixed {
                    match fixed_integer_with_currency(
                        value.ptr(),
                        dialect,
                        environment,
                        operation.currency(),
                    ) {
                        Ok(Some(integer)) => {
                            return Ok(Some(Owned::fresh(obj::new_wide_int_obj(integer))))
                        }
                        Ok(None) => {}
                        Err(error)
                            if error.native_execution_refusal.is_some()
                                || error.native_access_refusal.is_some() =>
                        {
                            return Err(error)
                        }
                        Err(_) => return Ok(None),
                    }
                }
                operation
                    .number(value.ptr())?
                    .map(native_numeric::kernel_operand)
                    .transpose()
            };
            let Some(left) = classify(left)? else {
                return Ok(None);
            };
            let Some(right) = classify(right)? else {
                return Ok(None);
            };
            Ok(bignum::compare(left.ptr(), right.ptr()))
        });
        operation.after(result)
    }
    fn compare_string(&mut self, left: &Owned, right: &Owned) -> Result<Ordering, ExprError> {
        let operation = NumericOperation::enter(self.ctx)?;
        let result = (|| {
            let model = operation.dialect().characters.ok_or_else(|| {
                ExprError::host_refusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable,
                )
            })?;
            let left_bytes = operation.string(left.ptr())?;
            let right_bytes = operation.string(right.ptr())?;
            let counts = if model == tcl_dialect::StringCharacterModel::Jim084Utf8 {
                (
                    obj::jim_character_count(left.ptr()),
                    obj::jim_character_count(right.ptr()),
                )
            } else {
                (0, 0)
            };
            let left = tcl_syntax::raw_string::RawString::from_bytes(left_bytes);
            let right = tcl_syntax::raw_string::RawString::from_bytes(right_bytes);
            left.compare_character_units(model, &right, counts)
                .map_err(ExprError::host_refusal)
        })();
        operation.after(result)
    }
    fn equal_string(&mut self, left: &Owned, right: &Owned) -> Result<bool, ExprError> {
        if self.ctx.invocation_dialect().characters
            == Some(tcl_dialect::StringCharacterModel::Jim084Utf8)
        {
            let operation = NumericOperation::enter(self.ctx)?;
            let result = (|| Ok(operation.string(left.ptr())? == operation.string(right.ptr())?))();
            return operation.after(result);
        }
        Ok(self.compare_string(left, right)?.is_eq())
    }
    fn in_list(&mut self, needle: &Owned, list: &Owned) -> Result<bool, ExprError> {
        native_values::in_list(self.ctx, needle, list)
    }

    fn to_bool(&mut self, _value: &Owned) -> Result<bool, ExprError> {
        Err(ExprError::from_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "explicit expression Boolean purpose",
            )
            .into(),
        ))
    }
    fn to_bool_for_purpose(
        &mut self,
        value: &Owned,
        purpose: tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose,
    ) -> Result<bool, ExprError> {
        self.ctx.original_boolean(value.ptr(), purpose)
    }
    fn logical_right_truth(
        &mut self,
        left: &Owned,
        right: &Owned,
        conjunction: bool,
    ) -> Result<bool, ExprError> {
        self.ctx
            .original_logical_right(left.ptr(), right.ptr(), conjunction)
    }
    fn bool_value(&mut self, b: bool) -> Owned {
        native_integer_result(self.ctx.invocation_dialect(), i64::from(b), &[])
            .expect("selected boolean result producer")
    }
    fn binary_other(&mut self, op: BinOp, left: Owned, right: Owned) -> Result<Owned, ExprError> {
        native_values::f5_predicate(self.ctx, op, left, right)
    }

    fn unsupported(&mut self, what: &str) -> ExprError {
        ExprError::from_bytes(what.as_bytes().to_vec())
    }
}

/// Evaluate `node` over the tower, resolving `$var`/`[cmd]` via `ctx`.
pub fn eval_expr<Text: tcl_syntax::expr::ExprText>(
    node: &ExprNode<Text>,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, ExprError> {
    let mut ops = TowerOps {
        ctx,
        jim: None,
        safe: false,
    };
    let value = float_result(eval(node, &mut ops)?)?;
    ops.ctx.original_expression_result(value.as_ptr(), tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi)
}

/// `value`, or the error the release raises for a double it holds that is
/// not finite ([`tcl_syntax::expr::errors::float_result_error`]).
fn float_result(value: Owned) -> Result<Owned, ExprError> {
    let release = tcl_syntax::expr::errors::ambient_release();
    let refused = if bignum::is_integer(value.ptr()) {
        None
    } else {
        bignum::read_double(value.ptr())
            .and_then(|f| tcl_syntax::expr::errors::float_result_error(f, release))
    };
    match refused {
        Some((message, code)) => Err(ExprError::with_code(message.as_bytes(), code.as_bytes())),
        None => Ok(value),
    }
}

/// Execute the compiler's temporary constant program without the public
/// expression command's final result normalisation or a string getter.
pub(crate) fn eval_compiled_expression_node(
    node: &tcl_syntax::expr::NativeExprNode,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, ExprError> {
    float_result(eval(
        node,
        &mut TowerOps {
            ctx,
            jim: None,
            safe: false,
        },
    )?)
}

/// C8.4 TRY_CVT_TO_NUMERIC on a compiled primary. Conversion failures keep
/// the same original nonnumeric header; a shared resident numeric header
/// yields an absent-string duplicate with the same long/wide distinction.
pub(crate) fn normalize_compiled_primary84(
    value: Owned,
    interpreter: &mut crate::interp::Interp,
) -> Result<Owned, ExprError> {
    use crate::interp::native_operation_currency::NativeOperationScope;
    let scope =
        NativeOperationScope::enter(interpreter).map_err(ExprError::from_execution_refusal)?;
    let dialect = interpreter.native_invocation_dialect();
    let result = if dialect
        .native_scalar_getter_protocol()
        .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
        != Some(tcl_dialect::TclVersion::V8_4)
    {
        Err(ExprError::from_cmd_error(
            tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into(),
        ))
    } else {
        crate::typed_value::normalize_numeric_instruction_for_interp(interpreter, value.ptr())
            .map_err(ExprError::from_cmd_error)
    };
    drop(value);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(ExprError::from_execution_refusal)?;
    result
}

/// Replay a selected instruction with explicitly supplied recorded host facts.
/// This test-only context issues no interpreter or original source authority.
#[cfg(test)]
fn normalize_compiled_primary84_with_recorded_environment(
    value: Owned,
    dialect: tcl_registry::InvocationDialect,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
) -> Result<Owned, ExprError> {
    let result = if dialect
        .native_scalar_getter_protocol()
        .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
        != Some(tcl_dialect::TclVersion::V8_4)
    {
        Err(ExprError::from_cmd_error(
            tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into(),
        ))
    } else {
        crate::typed_value::normalize_numeric_instruction_in(value.ptr(), dialect, environment)
            .map_err(ExprError::from_cmd_error)
    };
    drop(value);
    result
}

/// Execute the compiler's temporary constant program without the public
/// expression command's final result normalisation or a string getter.
pub(crate) fn eval_compiler_constant(
    node: &tcl_syntax::expr::NativeExprNode,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, ExprError> {
    // Retain the original entry before evaluating any reached physical stage.
    let operation = NumericOperation::enter_values(ctx)?;
    let result = (|| {
        let evaluated = eval_compiled_expression_node(node, ctx);
        operation.check()?;
        let value = evaluated?;
        if matches!(node, ExprNode::Ternary { .. }) {
            // The admitted constant program reaches its numeric instruction;
            // an inline condition's skipped producer cannot replace it.
            let result = if let Some(interpreter) = operation.interpreter() {
                crate::typed_value::normalize_numeric_instruction_for_interp(
                    &mut interpreter.clone(),
                    value.ptr(),
                )
                .map_err(ExprError::from_cmd_error)
            } else {
                operation.with_environment(|environment| {
                    crate::typed_value::normalize_numeric_instruction_in(
                        value.ptr(),
                        operation.dialect(),
                        environment,
                    )
                    .map_err(ExprError::from_cmd_error)
                })
            };
            drop(value);
            operation.after(result)
        } else {
            Ok(value)
        }
    })();
    operation.after(result)
}

/// Evaluate an admitted Jim tree using its retained original terms. Safe mode
/// returns original variable token bodies and rejects reached substitutions.
pub(crate) fn eval_jim_expr(
    node: &tcl_syntax::expr::NativeExprNode,
    ctx: &mut dyn ExprCtx,
    objects: Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Owned>>,
    safe: bool,
) -> Result<Owned, ExprError> {
    eval(
        node,
        &mut TowerOps {
            ctx,
            jim: Some(objects),
            safe,
        },
    )
}

/// Jim's safe integer-expression context. Effects are rejected when reached,
/// retaining the shared evaluator's short-circuit behaviour.
pub(crate) fn eval_index_expression<Text: tcl_syntax::expr::ExprText>(
    node: &ExprNode<Text>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<Owned, ExprError> {
    struct SafeIndex {
        surface: tcl_registry::expr_surface::RuntimeExprSurface,
        dialect: tcl_registry::InvocationDialect,
    }
    impl ExprCtx for SafeIndex {
        fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
            self.dialect
        }
        fn read_var(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unsafe index variable"))
        }
        fn eval_command(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unsafe index script"))
        }
        fn subst_string(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unsafe index substitution"))
        }
        fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
            if self.surface.builtin_math_function(name).is_none() {
                return Err(ExprError::msg(b"unknown native index math function"));
            }
            dispatch_shared_in(name, args, self.dialect)
        }
    }
    eval_expr(
        node,
        &mut SafeIndex {
            surface: tcl_registry::expr_surface::RuntimeExprSurface::for_profile(profile),
            dialect: tcl_registry::InvocationDialect::of_profile(profile),
        },
    )
}

/// Drive `::tcl::mathop::<op>` over the tower: the shared `tcl_cmd_core::mathop`
/// fold/chain logic, each primitive going through this runtime's `ExprOps` (so
/// the same bignum behaviour as `expr`). `args` are already-evaluated operands;
/// `ctx`'s `$var`/`[cmd]` resolution is never invoked (a trivial ctx suffices).
pub fn eval_mathop(
    op: &str,
    args: Vec<Owned>,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, tcl_cmd_core::mathop::MathopError<ExprError>> {
    let mut ops = TowerOps {
        ctx,
        jim: None,
        safe: false,
    };
    tcl_cmd_core::mathop::eval(&mut ops, op, args)
}

// value helpers

/// C8.4's compiled jump inspects numeric primaries before the primitive
/// Boolean getter. In particular it leaves a registered native-long literal
/// unchanged instead of replacing it with a word-Boolean primary.
pub(crate) fn native_jump_boolean84(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<bool, ExprError> {
    crate::typed_value::native_boolean_in(
        value,
        dialect,
        tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
        None,
    )
    .map_err(ExprError::from_cmd_error)
}

/// C8.4 eager LAND/LOR consumes the original normalised left header and the
/// reached right value. Numeric-looking strings follow GET_WIDE_OR_INT;
/// the result reuses only an actually unshared left header.
pub(crate) fn native_logical84(
    dialect: tcl_registry::InvocationDialect,
    left: Owned,
    right: Owned,
    conjunction: bool,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
) -> Result<Owned, ExprError> {
    let purpose = dialect
        .native_logical_final_operand_purpose(conjunction)
        .ok_or_else(|| {
            ExprError::from_cmd_error(
                tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into(),
            )
        })?;
    let a = crate::typed_value::native_boolean_in(left.as_ptr(), dialect, purpose, environment)
        .map_err(ExprError::from_cmd_error)?;
    let b = crate::typed_value::native_boolean_in(right.as_ptr(), dialect, purpose, environment)
        .map_err(ExprError::from_cmd_error)?;
    let result = i64::from(if conjunction { a && b } else { a || b });
    if obj::is_shared(left.as_ptr()) {
        native_integer_result(dialect, result, &[])
    } else {
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or_else(|| {
                ExprError::host_refusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
                )
            })?;
        obj::adopt_native_scalar_cache(
            left.as_ptr(),
            tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(result),
            protocol,
        )
        .map_err(|error| {
            ExprError::host_refusal(error.native_access_refusal().expect("scalar cache refusal"))
        })?;
        obj::invalidate_string(left.as_ptr());
        Ok(left)
    }
}

/// Build an object from a literal token: a number through the shared grammar,
/// otherwise its original string spelling. Tcl preserves boolean literal text
/// (`expr {yes}` returns `yes`); coercion happens only in a boolean context.
fn make_literal(text: &str, dialect: tcl_registry::InvocationDialect) -> Result<Owned, ExprError> {
    use tcl_syntax::number::{parse_whole_with, Number, ParseFlags};
    if let Some(number) = parse_whole_with(text, ParseFlags::for_syntax(dialect.numbers)) {
        if let Some(policy) = dialect
            .arithmetic()
            .filter(|policy| *policy != tcl_dialect::NativeArithmetic::TclBignum)
        {
            if matches!(number, Number::Int(_) | Number::Big { .. }) {
                let value = tcl_syntax::expr::wide::parsed_literal(policy, &number)
                    .map_err(|error| wide_error(error, policy))?;
                return Ok(Owned::fresh(obj::new_wide_int_obj(value)));
            }
        }
        return Ok(Owned::fresh(match number {
            Number::Int(value) => obj::new_wide_int_obj(value),
            Number::Double(value) => obj::new_double_obj(value),
            Number::Big {
                negative,
                radix,
                digits,
            } => bignum::from_big_digits(negative, radix, &digits),
            Number::Nan { .. } => obj::new_double_obj(f64::NAN),
        }));
    }
    Ok(Owned::fresh(obj::new_string_bytes(text.as_bytes())))
}

fn operand_overflow(mut error: ExprError, operator: &[u8]) -> ExprError {
    if error.msg == tcl_syntax::expr::errors::IOVERFLOW_MESSAGE.as_bytes() {
        error.msg = tcl_syntax::expr::errors::oversized_integer_operand_message(
            std::str::from_utf8(operator).expect("ASCII expression operator"),
        )
        .into_bytes();
    }
    error
}

fn native_integer_result(
    dialect: tcl_registry::InvocationDialect,
    integer: i64,
    operands: &[*mut TclObj],
) -> Result<Owned, ExprError> {
    let result = Owned::fresh(obj::new_wide_int_obj(integer));
    if let Some(protocol) = dialect.native_scalar_getter_protocol() {
        if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
            let caches = operands
                .iter()
                .map(|value| obj::native_scalar_cache(*value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    ExprError::host_refusal(
                        error.native_access_refusal().expect("scalar cache refusal"),
                    )
                })?;
            let cache = protocol
                .expression_integer_result84(integer, &caches)
                .expect("selected C84 integer result");
            obj::adopt_native_scalar_cache(result.as_ptr(), cache, protocol).map_err(|error| {
                ExprError::host_refusal(
                    error.native_access_refusal().expect("scalar cache refusal"),
                )
            })?;
        }
    }
    Ok(result)
}

fn fixed_integer_with_currency(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<Option<i64>, ExprError> {
    let check_current = || {
        currency.map_or(Ok(()), |currency| {
            currency
                .ensure_current_or_refuse()
                .map_err(ExprError::from_execution_refusal)
        })
    };
    check_current()?;
    let Some(policy) = dialect.arithmetic() else {
        return Err(ExprError::host_refusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
        ));
    };
    if policy == tcl_dialect::NativeArithmetic::Tcl84Wide {
        let protocol = dialect.native_scalar_getter_protocol().ok_or_else(|| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            )
        })?;
        let cache = obj::native_scalar_cache(value).map_err(|error| {
            ExprError::host_refusal(error.native_access_refusal().expect("scalar cache refusal"))
        })?;
        let integer = matches!(
            &cache,
            Some(
                tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(_)
                    | tcl_syntax::scalar_getter::NativeScalarCache::Number(
                        tcl_syntax::number::Number::Int(_)
                    )
            )
        );
        let absent_double = matches!(
            &cache,
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Double(_) | tcl_syntax::number::Number::Nan { .. }
            ))
        ) && !obj::has_string_rep(value);
        if !integer && !absent_double && currency.is_some() && environment.is_none() {
            return Err(ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            ));
        }
        if !integer && !absent_double {
            let original = crate::bytearray::scalar_getter_string(value, protocol);
            check_current()?;
            let original = original.map_err(|error| ExprError::from_cmd_error(error.into()))?;
            let cached_wide = cache
                .as_ref()
                .map(|cache| {
                    protocol.cached_conversion(
                        tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
                        cache,
                        None,
                    )
                })
                .transpose()
                .map_err(|_| {
                    ExprError::host_refusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
                )
                })?
                .flatten();
            if let Some(environment) = environment.filter(|_| {
                protocol.expression_integer_spelling84(&original) && cached_wide.is_none()
            }) {
                let converted = tcl_cmd_core::native_numeric::fresh_c84_conversion(
                    protocol,
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
                    &original,
                    environment,
                );
                check_current()?;
                converted.map_err(|error| ExprError::from_cmd_error(error.into()))?;
            }
            if !protocol.expression_integer_spelling84(&original) && environment.is_some() {
                let converted =
                    crate::typed_value::native_scalar_probe_with_environment_and_currency(
                        value,
                        dialect,
                        tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
                        environment,
                        currency,
                    );
                check_current()?;
                let _ = converted.map_err(|error| ExprError::from_cmd_error(error.into()))?;
            }

            if let Some(conversion) =
                protocol.expression_integer_conversion84(cache.as_ref(), &original)
            {
                let (_, cache, outcome) = conversion.into_parts();
                if let Some(cache) = cache {
                    obj::adopt_native_scalar_cache(value, cache, protocol).map_err(|error| {
                        ExprError::host_refusal(
                            error.native_access_refusal().expect("scalar cache refusal"),
                        )
                    })?;
                    check_current()?;
                }
                if outcome.is_err() {
                    // This preflight selected the integer conversion, not the
                    // enclosing operator's illegal-operand completion.
                    return Ok(None);
                }
            }
        }
    }
    let parsed = crate::typed_value::scalar_number_with_currency(value, dialect, true, currency);
    check_current()?;
    let parsed = parsed.map_err(ExprError::from_execution_refusal)?;
    parsed
        .filter(|number| {
            matches!(
                number,
                tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
            )
        })
        .map(|number| {
            tcl_syntax::expr::wide::parsed_literal(policy, &number)
                .map_err(|error| wide_error(error, policy))
        })
        .transpose()
}

fn wide_error(
    error: tcl_syntax::expr::wide::WideError,
    policy: tcl_dialect::NativeArithmetic,
) -> ExprError {
    use tcl_syntax::expr::wide::WideError;
    let error = match error {
        WideError::LiteralOverflow => ExprError::with_code(
            b"integer value too large to represent",
            b"ARITH IOVERFLOW {integer value too large to represent}",
        ),
        WideError::DivisionByZero if policy == tcl_dialect::NativeArithmetic::JimWide => {
            ExprError::msg(b"Division by zero")
        }
        WideError::DivisionByZero => arith_err(ArithError::DivideByZero),
        WideError::ZeroToNegativePower => arith_err(ArithError::ZeroToNegativePower),
        WideError::UndefinedNativeOperation => {
            ExprError::msg(b"undefined native integer operation")
        }
        WideError::Unsupported => ExprError::msg(b"unsupported native integer operation"),
    };
    if policy == tcl_dialect::NativeArithmetic::Tcl84Wide {
        error.with_numeric_string_result84()
    } else {
        error
    }
}

#[cfg(have_tommath)]
pub(crate) fn retained_i64_add(
    ctx: &mut dyn ExprCtx,
    left: Owned,
    right: Owned,
    primitive: impl FnOnce(&mut dyn ExprCtx) -> Result<bool, ExprError>,
) -> Result<(Owned, bool), ExprError> {
    native_numeric::retained_i64_add(ctx, left, right, primitive)
}

#[cfg(have_tommath)]
pub(crate) fn eval_compiled_expression_operand(
    node: &tcl_syntax::expr::NativeExprNode,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, ExprError> {
    eval(
        node,
        &mut TowerOps {
            ctx,
            jim: None,
            safe: false,
        },
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_unary_boolean_classifier_keeps_primitive_guest_acceptance_separate() {
        // Source/API: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Pinned Jim source JimExprOpNumUnary reaches its public Boolean getter;
        // these adapter controls assert no new original execution observation.
        let interp = crate::interp::Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let dialect = interp.native_invocation_dialect();
        for (word, expected) in [
            (
                b"true".as_slice(),
                b"can't use non-numeric string as operand of \"+\"".as_slice(),
            ),
            (
                b"invalid".as_slice(),
                b"expected boolean but got \"invalid\"".as_slice(),
            ),
        ] {
            let original = Owned::fresh(obj::new_string_bytes(word));
            let error = selected_unary_error(
                ArithError::NonNumeric,
                UnaryOp::Pos,
                original.as_ptr(),
                dialect,
                None,
                None,
            );
            assert_eq!(error.msg, expected);
            assert!(error.native_access_refusal.is_none());
            assert!(error.native_execution_refusal.is_none());
        }
    }

    #[test]
    fn original_unary_classifier_preserves_retirement_first_host_and_prior_result() {
        // Source/API: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = crate::interp::Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.set_result_bytes(b"PRIOR\0\xff");
        let prior = interp.get_obj_result();
        let original = Owned::fresh(obj::new_string_bytes(b"true"));
        let pointer = original.as_ptr();
        let lifetime = obj::NativeObjectLifetime::retain(pointer);
        drop(original);
        assert!(!obj::allocation_is_live(pointer));
        let error = selected_unary_error(
            ArithError::NonNumeric,
            UnaryOp::Neg,
            pointer,
            interp.native_invocation_dialect(),
            None,
            None,
        );
        let cause = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object",
            ),
        );
        assert_eq!(error.native_execution_refusal, Some(cause.clone()));
        assert!(
            error
                .retained_command_error()
                .unwrap()
                .native_execution_refusal()
                == Some(&cause)
        );
        interp.report_expr_error(error);
        assert_eq!(interp.native_execution_refusal(), Some(cause));
        assert_eq!(interp.get_obj_result(), prior);
        assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
        drop(lifetime);
    }

    use super::*;
    use tcl_syntax::expr::parser::parse_expr;

    /// A mock context: a `$var` table; `[cmd]` is unsupported in these tests.
    struct MockCtx(std::collections::HashMap<String, i64>);
    impl ExprCtx for MockCtx {
        fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
        }
        fn read_var(&mut self, name: &str) -> Result<Owned, ExprError> {
            self.0
                .get(name)
                .map(|&v| Owned::fresh(obj::new_wide_int_obj(v)))
                .ok_or_else(|| ExprError::msg(b"can't read var"))
        }
        fn eval_command(&mut self, _script: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"no commands"))
        }
        fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
            // No command table in the mock — use the shared built-in dispatch.
            dispatch_shared(name, args)
        }
    }

    struct NativeCtx(tcl_registry::InvocationDialect);
    impl ExprCtx for NativeCtx {
        fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
            self.0
        }
        fn read_var(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected variable read"))
        }
        fn eval_command(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected command"))
        }
        fn call_function(&mut self, _: &str, _: &[Owned]) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected function"))
        }
    }

    #[test]
    fn c84_reached_expression_matches_seven_original_cache_controls() {
        use tcl_syntax::scalar_getter::{
            NativeScalarCache as Cache, NativeScalarGetterKind as Getter,
        };
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        let class = |value| match obj::native_scalar_cache(value).unwrap() {
            Some(Cache::Tcl84Long(_)) => "int",
            Some(Cache::Number(tcl_syntax::number::Number::Int(_))) => "wideInt",
            Some(Cache::Number(tcl_syntax::number::Number::Double(_))) => "double",
            None => "none",
            _ => panic!("original scalar class"),
        };
        let words = [b"2".as_slice(), b"2.0", b"2", b"2", b"2", b"0x10", b"010"];
        let mut compared = 0;
        for row in include_str!("../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mode = fields[0].parse::<usize>().unwrap();
            let original = Owned::fresh(match mode {
                2 => obj::new_double_obj(2.0),
                4 => obj::new_wide_int_obj(2),
                _ => obj::new_string_bytes(words[mode]),
            });
            if mode < 2 { crate::typed_value::native_scalar_getter(original.ptr(), dialect, Getter::Double).unwrap(); }
            if mode == 3 { crate::typed_value::native_scalar_getter(original.ptr(), dialect, Getter::Wide).unwrap(); }
            assert_eq!(class(original.ptr()), fields[1], "native before mode {mode}");
            assert_eq!(usize::from(obj::has_string_rep(original.ptr())).to_string(), fields[2]);
            let one = native_integer_result(dialect, 1, &[]).unwrap();
            let mut context = NativeCtx(dialect);
            let mut ops = TowerOps { ctx: &mut context, jim: None, safe: false };
            let result = ops.arith(BinOp::Add, original.clone(), one).unwrap();
            assert_eq!(fields[3], "0");
            assert_eq!(class(original.ptr()), fields[4], "native reached mode {mode}");
            assert_eq!(usize::from(obj::has_string_rep(original.ptr())).to_string(), fields[5]);
            assert_eq!(class(result.ptr()), fields[6], "native result mode {mode}");
            assert_eq!(usize::from(obj::has_string_rep(result.ptr())).to_string(), fields[7]);
            assert_eq!(obj::bytes_of(result.ptr()), fields[8].as_bytes());
            compared += 1;
        }
        assert_eq!(compared, 7);
    }

    #[test]
    fn checked_comparison_separates_jim_bytes_counts_and_storage_refusal() {
        crate::counters::reset();
        {
            let profile = crate::environment::profile_for_dialect("jim");
            let mut context = NativeCtx(tcl_registry::InvocationDialect::of_profile(profile));
            let mut ops = TowerOps {
                ctx: &mut context,
                jim: None,
                safe: false,
            };
            let raw = Owned::fresh(obj::new_string_bytes(&[0xff]));
            let utf8 = Owned::fresh(obj::new_string_bytes(&[0xc3, 0xbf]));
            assert!(!ops.equal_string(&raw, &utf8).unwrap());
            assert_eq!(ops.compare_string(&raw, &utf8).unwrap(), Ordering::Equal);
            let list = Owned::fresh(crate::list::new_list_obj(&[utf8.ptr()]));
            assert!(!ops.in_list(&raw, &list).unwrap());
            assert!(ops.in_list(&utf8, &list).unwrap());
            let cached = Owned::fresh(obj::new_string_bytes(b"ab"));
            obj::retain_jim_string_count(cached.ptr(), 5);
            let error = ops.compare_string(&cached, &cached).unwrap_err();
            assert!(matches!(
                error.native_access_refusal,
                Some(tcl_syntax::raw_string::NativeValueAccessRefusal::StringAccess(_))
            ));
            assert!(error.msg.is_empty() && error.code.is_none());
        }
        assert_eq!(crate::counters::finalize(), 0, "comparison ownership leak");
    }

    #[test]
    fn expression_refusal_bypasses_guest_capture_and_finally() {
        crate::counters::reset();
        {
            let mut interp = crate::interp::Interp::new();
            interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            interp
                .var_set(b"a", obj::new_string_bytes(&[0xff]))
                .unwrap();
            let code = interp.eval_str(b"set b okay; set prior 1; try {catch {expr {$a lt $b}} captured; set recovered 1} finally {set final 1}; set after 1");
            assert_eq!(code, crate::interp::Code::Error);
            assert!(matches!(
                interp.native_access_refusal(),
                Some(tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(_))
            ));
            assert_eq!(
                interp.var_get(b"prior").map(obj::bytes_of),
                Some(b"1".to_vec())
            );
            for name in [b"captured".as_slice(), b"recovered", b"final", b"after"] {
                assert!(
                    interp.var_get(name).is_none(),
                    "unexpected guest write: {name:?}"
                );
            }
        }
        assert_eq!(crate::counters::finalize(), 0, "refusal ownership leak");
    }

    #[test]
    fn selected_numeric_input_preserves_original_bytes_and_caches_the_actual_object() {
        crate::counters::reset();
        {
            let jim = tcl_registry::InvocationDialect::of_profile(
                crate::environment::profile_for_dialect("jim"),
            );
            let c = tcl_registry::InvocationDialect::of_profile(
                crate::environment::profile_for_dialect("tcl8.6"),
            );
            let raw = Owned::fresh(obj::new_string_bytes(&[b'1', 0, 0xff]));
            assert_eq!(
                crate::typed_value::scalar_number(raw.ptr(), c, true).unwrap(),
                None
            );
            assert_eq!(
                crate::typed_value::scalar_number(raw.ptr(), jim, true).unwrap(),
                Some(tcl_syntax::number::Number::Int(1))
            );
            assert!(core::ptr::eq(
                obj::obj_type_ptr(raw.ptr()),
                &obj::TCL_INT_TYPE
            ));
            assert_eq!(obj::bytes_of(raw.ptr()), [b'1', 0, 0xff]);
            let bad = Owned::fresh(obj::new_string_bytes(&[0xff, 0, b'1']));
            assert_eq!(
                crate::typed_value::scalar_number(bad.ptr(), jim, false).unwrap(),
                None
            );
            assert_eq!(obj::bytes_of(bad.ptr()), [0xff, 0, b'1']);
        }
        assert_eq!(
            crate::counters::finalize(),
            0,
            "numeric input ownership leak"
        );
    }

    #[test]
    fn jim_explicit_double_getter_keeps_exact_integer_and_lazy_string_cache() {
        use tcl_syntax::value::ValueOps;
        crate::counters::reset();
        {
            let mut interp = crate::interp::Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            for integer in [7, i64::MAX] {
                let original = Owned::fresh(obj::new_wide_int_obj(integer));
                let alias = original.clone();
                assert!(!obj::has_string_rep(original.ptr()));
                assert_eq!(interp.as_double(&alias.ptr()).unwrap(), integer as f64);
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(original.ptr()),
                    &obj::JIM_COERCED_DOUBLE_TYPE
                ));
                assert_eq!(obj::has_string_rep(original.ptr()), integer == i64::MAX);
                assert_eq!(interp.as_int(&original.ptr()).unwrap(), integer);
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(alias.ptr()),
                    &obj::TCL_INT_TYPE
                ));
                assert_eq!(obj::bytes_of(alias.ptr()), integer.to_string().as_bytes());
            }
            interp.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            let c = Owned::fresh(obj::new_wide_int_obj(7));
            assert_eq!(interp.as_double(&c.ptr()).unwrap(), 7.0);
            assert!(core::ptr::eq(
                obj::obj_type_ptr(c.ptr()),
                &obj::TCL_INT_TYPE
            ));
            assert!(!obj::has_string_rep(c.ptr()));
        }
        assert_eq!(
            crate::counters::finalize(),
            0,
            "coerced getter ownership leak"
        );
    }

    fn ev(src: &str, vars: &[(&str, i64)]) -> Result<Vec<u8>, ExprError> {
        crate::counters::reset();
        let node = parse_expr(src, None);
        let mut ctx = MockCtx(vars.iter().map(|&(k, v)| (k.to_string(), v)).collect());
        let r = eval_expr(&node, &mut ctx)?;
        let out = obj::bytes_of(r.ptr());
        drop(r);
        assert_eq!(crate::counters::finalize(), 0, "leak");
        Ok(out)
    }

    fn ok(src: &str) -> Vec<u8> {
        ev(src, &[]).expect("eval")
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(ok("1 + 2 * 3"), b"7");
        assert_eq!(ok("(1 + 2) * 3"), b"9");
        assert_eq!(ok("2 ** 10"), b"1024");
        assert_eq!(ok("2 ** 64"), b"18446744073709551616"); // bignum
        assert_eq!(ok("7 / 2"), b"3");
        assert_eq!(ok("-7 / 2"), b"-4"); // floor
        assert_eq!(ok("7 % 3"), b"1");
        assert_eq!(ok("1 + 2.5"), b"3.5"); // double promotion
    }

    #[test]
    fn bitwise_and_shifts() {
        assert_eq!(ok("0xff & 0x0f"), b"15");
        assert_eq!(ok("12 | 3"), b"15");
        assert_eq!(ok("5 ^ 3"), b"6");
        assert_eq!(ok("~5"), b"-6");
        assert_eq!(ok("1 << 4"), b"16");
        assert_eq!(ok("256 >> 4"), b"16");
    }

    #[test]
    fn comparisons_numeric_and_string() {
        assert_eq!(ok("3 < 5"), b"1");
        assert_eq!(ok("5 <= 5"), b"1");
        assert_eq!(ok("3 == 3"), b"1");
        assert_eq!(ok("3 != 4"), b"1");
        assert_eq!(ok(r#""abc" eq "abc""#), b"1");
        assert_eq!(ok(r#""abc" ne "abd""#), b"1");
    }

    #[test]
    fn short_circuit_and_ternary() {
        assert_eq!(ok("1 && 0"), b"0");
        assert_eq!(ok("0 || 1"), b"1");
        assert_eq!(ok("!0"), b"1");
        assert_eq!(ok("1 ? 42 : 99"), b"42");
        assert_eq!(ok("0 ? 42 : 99"), b"99");
    }

    #[test]
    fn boolean_prefixes_share_the_canonical_converter() {
        for source in [
            "true", "tru", "t", "yes", "ye", "y", "false", "f", "no", "n", "off", "of",
        ] {
            assert_eq!(ok(source), source.as_bytes(), "{source}");
        }
        assert_eq!(ok("tru ? yes : no"), b"yes");
        assert_eq!(ok("!of"), b"1");
        assert!(ev("o", &[]).is_err(), "on/off share the prefix o");
    }

    #[test]
    fn variables_and_membership() {
        assert_eq!(ev("$x + $y", &[("x", 10), ("y", 32)]).unwrap(), b"42");
        assert_eq!(ev("$x * 2", &[("x", 21)]).unwrap(), b"42");
        assert_eq!(ok("3 in {1 2 3 4}"), b"1");
        assert_eq!(ok("9 ni {1 2 3 4}"), b"1");
    }

    #[test]
    fn math_functions() {
        // the shared tcl_syntax::expr::mathfunc dispatch, over the tower
        assert_eq!(ok("sqrt(4)"), b"2.0");
        assert_eq!(ok("max(1, 9, 3)"), b"9");
        assert_eq!(ok("min(5, 2)"), b"2");
        assert_eq!(ok("abs(-7)"), b"7");
        assert_eq!(ok("int(3.9)"), b"3");
        assert_eq!(ok("pow(2, 10)"), b"1024.0");
        // unknown function / domain error surface as errors
        assert!(ev("frobnicate(1)", &[]).is_err());
        assert!(ev("sqrt(-1)", &[]).is_err());
    }

    #[test]
    fn errors() {
        assert_eq!(
            ev("1 / 0", &[]),
            Err(ExprError::with_code(
                b"divide by zero",
                b"ARITH DIVZERO {divide by zero}"
            ))
        );
        assert!(ev("$missing + 1", &[]).is_err());
    }
}

#[cfg(test)]
#[path = "expr_float_tests.rs"]
mod float_error_tests;

#[cfg(test)]
#[path = "expr_reference_tests.rs"]
mod reference_root_tests;
