// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native compiler execution of a retained constant subtree.
use super::{
    BinOp, Env, ExprNode, FoldNativeInputs, FoldOps, FoldPolicy, FoldValue, NativeCoercionKind,
    TclValue, UnaryOp, make_fold_ops,
};
use tcl_registry::native_compilation::NativeCompilationFailure;
use tcl_syntax::expr::errors::{self, NativeArithmeticFailure, OperandDesc, OperandSide};

pub(crate) enum NativeConstantResult {
    Number(TclValue),
    Resident {
        bytes: Vec<u8>,
        number: Option<TclValue>,
    },
    Failure(NativeCompilationFailure),
    Unavailable,
}

pub(crate) fn eval_native_constant(node: &ExprNode, policy: FoldPolicy) -> NativeConstantResult {
    if policy
        .invocation_dialect
        .and_then(|dialect| dialect.tcl_version)
        .is_none_or(|version| version < tcl_dialect::TclVersion::V8_5)
    {
        return NativeConstantResult::Unavailable;
    }
    let env = Env::new();
    let mut ops = make_fold_ops(
        &env,
        policy,
        None,
        None,
        FoldNativeInputs::objects(None),
        false,
    );
    ops.constant_compilation.enabled = true;
    match tcl_syntax::expr::eval(node, &mut ops) {
        Ok(_) if ops.ambiguous => NativeConstantResult::Unavailable,
        Ok(FoldValue::Str(bytes)) => {
            let number = ops.strict_number(&FoldValue::Str(bytes.clone()));
            NativeConstantResult::Resident {
                bytes: bytes.into_bytes(),
                number,
            }
        }
        Ok(value) => ops
            .number_for(&value, NativeCoercionKind::ResultNormalization)
            .map_or(
                NativeConstantResult::Unavailable,
                NativeConstantResult::Number,
            ),
        Err(()) => ops.constant_compilation.failure.map_or(
            NativeConstantResult::Unavailable,
            NativeConstantResult::Failure,
        ),
    }
}

