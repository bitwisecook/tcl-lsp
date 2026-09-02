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

//! The native lowered IR (NLIR): a small typed vocabulary over SSA values
//! with explicit framing operations (plan §3.3).
//!
//! An NLIR function mirrors the executable semantic function block for block:
//! every executable block becomes one [`NativeBlock`] with the same index and
//! the same terminator shape, and every executable instruction becomes one
//! [`NativeStatement`] owning that instruction's completion. Inside a
//! statement the operations are straight-line; an operation that can fail
//! sets the statement's completion to a Tcl error and abandons the rest of
//! the statement, exactly as the runtime abandons a command at its first
//! failing substitution. The statement's completion then flows into the
//! block terminator's dispatch unchanged, so the completion spine of the
//! executable IR survives lowering intact.
//!
//! Nothing here names a Tcl command or a source span: a statement the
//! lowering declined carries the exact text it hands to the runtime as
//! [`NativeOp::EvalSource`] together with the typed reason, which is the one
//! and only rung that still evaluates source text.

use tcl_core_types::Code as CompletionCode;
use tcl_syntax::expr::{BinOp, UnaryOp};

use super::cells::CellPlace;
use super::elide::{BarrierDecision, IncrGuard};
use super::representation::Representation;
use crate::executable_ir::CompletionId;
use crate::ir::NodeId;

/// Identity of one NLIR value inside its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeValueId(pub u32);

/// Identity of one NLIR block; equal to the executable block index it mirrors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeBlockId(pub u32);

impl NativeBlockId {
    /// The block's position in [`NativeFunction::blocks`].
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// The machine type an NLIR value has after representation inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeType {
    /// A native signed 64-bit integer.
    I64,
    /// A native IEEE double.
    F64,
    /// A native truth value.
    Bool,
    /// An owned boxed Tcl object handle.
    Obj,
}

impl NativeType {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Bool => "bool",
            Self::Obj => "obj",
        }
    }
}

/// One NLIR SSA value.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeValue {
    /// The machine type the emitter allocates for the value.
    pub ty: NativeType,
    /// The representation-lattice element inferred for the value.
    pub rep: Representation,
}

/// A whole lowered function.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeFunction {
    /// Values indexed by [`NativeValueId`].
    pub values: Vec<NativeValue>,
    /// Blocks in executable block order.
    pub blocks: Vec<NativeBlock>,
    /// The entry block.
    pub entry: NativeBlockId,
    /// Number of completion identities the executable function allocated; the
    /// emitter reserves completion storage for every index below it.
    pub completion_count: usize,
    /// The largest argv the function passes to a generic invocation.
    pub max_argc: usize,
    /// Whether the function is a procedure body that must push a name
    /// addressable Tcl frame before its first cell access.
    pub pushes_frame: bool,
}

impl NativeFunction {
    /// The value record for `id`.
    #[must_use]
    pub fn value(&self, id: NativeValueId) -> &NativeValue {
        &self.values[id.0 as usize]
    }
}

/// One NLIR block.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeBlock {
    /// Block identity (its index).
    pub id: NativeBlockId,
    /// Statements in execution order.
    pub statements: Vec<NativeStatement>,
    /// The block's terminator.
    pub terminator: NativeTerminator,
}

/// One executable instruction after lowering: straight-line operations that
/// together produce one completion.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeStatement {
    /// The completion this statement produces.
    pub completion: CompletionId,
    /// The source-semantic node the statement belongs to, when it has one.
    pub node: Option<NodeId>,
    /// Straight-line operations.
    pub ops: Vec<NativeOp>,
}

/// The block terminator, mirroring the executable terminator vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeTerminator {
    /// Continue unconditionally.
    Goto(NativeBlockId),
    /// Branch on a native truth value.
    Branch {
        /// A [`NativeType::Bool`] value.
        condition: NativeValueId,
        /// Target when true.
        then_target: NativeBlockId,
        /// Target when false.
        else_target: NativeBlockId,
    },
    /// Dispatch a completion by its code.
    CompletionSwitch {
        /// The completion whose code selects the successor.
        completion: CompletionId,
        /// Explicit `(code, target)` arms.
        cases: Vec<(i32, NativeBlockId)>,
        /// Successor for every other code.
        default: NativeBlockId,
    },
    /// Leave the function with a completion.
    Return(CompletionId),
}

/// Native integer operators with a proven-in-range result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// Tcl `/`: floor division (the divisor is proven non-zero).
    Div,
    /// Tcl `%`: the remainder takes the divisor's sign (divisor non-zero).
    Mod,
    /// `&`
    And,
    /// `|`
    Or,
    /// `^`
    Xor,
    /// `<<` with a proven in-range shift count.
    Shl,
    /// `>>` with a proven in-range shift count.
    Shr,
}

