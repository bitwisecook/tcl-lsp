//! Value types for the typed BPF-IR. Deliberately fixed-width — there is no
//! dynamic Tcl shimmering here.

/// A bounded memory region a pointer can refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// The program context / packet buffer (the socket-filter `__sk_buff` data).
    Ctx,
}

/// A value type in the BPF-IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    /// A 64-bit integer. In v1 every scalar verb (`setint`/`seti32`/`setu32`,
    /// the `load*` results, `pktlen`) lands here; sub-64-bit width fidelity is
    /// a follow-on.
    Int,
    /// A pointer into a bounded [`Region`].
    Ptr(Region),
}

impl Ty {
    /// Whether this type may be used as an arithmetic / comparison operand.
    #[must_use]
    pub fn is_int(self) -> bool {
        matches!(self, Ty::Int)
    }
}

/// The width of a packet load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    /// 8-bit (1 byte).
    B8,
    /// 16-bit (2 bytes).
    B16,
    /// 32-bit (4 bytes).
    B32,
}
