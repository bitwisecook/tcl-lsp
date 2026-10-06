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

//! Shared fixed-width integer semantics for Tcl 8.4 and Jim.
//!
//! Adapters select the native policy, parse operands with its numeral grammar,
//! and preserve their own value/error representation. The C Tcl bignum tower
//! remains owned by [`crate::number_tower`].

use super::{BinOp, UnaryOp};
use crate::number_tower::BigIntOps;
use tcl_dialect::NativeArithmetic;

/// Why a fixed-width operation cannot yield an integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WideError {
    /// The operation belongs to a different numeric tower or operand type.
    Unsupported,
    /// The literal exceeds the native unsigned-wide conversion range.
    LiteralOverflow,
    /// Division or remainder by zero.
    DivisionByZero,
    /// Zero raised to a negative power.
    ZeroToNegativePower,
    /// The C implementation has no portable, defined result for this input.
    UndefinedNativeOperation,
}

/// Convert a parsed integer literal to the selected native wide representation.
/// Jim's `strtoull` conversion saturates magnitudes beyond unsigned-wide range;
/// Tcl 8.4 reports overflow. Both retain the low bits for in-range magnitudes.
pub fn literal<B: BigIntOps>(policy: NativeArithmetic, value: &B) -> Result<i64, WideError> {
    match policy {
        NativeArithmetic::TclBignum => Err(WideError::Unsupported),
        NativeArithmetic::Tcl84Wide if value.bit_len() > 64 => Err(WideError::LiteralOverflow),
        NativeArithmetic::JimWide if value.bit_len() > 64 => {
            Ok(if value.is_negative() { 1 } else { -1 })
        }
        _ => Ok(value.to_i64_wrapping()),
    }
}

/// Convert an integer already parsed by the selected numeral grammar.
/// This also serves runtimes without an arbitrary-precision backend: cleaned
/// magnitude digits are converted once, with the native overflow policy.
pub fn parsed_literal(
    policy: NativeArithmetic,
    value: &crate::number::Number,
) -> Result<i64, WideError> {
    use crate::number::Number;
    if policy == NativeArithmetic::TclBignum {
        return Err(WideError::Unsupported);
    }
    match value {
        Number::Int(value) => Ok(*value),
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let magnitude = u64::from_str_radix(digits, *radix as u32);
            let magnitude = match magnitude {
                Ok(value) => value,
                Err(_) if policy == NativeArithmetic::JimWide => u64::MAX,
                Err(_) => return Err(WideError::LiteralOverflow),
            };
            let signed = i64::from_ne_bytes(magnitude.to_ne_bytes());
            Ok(if *negative {
                signed.wrapping_neg()
            } else {
                signed
            })
        }
        _ => Err(WideError::Unsupported),
    }
}

/// Compute one integer operation under a selected fixed-width policy.
/// Native undefined operations are represented explicitly, so analysis never
/// proves a value by reproducing an accidental machine instruction result.
pub fn binary(policy: NativeArithmetic, op: BinOp, x: i64, y: i64) -> Result<i64, WideError> {
    if policy == NativeArithmetic::TclBignum {
        return Err(WideError::Unsupported);
    }
    match op {
        BinOp::Add => Ok(x.wrapping_add(y)),
        BinOp::Sub => Ok(x.wrapping_sub(y)),
        BinOp::Mul => Ok(x.wrapping_mul(y)),
        BinOp::Div | BinOp::Mod => divide(op, x, y),
        BinOp::Pow if policy == NativeArithmetic::JimWide => power(x, y),
        BinOp::LShift | BinOp::RShift => shift(policy, op, x, y),
        BinOp::BitAnd => Ok(x & y),
        BinOp::BitOr => Ok(x | y),
        BinOp::BitXor => Ok(x ^ y),
        _ => Err(WideError::Unsupported),
    }
}

fn divide(op: BinOp, x: i64, y: i64) -> Result<i64, WideError> {
    if y == 0 {
        return Err(WideError::DivisionByZero);
    }
    if x == i64::MIN && y == -1 {
        return if op == BinOp::Div {
            Ok(i64::MIN)
        } else {
            // Both audited native engines trap in this remainder case.
            Err(WideError::UndefinedNativeOperation)
        };
    }
    let q = x / y;
    let r = x % y;
    let opposite_sign_remainder = r != 0 && (r < 0) != (y < 0);
    Ok(if op == BinOp::Div {
        if opposite_sign_remainder { q - 1 } else { q }
    } else if opposite_sign_remainder {
        r + y
    } else {
        r
    })
}

fn shift(policy: NativeArithmetic, op: BinOp, x: i64, y: i64) -> Result<i64, WideError> {
    if policy == NativeArithmetic::Tcl84Wide {
        if y < 0 {
            return Err(WideError::UndefinedNativeOperation);
        }
        if y >= 64 {
            return Ok(if op == BinOp::RShift && x < 0 { -1 } else { 0 });
        }
    }
    let count = u32::try_from(y & 63).expect("masked shift count fits u32");
    Ok(if op == BinOp::LShift {
        x.wrapping_shl(count)
    } else {
        x.wrapping_shr(count)
    })
}

