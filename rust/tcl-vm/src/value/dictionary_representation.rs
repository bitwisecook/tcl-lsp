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

//! The Dictionary representation: construction, cached members, copy-on-write preparation and member updates, searches and cursors, and conversion of a value to its pairs.

use super::{
    DictRep, IntRep, NativeDictionarySearch, NativeObjectLifetimeLease, NativeStringAccess,
    OriginalNativeDictionaryCursor, PreparedNativeDictionary, Value,
};
use std::rc::Rc;
use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::value::{ValueError, canonical_dict_slots};

impl OriginalNativeDictionaryCursor {
    pub(crate) fn check(&self, original: &Value) -> Result<(), ValueError> {
        original.check_native_header()?;
        let cache = original.0.intrep.borrow();
        let current = matches!(&*cache, IntRep::Dict(actual)
            if self.backing.ptr_eq(&Rc::downgrade(actual)) && self.epoch == actual.epoch.get());
        if current {
            Ok(())
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "original Dictionary lookup table",
            ))
        }
    }

    pub(crate) fn member(
        &self,
        original: &Value,
        index: usize,
    ) -> Result<Option<(NativeObjectLifetimeLease, NativeObjectLifetimeLease)>, ValueError> {
        self.check(original)?;
        let cache = original.0.intrep.borrow();
        let IntRep::Dict(actual) = &*cache else {
            unreachable!("checked original Dictionary table")
        };
        let pair = actual.with_pairs(|pairs| {
            pairs
                .get(index)
                .map(|(key, value)| (key.native_lifetime_lease(), value.native_lifetime_lease()))
        });
        Ok(pair)
    }
}

impl NativeDictionarySearch {
    pub(crate) fn next_pair(
        &mut self,
    ) -> Result<Option<(Value, Value)>, tcl_syntax::value::ValueError> {
        let Some(backing) = self.backing.as_ref() else {
            return Ok(None);
        };
        if backing.epoch.get() != self.epoch {
            return Err(tcl_syntax::value::ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation,
            ));
        }
        let pair = backing.with_pairs(|pairs| pairs.get(self.next).cloned());
        self.next += usize::from(pair.is_some());
        if pair.is_none() {
            self.backing = None;
        }
        Ok(pair)
    }

    /// Borrow the next original pair at the native search frontier. The
    /// lifetime leases add no Tcl references; backing epoch checks remain the
    /// same as the owning result projection.
    #[cfg(test)]
    pub(crate) fn next_original_pair(
        &mut self,
    ) -> Result<Option<(Value, Value)>, tcl_syntax::value::ValueError> {
        self.next_original_pair_using(&mut None)
    }

    /// Carry the original caller across pair selection and exhausted-backing release.
    pub(crate) fn next_original_pair_checked(
        &mut self,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<Option<(Value, Value)>, tcl_cmd_core::CmdError> {
        self.next_original_pair_using(&mut Some(check))
    }

    fn next_original_pair_using<E: From<ValueError>>(
        &mut self,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<Option<(Value, Value)>, E> {
        Value::native_stage_check(check)?;
        if check.is_some()
            && let Some(root) = &self.root
        {
            root.check_native_header()?;
        }
        let Some(backing) = self.backing.as_ref() else {
            return Ok(None);
        };
        if backing.epoch.get() != self.epoch {
            return Err(tcl_syntax::value::ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation,
            )
            .into());
        }
        let pair = backing.with_pairs(|pairs| {
            pairs.get(self.next).map(|(key, value)| {
                (
                    key.native_lifetime_lease().into_value(),
                    value.native_lifetime_lease().into_value(),
                )
            })
        });
        self.next += usize::from(pair.is_some());
        if pair.is_none() {
            self.backing = None;
        }
        Value::native_stage_check(check)?;
        if check.is_some()
            && let Some(root) = &self.root
        {
            root.check_native_header()?;
        }
        Ok(pair)
    }

    /// Close an unfinished native search while retaining the iterator's root.
    pub(crate) fn close_search(&mut self) {
        self.backing = None;
    }

    /// Borrow the original root without manufacturing a native reference.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn original_root(&self) -> &Value {
        self.root.as_ref().expect("live native search root")
    }
}

