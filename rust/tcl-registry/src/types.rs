//! Tcl internal representation types.
//!
//! Tcl values are always strings but may cache a typed internal
//! representation. This enum models the set of known intreps used
//! throughout the registry, compiler, and analyser.

/// Known Tcl internal representation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TclType {
    /// Pure string (no cached intrep).
    String,
    /// Integer.
    Int,
    /// Double-precision float.
    Double,
    /// Boolean.
    Boolean,
    /// Tcl list.
    List,
    /// Tcl dict.
    Dict,
    /// Byte array.
    ByteArray,
    /// Abstract join of `Int` and `Double`.
    Numeric,
    /// `TclOO` object instance.
    Object,
    /// I/O channel handle.
    Channel,
}