fn power(mut base: i64, exponent: i64) -> Result<i64, WideError> {
    if exponent < 0 {
        return match base {
            0 => Err(WideError::ZeroToNegativePower),
            1 => Ok(1),
            -1 => Ok(if exponent & 1 == 0 { 1 } else { -1 }),
            _ => Ok(0),
        };
    }
    let mut exponent = exponent;
    let mut result = 1_i64;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = result.wrapping_mul(base);
        }
        exponent >>= 1;
        if exponent != 0 {
            base = base.wrapping_mul(base);
        }
    }
    Ok(result)
}

/// Compute an arithmetic unary operation on a native wide integer.
pub fn unary(policy: NativeArithmetic, op: UnaryOp, value: i64) -> Result<i64, WideError> {
    if policy == NativeArithmetic::TclBignum {
        return Err(WideError::Unsupported);
    }
    match op {
        UnaryOp::Pos => Ok(value),
        UnaryOp::Neg => Ok(value.wrapping_neg()),
        UnaryOp::BitNot => Ok(!value),
        _ => Err(WideError::Unsupported),
    }
}

/// Evaluate a literal integer expression through the shared expression walker.
/// No variable, script, string or function-table proof is inferred. Expressions
/// containing those operands decline even in a lazily skipped branch.
pub fn literal_expression(
    node: &super::ExprNode,
    numbers: crate::number::NumberSyntax,
    policy: NativeArithmetic,
) -> Result<i64, WideError> {
    if !integer_tree(node) {
        return Err(WideError::Unsupported);
    }
    super::eval::eval(node, &mut LiteralIntegerOps { numbers, policy })
}

fn integer_tree(node: &super::ExprNode) -> bool {
    use super::ExprNode;
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::Unary { operand, .. } => integer_tree(operand),
        ExprNode::Binary { op, left, right } => {
            matches!(
                op,
                BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::Mod
                    | BinOp::Pow
                    | BinOp::LShift
                    | BinOp::RShift
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor
                    | BinOp::And
                    | BinOp::Or
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
            ) && integer_tree(left)
                && integer_tree(right)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => integer_tree(condition) && integer_tree(true_branch) && integer_tree(false_branch),
        _ => false,
    }
}

struct LiteralIntegerOps {
    numbers: crate::number::NumberSyntax,
    policy: NativeArithmetic,
}

impl super::eval::ExprOps for LiteralIntegerOps {
    type Value = i64;
    type Error = WideError;

    fn literal(&mut self, text: &str) -> Result<i64, WideError> {
        let number = crate::number::parse_whole_with(
            text,
            crate::number::ParseFlags {
                integer_only: true,
                ..crate::number::ParseFlags::for_syntax(self.numbers)
            },
        )
        .ok_or(WideError::Unsupported)?;
        parsed_literal(self.policy, &number)
    }

    fn string(&mut self, _inner: &str, _substitutes: bool) -> Result<i64, WideError> {
        Err(WideError::Unsupported)
    }

    fn var(&mut self, _name: &str) -> Result<i64, WideError> {
        Err(WideError::Unsupported)
    }

    fn command(&mut self, _script: &str) -> Result<i64, WideError> {
        Err(WideError::Unsupported)
    }

    fn call(&mut self, _function: &str, _args: Vec<i64>) -> Result<i64, WideError> {
        Err(WideError::Unsupported)
    }

    fn arith(&mut self, op: BinOp, left: i64, right: i64) -> Result<i64, WideError> {
        binary(self.policy, op, left, right)
    }

    fn unary(&mut self, op: UnaryOp, value: i64) -> Result<i64, WideError> {
        unary(self.policy, op, value)
    }

    fn compare_numeric(&mut self, left: &i64, right: &i64) -> Option<super::NumericCompare> {
        Some(super::NumericCompare::Ordered(left.cmp(right)))
    }

    fn compare_string(
        &mut self,
        left: &i64,
        right: &i64,
    ) -> Result<core::cmp::Ordering, WideError> {
        Ok(left.to_string().cmp(&right.to_string()))
    }

    fn in_list(&mut self, _needle: &i64, _list: &i64) -> Result<bool, WideError> {
        Err(WideError::Unsupported)
    }

    fn to_bool(&mut self, value: &i64) -> Result<bool, WideError> {
        Ok(*value != 0)
    }

    fn bool_value(&mut self, value: bool) -> i64 {
        i64::from(value)
    }

    fn unsupported(&mut self, _what: &str) -> WideError {
        WideError::Unsupported
    }
}

#[cfg(test)]
mod parsed_tests {
    use super::*;
    use crate::number::{Number, Radix};

    #[test]
    fn parsed_unsigned_magnitudes_preserve_native_range_and_sign() {
        for (digits, negative, c, jim) in [
            ("18446744073709551615", false, Ok(-1), -1),
            ("18446744073709551615", true, Ok(1), 1),
            (
                "18446744073709551616",
                false,
                Err(WideError::LiteralOverflow),
                -1,
            ),
            (
                "18446744073709551616",
                true,
                Err(WideError::LiteralOverflow),
                1,
            ),
        ] {
            let value = Number::Big {
                negative,
                radix: Radix::Dec,
                digits: digits.into(),
            };
            assert_eq!(parsed_literal(NativeArithmetic::Tcl84Wide, &value), c);
            assert_eq!(parsed_literal(NativeArithmetic::JimWide, &value), Ok(jim));
        }
        let hexadecimal = Number::Big {
            negative: false,
            radix: Radix::Hex,
            digits: "ffffffffffffffff".into(),
        };
        assert_eq!(
            parsed_literal(NativeArithmetic::Tcl84Wide, &hexadecimal),
            Ok(-1)
        );
    }
}
