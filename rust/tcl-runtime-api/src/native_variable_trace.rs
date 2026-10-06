// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Direct native variable callbacks on an interpreter's retained storage cell.

use std::rc::Rc;

/// A native variable trace operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeVariableTraceOperation {
    /// Variable read.
    Read,
    /// Variable write.
    Write,
    /// Cell destruction or unset.
    Unset,
    /// Array operation.
    Array,
}
impl NativeVariableTraceOperation {
    /// Native operation spelling used by the shared trace coordinator.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Unset => "unset",
            Self::Array => "array",
        }
    }
    /// Select a reached operation, without interpreting variable names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "unset" => Some(Self::Unset),
            "array" => Some(Self::Array),
            _ => None,
        }
    }
}
/// Borrowed native reported name parts at the actual callback boundary.
#[derive(Clone, Copy, Debug)]
pub struct NativeVariableTraceAccess<'a> {
    /// Reached operation.
    pub operation: NativeVariableTraceOperation,
    /// Original reported variable root.
    pub name1: &'a [u8],
    /// Original reported element, empty for a scalar.
    pub name2: &'a [u8],
    /// Whether this callback belongs to the cell's destructive unset walk.
    pub destroyed: bool,
}
/// A direct callback. It executes in the current interpreter, without a script frame.
pub trait NativeVariableObserver<R> {
    /// Typed callback failure; hosts bind this to their command error carrier.
    type Error;
    /// Observe the actual reached cell operation.
    fn observe(
        &self,
        runtime: &mut R,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error>;
}
/// Identity of one registration. Only a matching live interpreter row authorizes removal.
#[derive(Clone, Default)]
pub struct NativeVariableTraceToken(Rc<()>);
impl NativeVariableTraceToken {
    /// Mint an identity for a new actual registration row.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Compare identities, independent of variable spelling or reused cell addresses.
    #[must_use]
    pub fn same_registration(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
