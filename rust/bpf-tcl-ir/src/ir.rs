//! The typed, backend-agnostic BPF-IR: a small three-address code over mutable
//! slots (C-like locals — no SSA / phi), produced from the Tcl front-end and
//! consumed by the eBPF (and later WASM) backends.

use tcl_lexer::Span;

use crate::ty::{Ty, Width};

/// A mutable storage slot (a typed local). Lowered to a fixed stack location by
/// the backend; slots are mutable like C locals, so control flow needs no phi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub u32);

/// A basic-block label in the BPF-IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// Integer binary operators (the verifier-friendly subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `&`
    And,
    /// `|`
    Or,
    /// `^`
    Xor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
}

/// Integer comparison operators (signed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// `-` (arithmetic negation)
    Neg,
    /// `!` (logical NOT: `x == 0 ? 1 : 0`)
    Not,
    /// `~` (bitwise complement)
    BitNot,
}

/// One typed instruction. Each defines (at most) one slot and carries a [`Span`].
#[derive(Debug, Clone)]
pub enum Inst {
    /// `dst = val`
    Const {
        /// Destination slot.
        dst: SlotId,
        /// The constant value.
        val: i64,
        /// Source span.
        span: Span,
    },
    /// `dst = src` (copy a temporary into a named variable's stable slot).
    Copy {
        /// Destination slot.
        dst: SlotId,
        /// Source slot.
        src: SlotId,
        /// Source span.
        span: Span,
    },
    /// `dst = a <op> b`
    Bin {
        /// Destination slot.
        dst: SlotId,
        /// The operator.
        op: IntBinOp,
        /// Left operand slot.
        a: SlotId,
        /// Right operand slot.
        b: SlotId,
        /// Source span.
        span: Span,
    },
    /// `dst = <op> a`
    Un {
        /// Destination slot.
        dst: SlotId,
        /// The operator.
        op: UnOp,
        /// Operand slot.
        a: SlotId,
        /// Source span.
        span: Span,
    },
    /// `dst = (a <cmp> b) ? 1 : 0`
    Cmp {
        /// Destination slot.
        dst: SlotId,
        /// The comparison.
        op: CmpOp,
        /// Left operand slot.
        a: SlotId,
        /// Right operand slot.
        b: SlotId,
        /// Source span.
        span: Span,
    },
    /// `dst = ctx pointer` (the packet base register).
    CtxPtr {
        /// Destination slot.
        dst: SlotId,
        /// Source span.
        span: Span,
    },
    /// `dst = ctx length` (the packet length).
    CtxLen {
        /// Destination slot.
        dst: SlotId,
        /// Source span.
        span: Span,
    },
    /// `dst = load<width>(ptr + off)` (bounds-checked by the runtime/verifier).
    Load {
        /// Destination slot.
        dst: SlotId,
        /// Load width.
        width: Width,
        /// Pointer slot (must be a `Ptr` region).
        ptr: SlotId,
        /// Byte offset.
        off: i32,
        /// Source span.
        span: Span,
    },
    /// `dst = map_get(map, key)` — the value for `key`, or `0` if absent (the
    /// null case is folded to zero by the helper).
    MapGet {
        /// Destination slot.
        dst: SlotId,
        /// Map index.
        map: u32,
        /// Key slot.
        key: SlotId,
        /// Source span.
        span: Span,
    },
    /// `map_set(map, key, val)` — store `val` for `key`.
    MapSet {
        /// Map index.
        map: u32,
        /// Key slot.
        key: SlotId,
        /// Value slot.
        val: SlotId,
        /// Source span.
        span: Span,
    },
}

/// How a basic block ends.
#[derive(Debug, Clone)]
pub enum Term {
    /// Unconditional jump.
    Goto {
        /// Target block.
        target: BlockId,
        /// Source span.
        span: Span,
    },
    /// Branch on `cond != 0`.
    BranchNz {
        /// Condition slot.
        cond: SlotId,
        /// Block taken when `cond != 0`.
        t: BlockId,
        /// Block taken when `cond == 0`.
        f: BlockId,
        /// Source span.
        span: Span,
    },
    /// Return the verdict (low 32 bits: bytes to accept; `0` = drop).
    Return {
        /// Slot holding the verdict.
        verdict: SlotId,
        /// Source span.
        span: Span,
    },
}

/// A basic block.
#[derive(Debug, Clone)]
pub struct Block {
    /// Block label.
    pub id: BlockId,
    /// Instructions in order.
    pub insts: Vec<Inst>,
    /// The terminator.
    pub term: Term,
}

/// The eBPF program type / attach point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgType {
    /// `BPF_PROG_TYPE_SOCKET_FILTER`.
    SocketFilter,
}

/// A declared BPF map. In v1 maps are integer-keyed, integer-valued (`key` and
/// `value` are passed by value to the map helpers); the declared sizes are kept
/// as metadata.
#[derive(Debug, Clone)]
pub struct MapDef {
    /// Map name as written.
    pub name: String,
    /// Dense map index (0-based, in declaration order).
    pub index: u32,
    /// Declared key size in bytes.
    pub key_size: u32,
    /// Declared value size in bytes.
    pub value_size: u32,
    /// Declared maximum entries.
    pub max_entries: u32,
    /// Source span of the declaration.
    pub span: Span,
}

/// A complete, typed, verifier-shaped program.
#[derive(Debug, Clone)]
pub struct BpfProgram {
    /// Program type / attach point.
    pub prog_type: ProgType,
    /// Entry block.
    pub entry: BlockId,
    /// All blocks, sorted by [`BlockId`].
    pub blocks: Vec<Block>,
    /// Number of slots; each occupies 8 bytes of stack.
    pub num_slots: u32,
    /// Type of each slot, indexed by [`SlotId`].
    pub slot_types: Vec<Ty>,
    /// Declared maps, indexed by [`MapDef::index`].
    pub maps: Vec<MapDef>,
}

/// One `when EVENT priority N { … }` declaration compiled to a program.
#[derive(Debug, Clone)]
pub struct BpfProgramDecl {
    /// The (BPF-native) event name as written.
    pub event: String,
    /// Handler priority (F5-inspired; default 500). Ordering metadata in v1.
    pub priority: u32,
    /// The compiled program.
    pub program: BpfProgram,
    /// Byte offset of the handler body within the original source (so
    /// body-relative diagnostic spans can be mapped back to the file).
    pub source_base: u32,
}

/// A whole `.bpftcl` translation unit: a priority-ordered bundle of programs.
#[derive(Debug, Clone)]
pub struct BpfModule {
    /// The programs, ordered by ascending priority then event name.
    pub programs: Vec<BpfProgramDecl>,
}
