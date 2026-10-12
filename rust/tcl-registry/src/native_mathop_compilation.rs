// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original math-operator compiler stack and operand visits.
use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationWordShape,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// Independently registered C mathematical operator compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeMathOperator {
    /// Bitwise complement (`~`).
    Invert,
    /// Logical negation (`!`).
    Not,
    /// Numeric addition (`+`).
    Add,
    /// Numeric multiplication (`*`).
    Multiply,
    /// Bitwise conjunction (`&`).
    BitAnd,
    /// Bitwise disjunction (`|`).
    BitOr,
    /// Bitwise exclusive disjunction (`^`).
    BitXor,
    /// Exponentiation (`**`).
    Power,
    /// Left integer shift (`<<`).
    LeftShift,
    /// Right integer shift (`>>`).
    RightShift,
    /// Integer remainder (`%`).
    Remainder,
    /// Numeric inequality (`!=`).
    NotEqual,
    /// String inequality (`ne`).
    StringNotEqual,
    /// List membership (`in`).
    In,
    /// List non-membership (`ni`).
    NotIn,
    /// Numeric subtraction (`-`).
    Subtract,
    /// Numeric division (`/`).
    Divide,
    /// Numeric less-than comparison (`<`).
    Less,
    /// Numeric less-than-or-equal comparison (`<=`).
    LessEqual,
    /// Numeric greater-than comparison (`>`).
    Greater,
    /// Numeric greater-than-or-equal comparison (`>=`).
    GreaterEqual,
    /// Numeric equality (`==`).
    Equal,
    /// String equality (`eq`).
    StringEqual,
    /// String less-than comparison (`lt`).
    StringLess,
    /// String less-than-or-equal comparison (`le`).
    StringLessEqual,
    /// String greater-than comparison (`gt`).
    StringGreater,
    /// String greater-than-or-equal comparison (`ge`).
    StringGreaterEqual,
}
impl NativeMathOperator {
    /// Authored operators present in the C mathOpCmds registration table.
    #[must_use]
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        Some(match spelling {
            "~" => Self::Invert,
            "!" => Self::Not,
            "+" => Self::Add,
            "*" => Self::Multiply,
            "&" => Self::BitAnd,
            "|" => Self::BitOr,
            "^" => Self::BitXor,
            "**" => Self::Power,
            "<<" => Self::LeftShift,
            ">>" => Self::RightShift,
            "%" => Self::Remainder,
            "!=" => Self::NotEqual,
            "ne" => Self::StringNotEqual,
            "in" => Self::In,
            "ni" => Self::NotIn,
            "-" => Self::Subtract,
            "/" => Self::Divide,
            "<" => Self::Less,
            "<=" => Self::LessEqual,
            ">" => Self::Greater,
            ">=" => Self::GreaterEqual,
            "==" => Self::Equal,
            "eq" => Self::StringEqual,
            "lt" => Self::StringLess,
            "le" => Self::StringLessEqual,
            "gt" => Self::StringGreater,
            "ge" => Self::StringGreaterEqual,
            _ => return None,
        })
    }
    /// Primitive operation spelling, independent of the invoked command name.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Invert => "~",
            Self::Not => "!",
            Self::Add => "+",
            Self::Multiply => "*",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Power => "**",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::Remainder => "%",
            Self::NotEqual => "!=",
            Self::StringNotEqual => "ne",
            Self::In => "in",
            Self::NotIn => "ni",
            Self::Subtract => "-",
            Self::Divide => "/",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Equal => "==",
            Self::StringEqual => "eq",
            Self::StringLess => "lt",
            Self::StringLessEqual => "le",
            Self::StringGreater => "gt",
            Self::StringGreaterEqual => "ge",
        }
    }
    /// First native registration carrying this compiler hook.
    #[must_use]
    pub const fn first_version(self) -> TclVersion {
        match self {
            Self::StringLess
            | Self::StringLessEqual
            | Self::StringGreater
            | Self::StringGreaterEqual => TclVersion::V9_0,
            _ => TclVersion::V8_5,
        }
    }
    const fn comparison(self) -> bool {
        matches!(
            self,
            Self::Less
                | Self::LessEqual
                | Self::Greater
                | Self::GreaterEqual
                | Self::Equal
                | Self::StringEqual
                | Self::StringLess
                | Self::StringLessEqual
                | Self::StringGreater
                | Self::StringGreaterEqual
        )
    }
    /// Original token-count and procedure-context decision before operand visits.
    #[must_use]
    pub fn accepts(
        self,
        count: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<bool> {
        if version < self.first_version() {
            return Some(false);
        }
        if self.comparison() && count > 2 {
            return match context.frame {
                NativeCompilationFrame::ProcedureCode => Some(true),
                NativeCompilationFrame::ScriptCode => Some(false),
                NativeCompilationFrame::Unknown => None,
            };
        }
        Some(match self {
            Self::Invert | Self::Not => count == 1,
            Self::LeftShift
            | Self::RightShift
            | Self::Remainder
            | Self::NotEqual
            | Self::StringNotEqual
            | Self::In
            | Self::NotIn => count == 2,
            Self::Subtract | Self::Divide => count > 0,
            _ => true,
        })
    }
}
/// One actual original compiler stack operation, in emission order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMathopStep {
    /// Evaluate the retained original source operand.
    Word(NativeCompilerWordOperand),
    /// Push literal bytes selected by the original compiler.
    Literal(Vec<u8>),
    /// Reverse the selected number of values on the operand stack.
    Reverse(usize),
    /// Execute the selected primitive operator.
    Primitive(NativeMathOperator),
    /// Negate the current numeric value.
    Negate,
    /// Declare the original compiler's anonymous temporary cell.
    DeclareTemporary,
    /// Store the stack value in the retained temporary cell.
    StoreTemporary,
    /// Read the retained temporary cell.
    LoadTemporary,
    /// Retire the retained temporary cell.
    UnsetTemporary,
    /// Discard the current stack value.
    Pop,
}
/// A genuine stack recipe; unvisited source operands are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMathopInstruction {
    /// Stack operations in original compiler emission order.
    pub steps: Vec<NativeMathopStep>,
}
/// Original parser geometry or compiler context is not available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeMathopUnavailable {
    /// Original parser geometry is unavailable.
    Geometry,
    /// The required original compiler context is unavailable.
    Context,
}
/// Compile original words after an independently authenticated C registration.
/// # Errors
/// Returns unavailable instead of inventing parser geometry or a procedure frame.
pub fn compile_native_mathop(
    words: &NativeCompilerWords<'_>,
    from: usize,
    operator: NativeMathOperator,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<Option<NativeMathopInstruction>, NativeMathopUnavailable> {
    let projected = project_native_compiler_words(words, version)
        .map_err(|_| NativeMathopUnavailable::Geometry)?;
    let operands = projected
        .get(from..)
        .filter(|_| from > 0)
        .ok_or(NativeMathopUnavailable::Geometry)?;
    if operands
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Ok(None);
    }
    if operands
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Opaque)
    {
        return Err(NativeMathopUnavailable::Geometry);
    }
    let count = operands.len();
    match operator.accepts(count, version, context) {
        Some(true) => {}
        Some(false) => return Ok(None),
        None => return Err(NativeMathopUnavailable::Context),
    }
    let steps = if operator.comparison() {
        comparison_steps(operands, operator, version)
    } else {
        arithmetic_steps(operands, operator)
    };
    Ok(Some(NativeMathopInstruction { steps }))
}

