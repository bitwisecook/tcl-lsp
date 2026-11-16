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

//! Alias names: the order each release reads and captures an alias target, the registration query and collision token, and the slot an alias publishes to.

use super::super::is_qualified;
use super::routing::NativeCommandSlotProjection;
use super::{
    NameProjectionUnavailable, NativeNameContext, NativeNameProjection, NativeNameProtocol,
    NativeNamePurpose, projection,
};
use std::borrow::Cow;
use tcl_core_types::{ByteCommandSlot, c_string_extent};
use tcl_dialect::TclVersion;

impl NativeNameProtocol {
    /// C8.4/8.5's original `interp alias` reaches the target String getter
    /// before retaining the alias operands. Later C recipes retain and publish
    /// first, then reach the target during loop lookup. Jim's core alias does
    /// not read the target at definition. This is stage data, not admission.
    #[must_use]
    pub const fn alias_target_string_precedes_capture(self) -> bool {
        matches!(self, Self::C(TclVersion::V8_4 | TclVersion::V8_5))
    }

    /// The empty-target pretest belongs only to the selected C8.4/8.5 stage.
    /// `None` means that operation is absent, not a guessed nonempty target.
    #[must_use]
    pub fn alias_pretested_target_is_empty(self, original: &[u8]) -> Option<bool> {
        self.alias_target_string_precedes_capture()
            .then(|| c_string_extent(original).is_empty())
    }

    /// Every C child-handle alias reads the target before capture. This
    /// selected entry is separate from the `interp alias` ensemble split.
    #[must_use]
    pub const fn child_alias_target_string_precedes_capture(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// The child-handle empty-target check consumes the selected C String
    /// extent. Jim's child alias has no such pretest.
    #[must_use]
    pub fn child_alias_pretested_target_is_empty(self, original: &[u8]) -> Option<bool> {
        self.child_alias_target_string_precedes_capture()
            .then(|| c_string_extent(original).is_empty())
    }

    /// C child-handle aliases return the original registration name. The
    /// Jim child extension leaves its empty result in place.
    #[must_use]
    pub const fn child_alias_returns_original_registration_name(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// C alias records own the original registration name. Jim owns its
    /// command-table key separately, and its child String-name constructor
    /// does not retain the original caller name.
    #[must_use]
    pub const fn alias_record_retains_original_registration_name(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// C reaches the original target after publication to reject alias loops;
    /// Jim admits loops and leaves all target/prefix Strings lazy until use.
    #[must_use]
    pub const fn alias_publication_checks_original_target(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// C query creates a fresh List retaining the original members. Jim's
    /// `info alias` returns its stored List header and therefore its own cache.
    #[must_use]
    pub const fn alias_query_returns_original_list(self) -> bool {
        matches!(self, Self::Jim084)
    }

    /// Jim sets the original name result before registering the command;
    /// C returns its retained registration token after successful publication.
    #[must_use]
    pub const fn alias_result_precedes_name_registration(self) -> bool {
        matches!(self, Self::Jim084)
    }

    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn alias_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        match self {
            Self::C(_) => {
                let context = if is_qualified(c_string_extent(original)) {
                    context
                } else {
                    NativeNameContext::root()
                };
                self.command_input(context, original, NativeNamePurpose::AliasPublication)
            }
            Self::Jim084 => Ok(projection(
                self,
                NativeNamePurpose::AliasPublication,
                original,
                Cow::Borrowed(original),
                Some(NativeNameContext::root()),
            )),
        }
    }

    /// Select the original C alias-table operand without namespace resolution.
    ///
    /// # Errors
    /// Jim's alias queries use its independent command-object lookup.
    pub fn alias_registration_query_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        match self {
            Self::C(_) => Ok(projection(
                self,
                NativeNamePurpose::AliasRegistrationQuery,
                original,
                Cow::Borrowed(c_string_extent(original)),
                None,
            )),
            Self::Jim084 => Err(NameProjectionUnavailable::PurposeNotModelled),
        }
    }

    /// Construct a later-C alias record token after an occupied registration.
    /// The complete getter result survives; its C-string table key is separate.
    ///
    /// # Errors
    /// C8.4 deletes the colliding token and Jim has no C alias table.
    pub fn alias_registration_collision_token(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_5) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut token = Vec::with_capacity(original.len().saturating_add(2));
        token.extend_from_slice(b"::");
        token.extend_from_slice(original);
        Ok(token)
    }

    /// Child alias extensions construct their command name independently of the
    /// standalone alias command. Jim's `Jim_RegisterCmd` receives `Jim_String` and
    /// builds a new -1-length string at the child root; original counted input
    /// remains separate from the selected C-string extent and report spelling.
    /// C uses its existing alias-registration rule in the selected child realm.
    ///
    /// # Errors
    /// Refuses an unavailable registration purpose or context.
    pub fn child_alias_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        match self {
            Self::C(_) => {
                let mut selected = self.alias_publication_input(context, original)?;
                selected.purpose = NativeNamePurpose::ChildAliasPublication;
                Ok(selected)
            }
            Self::Jim084 => Ok(projection(
                self,
                NativeNamePurpose::ChildAliasPublication,
                original,
                Cow::Borrowed(c_string_extent(original)),
                Some(NativeNameContext::root()),
            )),
        }
    }

    /// Alias commands publish separately from procedure declarations. C keeps
    /// an unqualified name global and retains the current holder for relative
    /// qualified names. Jim retains the complete original naming object at its
    /// global table, while shared table comparison strips leading root colons.
    ///
    /// # Errors
    /// Refuses an unsupported native publication projection.
    pub fn alias_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        self.alias_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }
}