impl FoldOps<'_> {
    fn constant_failure(&mut self, message: String, code: Option<String>) -> Result<(), ()> {
        self.constant_compilation.failure = Some(NativeCompilationFailure {
            message: Some(message),
            error_code: code,
            error_info: None,
        });
        Err(())
    }

    fn arithmetic_failure(&mut self, failure: NativeArithmeticFailure) -> Result<(), ()> {
        let (message, code) = failure.diagnostic();
        self.constant_failure(message.to_owned(), code.map(str::to_owned))
    }

    fn constant_operand_failure(
        &mut self,
        value: &FoldValue,
        desc: OperandDesc,
        side: OperandSide,
        operator: &str,
    ) -> Result<(), ()> {
        let Some(dialect) = self.invocation_dialect else {
            return Err(());
        };
        let Some(version) = dialect.tcl_version else {
            return Err(());
        };
        let format = dialect
            .double_string_policy()
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format);
        let Some(bytes) = value.to_string_val(format) else {
            return Err(());
        };
        let desc = if desc == OperandDesc::NonNumericString
            && version >= tcl_dialect::TclVersion::V9_0
            && tcl_syntax::list::split_list(&bytes).is_ok_and(|items| items.len() > 1)
        {
            OperandDesc::List
        } else {
            desc
        };
        self.constant_failure(
            errors::illegal_operand_message(desc, &bytes, side, operator, version),
            Some(errors::illegal_operand_error_code(desc, version)),
        )
    }

    pub(super) fn check_native_constant_arithmetic(
        &mut self,
        op: BinOp,
        left: &FoldValue,
        right: &FoldValue,
    ) -> Result<(), ()> {
        if !self.constant_compilation.enabled {
            return Ok(());
        }
        let integer = matches!(
            op,
            BinOp::Mod
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
        );
        let mut numbers = Vec::with_capacity(2);
        for (value, side) in [(left, OperandSide::Left), (right, OperandSide::Right)] {
            let Some(number) = self.strict_number(value) else {
                return self.constant_operand_failure(
                    value,
                    OperandDesc::NonNumericString,
                    side,
                    op.as_str(),
                );
            };
            if matches!(number, TclValue::Float(value) if value.is_nan()) {
                return self.constant_operand_failure(
                    value,
                    OperandDesc::NonNumericFloatingPointValue,
                    side,
                    op.as_str(),
                );
            }
            if integer && matches!(number, TclValue::Float(_)) {
                return self.constant_operand_failure(
                    value,
                    OperandDesc::FloatingPointValue,
                    side,
                    op.as_str(),
                );
            }
            numbers.push(number);
        }
        let (left, right) = (&numbers[0], &numbers[1]);
        let integer_zero = matches!(right, TclValue::Int(0))
            || matches!(right, TclValue::Big(value) if value == &num_bigint::BigInt::from(0));
        if matches!(op, BinOp::Div | BinOp::Mod)
            && integer_zero
            && !matches!(left, TclValue::Float(_))
        {
            return self.arithmetic_failure(NativeArithmeticFailure::DivideByZero);
        }
        if op == BinOp::Pow && left.as_f64() == 0.0 && right.as_f64() < 0.0 {
            return self.arithmetic_failure(NativeArithmeticFailure::ZeroToNegativePower);
        }
        if matches!(op, BinOp::LShift | BinOp::RShift) && right.as_f64() < 0.0 {
            return self.arithmetic_failure(NativeArithmeticFailure::NegativeShift);
        }
        if matches!(left, TclValue::Float(_)) || matches!(right, TclValue::Float(_)) {
            let (left, right) = (left.as_f64(), right.as_f64());
            let result = match op {
                BinOp::Add => left + right,
                BinOp::Sub => left - right,
                BinOp::Mul => left * right,
                BinOp::Div => left / right,
                BinOp::Pow => left.powf(right),
                _ => return Ok(()),
            };
            if result.is_nan() {
                return self.arithmetic_failure(NativeArithmeticFailure::NanResult);
            }
        }
        Ok(())
    }

    pub(super) fn check_native_constant_unary(
        &mut self,
        op: UnaryOp,
        value: &FoldValue,
    ) -> Result<(), ()> {
        if !self.constant_compilation.enabled {
            return Ok(());
        }
        if matches!(op, UnaryOp::Not | UnaryOp::WordNot) {
            return self.check_native_constant_boolean(value);
        }
        let Some(number) = self.strict_number(value) else {
            return self.constant_operand_failure(
                value,
                OperandDesc::NonNumericString,
                OperandSide::Unary,
                op.as_str(),
            );
        };
        if matches!(number, TclValue::Float(value) if value.is_nan()) {
            return self.constant_operand_failure(
                value,
                OperandDesc::NonNumericFloatingPointValue,
                OperandSide::Unary,
                op.as_str(),
            );
        }
        if op == UnaryOp::BitNot && matches!(number, TclValue::Float(_)) {
            return self.constant_operand_failure(
                value,
                OperandDesc::FloatingPointValue,
                OperandSide::Unary,
                op.as_str(),
            );
        }
        Ok(())
    }

    pub(super) fn check_native_constant_boolean(&mut self, value: &FoldValue) -> Result<(), ()> {
        if !self.constant_compilation.enabled {
            return Ok(());
        }
        match self.number_for(value, NativeCoercionKind::Boolean) {
            Some(TclValue::Float(value)) if value.is_nan() => self.constant_failure(
                errors::NAN_MESSAGE.to_owned(),
                Some(errors::NAN_CODE.to_owned()),
            ),
            Some(_) => Ok(()),
            None => {
                let Some(bytes) = value.to_string_val(None) else {
                    return Err(());
                };
                self.constant_failure(
                    tcl_syntax::value::ValueError::NotBooleanBytes(bytes.into_bytes()).to_string(),
                    Some(errors::BOOLEAN_OPERAND_CODE.to_owned()),
                )
            }
        }
    }
}
