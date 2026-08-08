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

//! The typed, backend-agnostic BPF-IR: a small three-address code over mutable
//! slots (C-like locals — no SSA / phi), produced from the Tcl front-end and
//! consumed by the eBPF (and later WASM) backends.

use tcl_lexer::Span;

use crate::ty::{ByteOrder, Ty, Width};

/// The reserved return value a handler yields for the `next` outcome — the
/// explicit, non-terminal continuation of handler composition (issue #1204).
///
/// It is the 32-bit all-ones sentinel (`-1` as a signed 32-bit action), a value
/// no real verdict uses: socket-filter accept counts are non-negative byte
/// counts and `0` drops; XDP actions are `0..=4`; TC actions are small
/// non-negative constants. The composition evaluator
/// ([`crate::compose`]) reads a run result equal to this (masked to 32 bits) as
/// "continue to the next handler"; every other value is a terminal decision.
pub const CONTINUE_SENTINEL: i64 = 0xffff_ffff;

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
    /// `dst = load<width>(ptr + off)` — a checked packet read.
    ///
    /// The contract every target must honour:
    ///
    /// - The load touches the constant byte range `off .. off + width.bytes()`
    ///   relative to the packet start (the range a dominating bounds proof
    ///   must cover — there is no dynamic-offset form yet).
    /// - No static in-bounds proof is attached: the **target** must emit a
    ///   dominating `ptr + off + width <= data_end` check (or equivalent)
    ///   before the dereference.
    /// - When the packet is too short, the program takes the explicit
    ///   [`OobAction`] — it never reads out of bounds and never traps.
    /// - Multi-byte results are interpreted per [`ByteOrder`] and
    ///   zero-extended to 64 bits.
    Load {
        /// Destination slot.
        dst: SlotId,
        /// Load width.
        width: Width,
        /// Pointer slot (must be a `Ptr` region).
        ptr: SlotId,
        /// Byte offset.
        off: i32,
        /// How the loaded bytes are interpreted.
        order: ByteOrder,
        /// What happens when `off + width` exceeds the packet.
        oob: OobAction,
        /// Source span.
        span: Span,
    },
    /// `dst = map_get(map, key)` — the value for `key`, or `0` if absent (the
    /// null case is folded to zero by the helper). Use [`Inst::MapHas`] to
    /// distinguish a missing key from a stored zero.
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
    /// `dst = map_has(map, key)` — 1 when `key` is present, 0 when absent.
    MapHas {
        /// Destination slot.
        dst: SlotId,
        /// Map index.
        map: u32,
        /// Key slot.
        key: SlotId,
        /// Source span.
        span: Span,
    },
    /// `map_set(map, key, val)` — store `val` for `key`. Fails silently (the
    /// stored state is unchanged) when a hash map is at capacity.
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

/// What an out-of-bounds packet read does. Attached to every [`Inst::Load`]
/// so the failure semantics are explicit in the IR rather than implied by a
/// target's runtime behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OobAction {
    /// Terminate the program immediately with this verdict value (the
    /// program type's drop verdict in v1).
    ReturnVerdict(i64),
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
    /// `BPF_PROG_TYPE_SOCKET_FILTER` — verdict is bytes to accept (`0` = drop).
    SocketFilter,
    /// `BPF_PROG_TYPE_XDP` — verdict is `XDP_ABORTED`(0)/`DROP`(1)/`PASS`(2)/`TX`(3).
    Xdp,
    /// `BPF_PROG_TYPE_SCHED_CLS` on the ingress hook — verdict is a TC action
    /// (`TC_ACT_OK`(0)/`TC_ACT_SHOT`(2)).
    TcIngress,
    /// `BPF_PROG_TYPE_SCHED_CLS` on the egress hook — same verdict encoding as
    /// [`ProgType::TcIngress`].
    TcEgress,
    /// `BPF_PROG_TYPE_CGROUP_SOCK_ADDR`, `connect4` attach — verdict is
    /// allow(1)/deny(0).
    CgroupInet4Connect,
    /// `BPF_PROG_TYPE_CGROUP_SOCK_ADDR`, `bind4` attach — same verdict
    /// encoding as [`ProgType::CgroupInet4Connect`].
    CgroupInet4Bind,
}

impl ProgType {
    /// Whether this program type is one of the two `BPF_PROG_TYPE_SCHED_CLS`
    /// hooks (ingress/egress differ only in attach direction, not verdict
    /// encoding or context).
    #[must_use]
    pub const fn is_tc(self) -> bool {
        matches!(self, Self::TcIngress | Self::TcEgress)
    }

