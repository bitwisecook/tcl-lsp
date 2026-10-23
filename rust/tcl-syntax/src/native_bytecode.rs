// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C executable-source storage rules, independent of admission.

use tcl_dialect::TclVersion;

/// A pure C Bytecode storage recipe; it grants no executable artifact authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBytecodeStorageRecipe(TclVersion);
impl NativeBytecodeStorageRecipe {
    /// Select a pure release recipe, without authenticating a compiler.
    #[must_use]
    pub const fn for_version(version: TclVersion) -> Self {
        Self(version)
    }
    /// Native origin of a genuinely published original-source primary.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.0
    }
    /// Modern C retains its executable parse-error body; C84 declines publication.
    #[must_use]
    pub const fn retains_parse_failure(self) -> bool {
        !matches!(self.0, TclVersion::V8_4)
    }

    /// Selected `TclProcCleanupProc` clears a surviving ByteCode.procPtr.
    /// C84/C85 release the body without this modern context mutation.
    #[must_use]
    pub const fn clears_retired_procedure_context(self) -> bool {
        matches!(
            self.0,
            TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
        )
    }
}