impl Drop for NativeDictionarySearch {
    fn drop(&mut self) {
        // Native iterator destruction closes its DictSearch before releasing
        // the original root object, including empty and exhausted searches.
        self.close_search();
        drop(self.root.take());
    }
}

impl PreparedNativeDictionary {
    pub(crate) fn invalidate_string(&self) {
        *self.value.0.string.borrow_mut() = None;
        self.value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.value.0.double_format.borrow_mut() = None;
        *self.value.0.source_location.borrow_mut() = None;
    }

    pub(crate) fn original(&self) -> &Value {
        &self.value
    }

    pub(crate) fn string_protocol(&self) -> NativeStringProtocol {
        self.protocol
    }

    pub(crate) fn is_same_object(&self, value: &Value) -> bool {
        Rc::ptr_eq(&self.value.0, &value.0)
    }

    pub(crate) fn duplicate_value(&self, value: &Value) -> Value {
        value.duplicate_native_object_in(self.protocol)
    }

    pub(crate) fn with_member<R>(
        &self,
        key: &Value,
        operation: impl FnOnce(Option<&Value>) -> R,
    ) -> Result<R, tcl_syntax::value::ValueError> {
        let bytes = key.native_string_bytes(self.protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary key updater",
            )
        })?;
        Ok(self
            .value
            .with_cached_dictionary_member(&bytes, operation)
            .expect("prepared Dictionary primary cache"))
    }

    pub(crate) fn set_member(
        &mut self,
        key: Value,
        value: Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.value
            .set_native_dictionary_member_in_place(key, value, self.protocol)
    }

    pub(crate) fn set_member_checked(
        &mut self,
        key: Value,
        value: Value,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.value.set_native_dictionary_member_using(
            key,
            value,
            self.protocol,
            &mut Some(check),
            &mut Some(strings),
        )
    }

    pub(crate) fn remove_member(
        &mut self,
        key: &Value,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let bytes = key.native_string_bytes(self.protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary key updater",
            )
        })?;
        let cache = self.value.0.intrep.borrow();
        let IntRep::Dict(dict) = &*cache else {
            unreachable!("prepared Dictionary cache");
        };
        let mut contents = dict.contents.borrow_mut();
        let Some(index) = contents
            .pairs
            .iter()
            .position(|(name, _)| name.resident_string_bytes().is_some_and(|key| key == bytes))
        else {
            return Ok(false);
        };
        let epoch = dict.next_epoch()?;
        Rc::make_mut(&mut contents.pairs).remove(index);
        contents.hash_order.remove(&bytes);
        dict.epoch.set(epoch);
        *self.value.0.string.borrow_mut() = None;
        self.value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.value.0.double_format.borrow_mut() = None;
        *self.value.0.source_location.borrow_mut() = None;
        Ok(true)
    }

    pub(crate) fn into_value(self) -> Value {
        self.value
    }
}

