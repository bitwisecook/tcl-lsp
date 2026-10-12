// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C executable-source storage selection.

use tcl_dialect::model::Family;
use tcl_syntax::native_bytecode::NativeBytecodeStorageRecipe;

/// Actual engine selection; the caller still supplies the admitted executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBytecodeStorageProtocol(NativeBytecodeStorageRecipe);
impl NativeBytecodeStorageProtocol {
    /// Pure physical storage recipe; not a compiler or original-object receipt.
    #[must_use]
    pub const fn recipe(self) -> NativeBytecodeStorageRecipe {
        self.0
    }
}
impl crate::InvocationDialect {
    /// Select actual native C storage, without deriving it from logical source grammar.
    #[must_use]
    pub fn native_bytecode_storage_protocol(self) -> Option<NativeBytecodeStorageProtocol> {
        (self.family()? == Family::Tcl).then_some(NativeBytecodeStorageProtocol(
            NativeBytecodeStorageRecipe::for_version(self.tcl_version?),
        ))
    }
}