/// Native double operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoubleOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/` (the divisor is dynamically checked; zero takes the slow edge).
    Div,
}

/// Native comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

/// Which native type a comparison compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompareKind {
    /// Two `i64` operands.
    I64,
    /// Two `f64` operands.
    F64,
}

/// Which native fast path a dynamic (boxed-operand) operation tries before
/// its runtime slow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericHint {
    /// Try the `i64` fast path first.
    Int,
    /// Try the `f64` fast path first.
    Double,
    /// No native fast path: always the runtime operation.
    None,
}

impl NumericHint {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Double => "double",
            Self::None => "none",
        }
    }
}

/// A branch inside a statement whose arms produce one merged value.
#[derive(Debug, Clone, PartialEq)]
pub struct IfElseResult {
    /// The merged value.
    pub dst: NativeValueId,
    /// The value the `then` arm leaves.
    pub then_src: NativeValueId,
    /// The value the `else` arm leaves.
    pub else_src: NativeValueId,
}

/// One NLIR operation.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeOp {
    /// `dst = i64 constant`.
    ConstInt {
        /// Destination.
        dst: NativeValueId,
        /// The constant.
        value: i64,
    },
    /// `dst = f64 constant`.
    ConstDouble {
        /// Destination.
        dst: NativeValueId,
        /// The constant.
        value: f64,
    },
    /// `dst = bool constant`.
    ConstBool {
        /// Destination.
        dst: NativeValueId,
        /// The constant.
        value: bool,
    },
    /// `dst = boxed string constant` from the module's constant pool.
    ConstStr {
        /// Destination.
        dst: NativeValueId,
        /// The exact Tcl value.
        text: String,
    },
    /// Box a native value into an owned Tcl object.
    Box {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// A native source.
        src: NativeValueId,
    },
    /// Convert a boxed value to a native one through the erroring runtime
    /// getter: failure is the ordinary Tcl error completion with C Tcl's
    /// message.
    Unbox {
        /// Destination (the `target` type).
        dst: NativeValueId,
        /// A boxed source.
        src: NativeValueId,
        /// The native type to read.
        target: NativeType,
    },
    /// The truth of a native numeric or boolean value.
    Truth {
        /// Destination (`Bool`).
        dst: NativeValueId,
        /// Native source.
        src: NativeValueId,
    },
    /// Widen an `i64` to an `f64`.
    IntToDouble {
        /// Destination (`F64`).
        dst: NativeValueId,
        /// `I64` source.
        src: NativeValueId,
    },
    /// Read a `Bool` as the integer `0`/`1`.
    BoolToInt {
        /// Destination (`I64`).
        dst: NativeValueId,
        /// `Bool` source.
        src: NativeValueId,
    },
    /// A native integer operation whose result is proven to fit `i64`.
    IntBinary {
        /// Destination (`I64`).
        dst: NativeValueId,
        /// The operator.
        op: IntOp,
        /// Left operand (`I64`).
        lhs: NativeValueId,
        /// Right operand (`I64`).
        rhs: NativeValueId,
    },
    /// Native integer negation with a proven in-range operand.
    IntNeg {
        /// Destination (`I64`).
        dst: NativeValueId,
        /// `I64` source.
        src: NativeValueId,
    },
    /// Native bitwise complement.
    IntBitNot {
        /// Destination (`I64`).
        dst: NativeValueId,
        /// `I64` source.
        src: NativeValueId,
    },
    /// A native double operation on finite operands.
    DoubleBinary {
        /// Destination (`F64`).
        dst: NativeValueId,
        /// The operator.
        op: DoubleOp,
        /// Left operand (`F64`).
        lhs: NativeValueId,
        /// Right operand (`F64`).
        rhs: NativeValueId,
    },
    /// Native double negation.
    DoubleNeg {
        /// Destination (`F64`).
        dst: NativeValueId,
        /// `F64` source.
        src: NativeValueId,
    },
    /// A native comparison producing a truth value.
    Compare {
        /// Destination (`Bool`).
        dst: NativeValueId,
        /// The comparison.
        op: CmpOp,
        /// The operand type.
        kind: CompareKind,
        /// Left operand.
        lhs: NativeValueId,
        /// Right operand.
        rhs: NativeValueId,
    },
    /// Logical negation of a truth value.
    NotBool {
        /// Destination (`Bool`).
        dst: NativeValueId,
        /// `Bool` source.
        src: NativeValueId,
    },
    /// A binary operator over operands of any representation: the emitter
    /// tries the hinted native fast path (with its overflow, division, and
    /// domain checks) and takes the runtime operator on the slow edge, so the
    /// result is always a boxed value carrying Tcl's exact numeric semantics.
    DynamicBinary {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// The Tcl operator.
        op: BinOp,
        /// Left operand (any type).
        lhs: NativeValueId,
        /// Right operand (any type).
        rhs: NativeValueId,
        /// Which native fast path to try.
        hint: NumericHint,
    },
    /// A comparison over operands of any representation, producing a truth
    /// value: the hinted native fast path or the runtime operator.
    DynamicCompare {
        /// Destination (`Bool`).
        dst: NativeValueId,
        /// The Tcl comparison operator.
        op: BinOp,
        /// Left operand (any type).
        lhs: NativeValueId,
        /// Right operand (any type).
        rhs: NativeValueId,
        /// Which native fast path to try.
        hint: NumericHint,
    },
    /// A unary operator over an operand of any representation.
    DynamicUnary {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// The Tcl operator.
        op: UnaryOp,
        /// Operand (any type).
        src: NativeValueId,
    },
    /// The runtime's `::tcl::mathop` implementation of an operator over
    /// boxed operands: the slow edge, and the only path for operators with
    /// no native shape (`**`, `eq`, `in`, string ordering).
    MathOp {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// The operator's exact spelling.
        op: &'static str,
        /// Boxed operands.
        args: Vec<NativeValueId>,
    },
    /// A math function through the runtime's `::tcl::mathfunc` dispatch.
    MathFunc {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// The function name as written.
        name: String,
        /// Boxed arguments.
        args: Vec<NativeValueId>,
    },
    /// The runtime expression intrinsic over a boxed expression object.
    ExprEval {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// The expression text handed to the runtime as one boxed object.
        text: String,
    },
    /// A branch inside a statement.
    IfElse {
        /// A `Bool` condition.
        condition: NativeValueId,
        /// Operations when true.
        then_ops: Vec<NativeOp>,
        /// Operations when false.
        else_ops: Vec<NativeOp>,
        /// The merged value, when the arms produce one.
        result: Option<IfElseResult>,
    },
    /// Read a Tcl variable cell into an owned boxed value.
    CellRead {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// The cell.
        place: CellPlace,
        /// The trace-barrier decision recorded for the read.
        barrier: BarrierDecision,
    },
    /// Write an owned boxed value into a Tcl variable cell.
    CellWrite {
        /// The cell.
        place: CellPlace,
        /// The value (`Obj`).
        src: NativeValueId,
        /// The trace-barrier decision recorded for the write.
        barrier: BarrierDecision,
    },
    /// Tcl `incr` on a cell by a native `i64` delta.
    CellIncr {
        /// Destination: the cell's new value (`Obj`).
        dst: NativeValueId,
        /// The cell.
        place: CellPlace,
        /// The delta (`I64`).
        delta: NativeValueId,
        /// How the native fast path is guarded.
        guard: IncrGuard,
        /// The trace-barrier decision recorded for the update.
        barrier: BarrierDecision,
    },
    /// Tcl `append` (`list` = false) or `lappend` (`list` = true) of boxed
    /// values onto a cell.
    CellAppend {
        /// The cell.
        place: CellPlace,
        /// The values to append (`Obj`).
        values: Vec<NativeValueId>,
        /// Whether this is `lappend`.
        list: bool,
        /// The trace-barrier decision recorded for the update.
        barrier: BarrierDecision,
    },
    /// Concatenate boxed word parts into one boxed word.
    Concat {
        /// Destination (`Obj`).
        dst: NativeValueId,
        /// Parts (`Obj`), in order.
        parts: Vec<NativeValueId>,
    },
    /// The channel-write intrinsic over one boxed value (`puts VALUE`).
    Puts {
        /// The value (`Obj`).
        src: NativeValueId,
    },
    /// A `[…]` command substitution: a generic runtime invocation whose
    /// result becomes the word's value. An abrupt completion becomes the
    /// enclosing statement's completion and abandons the statement.
    NestedInvoke {
        /// Destination (`Obj`): the invocation's result.
        dst: NativeValueId,
        /// Argv words (`Obj`), head first.
        argv: Vec<NativeValueId>,
    },
    /// A generic runtime invocation over a prebuilt argv; produces the
    /// statement's whole completion triple.
    Invoke {
        /// Argv words (`Obj`), head first.
        argv: Vec<NativeValueId>,
    },
    /// Complete the statement with a fixed code and an optional result.
    Complete {
        /// The completion code.
        code: CompletionCode,
        /// The completion result (`Obj`), or the empty string.
        result: Option<NativeValueId>,
    },
    /// The last rung: evaluate the statement's exact source text through the
    /// runtime, because the lowering declined it for the recorded reason.
    EvalSource {
        /// The exact command text.
        text: String,
        /// Why the lowering declined.
        reason: super::NativeLoweringDecline,
    },
}
