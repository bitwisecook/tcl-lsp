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

//! Command names by purpose: lookup, publication and rename inputs, the slot each publishes to, Jim's command table keys, `TclOO` methods and properties, and exposed and hidden commands.

use super::super::is_qualified;
use super::name_diagnostics::NativeNamespaceLookupError;
use super::routing::NativeCommandSlotProjection;
use super::{
    NameProjectionUnavailable, NativeNameContext, NativeNameProjection, NativeNameProtocol,
    NativeNamePurpose, join_jim, projection, strip_jim_root,
};
use std::borrow::Cow;
use tcl_core_types::{ByteCommandSlot, NameBytes, c_string_extent};
use tcl_dialect::TclVersion;

impl NativeNameProtocol {
    /// Lookup input in the retained current context. Path/fallback lookup and
    /// binding selection remain runtime operations on actual namespace identities.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_lookup_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::CommandLookup)
    }

    /// Script publication input, including procedure declarations.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::CommandPublication)
    }

    /// C ABI publication differs from script declarations: an unqualified name
    /// belongs to the global table; a qualified relative name uses current context.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_c_api_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let context = if is_qualified(c_string_extent(original)) {
            context
        } else {
            NativeNameContext::root()
        };
        self.command_input(context, original, NativeNamePurpose::CommandCApiPublication)
    }

    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn rename_source_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::RenameSource)
    }

    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn rename_destination_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::RenameDestination)
    }

    /// Select the `CString` name reported by a refused alias definition or
    /// rename. C8.4 reports the alias record's own name (`aliasPtr->namePtr`,
    /// the name the alias was created with); C8.5+ reports the attempted
    /// destination's simple name from its selected binding slot. Jim has no C
    /// alias-loop purpose.
    #[must_use]
    pub fn rename_alias_loop_name<'a>(
        self,
        source_simple: &'a [u8],
        destination_simple: &'a [u8],
    ) -> Option<&'a [u8]> {
        match self {
            Self::C(TclVersion::V8_4) => Some(c_string_extent(source_simple)),
            Self::C(_) => Some(c_string_extent(destination_simple)),
            Self::Jim084 => None,
        }
    }

    /// Select the exact rename destination without procedure-publication
    /// qualification or display reparsing.
    ///
    /// # Errors
    /// Refuses an unavailable native operation or missing actual Jim namespace.
    pub fn rename_destination_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        self.rename_destination_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    pub(super) fn command_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        let selected = match self {
            Self::C(_) => Cow::Borrowed(c_string_extent(original)),
            Self::Jim084 => {
                let namespace = context
                    .jim_namespace_object
                    .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
                if namespace.is_empty() || original.starts_with(b"::") {
                    Cow::Borrowed(original)
                } else {
                    Cow::Owned(join_jim(namespace, c_string_extent(original)))
                }
            }
        };
        Ok(projection(self, purpose, original, selected, Some(context)))
    }

    /// Construct a publication slot once. Jim's primary table is flat; it is
    /// represented at root with its complete comparison key, without C reparsing.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        self.command_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Counted data interpolated by Jim's two-word member-name construction.
    /// This is readonly value data, without a source word, publication or frame.
    ///
    /// # Errors
    /// Refuses engines without the audited Jim construction recipe.
    pub fn jim_two_word_member_value(
        self,
        class_name: &[u8],
        member: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut name = class_name.to_vec();
        name.push(b' ');
        name.extend_from_slice(member);
        Ok(name)
    }

    /// Publication geometry of Jim's factory-generated two-word commands.
    /// The provider interpolates the original counted class-name value, one
    /// space and the selected member units before the independently selected
    /// procedure or alias publication. This
    /// pure slot grants no class identity, installed command or live method.
    ///
    /// # Errors
    /// Refuses other engines, unsupported publication purposes and a missing
    /// actual Jim namespace object for procedure publication.
    pub fn jim_two_word_member_publication_slot(
        self,
        context: NativeNameContext<'_>,
        class_name: &[u8],
        member: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        let name = self.jim_two_word_member_value(class_name, member)?;
        match purpose {
            NativeNamePurpose::CommandPublication => self.command_publication_slot(context, &name),
            NativeNamePurpose::AliasPublication => self.alias_publication_slot(context, &name),
            _ => Err(NameProjectionUnavailable::PurposeNotModelled),
        }
    }

    /// Coroutine publication selects a `CString` command name in the actual
    /// current holder. Runtime traversal must refuse missing qualifiers and
    /// retain exact namespace tokens; this pure slot creates no namespace.
    /// Empty names and trailing separators select the holder's empty command.
    ///
    /// # Errors
    /// Refuses releases without the audited C coroutine command.
    pub fn coroutine_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        self.coroutine_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Report refusal of a coroutine's missing qualifier namespace, retaining
    /// the original name separately from the native formatter's `CString`.
    ///
    /// # Errors
    /// Refuses releases without the audited C coroutine command.
    pub fn coroutine_unknown_namespace_error(
        self,
        original: &[u8],
    ) -> Result<NativeNamespaceLookupError, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut message = b"can't create procedure \"".to_vec();
        message.extend_from_slice(c_string_extent(original));
        message.extend_from_slice(b"\": unknown namespace");
        Ok(NativeNamespaceLookupError {
            message,
            error_code: b"TCL LOOKUP NAMESPACE".to_vec(),
        })
    }

    /// `TclOO` object commands select their `CString` name against the actual
    /// current holder and create missing qualifier namespaces. Method-table
    /// keys and original invocation words do not use this extent.
    ///
    /// # Errors
    /// Refuses engines without audited `TclOO` object creation.
    pub fn oo_object_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        self.oo_object_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Public collision message for an original `TclOO` object publication.
    /// The caller independently supplies the actual occupied slot and result producer.
    ///
    /// # Errors
    /// Refuses engines without audited `TclOO` object publication.
    pub fn oo_object_collision_message(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut message = b"can't create object \"".to_vec();
        message.extend_from_slice(c_string_extent(original));
        message.extend_from_slice(b"\": command already exists with that name");
        Ok(message)
    }

    /// Select a namespace ensemble command slot. The default names the actual
    /// namespace in its parent, rather than reparsing that namespace's display.
    /// Tcl 8.5 constructs and reparses a qualified command through its C API.
    ///
    /// # Errors
    /// Refuses unsupported engines or an absent actual Jim namespace object.
    pub fn ensemble_publication_slot(
        self,
        context: NativeNameContext<'_>,
        explicit: Option<&[u8]>,
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        let mut parent = context.namespace.clone();
        parent.pop();
        self.ensemble_publication_projection(context, explicit, Some(&parent))
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// First C lookup candidate in the actual retained namespace context.
    /// Namespace-path and global fallback traversal remain runtime operations;
    /// Jim uses `jim_command_lookup_keys` and its flat command table instead.
    ///
    /// # Errors
    /// Returns an error when this engine uses a different command-table model.
    pub fn command_lookup_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        self.command_lookup_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_c_api_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        self.command_c_api_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Jim's command hash comparison keeps counted bytes and removes leading root markers.
    /// This supplies no live header, namespace or table-entry authority.
    ///
    /// # Errors
    /// Returns unavailable for an engine without Jim's command hash protocol.
    pub fn jim_command_table_comparison(
        self,
        original: &[u8],
    ) -> Result<&[u8], NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(strip_jim_root(original))
    }

    /// Jim's current-namespace candidate and independent global fallback key.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn jim_command_lookup_keys(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<Vec<NameBytes>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let projected = self.command_lookup_input(context, original)?;
        let mut keys = vec![NameBytes::from(
            projected.jim_flat_key().expect("Jim command projection"),
        )];
        if !original.starts_with(b"::") {
            let global = NameBytes::from(strip_jim_root(original));
            if keys[0] != global {
                keys.push(global);
            }
        }
        Ok(keys)
    }

    /// `TclOO` method publication and dispatch retain counted original bytes.
    ///
    /// # Errors
    /// Returns an unavailable purpose for engines without audited `TclOO` support.
    pub fn oo_method_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            NativeNamePurpose::OoMethod,
            original,
            Cow::Borrowed(original),
            None,
        ))
    }

    /// C9 property declarations form their option name with `"-%s"` from
    /// the declaration's C string. The original counted name stays separate.
    /// This recipe grants no property installation, accessor or lookup cache.
    ///
    /// # Errors
    /// Returns an unavailable purpose outside the audited C9 property releases.
    pub fn oo_property_option_name(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V9_0) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut option = vec![b'-'];
        option.extend_from_slice(c_string_extent(original));
        Ok(projection(
            self,
            NativeNamePurpose::OoPropertyOption,
            original,
            Cow::Owned(option),
            None,
        ))
    }

    /// Global C visibility destination, preserving an empty hash key.
    /// Qualification is rejected before publication; Jim has no exposed table.
    ///
    /// # Errors
    /// Refuses unaudited engines and namespace-qualified destinations.
    pub fn exposed_command_publication_slot(
        self,
        original: &[u8],
    ) -> Result<tcl_core_types::ByteCommandSlot, NameProjectionUnavailable> {
        if self.is_jim084() || is_qualified(c_string_extent(original)) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected = projection(
            self,
            NativeNamePurpose::ExposedCommandPublication,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        );
        Ok(tcl_core_types::ByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::root(),
            tcl_core_types::NameBytes::from(selected.selected()),
        ))
    }

    /// C hidden-table tokens use their own `CString` namespace, without qualifiers.
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for an unaudited engine operation.
    pub fn hidden_token_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            NativeNamePurpose::HiddenToken,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        ))
    }
}