impl Value {
    /// Inspect original Dictionary members without list conversion or string generation.
    #[must_use]
    pub fn cached_dictionary_representation(&self) -> Option<Vec<(Value, Value)>> {
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Some(dict.with_pairs(<[(Value, Value)]>::to_vec)),
            _ => None,
        }
    }

    /// Reconstruct a Dictionary cache from explicitly transported original members.
    #[must_use]
    pub fn from_native_dictionary_cache(pairs: Vec<(Value, Value)>) -> Self {
        Self::dict(pairs)
    }

    /// Borrow cached original Dictionary storage before retaining child capability handles.
    #[must_use]
    pub fn with_cached_dictionary_representation<R>(
        &self,
        operation: impl FnOnce(&[(Value, Value)], usize) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => {
                Some(dict.with_pairs(|pairs| operation(pairs, dict.bucket_count())))
            }
            _ => None,
        }
    }

    /// Inspect original dictionary bucket capacity without conversion or string generation.
    #[must_use]
    pub fn cached_dictionary_bucket_count(&self) -> Option<usize> {
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Some(dict.bucket_count()),
            _ => None,
        }
    }

    /// Reconstruct explicitly transported dictionary members and native bucket capacity.
    #[must_use]
    pub fn from_native_dictionary_cache_with_bucket_count(
        pairs: Vec<(Value, Value)>,
        buckets: Option<usize>,
    ) -> Self {
        Self::dict_with_hash_bucket_count(pairs, buckets)
    }

    /// Retain a dictionary member's existing native header after its view closes.
    pub(crate) fn retain_borrowed_member(original: Option<&Self>) -> Option<Self> {
        original.cloned()
    }

    /// Borrow an original cached dictionary member before acquiring a working value handle.
    pub(crate) fn with_cached_dictionary_member<R>(
        &self,
        key: &[u8],
        operation: impl FnOnce(Option<&Value>) -> R,
    ) -> Option<R> {
        let cache = self.0.intrep.borrow();
        let IntRep::Dict(dict) = &*cache else {
            return None;
        };
        Some(dict.with_pairs(|pairs| {
            let value = pairs
                .iter()
                .find(|(name, _)| {
                    name.resident_string_bytes()
                        .is_some_and(|bytes| bytes.as_ref() == key)
                })
                .map(|(_, value)| value);
            operation(value)
        }))
    }

    /// Prepare from borrowed original storage before retaining a working handle.
    pub(crate) fn prepare_native_dictionary(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<PreparedNativeDictionary, tcl_syntax::value::ValueError> {
        let value = if self.native_object_is_shared() {
            let value = self.duplicate_native_object_in(protocol);
            drop(value.native_object_dict_pairs(protocol)?);
            value
        } else {
            drop(self.native_object_dict_pairs(protocol)?);
            self.clone()
        };
        Ok(PreparedNativeDictionary { value, protocol })
    }

    /// Convert the borrowed original before observing and applying native COW.
    /// Existing path children and integer/body updates retain the conversion on aliases.
    pub(crate) fn prepare_native_dictionary_after_conversion(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<PreparedNativeDictionary, ValueError> {
        self.prepare_native_dictionary_after_conversion_using(protocol, &mut None, &mut None)
    }

    pub(crate) fn prepare_native_dictionary_after_conversion_checked(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<PreparedNativeDictionary, tcl_cmd_core::CmdError> {
        self.prepare_native_dictionary_after_conversion_using(
            protocol,
            &mut Some(check),
            &mut Some(strings),
        )
    }

    fn prepare_native_dictionary_after_conversion_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<PreparedNativeDictionary, E> {
        drop(self.native_object_dict_pairs_using(protocol, check, strings)?);
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let value = if self.native_object_is_shared() {
            self.duplicate_native_object_in(protocol)
        } else {
            self.clone()
        };
        Self::native_stage_check(check)?;
        Ok(PreparedNativeDictionary { value, protocol })
    }

    /// Integer updates duplicate a shared C Dictionary without its resident string.
    pub(crate) fn prepare_native_dictionary_for_increment(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<PreparedNativeDictionary, tcl_syntax::value::ValueError> {
        let shared = self.native_object_is_shared();
        let prepared = self.prepare_native_dictionary_after_conversion(protocol)?;
        if shared && protocol.tcl_version().is_some() {
            *prepared.value.0.string.borrow_mut() = None;
            prepared
                .value
                .0
                .string_storage
                .set(NativeStringStorageIdentity::Unknown);
        }
        Ok(prepared)
    }

    /// Update a dictionary member from a borrowed original root, applying native COW first.
    pub(crate) fn native_dictionary_set_member(
        &self,
        key: Value,
        value: Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        let mut prepared = self.prepare_native_dictionary(protocol)?;
        prepared.set_member(key, value)?;
        Ok(prepared.into_value())
    }

    pub(super) fn set_native_dictionary_member_in_place(
        &self,
        key: Value,
        value: Value,
        protocol: NativeStringProtocol,
    ) -> Result<(), ValueError> {
        self.set_native_dictionary_member_using(key, value, protocol, &mut None, &mut None)
    }

    fn set_native_dictionary_member_using<E: From<ValueError>>(
        &self,
        key: Value,
        value: Value,
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        key.check_native_header()?;
        let reached = key.native_compound_string_using(protocol, strings);
        Self::native_stage_check(check)?;
        let key_bytes = reached?;
        value.check_native_header()?;
        if !matches!(*self.0.intrep.borrow(), IntRep::Dict(_)) {
            drop(self.native_object_dict_pairs_using(protocol, check, strings)?);
        }
        let dict = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Rc::clone(dict),
            _ => unreachable!("native dict conversion"),
        };
        let epoch = dict.next_epoch()?;
        let mut contents = dict.contents.borrow_mut();
        let existing = contents.pairs.iter().position(|(name, _)| {
            name.resident_string_bytes()
                .is_some_and(|bytes| bytes == key_bytes)
        });
        let retired = if let Some(index) = existing {
            Some(std::mem::replace(
                &mut Rc::make_mut(&mut contents.pairs)[index].1,
                value.into_native_reference(),
            ))
        } else {
            contents.hash_order.insert(&key_bytes);
            // Dictionary storage owns each original child independently of
            // borrowed invocation/search transport leases, just as DictRep::new does.
            Rc::make_mut(&mut contents.pairs)
                .push((key.into_native_reference(), value.into_native_reference()));
            None
        };
        dict.epoch.set(epoch);
        drop(contents);
        drop(dict);
        drop(retired);
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.double_format.borrow_mut() = None;
        *self.0.source_location.borrow_mut() = None;
        Ok(())
    }

    /// Transfer an original Dictionary into an actual search backing lease.
    pub(crate) fn into_native_dictionary_search(
        self,
        protocol: NativeStringProtocol,
    ) -> Result<NativeDictionarySearch, tcl_syntax::value::ValueError> {
        self.into_native_dictionary_search_using(protocol, &mut None, &mut None)
    }

    pub(crate) fn into_native_dictionary_search_with_strings(
        self,
        protocol: NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<NativeDictionarySearch, tcl_cmd_core::CmdError> {
        self.into_native_dictionary_search_using(protocol, &mut Some(check), &mut Some(strings))
    }

    fn into_native_dictionary_search_using<E: From<ValueError>>(
        self,
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<NativeDictionarySearch, E> {
        Self::native_stage_check(check)?;
        if check.is_some() {
            self.check_native_header()?;
        }
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V8_5)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native C Dictionary search",
            )
            .into());
        }
        if !matches!(*self.0.intrep.borrow(), IntRep::Dict(_)) {
            drop(self.native_object_dict_pairs_using(protocol, check, strings)?);
            Self::native_stage_check(check)?;
            if check.is_some() {
                self.check_native_header()?;
            }
        }
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                tcl_syntax::value::ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        let backing = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) if dict.with_pairs(|pairs| !pairs.is_empty()) => {
                Some(Rc::clone(dict))
            }
            IntRep::Dict(_) => None,
            _ => unreachable!("native Dictionary search conversion"),
        };
        let epoch = backing.as_ref().map_or(0, |dict| dict.epoch.get());
        let search = NativeDictionarySearch {
            root: Some(self),
            backing,
            next: 0,
            epoch,
        };
        Self::native_stage_check(check)?;
        Ok(search)
    }

    /// Convert through the actual native object-list recipe, retaining original member values.
    ///
    /// # Errors
    /// Preserves checked storage refusal and native dictionary parse failures.
    pub fn native_object_dict_pairs(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Vec<(Value, Value)>, ValueError> {
        self.native_object_dict_pairs_using(protocol, &mut None, &mut None)
    }

    pub(crate) fn native_object_dict_pairs_with_strings(
        &self,
        protocol: NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        strings: &mut NativeStringAccess<'_, tcl_cmd_core::CmdError>,
    ) -> Result<Vec<(Value, Value)>, tcl_cmd_core::CmdError> {
        self.native_object_dict_pairs_using(protocol, &mut Some(check), &mut Some(strings))
    }

    pub(crate) fn original_dictionary_cursor(
        &self,
    ) -> Result<OriginalNativeDictionaryCursor, ValueError> {
        self.check_native_header()?;
        let cache = self.0.intrep.borrow();
        let IntRep::Dict(actual) = &*cache else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Dictionary lookup table",
            ));
        };
        Ok(OriginalNativeDictionaryCursor {
            backing: Rc::downgrade(actual),
            epoch: actual.epoch.get(),
        })
    }

    fn native_compound_string_using<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<Rc<[u8]>, E> {
        match strings.as_deref_mut() {
            Some(strings) => strings(self),
            None => self
                .native_string_bytes(protocol)
                .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                .map_err(ValueError::from)
                .map_err(E::from),
        }
    }

    fn native_object_dict_pairs_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<Vec<(Value, Value)>, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        if let Some(pairs) = self.cached_dictionary_representation() {
            return Ok(pairs);
        }
        let cached_list = self.cached_list_representation();
        if protocol.is_jim084() && cached_list.is_some() && self.native_object_is_shared() {
            let reached = self.native_compound_string_using(protocol, strings);
            Self::native_stage_check(check)?;
            reached?;
        }
        let items = match cached_list.as_ref() {
            Some((items, _)) => items.lifetime_view(),
            None if protocol.is_jim084() => {
                self.native_object_list_elements_using(protocol, check, strings)?
            }
            None => {
                let reached = self.native_compound_string_using(protocol, strings);
                Self::native_stage_check(check)?;
                let bytes = reached?;
                let parsed = tcl_syntax::list::split_native_list_bytes(&bytes, protocol).map_err(
                    |error| ValueError::DictionaryParse {
                        error,
                        source: bytes.to_vec(),
                    },
                )?;
                crate::NativeListItems::new(
                    parsed
                        .into_iter()
                        .map(|element| Value::new_native_string_bytes(element.into_owned()))
                        .collect(),
                    false,
                )
            }
        };
        if items.len() % 2 != 0 {
            return Err(ValueError::MissingDictionaryValue.into());
        }
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = false;
        for pair in items.as_chunks::<2>().0 {
            if check.is_some() {
                items.check_generation()?;
            }
            pair[0].check_native_header()?;
            let reached = pair[0].native_compound_string_using(protocol, strings);
            Self::native_stage_check(check)?;
            let bytes = reached?;
            duplicates |= !seen.insert(bytes);
        }
        if !protocol.is_jim084() && cached_list.is_some() && duplicates {
            let reached = self.native_compound_string_using(protocol, strings);
            Self::native_stage_check(check)?;
            reached?;
        }
        let selected_items = if check.is_some() {
            items.elements()?
        } else {
            &items
        };
        let (primary, pairs) = self.dictionary_primary(selected_items);
        self.replace_primary_using(primary, check)?;
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        Ok(pairs)
    }

    /// A native dictionary value from canonical key/value pairs.
    #[must_use]
    pub(crate) fn dict(pairs: Vec<(Value, Value)>) -> Self {
        Self::dict_with_hash_bucket_count(pairs, None)
    }

    /// A native dictionary retaining the source table's bucket-array size.
    #[must_use]
    pub(crate) fn dict_with_hash_bucket_count(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
    ) -> Self {
        let mut hash_order = TclStringHashOrder::default();
        if let Some(bucket_count) = bucket_count {
            hash_order.retain_bucket_count(bucket_count);
        }
        for (key, _) in &pairs {
            hash_order.insert(&key.string_bytes());
        }
        Self::from_parts(
            None,
            IntRep::Dict(Rc::new(DictRep::new(pairs, hash_order, None))),
        )
    }

    /// Construct a dictionary from original members under the actual string recipe.
    /// Key conversion reaches each original object before allocating its hash entry.
    ///
    /// # Errors
    /// Returns a checked native key materialisation refusal.
    pub fn native_dictionary_constructor(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        Self::native_dictionary_constructor_using(
            pairs,
            bucket_count,
            protocol,
            &mut (),
            &mut |(), actual| {
                actual.native_string_bytes(protocol).map_err(|error| {
                    ValueError::NativeStringAccess(
                        tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                    )
                })
            },
            &mut |()| Ok(()),
        )
    }

    pub(crate) fn native_dictionary_constructor_checked<C>(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
        protocol: NativeStringProtocol,
        context: &mut C,
        string: &mut impl FnMut(&mut C, &Value) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError>,
        check: &mut impl FnMut(&mut C) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        Self::native_dictionary_constructor_using(
            pairs,
            bucket_count,
            protocol,
            context,
            string,
            check,
        )
    }

    fn native_dictionary_constructor_using<C, E: From<ValueError>>(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
        protocol: NativeStringProtocol,
        context: &mut C,
        string: &mut impl FnMut(&mut C, &Value) -> Result<Rc<[u8]>, E>,
        check: &mut impl FnMut(&mut C) -> Result<(), E>,
    ) -> Result<Self, E> {
        let mut hash_order = TclStringHashOrder::default();
        if let Some(bucket_count) = bucket_count {
            hash_order.retain_bucket_count(bucket_count);
        }
        let mut keys = Vec::with_capacity(pairs.len());
        for (key, _) in &pairs {
            check(context)?;
            key.check_native_header()?;
            let reached = string(context, key);
            check(context)?;
            let bytes = reached?;
            key.check_native_header()?;
            hash_order.insert(&bytes);
            keys.push(bytes);
        }
        check(context)?;
        let mut original_pairs: Vec<_> = pairs.into_iter().map(Some).collect();
        let pairs = canonical_dict_slots(keys.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|(key, value)| {
                let first = original_pairs[key].take().expect("original dictionary key");
                if key == value {
                    first
                } else {
                    let last = original_pairs[value]
                        .take()
                        .expect("original dictionary value");
                    (first.0, last.1)
                }
            })
            .collect();
        let value = Self::from_parts(
            None,
            IntRep::Dict(Rc::new(DictRep::new(pairs, hash_order, Some(protocol)))),
        );
        check(context)?;
        Ok(value)
    }

    pub(crate) fn native_dict_pairs(
        &self,
        syntax: tcl_dialect::ListParse,
        escapes: tcl_dialect::EscapeSyntax,
    ) -> Result<Vec<(Value, Value)>, tcl_syntax::value::ValueError> {
        if let IntRep::Dict(dict) = &*self.0.intrep.borrow() {
            return Ok(dict.with_pairs(<[(Value, Value)]>::to_vec));
        }
        let items = self
            .native_list_elements(syntax, escapes)
            .map_err(|error| tcl_syntax::value::ValueError::DictionaryParse {
                error,
                source: self.string_bytes().to_vec(),
            })?;
        if items.len() % 2 != 0 {
            return Err(tcl_syntax::value::ValueError::MissingDictionaryValue);
        }
        Ok(self.install_dictionary(&items))
    }

    fn install_dictionary(&self, items: &[Value]) -> Vec<(Value, Value)> {
        let (primary, pairs) = self.dictionary_primary(items);
        self.replace_primary(primary);
        pairs
    }

    fn dictionary_primary(&self, items: &[Value]) -> (IntRep, Vec<(Value, Value)>) {
        let string_protocol = match &*self.0.intrep.borrow() {
            IntRep::List {
                string_protocol, ..
            } => string_protocol.get(),
            IntRep::Dict(dict) => dict.string_protocol.get(),
            _ => None,
        };
        let keys: Vec<Rc<[u8]>> = items
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| chunk[0].string_bytes())
            .collect();
        let pairs: Vec<(Value, Value)> = canonical_dict_slots(keys.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|(key_slot, value_slot)| {
                (
                    items[key_slot * 2].clone(),
                    items[value_slot * 2 + 1].clone(),
                )
            })
            .collect();
        let mut hash_order = TclStringHashOrder::default();
        for (key, _) in &pairs {
            hash_order.insert(&key.string_bytes());
        }
        (
            IntRep::Dict(Rc::new(DictRep::new(
                pairs.clone(),
                hash_order,
                string_protocol,
            ))),
            pairs,
        )
    }

    /// Retained bucket count of the native dictionary representation.
    pub(crate) fn dict_hash_bucket_count(&self) -> Result<usize, tcl_syntax::value::ValueError> {
        self.native_dict_pairs(
            tcl_dialect::ListParse::Strict,
            tcl_dialect::EscapeSyntax::default(),
        )?;
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Ok(dict.bucket_count()),
            _ => unreachable!("dict_pairs installs the dictionary representation"),
        }
    }
}
