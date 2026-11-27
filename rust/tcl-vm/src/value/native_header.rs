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

//! The native object header behind a value: liveness, lifetime pins and references, adopting a bridge mirror, and duplication.

#[cfg(test)]
use super::observe_native_retirement;
use super::{
    DictRep, IntRep, NativeObjectLifetimeLease, NativeStringAccess, NativeValueHandleOwnership,
    ObjectRepresentations, Value, native_jim_pool, native_retirement,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::raw_string::RawString;
use tcl_syntax::value::ValueError;

impl Value {
    /// Whether the same original header still has native storage lifetime.
    #[must_use]
    pub fn native_object_is_live(&self) -> bool {
        !self.0.retired.get()
    }

    pub(crate) fn check_native_header(&self) -> Result<(), tcl_syntax::value::ValueError> {
        self.original_native_header_currentness()
            .map_err(Into::into)
    }

    pub(crate) fn original_native_header_currentness(
        &self,
    ) -> Result<(), tcl_syntax::raw_string::NativeValueAccessRefusal> {
        if self.native_object_is_live() {
            Ok(())
        } else {
            Err(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object header",
                ),
            )
        }
    }

    pub(super) fn retire_native_header(&self) {
        if self.0.retired.replace(true) {
            return;
        }
        #[cfg(test)]
        observe_native_retirement(self);
        // Detach everything before releasing children or filename owners.
        let primary = self.0.intrep.replace(IntRep::Str);
        let string = self.0.string.replace(None);
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        let format = self.0.double_format.replace(None);
        let location = self.0.source_location.replace(None);
        let context = self.0.jim_context.replace(None);
        native_retirement::release(native_retirement::RetiredNativeHeader {
            primary,
            string,
            format,
            location,
            context,
        });
        let allocation = self.0.jim_allocation.borrow_mut().take();
        if let Some(allocation) = allocation {
            allocation.release();
        }
    }

    /// Ordinary native release, distinct from Subst's deliberate no-free undo.
    pub(crate) fn retire_unowned_native_header(&self) {
        if Rc::strong_count(&self.0) == self.0.lifetime_pins.get() {
            self.retire_native_header();
        }
    }

    pub(crate) fn retain_native_lifetime_pin(&self) {
        self.0.lifetime_pins.set(
            self.0
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("native lifetime pins exhausted"),
        );
    }

    pub(crate) fn release_native_lifetime_pin(&self) {
        self.0.lifetime_pins.set(
            self.0
                .lifetime_pins
                .get()
                .checked_sub(1)
                .expect("native lifetime pin underflow"),
        );
    }

    /// Retain the actual header while keeping transport ownership out of Tcl sharing.
    #[must_use]
    pub fn native_lifetime_lease(&self) -> NativeObjectLifetimeLease {
        self.retain_native_lifetime_pin();
        NativeObjectLifetimeLease(Some(self.clone()))
    }

    /// Preserve an allocating producer's live rc0 header in lifetime-only transport.
    /// This consumes the actual owner without running ordinary last-owner release.
    pub(crate) fn into_native_unowned_lifetime(mut self) -> Self {
        if self.1 == NativeValueHandleOwnership::Reference {
            self.retain_native_lifetime_pin();
            self.1 = NativeValueHandleOwnership::LifetimeOnly;
        }
        self
    }

    /// Retain a completion transport at an actual native ownership boundary.
    #[must_use]
    pub fn into_native_reference(mut self) -> Self {
        if self.1 == NativeValueHandleOwnership::LifetimeOnly {
            self.release_native_lifetime_pin();
            self.1 = NativeValueHandleOwnership::Reference;
        }
        self
    }

    /// Reset an unshared C interpreter result on its same original header.
    pub(crate) fn reset_native_c_result(&self) {
        debug_assert!(!self.native_object_is_shared());
        *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::<[u8]>::from([])));
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::CanonicalEmpty);
        self.replace_primary(IntRep::Str);
    }

    /// Materialise the C8.5 standalone ensemble namespace legacy string result
    /// on its reset unshared original result header, without resetting error state.
    pub(crate) fn initialise_original_ensemble_namespace_result(
        &self,
        original: &crate::interp::OriginalEnsembleNamespaceResult<'_>,
        check: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        check()?;
        original.bytes_for(self)?;
        *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::<[u8]>::from([])));
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::CanonicalEmpty);
        self.replace_primary_using(IntRep::Str, &mut Some(&mut *check))?;
        let bytes = Rc::<[u8]>::from(original.bytes_for(self)?);
        *self.0.string.borrow_mut() = Some(RawString::from_bytes(bytes));
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Allocated);
        check()
    }

    /// Opaque identity of this retained original object, for equality and lifetime tracking.
    /// It supplies neither a native pointer ABI nor execution authority.
    #[must_use]
    pub fn native_object_identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    /// Inspect original sharing before acquiring bridge or working-handle references.
    /// Callers must capture this fact before retaining their capability clone.
    #[must_use]
    pub fn native_object_is_shared(&self) -> bool {
        Rc::strong_count(&self.0).saturating_sub(self.0.lifetime_pins.get()) > 1
    }

    /// Modelled ownership references, excluding lifetime-only bridge pins.
    /// This observation supplies no object-liveness or invocation authority.
    #[must_use]
    pub fn native_object_reference_count(&self) -> usize {
        Rc::strong_count(&self.0).saturating_sub(self.0.lifetime_pins.get())
    }

    /// Apply a checked bridge mirror to this same original physical object.
    /// Getter failure does not suppress reached cache/string effects; aliases
    /// retain this object identity. This operation deliberately performs no COW.
    ///
    /// # Errors
    /// Refuses unknown engines, foreign cache origins, or unsupported opaque caches.
    pub fn adopt_native_object_representation(
        &self,
        donor: &Value,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.adopt_native_object_representation_with_string_mutation(
            donor,
            dialect,
            tcl_core_types::ResidentStringMutation::Replace,
        )
    }

    /// Apply a checked mirror using its authenticated resident-allocation receipt.
    /// Preserve retains this object's allocation even when its primary cache changes.
    /// Replace adopts the donor allocation, including equal-content replacements.
    ///
    /// # Errors
    /// Refuses inconsistent receipts or unsupported native cache origins before mutation.
    pub fn adopt_native_object_representation_with_string_mutation(
        &self,
        donor: &Value,
        dialect: tcl_native_model::InvocationDialect,
        string_mutation: tcl_core_types::ResidentStringMutation,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.adopt_native_object_representation_with_string_mutation_using::<ValueError>(
            donor,
            dialect,
            string_mutation,
            &mut None,
        )
    }

    pub(crate) fn adopt_native_object_representation_with_string_mutation_checked<
        E: From<ValueError>,
    >(
        &self,
        donor: &Value,
        dialect: tcl_native_model::InvocationDialect,
        string_mutation: tcl_core_types::ResidentStringMutation,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.adopt_native_object_representation_with_string_mutation_using(
            donor,
            dialect,
            string_mutation,
            &mut Some(check),
        )
    }

    fn adopt_native_object_representation_with_string_mutation_using<E: From<ValueError>>(
        &self,
        donor: &Value,
        dialect: tcl_native_model::InvocationDialect,
        string_mutation: tcl_core_types::ResidentStringMutation,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        donor.check_native_header()?;
        let protocol =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native object mirror issuer",
                ))?;
        donor.validate_native_mirror_primary(protocol)?;
        let string = match string_mutation {
            tcl_core_types::ResidentStringMutation::Preserve => {
                if self.resident_string_bytes() != donor.resident_string_bytes()
                    || self.0.string_storage.get() != donor.0.string_storage.get()
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror preserved string receipt",
                    )
                    .into());
                }
                self.0.string.borrow().clone()
            }
            tcl_core_types::ResidentStringMutation::Replace => donor.0.string.borrow().clone(),
            tcl_core_types::ResidentStringMutation::Discard => {
                if donor.0.string.borrow().is_some() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror discarded string receipt",
                    )
                    .into());
                }
                None
            }
        };
        let cache = donor.0.intrep.borrow().clone();
        let format = donor.0.double_format.borrow().clone();
        if check.is_none() {
            *self.0.string.borrow_mut() = string;
            self.0.string_storage.set(donor.0.string_storage.get());
            *self.0.intrep.borrow_mut() = cache;
            *self.0.double_format.borrow_mut() = format;
            return Ok(());
        }
        let storage = donor.0.string_storage.get();
        self.replace_primary_using_with(
            || {
                donor.check_native_header()?;
                donor.validate_native_mirror_primary(protocol)?;
                if string_mutation == tcl_core_types::ResidentStringMutation::Preserve
                    && (self.resident_string_bytes() != donor.resident_string_bytes()
                        || self.0.string_storage.get() != storage)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror preserved string receipt",
                    )
                    .into());
                }
                *self.0.string.borrow_mut() = string;
                self.0.string_storage.set(storage);
                Ok(cache)
            },
            check,
        )?;
        let retired_format = self.0.double_format.replace(format);
        drop(retired_format);
        Self::native_stage_check(check)
    }

    fn validate_native_mirror_primary(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        {
            let cache = self.0.intrep.borrow();
            match &*cache {
                IntRep::NativeString {
                    protocol: origin, ..
                } if *origin != protocol => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror String origin",
                    ));
                }
                IntRep::JimString(_) if !protocol.is_jim084() => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror Jim String origin",
                    ));
                }
                IntRep::WordBoolean { version, .. }
                    if protocol != NativeStringProtocol::C(*version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror Boolean origin",
                    ));
                }
                IntRep::NativeIndex { version, .. }
                    if protocol != NativeStringProtocol::C(*version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror Index origin",
                    ));
                }
                IntRep::NativeCommandName(cache)
                    if protocol != NativeStringProtocol::C(cache.version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror command-name origin",
                    ));
                }
                IntRep::NativeNamespaceName(cache)
                    if protocol != NativeStringProtocol::C(cache.version())
                        || (!tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(cache.version()).has_string_updater()
                            && self.resident_string_bytes().is_none()) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native namespace-name mirror origin or storage",
                    ));
                }
                IntRep::NativeCommandNameUnresolved(version)
                    if protocol != NativeStringProtocol::C(*version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native unresolved command-name mirror origin",
                    ));
                }
                IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_)
                    if self.resident_string_bytes().is_none() =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native command-name mirror resident storage",
                    ));
                }
                IntRep::JimCommand(_) | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
                | IntRep::JimSubcommand(_)
                | IntRep::NativeParsedVariableName(_)
                | IntRep::NativeLocalVariableName(_)
                | IntRep::JimSource(_) | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
                | IntRep::JimInterpolated(_)
                | IntRep::JimScriptLine { .. }
                | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
                | IntRep::CompletionCode(_)
                | IntRep::NativeArraySearch { .. }
                | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_) => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror opaque primary cache",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Duplicate under the selected native object recipe without string generation.
    #[must_use]
    pub fn duplicate_native_object_in(&self, protocol: NativeStringProtocol) -> Self {
        self.duplicate_native_object_using::<ValueError>(
            protocol,
            &mut None,
            &mut None,
            &mut |_| {},
        )
        .expect("neutral duplicate has no checked stage")
    }

    /// The same original duplication with a retained actual operation and
    /// node-aware String producer. Source context is captured before any reached
    /// callback; a refused fresh result is retired before returning the first cause.
    pub(crate) fn duplicate_native_object_checked<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        check: &mut dyn FnMut() -> Result<(), E>,
        strings: &mut NativeStringAccess<'_, E>,
        retain: &mut dyn FnMut(&E),
    ) -> Result<Self, E> {
        self.duplicate_native_object_using(protocol, &mut Some(check), &mut Some(strings), retain)
    }

    fn duplicate_native_object_using<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
        retain: &mut dyn FnMut(&E),
    ) -> Result<Self, E> {
        Self::native_stage_check(check)?;
        if check.is_some() {
            self.check_native_header()?;
        }
        let source_context = self.0.jim_context.borrow().clone();
        let finish = |duplicate: Self,
                      check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
                      retain: &mut dyn FnMut(&E)|
         -> Result<Self, E> {
            let reached = Self::native_stage_check(check).and_then(|()| {
                if check.is_some() {
                    self.check_native_header()?;
                    duplicate.check_native_header()?;
                }
                Ok(())
            });
            if let Err(first) = reached {
                retain(&first);
                drop(duplicate);
                return Err(first);
            }
            if check.is_some() {
                duplicate
                    .0
                    .jim_context
                    .borrow_mut()
                    .clone_from(&source_context);
            }
            Ok(duplicate)
        };
        if protocol.object_header_duplicate_action(
            self.resident_string_bytes()
                .as_ref()
                .map(|bytes| bytes.len()),
        ) == tcl_syntax::native_string::NativeObjectHeaderDuplicateAction::CanonicalEmptyString
        {
            let duplicate = Self::new_native_string_bytes(b"".as_slice());
            duplicate
                .0
                .jim_context
                .borrow_mut()
                .clone_from(&self.0.jim_context.borrow());
            return finish(duplicate, check, retain);
        }
        if protocol.is_jim084()
            && matches!(
                &*self.0.intrep.borrow(),
                IntRep::Expression(_) | IntRep::JimScript(_)
            )
        {
            let resident = self
                .resident_string_bytes()
                .expect("native parsed source retains resident bytes");
            let duplicate = Self::new_native_string_bytes(resident.as_ref());
            duplicate
                .0
                .jim_context
                .borrow_mut()
                .clone_from(&self.0.jim_context.borrow());
            return finish(duplicate, check, retain);
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeBytecode(_)) {
            let duplicate = Self::new_native_string_bytes(
                self.resident_string_bytes()
                    .expect("native bytecode requires resident source")
                    .as_ref(),
            );
            return finish(duplicate, check, retain);
        }
        let duplicate = self.duplicate_native_object();
        if let IntRep::NativeString {
            protocol,
            num_chars,
            ..
        } = &*self.0.intrep.borrow()
            && !protocol.string_primary_survives_duplicate(*num_chars)
        {
            *duplicate.0.intrep.borrow_mut() = IntRep::Str;
            return finish(duplicate, check, retain);
        }
        self.copy_native_variable_primary_to(&duplicate);
        let duplicate =
            self.copy_native_duplicate_primary_using(duplicate, protocol, check, strings, retain)?;
        finish(duplicate, check, retain)
    }

    fn copy_native_duplicate_primary_using<E: From<ValueError>>(
        &self,
        duplicate: Self,
        protocol: NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
        retain: &mut dyn FnMut(&E),
    ) -> Result<Self, E> {
        // Neutral duplication retains its eager member snapshot. Checked
        // duplication captures allocation-only Dictionary identity/currentness;
        // each genuine target member hold follows its reached key insertion.
        let dictionary = {
            let cache = self.0.intrep.borrow();
            if let IntRep::Dict(dict) = &*cache {
                Some((
                    check
                        .is_none()
                        .then(|| dict.with_pairs(<[(Value, Value)]>::to_vec)),
                    if protocol.is_jim084() {
                        dict.contents.borrow().hash_order.clone()
                    } else {
                        TclStringHashOrder::default()
                    },
                    dict.string_protocol.get(),
                    Rc::downgrade(dict),
                    dict.epoch.get(),
                    dict.with_pairs(<[(Value, Value)]>::len),
                ))
            } else {
                None
            }
        };
        if let Some((
            neutral_pairs,
            mut hash_order,
            dictionary_protocol,
            original_dictionary,
            original_epoch,
            original_length,
        )) = dictionary
        {
            let mut pairs = neutral_pairs.unwrap_or_default();
            if check.is_some() {
                // The temporary shallow copy is not a target Dictionary owner.
                // Release its backing lease before any reached key getter.
                *duplicate.0.intrep.borrow_mut() = IntRep::Str;
            }
            let dictionary_current = || -> Result<Rc<DictRep>, E> {
                self.check_native_header()?;
                let primary = self.0.intrep.borrow();
                if let IntRep::Dict(dict) = &*primary
                    && std::rc::Weak::ptr_eq(&Rc::downgrade(dict), &original_dictionary)
                    && dict.epoch.get() == original_epoch
                    && dict.with_pairs(<[(Value, Value)]>::len) == original_length
                {
                    return Ok(Rc::clone(dict));
                }
                Err(ValueError::CommandProtocolUnavailable(
                    "changed original duplicate Dictionary primary",
                )
                .into())
            };
            let reached = Self::copy_native_dictionary_members_using(
                protocol,
                original_length,
                &mut pairs,
                &mut hash_order,
                &dictionary_current,
                check,
                strings,
            );
            if let Err(first) = reached {
                retain(&first);
                drop(pairs);
                drop(duplicate);
                return Err(first);
            }
            *duplicate.0.intrep.borrow_mut() = IntRep::Dict(Rc::new(DictRep::new(
                pairs,
                hash_order,
                dictionary_protocol,
            )));
        } else {
            let cache = self.0.intrep.borrow();
            match &*cache {
                IntRep::ByteArray(backing) => {
                    *duplicate.0.intrep.borrow_mut() =
                        IntRep::ByteArray(Rc::new((**backing).clone()));
                }
                IntRep::List {
                    items,
                    string_protocol,
                    ..
                } if protocol.is_jim084() => {
                    *duplicate.0.intrep.borrow_mut() =
                        IntRep::native_list(items.as_ref().clone(), false, string_protocol.get());
                }
                _ => {}
            }
        }
        Ok(duplicate)
    }

    fn copy_native_dictionary_members_using<E: From<ValueError>>(
        protocol: NativeStringProtocol,
        original_length: usize,
        pairs: &mut Vec<(Value, Value)>,
        hash_order: &mut TclStringHashOrder,
        dictionary_current: &dyn Fn() -> Result<Rc<DictRep>, E>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        strings: &mut Option<&mut NativeStringAccess<'_, E>>,
    ) -> Result<(), E> {
        if check.is_some() {
            for index in 0..original_length {
                Self::native_stage_check(check)?;
                let key = dictionary_current()?
                    .with_pairs(|original| original[index].0.native_lifetime_lease().into_value());
                if !protocol.is_jim084() {
                    let bytes = match strings {
                        Some(strings) => strings(&key)?,
                        None => unreachable!("checked duplicate requires original String producer"),
                    };
                    Self::native_stage_check(check)?;
                    // The exact original table/epoch is verified before
                    // native key/value references enter the target.
                    drop(dictionary_current()?);
                    hash_order.insert(&bytes);
                }
                Self::native_stage_check(check)?;
                let pair = dictionary_current()?.with_pairs(|original| {
                    (key.into_native_reference(), original[index].1.clone())
                });
                pairs.push(pair);
            }
            Self::native_stage_check(check)?;
            drop(dictionary_current()?);
        } else if !protocol.is_jim084() {
            for (key, _) in pairs.iter() {
                hash_order.insert(&key.string_bytes());
            }
        }
        Ok::<(), E>(())
    }

    /// Duplicate physical representations without reaching their string updater.
    pub(crate) fn duplicate_native_object(&self) -> Self {
        Self(
            native_jim_pool::construct(ObjectRepresentations {
                string: RefCell::new(self.0.string.borrow().clone()),
                string_storage: Cell::new(self.0.string_storage.get()),
                intrep: RefCell::new(match &*self.0.intrep.borrow() {
                    IntRep::JimRegexp(cache) => IntRep::JimRegexp(cache.duplicate()),
                    primary => primary.clone(),
                }),
                double_format: RefCell::new(self.0.double_format.borrow().clone()),
                source_location: RefCell::new(self.0.source_location.borrow().clone()),
                jim_context: RefCell::new(self.0.jim_context.borrow().clone()),
            }),
            NativeValueHandleOwnership::Reference,
        )
    }

    /// Duplicate original representations into this private receiver through the
    /// shared checked primary retirement owner. Capture every source fact before
    /// retiring the receiver; allocation handles grant no native reference.
    pub(crate) fn native_duplicate_into_checked<E: From<ValueError>>(
        &self,
        source: &Value,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        check()?;
        self.check_native_header()?;
        source.check_native_header()?;
        let string = source.0.string.borrow().clone();
        let storage = source.0.string_storage.get();
        let cache = {
            let original = source.0.intrep.borrow();
            match &*original {
                IntRep::NativeString {
                    protocol,
                    num_chars,
                    ..
                } if !protocol.string_primary_survives_duplicate(*num_chars) => IntRep::Str,
                IntRep::JimRegexp(cache) => IntRep::JimRegexp(cache.duplicate()),
                _ => original.clone(),
            }
        };
        let format = source.0.double_format.borrow().clone();
        let location = source.0.source_location.borrow().clone();
        let context = source.0.jim_context.borrow().clone();
        check()?;
        source.check_native_header()?;
        self.replace_primary_using(cache, &mut Some(&mut *check))?;
        *self.0.string.borrow_mut() = string;
        self.0.string_storage.set(storage);
        *self.0.double_format.borrow_mut() = format;
        *self.0.source_location.borrow_mut() = location;
        *self.0.jim_context.borrow_mut() = context;
        check()
    }
}
