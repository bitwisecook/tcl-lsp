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

//! Actual C command-name object protocol and command-literal registration
//! results, independent of the compiler words that select them.

use tcl_runtime_api::native_command_name::{NativeCommandNamePriming, NativeLiteralContext};
use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable;

/// Selected actual native C command-name object protocol.
/// This selects pure rules and never grants a live binding or cache hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCommandNameProtocol(tcl_dialect::TclVersion);

impl NativeCommandNameProtocol {
    /// Actual C command-node epoch stage; no live node or table is supplied.
    #[must_use]
    pub const fn deletion_epoch_before_trace(self, recursive: bool) -> bool {
        use tcl_dialect::TclVersion;
        match self.0 {
            TclVersion::V8_4 => !recursive,
            TclVersion::V8_5 | TclVersion::V8_6 => true,
            TclVersion::V9_0 | TclVersion::V9_1 => false,
        }
    }

    /// Reached hash unlink advances C8.6 only on the final outer branch;
    /// C9 advances both recursive and final branches. The adapter supplies
    /// actual hash occupancy, independently of this source predicate.
    #[must_use]
    pub const fn deletion_epoch_after_unlink(self, recursive: bool) -> bool {
        use tcl_dialect::TclVersion;
        match self.0 {
            TclVersion::V8_4 | TclVersion::V8_5 => false,
            TclVersion::V8_6 => !recursive,
            TclVersion::V9_0 | TclVersion::V9_1 => true,
        }
    }

    /// Original namespace lookup invalidation follows the outer trace pass.
    /// C8.4 has no `TclInvalidateNsCmdLookup` at this deletion site.
    #[must_use]
    pub const fn deletion_invalidates_namespace_after_trace(self) -> bool {
        !matches!(self.0, tcl_dialect::TclVersion::V8_4)
    }

    /// Actual selected object-type release.
    #[must_use]
    pub const fn version(self) -> tcl_dialect::TclVersion {
        self.0
    }

    /// Whether a transported descriptor has this exact native object-type origin.
    #[must_use]
    pub fn accepts_cache_origin(self, version: tcl_dialect::TclVersion) -> bool {
        self.0 == version
    }

    /// Validate the command selected by a fresh original lookup across reached
    /// representation retirement. A dying holder can still own this binding.
    #[must_use]
    pub fn selected_binding_is_current(
        self,
        cache: &tcl_runtime_api::native_command_name::NativeCommandNameCache,
        state: &tcl_runtime_api::native_command_name::NativeCommandNameLookupState,
    ) -> bool {
        self.accepts_cache_origin(cache.version)
            && cache.interpreter == state.interpreter
            && cache
                .reference
                .is_none_or(|reference| reference == state.reference)
            && state.target.as_ref().is_some_and(|target| {
                cache.token == target.token
                    && cache.implementation_generation == target.implementation_generation
                    && cache.command_epoch == target.command_epoch
            })
    }

    /// Validate cached reuse against the original release's namespace gate.
    /// A false answer requests original native lookup; it does not erase cmdName.
    #[must_use]
    pub fn cache_is_current(
        self,
        cache: &tcl_runtime_api::native_command_name::NativeCommandNameCache,
        state: &tcl_runtime_api::native_command_name::NativeCommandNameLookupState,
    ) -> bool {
        self.selected_binding_is_current(cache, state)
            && state.target.as_ref().is_some_and(|target| {
                self.0 == tcl_dialect::TclVersion::V8_4 || !target.namespace_dying
            })
    }

    /// Referencing namespace installed by compile-time `TclSetCmdNameObj`.
    #[must_use]
    pub fn priming_reference(
        self,
        absolute: bool,
        current: tcl_runtime_api::native_command_name::NativeCommandNameReference,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameReference> {
        (!absolute || self.0 == tcl_dialect::TclVersion::V8_4).then_some(current)
    }

    /// Referencing namespace installed by original `Tcl_GetCommandFromObj`.
    /// C8.4 executes absolute-name conversion in the actual global context.
    #[must_use]
    pub fn lookup_reference(
        self,
        absolute: bool,
        current: tcl_runtime_api::native_command_name::NativeCommandNameReference,
        global: tcl_runtime_api::native_command_name::NativeCommandNameReference,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameReference> {
        if absolute && self.0 == tcl_dialect::TclVersion::V8_4 {
            Some(global)
        } else {
            (!absolute).then_some(current)
        }
    }

    /// Failed actual name lookup installs a null cmdName descriptor only on C8.
    /// C9 preserves the existing primary representation when no named node exists.
    #[must_use]
    pub fn installs_unresolved_on_miss(self) -> bool {
        self.0 < tcl_dialect::TclVersion::V9_0
    }

    /// C8.5 object-command creation advances reverse-path reference epochs
    /// before resolving occupancy; later releases do so for fresh publication.
    #[must_use]
    pub fn invalidates_path_before_object_create(self) -> bool {
        self.0 == tcl_dialect::TclVersion::V8_5
    }

    /// `TclSetCmdNameObj`'s release-specific early return, independently of getters.
    #[must_use]
    pub fn preserves_primed_cache(
        self,
        existing: Option<&tcl_runtime_api::native_command_name::NativeCommandNameCache>,
        existing_unresolved: bool,
        incoming: &tcl_runtime_api::native_command_name::NativeCommandNameCache,
    ) -> bool {
        if self.0 <= tcl_dialect::TclVersion::V8_5 {
            existing.is_some() || existing_unresolved
        } else {
            existing.is_some_and(|cache| {
                cache.interpreter == incoming.interpreter && cache.token == incoming.token
            })
        }
    }
}

impl crate::InvocationDialect {
    /// Select the actual C command-name object protocol, independently of assistance
    /// and authored logical simulation. Unknown/Jim/vendor engines abstain.
    #[must_use]
    pub fn native_command_name_protocol(self) -> Option<NativeCommandNameProtocol> {
        match self.native_string_protocol()? {
            tcl_syntax::native_string::NativeStringProtocol::C(version) => {
                Some(NativeCommandNameProtocol(version))
            }
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => None,
        }
    }
}

/// Retained generic command-head registration, independently of its cache action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompiledCommandLiteral {
    /// Actual original compiler context, including its namespace incarnation.
    pub context: NativeLiteralContext,
    /// Complete original command-head value bytes.
    pub bytes: Vec<u8>,
    /// Absolute native lookup selects the release-specific root partition.
    pub fully_qualified: bool,
    /// C8.5 hides each generic command's single-word literal after priming.
    pub hide: bool,
    /// Named native binding found by the compiler; closed absence has no priming.
    pub priming: Option<NativeCommandNamePriming>,
}

/// Missing actual command-literal compilation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandLiteralUnavailable {
    /// Actual canonical C compiler/name policy or retained namespace is missing.
    Context,
    /// The actual command lookup cannot settle a named binding or closed absence.
    Lookup(NativeCommandLookupUnavailable),
}