    /// Whether this program type is one of the two
    /// `BPF_PROG_TYPE_CGROUP_SOCK_ADDR` hooks (connect4/bind4 differ only in
    /// attach point, not verdict encoding or context).
    #[must_use]
    pub const fn is_cgroup_sock_addr(self) -> bool {
        matches!(self, Self::CgroupInet4Connect | Self::CgroupInet4Bind)
    }
}

/// The kind of a declared map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapKind {
    /// `BPF_MAP_TYPE_HASH` — sparse; a key is absent until stored.
    Hash,
    /// `BPF_MAP_TYPE_ARRAY` — preallocated and zero-filled; the key is an
    /// index (`0 .. max_entries`, key size fixed at 4 bytes) and every
    /// in-range key is always present.
    Array,
}

impl MapKind {
    /// Parse the DSL map-kind word.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "hash" => Some(MapKind::Hash),
            "array" => Some(MapKind::Array),
            _ => None,
        }
    }

    /// Stable display spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MapKind::Hash => "hash",
            MapKind::Array => "array",
        }
    }
}

/// The declared concurrency model of a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapConcurrency {
    /// One value per key, shared by every invocation (the default). Updates
    /// from concurrent invocations race — last write wins; there is no
    /// atomic read-modify-write in v1.
    #[default]
    Shared,
    /// One value per key **per CPU** (`BPF_MAP_TYPE_PERCPU_*`). Declared for
    /// the kernel target; the single-threaded rbpf harness runs it exactly
    /// like [`MapConcurrency::Shared`].
    PerCpu,
}

impl MapConcurrency {
    /// Parse the DSL concurrency word.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "shared" => Some(MapConcurrency::Shared),
            "percpu" => Some(MapConcurrency::PerCpu),
            _ => None,
        }
    }
}

/// A declared BPF map. In v1 maps are integer-keyed, integer-valued (`key` and
/// `value` are passed by value to the map helpers); key/value sizes must be
/// 1, 2, 4, or 8 bytes so a value fits one register.
#[derive(Debug, Clone)]
pub struct MapDef {
    /// Map name as written.
    pub name: String,
    /// Dense map index (0-based, in declaration order).
    pub index: u32,
    /// Map kind (validated at declaration).
    pub kind: MapKind,
    /// Declared key size in bytes (1, 2, 4, or 8; exactly 4 for an array).
    pub key_size: u32,
    /// Declared value size in bytes (1, 2, 4, or 8).
    pub value_size: u32,
    /// Declared maximum entries (enforced: a hash update on a full map with
    /// a new key fails; an array key is valid only below this bound).
    pub max_entries: u32,
    /// Declared concurrency model.
    pub concurrency: MapConcurrency,
    /// Source span of the declaration.
    pub span: Span,
}

/// A complete, typed, verifier-shaped program.
///
/// Slots are allocated by the liveness-based allocator
/// ([`crate::alloc::assign_slots`]): values with disjoint live ranges share a
/// stack slot, and `zero_init` lists exactly the slots that may be read
/// before every write (the only ones a target needs to zero at entry).
#[derive(Debug, Clone)]
pub struct BpfProgram {
    /// Program type / attach point.
    pub prog_type: ProgType,
    /// Entry block.
    pub entry: BlockId,
    /// All blocks, sorted by [`BlockId`].
    pub blocks: Vec<Block>,
    /// Number of allocated slots; each occupies 8 bytes of stack.
    pub num_slots: u32,
    /// Type of each slot, indexed by [`SlotId`].
    pub slot_types: Vec<Ty>,
    /// Number of values the program computed with *before* liveness-based
    /// slot reuse (diagnostic metadata for `check` output).
    pub raw_slot_count: u32,
    /// Slots that may be read on some path before they are written — the
    /// target must zero-initialise exactly these at entry. Sorted, deduped.
    pub zero_init: Vec<SlotId>,
    /// Declared maps, indexed by [`MapDef::index`].
    pub maps: Vec<MapDef>,
}

/// Where and how a program is deployed (the attach/deployment facet). Pure
/// metadata in v1: carried for a future ELF/loader, not used by codegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachSpec {
    /// Attach kind as written (`xdp`, `socket_filter`, `socket`).
    pub kind: String,
    /// Attach target (an interface like `eth0`, a cgroup path, …).
    pub target: String,
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
    /// The deployment target, if a matching `attach` was declared.
    pub attach: Option<AttachSpec>,
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