fn comparison_steps(
    operands: &[crate::native_compiler_word_projection::NativeProjectedCompilerWord],
    operator: NativeMathOperator,
    version: TclVersion,
) -> Vec<NativeMathopStep> {
    use NativeMathopStep as S;
    let count = operands.len();
    let mut steps = Vec::new();
    let literal = |value: &str| S::Literal(value.as_bytes().to_vec());
    match count {
        0 | 1 => steps.push(literal("1")),
        2 => {
            steps.extend(operands.iter().map(|word| S::Word(word.operand.clone())));
            steps.push(S::Primitive(operator));
        }
        _ => {
            steps.push(S::DeclareTemporary);
            steps.push(S::Word(operands[0].operand.clone()));
            steps.push(S::Word(operands[1].operand.clone()));
            steps.push(S::StoreTemporary);
            steps.push(S::Primitive(operator));
            for (index, word) in operands.iter().enumerate().skip(2) {
                steps.push(S::LoadTemporary);
                steps.push(S::Word(word.operand.clone()));
                if index + 1 < count {
                    steps.push(S::StoreTemporary);
                }
                steps.push(S::Primitive(operator));
            }
            for _ in 2..count {
                steps.push(S::Primitive(NativeMathOperator::BitAnd));
            }
            if version == TclVersion::V8_5 {
                steps.push(literal(""));
                steps.push(S::StoreTemporary);
                steps.push(S::Pop);
            } else {
                steps.push(S::UnsetTemporary);
            }
        }
    }
    steps
}

fn arithmetic_steps(
    operands: &[crate::native_compiler_word_projection::NativeProjectedCompilerWord],
    operator: NativeMathOperator,
) -> Vec<NativeMathopStep> {
    use NativeMathopStep as S;
    let count = operands.len();
    let mut steps = Vec::new();
    let literal = |value: &str| S::Literal(value.as_bytes().to_vec());
    if operator == NativeMathOperator::Divide && count == 1 {
        steps.push(literal("1.0"));
    }
    steps.extend(operands.iter().map(|word| S::Word(word.operand.clone())));
    match operator {
        NativeMathOperator::Invert | NativeMathOperator::Not => steps.push(S::Primitive(operator)),
        NativeMathOperator::Subtract if count == 1 => steps.push(S::Negate),
        NativeMathOperator::Subtract | NativeMathOperator::Divide => {
            if count <= 2 {
                steps.push(S::Primitive(operator));
            } else {
                steps.push(S::Reverse(count));
                for _ in 1..count {
                    steps.push(S::Reverse(2));
                    steps.push(S::Primitive(operator));
                }
            }
        }
        _ => {
            let identity = match operator {
                NativeMathOperator::Add
                | NativeMathOperator::BitOr
                | NativeMathOperator::BitXor => Some("0"),
                NativeMathOperator::Multiply | NativeMathOperator::Power => Some("1"),
                NativeMathOperator::BitAnd => Some("-1"),
                _ => None,
            };
            let mut stack_count = count;
            if count <= 1
                && let Some(identity) = identity
            {
                steps.push(literal(identity));
                stack_count += 1;
            }
            if stack_count > 2 && operator != NativeMathOperator::Power {
                steps.push(S::Reverse(stack_count));
            }
            for _ in 1..stack_count {
                steps.push(S::Primitive(operator));
            }
        }
    }
    steps
}

#[cfg(test)]
mod tests;
