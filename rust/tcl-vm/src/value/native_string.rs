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

//! The native String representation: resident bytes and their storage, the string updater that materialises them, character counts and Unicode units.

use super::native_compound::{CompoundStringBacking, CompoundStringNode};
use super::{
    IntRep, NativeValueCheck, NativeValueHandleOwnership, ObjectRepresentations, Value,
    host_formats_integers, native_jim_pool,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::number::Number;
use tcl_syntax::raw_string::{RawString, UnicodeAccessError};
use tcl_syntax::scalar_getter::NativeScalarCache;
use tcl_syntax::value::ValueError;

impl Value {
    /// Inspect bytes already present on the object without triggering conversion.
    pub(crate) fn existing_string_representation(&self) -> Option<Rc<str>> {
        self.0
            .string
            .borrow()
            .as_ref()
            .and_then(|string| string.unicode().ok())
    }

    /// Inspect exact bytes already resident on the object without conversion.
    #[must_use]
    pub fn resident_string_bytes(&self) -> Option<Rc<[u8]>> {
        self.0.string.borrow().as_ref().map(RawString::bytes)
    }

    /// Test the public native invalidation operation without retiring a primary.
    #[cfg(test)]
    pub(crate) fn invalidate_native_string_for_test(&self) {
        drop(self.0.string.borrow_mut().take());
        drop(self.0.source_location.borrow_mut().take());
    }

    /// Independently retained resident storage identity; absence means no string.
    #[must_use]
    pub fn resident_string_storage_identity(&self) -> Option<NativeStringStorageIdentity> {
        self.0
            .string
            .borrow()
            .is_some()
            .then(|| self.0.string_storage.get())
    }

    /// Supply actual resident bytes and their independently authenticated storage.
    /// This constructs a new object, preserving the original typed backing.
    ///
    /// # Errors
    /// Refuses a canonical-empty receipt attached to nonempty bytes.
    pub fn with_resident_string_bytes_and_storage(
        self,
        bytes: Rc<[u8]>,
        storage: NativeStringStorageIdentity,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if storage == NativeStringStorageIdentity::CanonicalEmpty && !bytes.is_empty() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native string storage identity",
            ));
        }
        let result = self.with_resident_string_bytes(bytes);
        result.0.string_storage.set(storage);
        Ok(result)
    }

    /// Retain a selected recipe on the actual compound backing. Resident
    /// bytes remain authoritative independently of the object's current engine.
    pub(crate) fn seal_compound_string_protocol(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<(), tcl_syntax::native_string::NativeStringUnavailable> {
        let primary = self.0.intrep.borrow();
        let retained = match &*primary {
            IntRep::List {
                string_protocol, ..
            } => string_protocol,
            IntRep::Dict(dict) => &dict.string_protocol,
            _ => return Ok(()),
        };
        if retained.get().is_some_and(|existing| existing != protocol) {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable);
        }
        retained.set(Some(protocol));
        Ok(())
    }

    /// Obtain the native string at an independently selected materialisation
    /// boundary. Existing string bytes are authoritative; a pure byte array
    /// retains its first selected recipe and installs exact native bytes on the
    /// original object without replacing its binary internal representation.
    ///
    /// # Errors
    /// Returns a host refusal for an unaudited native storage/recipe pair.
    pub fn native_string_bytes(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        self.native_string_bytes_with_integer_formatter(protocol, None)
    }

    /// Materialize through an explicitly supplied native-build integer updater.
    /// Resident bytes still bypass the updater. The capability propagates to
    /// actual compound children without taking additional child references.
    pub fn native_string_bytes_with_integer_formatter(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        self.native_string_bytes_using(protocol, formatter, &std::convert::identity, &mut None)
    }

    /// Reach the same String updater with the original caller's currentness check.
    /// Formatter callbacks are checked before consuming their result or publishing bytes.
    #[cfg(test)]
    pub(crate) fn native_string_bytes_checked<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<Rc<[u8]>, E> {
        self.native_string_bytes_checked_for_value(protocol, formatter, &mut |_| check())
    }

    pub(crate) fn native_string_bytes_checked_for_value<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        check: &mut NativeValueCheck<'_, E>,
    ) -> Result<Rc<[u8]>, E> {
        self.native_string_bytes_checked_for_value_using(protocol, formatter, check, None)
    }

    /// Retain a reached intrinsic failure before the original stage is checked.
    /// The caller supplies its existing full-cause owner; no new role is acquired.
    pub(crate) fn native_string_bytes_checked_for_value_retaining<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        check: &mut NativeValueCheck<'_, E>,
        retain: &dyn Fn(&ValueError),
    ) -> Result<Rc<[u8]>, E> {
        self.native_string_bytes_checked_for_value_using(protocol, formatter, check, Some(retain))
    }

    pub(super) fn native_string_bytes_checked_for_value_using<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        check: &mut NativeValueCheck<'_, E>,
        retain: Option<&dyn Fn(&ValueError)>,
    ) -> Result<Rc<[u8]>, E> {
        let mut original = |current: &Value| {
            check(current)?;
            for value in [self, current] {
                if let Err(error) = value.check_native_header() {
                    if let Some(retain) = retain {
                        retain(&error);
                    }
                    return Err(error.into());
                }
            }
            Ok(())
        };
        self.native_string_bytes_using(
            protocol,
            formatter,
            &|error| {
                let error = ValueError::from(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                );
                if let Some(retain) = retain {
                    retain(&error);
                }
                error.into()
            },
            &mut Some(&mut original),
        )
    }

    pub(super) fn native_string_stage_check<E>(
        &self,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
    ) -> Result<(), E> {
        if let Some(check) = check {
            let mut stage = || check(self);
            Self::native_stage_check(&mut Some(&mut stage))
        } else {
            Ok(())
        }
    }

    fn validate_native_string_primary<E>(
        &self,
        protocol: NativeStringProtocol,
        failure: &dyn Fn(tcl_syntax::native_string::NativeStringUnavailable) -> E,
    ) -> Result<(), E> {
        if !self.native_object_is_live() {
            #[cfg(test)]
            self.report_native_string_unavailable(protocol, "retired-header");
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
            ));
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::FrameLevel { version, .. }
            if protocol != NativeStringProtocol::C(*version))
        {
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            ));
        }
        if self
            .native_instruction_name()
            .is_some_and(|name| protocol != NativeStringProtocol::C(name.version()))
        {
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            ));
        }
        if self.native_jim_option_cache().is_some() && !protocol.is_jim084() {
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            ));
        }
        if self.native_property_name_is_cached()
            && protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1)
        {
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            ));
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::Tcl84Wide(_))
            && protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4)
        {
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            ));
        }
        Ok(())
    }

    fn native_string_bytes_using<E>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        failure: &dyn Fn(tcl_syntax::native_string::NativeStringUnavailable) -> E,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
    ) -> Result<Rc<[u8]>, E> {
        self.native_string_stage_check(check)?;
        self.validate_native_string_primary(protocol, failure)?;
        if let Some(resident) = self.resident_string_bytes() {
            return Ok(resident);
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeBytecode(_)) {
            return Err(failure(
                tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
            ));
        }
        if let Some(result) = self.parsed_variable_string_bytes(protocol) {
            let bytes = result.map_err(failure)?;
            self.native_string_stage_check(check)?;
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        if let Some(cache) = self.native_namespace_name_cache() {
            if protocol != NativeStringProtocol::C(cache.version()) {
                return Err(failure(
                    tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
                ));
            }
            let recipe =
                tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(
                    cache.version(),
                );
            let bytes: Rc<[u8]> = Rc::from(cache.string_update_bytes(recipe).ok_or_else(|| {
                failure(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)
            })?);
            self.native_string_stage_check(check)?;
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0.string_storage.set(if bytes.is_empty() {
                NativeStringStorageIdentity::CanonicalEmpty
            } else {
                NativeStringStorageIdentity::Allocated
            });
            return Ok(bytes);
        }
        if host_formats_integers(protocol) {
            let scalar = match self.native_scalar_cache() {
                Some(NativeScalarCache::Tcl84Long(value)) => {
                    Some((tcl_platform::NativeIntegerKind::Long, value))
                }
                Some(NativeScalarCache::Number(Number::Int(value))) => {
                    Some((tcl_platform::NativeIntegerKind::Wide, value))
                }
                _ => None,
            };
            if let Some((kind, value)) = scalar
                && let Some(formatter) = formatter
            {
                let build = formatter.build();
                self.native_string_stage_check(check)?;
                if build.version[..2] != [8, 4] {
                    return Err(failure(
                        tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                    ));
                }
                let Ok(reached) = formatter.format(kind, value) else {
                    // The failure is reported after release's own stage checks,
                    // so a debug re-proof cannot pre-empt it.
                    crate::with_release_checks(|| self.native_string_stage_check(check))?;
                    return Err(failure(
                        tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
                    ));
                };
                self.native_string_stage_check(check)?;
                let bytes: Rc<[u8]> = Rc::from(reached);
                self.native_string_stage_check(check)?;
                *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
                self.0
                    .string_storage
                    .set(NativeStringStorageIdentity::Allocated);
                return Ok(bytes);
            }
        }
        // A release whose integer updater garbles INT64_MIN spells it `-8`.
        if let NativeStringProtocol::C(version) = protocol
            && version.garbles_wide_min()
            && matches!(
                self.native_scalar_cache(),
                Some(
                    NativeScalarCache::Tcl84Long(i64::MIN)
                        | NativeScalarCache::Number(Number::Int(i64::MIN))
                )
            )
        {
            let bytes: Rc<[u8]> =
                Rc::from(tcl_syntax::number::native_integer_string(i64::MIN, true).into_bytes());
            self.native_string_stage_check(check)?;
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        self.materialize_native_selected_primary_using(protocol, formatter, failure, check)
    }

    fn materialize_native_selected_primary_using<E>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        failure: &dyn Fn(tcl_syntax::native_string::NativeStringUnavailable) -> E,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
    ) -> Result<Rc<[u8]>, E> {
        self.native_string_stage_check(check)?;
        self.seal_compound_string_protocol(protocol)
            .map_err(failure)?;
        if matches!(
            *self.0.intrep.borrow(),
            IntRep::List { .. } | IntRep::Dict(_)
        ) {
            return self.materialize_native_compound_using(protocol, formatter, failure, check);
        }
        self.native_string_stage_check(check)?;
        let reached = self
            .materialize_native_cache_string(protocol)
            .map_err(failure);
        self.native_string_stage_check(check)?;
        reached
    }

    fn materialize_native_cache_string(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        if let IntRep::JimIndex(index) = &*self.0.intrep.borrow() {
            if protocol != NativeStringProtocol::Jim084 {
                return Err(
                    tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                );
            }
            let bytes: Rc<[u8]> = Rc::from(index.string_bytes());
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        if let Some(offset) = self.native_end_offset() {
            if protocol != NativeStringProtocol::C(offset.version()) {
                return Err(
                    tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                );
            }
            let bytes: Rc<[u8]> = Rc::from(
                offset
                    .string_update()
                    .ok_or(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)?,
            );
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        let index_cache = self.native_index_cache();
        if let Some((cache, version)) = index_cache {
            if protocol != tcl_syntax::native_string::NativeStringProtocol::C(version) {
                return Err(
                    tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                );
            }
            let bytes = cache
                .word()
                .map_err(|_| tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)?;
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0.string_storage.set(
                if bytes.is_empty() && version >= tcl_dialect::TclVersion::V9_0 {
                    NativeStringStorageIdentity::CanonicalEmpty
                } else {
                    NativeStringStorageIdentity::Allocated
                },
            );
            return Ok(bytes);
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeString {
            protocol: origin, num_chars: Some(0), unicode: None,
        } if *origin == protocol)
        {
            let bytes: Rc<[u8]> = Rc::from([]);
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        if matches!(
            *self.0.intrep.borrow(),
            IntRep::CompletionCode(_)
                | IntRep::NativeArraySearch { .. }
                | IntRep::FrameLevel { .. }
                | IntRep::NativePropertyName(_)
                | IntRep::NativeMethodName(_)
                | IntRep::NativeRegexp(_)
                | IntRep::JimRegexp(_)
                | IntRep::NativeIndex { .. }
                | IntRep::NativeCommandName(_)
                | IntRep::NativeCommandNameUnresolved(_)
                | IntRep::JimCommand(_)
                | IntRep::JimVariable(_)
                | IntRep::JimOption(_)
                | IntRep::JimSubcommand(_)
                | IntRep::NativeParsedVariableName(_)
                | IntRep::NativeLocalVariableName(_)
                | IntRep::NativeNamespaceName(_)
                | IntRep::NativeString { unicode: None, .. }
                | IntRep::JimSource(_)
                | IntRep::JimScript(_)
                | IntRep::JimDictionarySubstitution { .. }
                | IntRep::JimInterpolated(_)
                | IntRep::JimScriptLine { .. }
                | IntRep::Expression(_)
        ) {
            #[cfg(test)]
            self.report_native_string_unavailable(protocol, "primary-has-no-updater");
            return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
        }
        self.materialize_native_bytearray_string(protocol)
    }

    #[cfg(test)]
    fn report_native_string_unavailable(&self, protocol: NativeStringProtocol, reason: &str) {
        // Failure context observes only the original header. No getter, object
        // clone, payload, address or result storage is produced by this report.
        eprintln!(
            "native-string-unavailable reason={reason} protocol={protocol:?} primary={} live={} references={} resident={}",
            self.native_object_type_name(),
            self.native_object_is_live(),
            self.native_object_reference_count(),
            self.0.string.borrow().is_some(),
        );
    }

    #[cfg(test)]
    pub(crate) fn report_native_compound_ownership(&self, label: &str) {
        fn visit(value: &Value, label: &str, path: &mut Vec<usize>, remaining: &mut usize) {
            if *remaining == 0 || path.len() > 16 {
                return;
            }
            *remaining -= 1;
            let primary = value.0.intrep.borrow();
            let list = match &*primary {
                IntRep::List { items, .. } => Some(items),
                _ => None,
            };
            eprintln!(
                "native-compound-ownership label={label} path={path:?} primary={} live={} references={} lifetime_only={} resident={} list_owner={:?} list_backing={:?} list_headers={:?}",
                value.native_object_type_name(),
                value.native_object_is_live(),
                value.native_object_reference_count(),
                value.1 == NativeValueHandleOwnership::LifetimeOnly,
                value.resident_string_bytes().is_some(),
                list.map(crate::NativeListItems::owns_native_header),
                list.map(crate::NativeListItems::is_whole_backing),
                list.map(crate::NativeListItems::native_header_reference_count),
            );
            if let Some(items) = list {
                for (index, member) in items.iter().enumerate() {
                    path.push(index);
                    visit(member, label, path, remaining);
                    path.pop();
                }
            } else if let IntRep::Dict(dictionary) = &*primary {
                dictionary.with_pairs(|pairs| {
                    for (index, (key, member)) in pairs.iter().enumerate() {
                        path.push(index * 2);
                        visit(key, label, path, remaining);
                        *path.last_mut().expect("dictionary member path") += 1;
                        visit(member, label, path, remaining);
                        path.pop();
                    }
                });
            }
        }
        visit(self, label, &mut Vec::new(), &mut 80);
    }

    fn materialize_native_bytearray_string(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        let backing = match &*self.0.intrep.borrow() {
            IntRep::ByteArray(backing) => Some(Rc::clone(backing)),
            _ => None,
        };
        if let Some(backing) = backing {
            let selected = backing.string_protocol.get().unwrap_or(protocol);
            let materialized = selected.materialize(
                tcl_syntax::native_string::NativeStringInput::PureByteArray(&backing.bytes),
            )?;
            let bytes: Rc<[u8]> = Rc::from(materialized.as_ref());
            backing.string_protocol.set(Some(selected));
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0.string_storage.set(
                if bytes.is_empty()
                    && selected
                        .tcl_version()
                        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
                {
                    NativeStringStorageIdentity::CanonicalEmpty
                } else {
                    NativeStringStorageIdentity::Allocated
                },
            );
            return Ok(bytes);
        }
        Ok(self.string_bytes())
    }

    fn materialize_native_compound_using<E>(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
        failure: &dyn Fn(tcl_syntax::native_string::NativeStringUnavailable) -> E,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
    ) -> Result<Rc<[u8]>, E> {
        // Typed backing leases retain original members without acquiring child
        // object references. Indices remain valid even if an object shimmers.
        let mut backings = Vec::<CompoundStringBacking>::new();
        let mut pending = vec![(CompoundStringNode::Root, false)];
        while let Some((node, expanded)) = pending.pop() {
            let current = node.value(self, &backings);
            current.native_string_stage_check(check)?;
            if current.resident_string_bytes().is_some() {
                continue;
            }
            current
                .seal_compound_string_protocol(protocol)
                .map_err(failure)?;
            let backing = match &*current.0.intrep.borrow() {
                IntRep::List { items, .. } => {
                    Some(CompoundStringBacking::List(items.lifetime_view()))
                }
                IntRep::Dict(dict) => Some(CompoundStringBacking::Dictionary(dict.pairs_backing())),
                _ => None,
            };
            let Some(backing) = backing else {
                drop(current.native_string_bytes_using(protocol, formatter, failure, check)?);
                current.native_string_stage_check(check)?;
                continue;
            };
            if !expanded {
                let count = backing.len();
                let slot = backings.len();
                backings.push(backing);
                pending.push((node, true));
                pending.extend((0..count).rev().map(|index| {
                    (
                        CompoundStringNode::Member {
                            backing: slot,
                            index,
                        },
                        false,
                    )
                }));
                continue;
            }
            let materialized = (0..backing.len())
                .map(|index| {
                    backing
                        .member(index)
                        .resident_string_bytes()
                        .expect("reached child updater")
                })
                .collect::<Vec<_>>();
            let bytes: Rc<[u8]> = Rc::from(
                tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
                    protocol,
                )
                .render(&materialized),
            );
            current.native_string_stage_check(check)?;
            *current.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            if let IntRep::List {
                items, canonical, ..
            } = &*current.0.intrep.borrow()
            {
                let shared = items.native_is_shared();
                canonical.set(protocol.updated_list_canonical(canonical.get(), shared));
            }
            current.0.string_storage.set(if bytes.is_empty() {
                protocol.compound_updater_storage()
            } else {
                NativeStringStorageIdentity::Allocated
            });
        }
        self.native_string_stage_check(check)?;
        Ok(self
            .resident_string_bytes()
            .expect("completed native compound updater"))
    }

    /// Construct the same typed value with an explicitly supplied resident
    /// string. No parsing, string generation or payload conversion occurs.
    /// Other handles to the original object retain their representations.
    #[must_use]
    pub fn with_resident_string_bytes(self, bytes: Rc<[u8]>) -> Self {
        Self(
            native_jim_pool::construct(ObjectRepresentations {
                string: RefCell::new(Some(RawString::from_bytes(bytes))),
                string_storage: Cell::new(NativeStringStorageIdentity::Unknown),
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

    /// Retain the unknown-count String primary of an authenticated native
    /// append or format producer on the same live original header.
    pub(crate) fn retain_native_string_representation(
        &self,
        materialization: tcl_registry::native_string_materialization::NativeStringMaterialization,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native String producer resident storage",
            ));
        }
        let protocol = materialization.protocol();
        *self.0.intrep.borrow_mut() = if protocol.is_jim084() {
            IntRep::JimString(None)
        } else {
            IntRep::NativeString {
                protocol,
                num_chars: None,
                unicode: None,
            }
        };
        Ok(())
    }

    /// Reconstruct an actual String primary cache without running a getter.
    /// Engine origin, resident allocation and retained units are independent.
    ///
    /// # Errors
    /// Refuses foreign engines, inconsistent unit/count storage, and missing updaters.
    pub fn from_native_string_cache(
        cache: tcl_syntax::native_object::NativeObjectCacheSnapshot,
        dialect: tcl_registry::InvocationDialect,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        let selected =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native String carrier issuer",
                ))?;
        let primary = match cache {
            Cache::String {
                protocol,
                num_chars,
                unicode,
            } if selected == protocol && protocol.tcl_version().is_some() => {
                if unicode.as_ref().is_some_and(|units| {
                    Some(units.len()) != num_chars
                        || tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(
                            protocol.tcl_version().unwrap(),
                        )
                        .encode_units(units)
                        .is_none()
                }) || (resident.is_none() && unicode.is_none() && num_chars != Some(0))
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native String carrier storage",
                    ));
                }
                IntRep::NativeString {
                    protocol,
                    num_chars,
                    unicode,
                }
            }
            Cache::JimString { num_chars } if selected.is_jim084() && resident.is_some() => {
                IntRep::JimString(num_chars)
            }
            _ => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native String carrier origin",
                ));
            }
        };
        if resident.as_ref().is_some_and(|(bytes, marker)| {
            *marker == NativeStringStorageIdentity::CanonicalEmpty && !bytes.is_empty()
        }) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native String resident storage",
            ));
        }
        let storage = resident
            .as_ref()
            .map_or(NativeStringStorageIdentity::Unknown, |(_, marker)| *marker);
        let value = Self::from_raw_parts(
            resident.map(|(bytes, _)| RawString::from_bytes(bytes)),
            primary,
        );
        value.0.string_storage.set(storage);
        Ok(value)
    }

    /// Reach the selected native character-length accessor on this object.
    /// Existing caches and byte-array shortcuts are inspected before strings.
    ///
    /// # Errors
    /// Refuses unavailable storage or a foreign retained String-unit recipe.
    pub fn native_character_count_with_protocol(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
    ) -> Result<usize, tcl_syntax::value::ValueError> {
        self.native_character_count_using(protocol, representation, &mut None, &mut || {
            self.native_string_bytes(protocol)
                .map_err(|_| ValueError::CommandProtocolUnavailable("native string length storage"))
        })
    }

    pub(crate) fn native_character_count_checked(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
        current: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, tcl_cmd_core::CmdError>,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        self.native_character_count_using(
            protocol,
            representation,
            &mut Some(current),
            original_string,
        )
    }

    fn native_character_count_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
        current: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<usize, E> {
        Self::native_stage_check(current)?;
        self.check_native_header()?;
        if representation.preserves_short_string()
            && let Some(bytes) = self.resident_string_bytes()
            && bytes.len() < 2
        {
            return Ok(bytes.len());
        }
        if let IntRep::ByteArray(bytes) = &*self.0.intrep.borrow()
            && representation.counts_byte_array(self.0.string.borrow().is_none(), bytes.proper)
        {
            return Ok(bytes.bytes.len());
        }
        if protocol.is_jim084() {
            if let IntRep::JimString(Some(count)) = *self.0.intrep.borrow() {
                return Ok(count);
            }
            let reached = original_string();
            Self::native_stage_check(current)?;
            let bytes = reached?;
            let count = RawString::from_bytes(bytes).jim084_characters().count();
            self.replace_primary_using(IntRep::JimString(Some(count)), current)?;
            return Ok(count);
        }
        if let IntRep::NativeString {
            protocol: origin,
            num_chars: Some(count),
            ..
        } = &*self.0.intrep.borrow()
        {
            if *origin != protocol {
                return Err(
                    ValueError::CommandProtocolUnavailable("native string cache origin").into(),
                );
            }
            return Ok(*count);
        }
        let reached = original_string();
        Self::native_stage_check(current)?;
        let bytes = reached?;
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native string units",
            ))?;
        let count = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .decode_units(&bytes)
            .len();
        let unicode =
            (version <= tcl_dialect::TclVersion::V8_5 && count != bytes.len()).then(|| {
                Rc::from(
                    tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
                        .decode_storage_units(
                            &bytes,
                            tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
                        ),
                )
            });
        self.replace_primary_using(
            IntRep::NativeString {
                protocol,
                num_chars: Some(count),
                unicode,
            },
            current,
        )?;
        Ok(count)
    }

    /// Selected SetStringFromAny/count purpose, without public length shortcuts.
    pub(crate) fn native_c_string_storage_checked<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        current: &mut dyn FnMut() -> Result<(), E>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<tcl_cmd_core::string::NativeCStringStorage, E> {
        let count = self.native_character_count_using(
            protocol,
            tcl_registry::native_string_length::NativeStringLengthRepresentation::String,
            &mut Some(&mut *current),
            original_string,
        )?;
        current()?;
        let snapshot = self.native_object_snapshot();
        let tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
            protocol: origin,
            unicode,
            ..
        } = snapshot.cache
        else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C String storage primary",
            )
            .into());
        };
        if origin != protocol {
            return Err(
                ValueError::CommandProtocolUnavailable("original C String storage issuer").into(),
            );
        }
        Ok(tcl_cmd_core::string::NativeCStringStorage {
            count,
            bytes: snapshot.resident,
            unicode,
        })
    }

    /// Reach native C Unicode preparation and retain actual decoded units.
    ///
    /// # Errors
    /// Refuses unaudited storage, Jim's different string protocol or foreign cache units.
    pub fn native_unicode_units(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError> {
        self.native_unicode_units_using(protocol, &mut None, &mut || {
            self.native_string_bytes(protocol)
                .map_err(|_| ValueError::CommandProtocolUnavailable("native Unicode storage"))
        })
    }

    pub(crate) fn native_unicode_units_checked<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        current: &mut dyn FnMut() -> Result<(), E>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<Rc<[u32]>, E> {
        self.native_unicode_units_using(protocol, &mut Some(current), original_string)
    }

    fn native_unicode_units_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        current: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<Rc<[u32]>, E> {
        Self::native_stage_check(current)?;
        self.check_native_header()?;
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native C Unicode units",
            ))?;
        let cached = match &*self.0.intrep.borrow() {
            IntRep::NativeString {
                protocol: origin,
                unicode: Some(units),
                ..
            } => {
                if *origin != protocol {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native string cache origin",
                    )
                    .into());
                }
                Some(Rc::clone(units))
            }
            _ => None,
        };
        if let Some(units) = cached {
            Self::native_stage_check(current)?;
            return Ok(units);
        }
        let reached = original_string();
        Self::native_stage_check(current)?;
        let bytes = reached?;
        let units: Rc<[u32]> = Rc::from(
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_storage_units(
                &bytes,
                tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
            ),
        );
        self.replace_primary_using(
            IntRep::NativeString {
                protocol,
                num_chars: Some(units.len()),
                unicode: Some(Rc::clone(&units)),
            },
            current,
        )?;
        Self::native_stage_check(current)?;
        Ok(units)
    }

    /// Count native character units without changing original string bytes.
    /// Jim's `Utf8Length` converts to its string intrep and retains that count;
    /// a later list/dictionary/numeric conversion withdraws the receipt.
    pub(crate) fn native_character_count(
        &self,
        model: tcl_dialect::StringCharacterModel,
        representation: Option<
            tcl_registry::native_string_length::NativeStringLengthRepresentation,
        >,
    ) -> Result<usize, UnicodeAccessError> {
        use tcl_registry::native_string_length::NativeStringLengthRepresentation;
        if representation.is_some_and(NativeStringLengthRepresentation::preserves_short_string)
            && let Some(string) = self.0.string.borrow().as_ref()
            && string.bytes().len() < 2
        {
            return Ok(string.bytes().len());
        }
        if let IntRep::ByteArray(bytes) = &*self.0.intrep.borrow()
            && representation.is_some_and(|policy| {
                policy.counts_byte_array(self.0.string.borrow().is_none(), bytes.proper)
            })
        {
            return Ok(bytes.bytes.len());
        }
        if model == tcl_dialect::StringCharacterModel::Jim084Utf8 {
            if let IntRep::JimString(Some(count)) = *self.0.intrep.borrow() {
                return Ok(count);
            }
            let count = self.to_raw_string_at_depth(0).0.character_count(model)?;
            *self.0.intrep.borrow_mut() = IntRep::JimString(Some(count));
            return Ok(count);
        }
        let count = self.to_raw_string_at_depth(0).0.character_count(model)?;
        if representation.is_some() {
            *self.0.intrep.borrow_mut() = IntRep::Str;
        }
        Ok(count)
    }
}
