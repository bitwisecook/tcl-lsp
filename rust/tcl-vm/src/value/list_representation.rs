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

//! The List representation: native construction, replacement, append and copy, retained backing, and conversion of a value to its elements.

use super::{IntRep, NativeJimSourceInfo, NativeStringAccess, Value};
use crate::error::TclError;
use std::cell::Cell;
use std::rc::Rc;
use tcl_syntax::list;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::value::ValueError;

impl Value {
    /// Apply the selected native List constructor, independently of cached List transport.
    /// C's zero-element constructor returns canonical empty NULL-type storage;
    /// Jim's constructor retains a pure List primary representation.
    #[must_use]
    pub fn native_list_constructor(
        items: Vec<Value>,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self {
        if items.is_empty() && protocol.tcl_version().is_some() {
            Self::new_native_string_bytes(&b""[..])
        } else {
            let value = Self::list(items);
            value
                .seal_compound_string_protocol(protocol)
                .expect("fresh native List recipe");
            value
        }
    }

    /// Replace the entire private List contents with original member objects.
    /// Receiver sharing is sampled before acquiring a working reference.
    pub(crate) fn native_list_replace_elements(
        original: &Self,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let value = if original.native_object_is_shared() {
            original.duplicate_native_object_in(protocol)
        } else {
            original.clone()
        };
        value.native_list_replace_prepared_using::<ValueError>(
            0,
            usize::MAX,
            elements,
            protocol,
            &mut None,
            &mut None,
        )?;
        Ok(value)
    }

    /// Replace a reached range on the already selected private List header.
    /// The caller owns the real working hold and its original operation.
    pub(crate) fn native_list_replace_prepared_range_checked(
        &self,
        first: usize,
        delete: usize,
        elements: &[Self],
        protocol: NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.native_list_replace_prepared_using(
            first,
            delete,
            elements,
            protocol,
            &mut Some(check),
            &mut Some(strings),
        )
    }

    fn native_list_replace_prepared_using<E: From<ValueError>>(
        &self,
        first: usize,
        delete: usize,
        elements: &[Self],
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<(), E> {
        drop(self.native_object_list_elements_using(protocol, check, strings)?);
        Self::native_stage_check(check)?;
        let mut primary = self.0.intrep.borrow_mut();
        let IntRep::List {
            items, canonical, ..
        } = &mut *primary
        else {
            unreachable!("selected native List replacement cache");
        };
        let first = first.min(items.len());
        let delete = delete.min(items.len() - first);
        if let Some(version) = protocol.tcl_version() {
            items.replace_native(first, delete, elements, version)?;
            *canonical = items.canonical_state();
        } else {
            let members = items.as_ref()[..first]
                .iter()
                .chain(elements)
                .chain(&items.as_ref()[first + delete..])
                .cloned()
                .collect();
            *items = crate::NativeListItems::new(members, false);
            *canonical = items.canonical_state();
        }
        drop(primary);
        Self::native_stage_check(check)?;
        self.invalidate_native_list_string();
        Self::native_stage_check(check)
    }

    /// Append original member objects under the selected native List recipe.
    /// An empty dictionary-member batch deliberately avoids conversion.
    pub(crate) fn native_list_append_elements(
        original: Option<&Self>,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let value = match original {
            Some(value) if value.native_object_is_shared() => {
                let duplicate = value.duplicate_native_object_in(protocol);
                if elements.is_empty() {
                    return Ok(duplicate);
                }
                drop(duplicate.native_object_list_elements(protocol)?);
                duplicate
            }
            Some(value) => {
                if elements.is_empty() {
                    return Ok(value.clone());
                }
                drop(value.native_object_list_elements(protocol)?);
                value.clone()
            }
            None => return Ok(Self::native_list_constructor(elements.to_vec(), protocol)),
        };
        value.native_list_append_prepared_elements(elements, protocol)?;
        Ok(value)
    }

    /// Mutate the physical header already selected by a native variable/list
    /// instruction, without treating its working reference as another COW test.
    pub(crate) fn native_list_append_prepared_elements(
        &self,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.native_list_append_prepared_using(elements, protocol, &mut None, &mut None)
    }

    /// The existing append kernel under the caller's retained original checks.
    pub(crate) fn native_list_append_prepared_elements_checked(
        &self,
        elements: &[Self],
        protocol: NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.native_list_append_prepared_using(
            elements,
            protocol,
            &mut Some(check),
            &mut Some(strings),
        )
    }

    fn native_list_append_prepared_using<E: From<ValueError>>(
        &self,
        elements: &[Self],
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<(), E> {
        drop(self.native_object_list_elements_using(protocol, check, strings)?);
        Self::native_stage_check(check)?;
        let mut primary = self.0.intrep.borrow_mut();
        let IntRep::List {
            items, canonical, ..
        } = &mut *primary
        else {
            unreachable!("selected native List append cache");
        };
        if elements.is_empty() {
            let preserve = protocol
                .tcl_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0 && canonical.get());
            drop(primary);
            if !preserve {
                self.invalidate_native_list_string();
            }
            return Self::native_stage_check(check);
        }
        let flag = if items.native_is_shared() {
            protocol.copied_list_canonical(canonical.get())
        } else {
            canonical.get()
        };
        if let Some(version) = protocol.tcl_version() {
            items.replace_native(items.len(), 0, elements, version)?;
            if version < tcl_dialect::TclVersion::V9_0 {
                items.canonical_state().set(flag);
            }
        } else {
            let mut members = items.as_ref().clone();
            members.extend(elements.iter().cloned());
            *items = crate::NativeListItems::new(members, flag);
        }
        *canonical = items.canonical_state();
        drop(primary);
        Self::native_stage_check(check)?;
        self.invalidate_native_list_string();
        Self::native_stage_check(check)
    }

    pub(super) fn invalidate_native_list_string(&self) {
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.double_format.borrow_mut() = None;
        *self.0.source_location.borrow_mut() = None;
    }

    /// `LAPPEND_LIST` validates/converts its original receiver before header COW.
    pub(crate) fn native_list_append_list_elements(
        &self,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        drop(self.native_object_list_elements(protocol)?);
        let value = if self.native_object_is_shared() {
            self.duplicate_native_object_in(protocol)
        } else {
            self.clone()
        };
        value.native_list_append_prepared_elements(elements, protocol)?;
        Ok(value)
    }

    /// `LIST_CONCAT` duplicates the target before source List conversion.
    pub(crate) fn native_list_concatenate(
        &self,
        source: &Self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let value = if self.native_object_is_shared() {
            self.duplicate_native_object_in(protocol)
        } else {
            self.clone()
        };
        let source = source.native_object_list_elements(protocol)?;
        value.native_list_append_prepared_elements(&source, protocol)?;
        Ok(value)
    }

    /// C9 full-range List opcode used for one expanded lappend operand.
    pub(crate) fn native_full_list_range(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V9_0)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "C9 full List range",
            ));
        }
        let members = self.native_object_list_elements(protocol)?;
        if members.is_empty() {
            return Ok(
                if self
                    .0
                    .string
                    .borrow()
                    .as_ref()
                    .is_none_or(|bytes| bytes.bytes().is_empty())
                {
                    self.clone()
                } else {
                    Self::new_native_string_bytes(&b""[..])
                },
            );
        }
        let value = if self.native_object_is_shared() {
            self.native_list_copy(protocol)?
        } else {
            self.clone()
        };
        value.invalidate_native_list_string();
        Ok(value)
    }

    /// Inspect original List storage without conversion or string generation.
    #[must_use]
    pub fn cached_list_representation(&self) -> Option<(crate::NativeListItems, bool)> {
        match &*self.0.intrep.borrow() {
            IntRep::List {
                items, canonical, ..
            } => Some((items.lifetime_view(), canonical.get())),
            _ => None,
        }
    }

    /// Reach `TclListObjCopy`: convert the original, then create a fresh header
    /// with absent string storage sharing the authentic whole List backing.
    /// This purpose belongs to C Tcl 8.5 and later, independently of ordinary
    /// header duplication and the lifetime-only inspection view.
    pub(crate) fn native_list_copy(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        self.native_list_copy_using(protocol, &mut None, &mut None)
    }

    pub(crate) fn native_list_copy_checked(
        &self,
        protocol: NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        self.native_list_copy_using(protocol, &mut Some(check), &mut Some(strings))
    }

    fn native_list_copy_using<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<Self, E> {
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V8_5)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "TclListObjCopy",
            )
            .into());
        }
        let backing = self.native_object_list_elements_using(protocol, check, strings)?;
        Self::native_stage_check(check)?;
        let value = Self::from_retained_native_list_backing(&backing, protocol)?;
        Self::native_stage_check(check)?;
        Ok(value)
    }

    /// Retain original whole storage after independently authenticating its updater.
    ///
    /// # Errors
    /// Refuses a foreign retained recipe or compiler-private invocation storage.
    pub fn native_list_backing_in(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Option<crate::NativeListItems>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let Some((backing, _)) = self.cached_list_representation() else {
            return Ok(None);
        };
        if !backing.is_whole_backing() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native whole List backing shape",
            ));
        }
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                tcl_syntax::value::ValueError::from(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        Ok(Some(backing))
    }

    /// Check the current original-header attachment without reparsing members.
    #[must_use]
    pub fn native_list_attachment_matches(&self, backing: &crate::NativeListItems) -> bool {
        matches!(&*self.0.intrep.borrow(), IntRep::List { items, .. } if items.same_backing(backing))
    }

    /// Create one genuine header for a retained, authenticated whole backing.
    /// The engine bridge separately checks issuer, scope and private receipt.
    ///
    /// # Errors
    /// Refuses non-whole storage and backings with no remaining native header.
    pub fn from_retained_native_list_backing(
        backing: &crate::NativeListItems,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if !backing.is_whole_backing() || !backing.has_native_header() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retained native List backing",
            ));
        }
        let items = backing.native_header()?;
        Ok(Self::from_parts(
            None,
            IntRep::List {
                canonical: items.canonical_state(),
                items,
                string_protocol: Cell::new(Some(protocol)),
            },
        ))
    }

    /// Reconstruct original List storage with its explicitly transported canonical flag.
    #[must_use]
    pub fn list_with_native_canonical(items: Vec<Value>, canonical: bool) -> Self {
        Self::from_parts(None, IntRep::native_list(items, canonical, None))
    }

    /// Reconstruct cached List storage under its selected physical string recipe.
    #[must_use]
    pub fn list_with_native_canonical_in(
        items: Vec<Value>,
        canonical: bool,
        protocol: NativeStringProtocol,
    ) -> Self {
        let value = Self::list_with_native_canonical(items, canonical);
        value
            .seal_compound_string_protocol(protocol)
            .expect("new native List recipe");
        value
    }

    /// The value as a list of elements (`Tcl_ListObjGetElements`), caching the
    /// typed rep on first parse.
    pub fn as_list(&self) -> Result<crate::NativeListItems, TclError> {
        self.native_list_elements(
            tcl_dialect::ListParse::Strict,
            tcl_dialect::EscapeSyntax::default(),
        )
        .map_err(|error| {
            TclError::with_error_code(
                error.full_message_bytes(&self.string_bytes()),
                error.error_code(),
            )
        })
    }

    /// Parse exact native bytes under retained list and escape grammars.
    /// Existing list elements remain the original objects; a dictionary keeps
    /// its original key/value objects when converted to a list.
    pub(crate) fn native_list_elements(
        &self,
        syntax: tcl_dialect::ListParse,
        escapes: tcl_dialect::EscapeSyntax,
    ) -> Result<crate::NativeListItems, list::ListError> {
        self.native_list_elements_using::<ValueError>(syntax, escapes, &mut None, &mut None)
            .map_err(|error| match error {
                ValueError::ListParse { error, .. } => error,
                _ => unreachable!("neutral configured List conversion has only parse failures"),
            })
    }

    pub(crate) fn native_list_elements_with_strings<E: From<ValueError>>(
        &self,
        syntax: tcl_dialect::ListParse,
        escapes: tcl_dialect::EscapeSyntax,
        check: &mut dyn FnMut() -> Result<(), E>,
        strings: &mut NativeStringAccess<'_, E>,
    ) -> Result<crate::NativeListItems, E> {
        self.native_list_elements_using(syntax, escapes, &mut Some(check), &mut Some(strings))
    }

    fn native_list_elements_using<E: From<ValueError>>(
        &self,
        syntax: tcl_dialect::ListParse,
        escapes: tcl_dialect::EscapeSyntax,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<crate::NativeListItems, E> {
        Self::native_stage_check(check)?;
        if let IntRep::List { items, .. } = &*self.0.intrep.borrow() {
            if check.is_some() {
                items.check_generation()?;
            }
            return Ok(items.lifetime_view());
        }
        let dict = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) if self.0.string.borrow().is_none() => Some(Rc::clone(dict)),
            _ => None,
        };
        let items: Rc<Vec<Value>> = if let Some(dict) = dict {
            Rc::new(
                dict.pairs_backing()
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect(),
            )
        } else {
            let reached = match strings.as_deref_mut() {
                Some(strings) => strings(self),
                None => Ok(self.string_bytes()),
            };
            Self::native_stage_check(check)?;
            let bytes = reached?;
            let elements = list::split_list_bytes_in(&bytes, syntax, escapes).map_err(|error| {
                ValueError::ListParse {
                    error,
                    source: bytes.to_vec(),
                }
            })?;
            Rc::new(
                elements
                    .into_iter()
                    .map(|element| Self::new_native_string_bytes(element.into_owned()))
                    .collect(),
            )
        };
        let replacement = IntRep::native_list(items.as_ref().clone(), false, None);
        if check.is_some() {
            self.replace_primary_using(replacement, check)?;
        } else {
            *self.0.intrep.borrow_mut() = replacement;
        }
        Ok(self
            .cached_list_representation()
            .expect("installed native List")
            .0)
    }

    /// Reach native object-list conversion under its actual engine grammar.
    /// Current List members and pure Dictionary members retain object identity;
    /// resident dictionary spelling remains authoritative when parsing a list.
    ///
    /// # Errors
    /// Returns exact native list parse failure or checked storage refusal.
    pub fn native_object_list_elements(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::NativeListItems, ValueError> {
        self.native_object_list_elements_using(protocol, &mut None, &mut None)
    }

    #[cfg(test)]
    pub(crate) fn native_object_list_elements_checked(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<crate::NativeListItems, tcl_cmd_core::CmdError> {
        self.native_object_list_elements_using(protocol, &mut Some(check), &mut None)
    }

    pub(crate) fn native_object_list_elements_with_strings<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), E>,
        strings: &mut NativeStringAccess<'_, E>,
    ) -> Result<crate::NativeListItems, E> {
        self.native_object_list_elements_using(protocol, &mut Some(check), &mut Some(strings))
    }

    pub(super) fn native_object_list_elements_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<crate::NativeListItems, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        if let IntRep::List { items, .. } = &*self.0.intrep.borrow() {
            if check.is_some() {
                items.check_generation()?;
            }
            return Ok(items.lifetime_view());
        }
        let dict = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) if self.0.string.borrow().is_none() => Some(Rc::clone(dict)),
            _ => None,
        };
        let mut allocated = None;
        let items: Rc<Vec<Value>> = if let Some(dict) = dict {
            Rc::new(
                dict.pairs_backing()
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect(),
            )
        } else {
            let context = if protocol.is_jim084() {
                Some(self.native_jim_context()?)
            } else {
                None
            };
            let source_info = context
                .as_ref()
                .map(|context| self.pin_native_jim_source_info(context))
                .transpose()?;
            let reached = match strings.as_deref_mut() {
                Some(strings) => strings(self),
                None => self
                    .native_string_bytes(protocol)
                    .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                    .map_err(ValueError::from)
                    .map_err(E::from),
            };
            Self::native_stage_check(check)?;
            let original = reached?;
            if protocol
                .tcl_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
            {
                allocated = Some(list::max_list_length_bytes(&original).max(1));
            }
            let elements =
                list::split_native_list_elements(&original, protocol).map_err(|error| {
                    ValueError::NativeListParse {
                        error,
                        source: original.to_vec(),
                        protocol,
                    }
                })?;
            let mut members = Vec::with_capacity(elements.len());
            for element in elements {
                let member = Self::new_native_string_bytes(element.value.into_owned());
                if let (Some(context), Some(info)) = (&context, &source_info) {
                    member.install_native_jim_source(
                        NativeJimSourceInfo {
                            filename: info.filename.clone(),
                            line: info.line.wrapping_add_unsigned(element.line_delta),
                        },
                        context,
                    )?;
                }
                members.push(member);
            }
            Rc::new(members)
        };
        self.replace_primary_using(
            IntRep::native_list(items.as_ref().clone(), false, Some(protocol)),
            check,
        )?;
        if let Some(allocated) = allocated
            && let IntRep::List { items, .. } = &*self.0.intrep.borrow()
        {
            items.set_capacity(allocated);
        }
        Ok(self
            .cached_list_representation()
            .expect("installed native List")
            .0)
    }
}
