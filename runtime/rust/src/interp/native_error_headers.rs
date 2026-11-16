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

//! Original private List/Dictionary headers; saved state retains headers, not child copies.

type OriginalErrorHeaderString<'a> =
    &'a dyn Fn(
        *mut TclObj,
        &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>;

use crate::{obj, obj::TclObj};
use tcl_cmd_core::return_options::ReturnOptionPair;
#[cfg(test)]
use tcl_runtime_api::error_stack::ErrorStackValueError;
use tcl_runtime_api::error_stack::{ErrorStack, ErrorStackFrame};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

// Read-only diagnostic metadata: address, native refs, resident length, String
// absence, descriptor address and internal bits. Never invoke an object getter.
type HeldHeapHeader = (
    usize,
    crate::obj::TclSize,
    crate::obj::TclSize,
    bool,
    usize,
    u64,
);

impl super::Interp {
    pub(crate) fn observe_native_heap_roots(&self) {
        let inspect = |pointer: *mut TclObj| -> Option<HeldHeapHeader> {
            // SAFETY: each caller below borrows an existing native owner; no
            // callback, conversion or retirement occurs while copying fields.
            let header = unsafe { pointer.as_ref() }?;
            Some((
                pointer as usize,
                header.ref_count,
                header.length,
                header.bytes.is_null(),
                header.type_ptr as usize,
                header.internal_rep,
            ))
        };
        let result = inspect(self.result.get());
        let (info, code, legacy_copy, exception_busy) = match self.exc.try_borrow() {
            Ok(exception) => (
                exception
                    .native
                    .info
                    .as_ref()
                    .and_then(|v| inspect(v.as_ptr())),
                exception
                    .native
                    .code
                    .as_ref()
                    .and_then(|v| inspect(v.as_ptr())),
                Some(exception.native.legacy_copy),
                false,
            ),
            Err(_) => (None, None, None, true),
        };
        // Exact physical root slots only. None also covers busy/non-scalar cells;
        // it does not establish semantic absence or authorize a variable read.
        let globals = self.namespaces.try_borrow().ok().and_then(|namespaces| {
            let table = namespaces.original_variable_table_at(crate::namespace::GLOBAL)?;
            Some((
                table
                    .inspect_existing_scalar(b"errorInfo", inspect)
                    .flatten(),
                table
                    .inspect_existing_scalar(b"errorCode", inspect)
                    .flatten(),
            ))
        });
        let (options, options_busy) = match self.return_options.try_borrow() {
            Ok(options) => (options.header().and_then(inspect), false),
            Err(_) => (None, true),
        };
        let (stack, inner, stack_busy, inner_busy) = match self.error_stack.try_borrow() {
            Ok(stack) => {
                let header = stack.header.as_ref().and_then(|v| inspect(v.as_ptr()));
                let (inner, busy) = match &stack.inner_context {
                    Some(context) => match context.try_borrow() {
                        Ok(value) => (inspect(value.as_ptr()), false),
                        Err(_) => (None, true),
                    },
                    None => (None, false),
                };
                (header, inner, false, busy)
            }
            Err(_) => (None, None, true, false),
        };
        let arrays = self.native_literal_arrays.try_borrow().ok().map(|arrays| {
            (
                arrays.len(),
                arrays.iter().filter(|v| v.strong_count() != 0).count(),
            )
        });
        // Every owner/RefCell borrow is already gone before the diagnostic I/O.
        // The counters count native headers/buffers, not these copied Rust tuples.
        eprintln!(
            "WASM_HELD_HEAP objs={} bufs={} result={result:?} info={info:?} code={code:?} legacy_copy={legacy_copy:?} globals={globals:?} options={options:?} stack={stack:?} inner={inner:?} arrays={arrays:?} busy={exception_busy}/{options_busy}/{stack_busy}/{inner_busy}",
            crate::counters::live_objs(),
            crate::counters::live_bufs(),
        );
    }
}

#[derive(Clone, Default)]
pub(super) struct NativeReturnOptions {
    incarnation: std::rc::Rc<()>,
    header: Option<obj::Owned>,
    protocol: Option<NativeStringProtocol>,
    keys: Vec<Vec<u8>>,
    other: std::rc::Rc<Vec<ReturnOptionPair<obj::Owned>>>,
}
impl NativeReturnOptions {
    #[cfg(test)]
    pub(super) fn new(
        options: Vec<ReturnOptionPair<obj::Owned>>,
        recipe: Option<tcl_registry::native_error_objects::NativeErrorObjectsProtocol>,
    ) -> Result<Self, ValueError> {
        let Some(recipe) = recipe.filter(|recipe| recipe.has_return_options()) else {
            return Ok(Self {
                other: std::rc::Rc::new(options),
                ..Self::default()
            });
        };
        let pairs: Vec<_> = options
            .iter()
            .map(|pair| (pair.key.as_ptr(), pair.value.as_ptr()))
            .collect();
        let header = if pairs.is_empty() {
            let header = obj::Owned::fresh(obj::new_obj());
            drop(crate::dict::native_dict_pairs(
                header.as_ptr(),
                recipe.strings(),
            )?);
            header
        } else {
            obj::Owned::fresh(crate::dict::new_dict_obj_native(
                &pairs,
                None,
                recipe.strings(),
            )?)
        };
        let mut keys = Vec::new();
        for pair in &options {
            if !keys.contains(&pair.key_bytes) {
                keys.push(pair.key_bytes.clone());
            }
        }
        Ok(Self {
            header: Some(header),
            protocol: Some(recipe.strings()),
            keys,
            other: Default::default(),
            incarnation: Default::default(),
        })
    }
    // Only the Rust publication identity changes; the native header/member
    // roles are unchanged. A held Rc identity cannot be reused or wrap.
    pub(super) fn renew_incarnation(&mut self) {
        self.incarnation = std::rc::Rc::new(());
    }

    pub(super) fn new_checked(
        options: Vec<ReturnOptionPair<obj::Owned>>,
        recipe: Option<tcl_registry::native_error_objects::NativeErrorObjectsProtocol>,
        check: &dyn Fn() -> Result<(), tcl_runtime_api::NativeExecutionError>,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        check()?;
        let reached = (|| {
            let Some(recipe) = recipe.filter(|recipe| recipe.has_return_options()) else {
                return Ok(Self {
                    other: std::rc::Rc::new(options),
                    ..Self::default()
                });
            };
            let pairs: Vec<_> = options
                .iter()
                .map(|pair| (pair.key.as_ptr(), pair.value.as_ptr()))
                .collect();
            let header = if pairs.is_empty() {
                let header = obj::Owned::fresh(obj::new_obj());
                let reached = crate::dict::native_dict_pairs_using(
                    header.as_ptr(),
                    recipe.strings(),
                    &|| check().map_err(tcl_cmd_core::CmdError::from),
                    &tcl_cmd_core::CmdError::from,
                );
                check()?;
                drop(reached?);
                header
            } else {
                obj::Owned::fresh(crate::dict::new_dict_obj_native_with_stage_check(
                    &pairs,
                    None,
                    recipe.strings(),
                    check,
                )?)
            };
            let mut keys = Vec::new();
            for pair in &options {
                if !keys.contains(&pair.key_bytes) {
                    keys.push(pair.key_bytes.clone());
                }
            }
            let prepared = Self {
                header: Some(header),
                protocol: Some(recipe.strings()),
                keys,
                other: Default::default(),
                incarnation: Default::default(),
            };
            // Input roles retire outside every interpreter borrow before the
            // prepared owning header can be published.
            drop(options);
            check()?;
            Ok(prepared)
        })();
        check()?;
        reached
    }

    /// The original instruction retains its merged operand before conversion.
    /// The supplied check belongs to that actual entered operation, not a new
    /// Dict or source issuer. No key getter runs under interpreter state borrows.
    #[cfg(test)]
    #[cfg(debug_assertions)]
    pub(super) fn from_original_checked(
        original: *mut TclObj,
        protocol: NativeStringProtocol,
        check: &dyn Fn() -> Result<(), tcl_runtime_api::NativeExecutionError>,
        refuse: &impl Fn(ValueError) -> tcl_cmd_core::CmdError,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        Self::from_original_using(
            original,
            protocol,
            &|| check().map_err(tcl_cmd_core::CmdError::from),
            &|key, current| {
                crate::list::with_native_conversion_check(current, refuse, |stage, _| {
                    crate::dict::native_object_bytes_with_stage_handlers(
                        key,
                        protocol,
                        None,
                        Some(stage),
                    )
                    .map(std::rc::Rc::from)
                })
            },
            refuse,
        )
    }

    /// Preserve the actual merged-header retain before conversion while the
    /// entered caller checks its independent frame/backing inside the String worker.
    pub(super) fn from_original_using(
        original: *mut TclObj,
        protocol: NativeStringProtocol,
        check: &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
        string: OriginalErrorHeaderString<'_>,
        refuse: &impl Fn(ValueError) -> tcl_cmd_core::CmdError,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        check()?;
        obj::check_native_liveness(original).map_err(refuse)?;
        let header = obj::Owned::retain(original);
        let reached = crate::dict::native_dict_pairs_using(original, protocol, check, refuse);
        check()?;
        let pairs = reached?;
        let backing = crate::dict::original_dictionary_backing(original).map_err(refuse)?;
        let lifetimes = pairs
            .iter()
            .map(|&(key, value)| {
                obj::check_native_liveness(key).map_err(refuse)?;
                obj::check_native_liveness(value).map_err(refuse)?;
                Ok((
                    obj::NativeObjectLifetime::retain(key),
                    obj::NativeObjectLifetime::retain(value),
                ))
            })
            .collect::<Result<Vec<_>, tcl_cmd_core::CmdError>>()?;
        let mut keys = Vec::new();
        for &(key, value) in &pairs {
            obj::check_native_liveness(key).map_err(refuse)?;
            obj::check_native_liveness(value).map_err(refuse)?;
            let key_lifetime = obj::NativeObjectLifetime::retain(key);
            let current = || {
                check()?;
                crate::dict::original_dictionary_pairs_current(original, &backing, &pairs)
                    .map_err(refuse)?;
                obj::check_native_liveness(key_lifetime.as_ptr()).map_err(refuse)
            };
            let reached = string(key, &current);
            current()?;
            check()?;
            crate::dict::original_dictionary_pairs_current(original, &backing, &pairs)
                .map_err(refuse)?;
            obj::check_native_liveness(key_lifetime.as_ptr()).map_err(refuse)?;
            let key = reached?.to_vec();
            if !keys.contains(&key) {
                keys.push(key);
            }
            drop(key_lifetime);
            check()?;
        }
        drop(lifetimes);
        check()?;
        Ok(Self {
            header: Some(header),
            protocol: Some(protocol),
            keys,
            other: Default::default(),
            incarnation: Default::default(),
        })
    }

    /// Software birth of the installed carried record; cloning this Rust
    /// identity changes no native header/member role.
    pub(super) fn original_incarnation(&self) -> std::rc::Rc<()> {
        std::rc::Rc::clone(&self.incarnation)
    }
    pub(super) fn has_original_incarnation(&self, original: &std::rc::Rc<()>) -> bool {
        std::rc::Rc::ptr_eq(original, &self.incarnation)
    }

    pub(super) fn is_original(&self, original: *mut TclObj) -> bool {
        self.header
            .as_ref()
            .is_some_and(|header| header.as_ptr() == original)
    }
    #[cfg(test)]
    pub(super) fn pairs(&self) -> Vec<ReturnOptionPair<obj::Owned>> {
        let (Some(header), Some(protocol)) = (&self.header, self.protocol) else {
            return self.other.as_ref().clone();
        };
        let pairs = crate::dict::native_dict_pairs(header.as_ptr(), protocol)
            .expect("private native Dictionary header");
        self.keys
            .iter()
            .filter_map(|key| {
                pairs
                    .iter()
                    .find(|(original, _)| obj::bytes_of(*original) == *key)
                    .map(|&(original, value)| ReturnOptionPair {
                        key: obj::Owned::retain(original),
                        value: obj::Owned::retain(value),
                        key_bytes: key.clone(),
                    })
            })
            .collect()
    }
    pub(super) fn header(&self) -> Option<*mut TclObj> {
        self.header.as_ref().map(obj::Owned::as_ptr)
    }
}

#[derive(Clone, Default)]
pub(super) struct NativeErrorStack {
    incarnation: std::rc::Rc<()>,
    header: Option<obj::Owned>,
    // Exception snapshots share this interpreter role instead of retaining or
    // restoring the mutable native innerContext header.
    inner_context: Option<std::rc::Rc<std::cell::RefCell<obj::Owned>>>,
    protocol: Option<NativeStringProtocol>,
    metadata: ErrorStack<()>,
}
impl NativeErrorStack {
    // Pure Rust mutation identity is not a native header reference or policy.
    pub(super) fn renew_incarnation(&mut self) {
        self.incarnation = std::rc::Rc::new(());
    }

    #[cfg(test)]
    pub(super) fn original_header(&self) -> Option<*mut TclObj> {
        self.header.as_ref().map(obj::Owned::as_ptr)
    }
    #[cfg(test)]
    pub(super) fn original_inner_context(&self) -> Option<*mut TclObj> {
        self.inner_context
            .as_ref()
            .map(|context| context.borrow().as_ptr())
    }
    #[cfg(test)]
    pub(super) fn configure(
        &mut self,
        recipe: Option<tcl_registry::native_error_objects::NativeErrorObjectsProtocol>,
    ) {
        let protocol = recipe
            .filter(|recipe| recipe.has_error_stack())
            .map(|recipe| recipe.strings());
        if self.protocol != protocol {
            self.renew_incarnation();
            self.protocol = protocol;
            self.header = protocol
                .map(|protocol| obj::Owned::fresh(crate::list::new_list_obj_native(&[], protocol)));
            self.inner_context = protocol.map(|protocol| {
                std::rc::Rc::new(std::cell::RefCell::new(obj::Owned::fresh(
                    crate::list::new_list_obj_native(&[], protocol),
                )))
            });
            self.metadata = ErrorStack::default();
        }
    }
    pub(super) fn is_reset(&self) -> bool {
        self.metadata.is_reset()
    }
    pub(super) fn mark_reset(&mut self) {
        self.renew_incarnation();
        self.metadata.mark_reset();
    }
    #[cfg(test)]
    fn replace_objects(&mut self, values: &[*mut TclObj]) {
        self.renew_incarnation();
        if let (Some(header), Some(protocol)) = (&self.header, self.protocol) {
            self.header = Some(
                crate::list::replace_elements_native(header.as_ptr(), values, protocol)
                    .expect("private native List header"),
            );
        }
    }
    #[cfg(test)]
    fn replace_bytes(&mut self, values: &[Vec<u8>]) {
        let values: Vec<_> = values
            .iter()
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
            .collect();
        self.replace_objects(&values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>());
    }
    #[cfg(test)]
    pub(super) fn adopt(&mut self, values: Vec<Vec<u8>>) -> Result<(), ErrorStackValueError> {
        if values.len() % 2 != 0 {
            return Err(ErrorStackValueError::OddSized);
        }
        self.replace_bytes(&values);
        self.metadata.adopt(vec![(); values.len()])
    }
    #[cfg(test)]
    pub(super) fn adopt_original(&mut self, original: *mut TclObj) -> Result<(), ValueError> {
        let protocol = self.protocol.ok_or(ValueError::CommandProtocolUnavailable(
            "private error-stack header",
        ))?;
        // TclProcessReturn duplicates the private header before converting
        // its supplied operand, which may be that same shared header.
        if let Some(header) = &self.header {
            if obj::is_shared(header.as_ptr()) {
                let duplicate = obj::Owned::fresh(obj::duplicate(header.as_ptr()));
                crate::list::duplicate_native_backing(
                    header.as_ptr(),
                    duplicate.as_ptr(),
                    protocol,
                );
                self.header = Some(duplicate);
            }
        }
        let members = crate::list::list_elements_native_checked(original, protocol)?;
        if members.len() % 2 != 0 {
            return Err(ValueError::CommandProtocolUnavailable(
                "validated error-stack pairs",
            ));
        }
        self.replace_objects(&members);
        self.metadata
            .adopt(vec![(); members.len()])
            .expect("validated pairs");
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn begin_inner(&mut self, tag: Vec<u8>, context: Vec<u8>) -> bool {
        if !self.is_reset() {
            return false;
        }
        self.replace_bytes(&[tag.clone(), context.clone()]);
        self.metadata.begin_inner((), ())
    }
    #[cfg(test)]
    pub(super) fn entries(&self) -> Vec<Vec<u8>> {
        let protocol = self.protocol.expect("selected private header");
        crate::list::list_elements_native_checked(self.header.as_ref().unwrap().as_ptr(), protocol)
            .unwrap()
            .into_iter()
            .map(|value| crate::dict::native_object_bytes(value, protocol).unwrap())
            .collect()
    }
    #[cfg(test)]
    pub(super) fn value(&self) -> obj::Owned {
        self.header
            .as_ref()
            .expect("selected private error-stack header")
            .clone()
    }
}

struct OriginalStackProjection {
    header: *mut TclObj,
    protocol: NativeStringProtocol,
    ticket: std::rc::Rc<()>,
    _allocation: obj::NativeObjectLifetime,
}

impl super::Interp {
    fn prepare_original_error_stack(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<OriginalStackProjection, tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let (header, protocol, ticket) = {
            let stack = self.error_stack.borrow();
            (
                stack.header.as_ref().map(obj::Owned::as_ptr),
                stack.protocol,
                std::rc::Rc::clone(&stack.incarnation),
            )
        };
        let (Some(header), Some(protocol)) = (header, protocol) else {
            return Err(self.clone().refuse_completion_value_access(
                ValueError::CommandProtocolUnavailable("current original error-stack projection"),
            ));
        };
        obj::check_native_liveness(header)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let projection = OriginalStackProjection {
            header,
            protocol,
            ticket,
            _allocation: obj::NativeObjectLifetime::retain(header),
        };
        self.check_original_error_stack_projection(currency, Some(header), &projection.ticket)?;
        Ok(projection)
    }

    fn publish_original_error_stack(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        projection: &mut OriginalStackProjection,
        value: obj::Owned,
        metadata: ErrorStack<()>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.check_original_error_stack_projection(
            currency,
            Some(projection.header),
            &projection.ticket,
        )?;
        let allocation = obj::NativeObjectLifetime::retain(value.as_ptr());
        let retired = {
            let mut stack = self.error_stack.borrow_mut();
            let retired = stack.header.replace(value);
            stack.metadata = metadata;
            stack.renew_incarnation();
            projection.header = stack.header.as_ref().unwrap().as_ptr();
            projection.ticket = std::rc::Rc::clone(&stack.incarnation);
            retired
        };
        let old_allocation = std::mem::replace(&mut projection._allocation, allocation);
        drop(retired);
        drop(old_allocation);
        self.check_original_error_stack_projection(
            currency,
            Some(projection.header),
            &projection.ticket,
        )
    }

    /// Publish freshly configured roles, then retire the actual displaced fields
    /// outside the stack borrow. Coroutine callers still complete their mandatory
    /// field handoff after a retained refusal; this supplies no later body grant.
    pub(super) fn configure_native_error_stack_checked(
        &self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let protocol = self
            .native_invocation_dialect()
            .native_error_objects_protocol()
            .filter(|recipe| recipe.has_error_stack())
            .map(|recipe| recipe.strings());
        if self.error_stack.borrow().protocol == protocol {
            return scope.currency().ensure_current_or_refuse();
        }
        let defaults = NativeErrorStack::default();
        let header = protocol
            .map(|protocol| obj::Owned::fresh(crate::list::new_list_obj_native(&[], protocol)));
        let inner_context = protocol.map(|protocol| {
            std::rc::Rc::new(std::cell::RefCell::new(obj::Owned::fresh(
                crate::list::new_list_obj_native(&[], protocol),
            )))
        });
        let replacement = NativeErrorStack {
            protocol,
            header,
            inner_context,
            ..defaults
        };
        scope.currency().ensure_current_or_refuse()?;
        let ticket = std::rc::Rc::clone(&replacement.incarnation);
        let header = replacement.header.as_ref().map(obj::Owned::as_ptr);
        let retired = self.error_stack.replace(replacement);
        drop(retired);
        self.check_original_error_stack_projection(scope.currency(), header, &ticket)
    }

    /// Byte-authored complete pairs use the same checked private List worker.
    pub(super) fn adopt_error_stack_bytes_checked(
        &self,
        values: &[Vec<u8>],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| {
            if values.len() % 2 != 0 {
                return Err(self.clone().refuse_completion_value_access(
                    ValueError::CommandProtocolUnavailable("validated error-stack pairs"),
                ));
            }
            let mut original = self.prepare_original_error_stack(scope.currency())?;
            let values = values
                .iter()
                .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
                .collect::<Vec<_>>();
            let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
            let current = || {
                self.check_original_error_stack_projection(
                    scope.currency(),
                    Some(original.header),
                    &original.ticket,
                )
            };
            let working = crate::list::replace_elements_native_using(
                original.header,
                &pointers,
                original.protocol,
                &current,
                &|error| self.clone().refuse_completion_value_access(error),
            )?;
            current()?;
            let mut metadata = self.error_stack.borrow().metadata.clone();
            metadata
                .adopt(vec![(); pointers.len()])
                .expect("validated pairs");
            self.publish_original_error_stack(scope.currency(), &mut original, working, metadata)?;
            drop(values);
            scope.currency().ensure_current_or_refuse()
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    /// Capture the actual innerContext and its private stack membership without
    /// retaining another native header role or borrowing state across retirement.
    pub(super) fn begin_original_error_stack_instruction_checked(
        &self,
        name: tcl_syntax::native_instruction_name::NativeInstructionName,
        operands: &[*mut TclObj],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| {
            if !self.error_stack.borrow().is_reset() {
                return Ok(());
            }
            let mut original = self.prepare_original_error_stack(scope.currency())?;
            let inner = {
                let stack = self.error_stack.borrow();
                stack.inner_context.as_ref().map(|inner| {
                    let header = inner.borrow().as_ptr();
                    (std::rc::Rc::downgrade(inner), header)
                })
            };
            let Some((inner, mut inner_header)) = inner else {
                return Err(self.clone().refuse_completion_value_access(
                    ValueError::CommandProtocolUnavailable("current original innerContext"),
                ));
            };
            obj::check_native_liveness(inner_header)
                .map_err(|error| self.clone().refuse_completion_value_access(error))?;
            let mut inner_allocation = obj::NativeObjectLifetime::retain(inner_header);
            let check_inner = |header: *mut TclObj,
                               ticket: &std::rc::Rc<()>,
                               inner_header: *mut TclObj| {
                self.check_original_error_stack_projection(scope.currency(), Some(header), ticket)?;
                let same = {
                    let stack = self.error_stack.borrow();
                    stack.inner_context.as_ref().is_some_and(|current| {
                        std::rc::Weak::ptr_eq(&inner, &std::rc::Rc::downgrade(current))
                            && current.borrow().as_ptr() == inner_header
                    })
                };
                if !same {
                    return Err(self.clone().refuse_completion_value_access(
                        ValueError::CommandProtocolUnavailable("changed original innerContext"),
                    ));
                }
                obj::check_native_liveness(inner_header)
                    .map_err(|error| self.clone().refuse_completion_value_access(error))
            };
            let instruction = obj::Owned::fresh(obj::native_instruction_name::fresh(name));
            let mut values = Vec::with_capacity(operands.len() + 1);
            values.push(instruction.as_ptr());
            values.extend_from_slice(operands);
            let current = || check_inner(original.header, &original.ticket, inner_header);
            let context = crate::list::replace_elements_native_using(
                inner_header,
                &values,
                original.protocol,
                &current,
                &|error| self.clone().refuse_completion_value_access(error),
            )?;
            current()?;
            inner_header = context.as_ptr();
            let allocation = obj::NativeObjectLifetime::retain(inner_header);
            let retired = {
                let mut stack = self.error_stack.borrow_mut();
                let retired = stack.inner_context.as_ref().unwrap().replace(context);
                stack.renew_incarnation();
                original.ticket = std::rc::Rc::clone(&stack.incarnation);
                retired
            };
            let old_allocation = std::mem::replace(&mut inner_allocation, allocation);
            drop(retired);
            drop(old_allocation);
            check_inner(original.header, &original.ticket, inner_header)?;
            let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
            let current = || check_inner(original.header, &original.ticket, inner_header);
            let working = crate::list::replace_elements_native_using(
                original.header,
                &[tag.as_ptr(), inner_header],
                original.protocol,
                &current,
                &|error| self.clone().refuse_completion_value_access(error),
            )?;
            current()?;
            let mut metadata = self.error_stack.borrow().metadata.clone();
            metadata.begin_inner((), ());
            self.publish_original_error_stack(scope.currency(), &mut original, working, metadata)?;
            drop(inner_allocation);
            drop(instruction);
            drop(tag);
            check_inner(original.header, &original.ticket, inner_header)
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    pub(super) fn begin_error_stack_bytes_checked(
        &self,
        command: &[u8],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| {
            if !self.error_stack.borrow().is_reset() {
                return Ok(());
            }
            let mut original = self.prepare_original_error_stack(scope.currency())?;
            let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
            let value = obj::Owned::fresh(obj::new_string_bytes(command));
            let current = || {
                self.check_original_error_stack_projection(
                    scope.currency(),
                    Some(original.header),
                    &original.ticket,
                )
            };
            let working = crate::list::replace_elements_native_using(
                original.header,
                &[tag.as_ptr(), value.as_ptr()],
                original.protocol,
                &current,
                &|error| self.clone().refuse_completion_value_access(error),
            )?;
            current()?;
            let mut metadata = self.error_stack.borrow().metadata.clone();
            metadata.begin_inner((), ());
            self.publish_original_error_stack(scope.currency(), &mut original, working, metadata)?;
            drop(tag);
            drop(value);
            scope.currency().ensure_current_or_refuse()
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    pub(super) fn log_original_error_stack_frame_checked(
        &self,
        frame: ErrorStackFrame<obj::Owned>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| {
            if self.error_stack.borrow().is_reset() {
                return Ok(());
            }
            let (tag, value) = match frame {
                ErrorStackFrame::Unreported => return Ok(()),
                ErrorStackFrame::Redirect(value) => (b"UP".as_slice(), value),
                ErrorStackFrame::Call(value) => (b"CALL".as_slice(), value),
            };
            let mut original = self.prepare_original_error_stack(scope.currency())?;
            let tag = obj::Owned::fresh(obj::new_string_bytes(tag));
            let current = || {
                self.check_original_error_stack_projection(
                    scope.currency(),
                    Some(original.header),
                    &original.ticket,
                )
            };
            let working = crate::list::append_native_elements_using(
                Some(original.header),
                &[tag.as_ptr(), value.as_ptr()],
                original.protocol,
                &current,
                &|error| self.clone().refuse_completion_value_access(error),
            )?;
            current()?;
            let mut metadata = self.error_stack.borrow().metadata.clone();
            metadata.push_pair((), ());
            self.publish_original_error_stack(scope.currency(), &mut original, working, metadata)?;
            drop(tag);
            drop(value);
            scope.currency().ensure_current_or_refuse()
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
}

impl super::Interp {
    /// Prepare the genuine C private stack without state borrows across List
    /// conversion, duplication or actual member/header retirement callbacks.
    pub(super) fn seed_original_error_stack_checked(
        &self,
        original: *mut TclObj,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let arguments = super::native_arguments::OriginalArguments::enter(self, &[original])?;
        self.seed_original_error_stack_using(
            original,
            &|| arguments.check_original_for(self, 0, original),
            &|error| arguments.retain_refusal(error),
        )
    }

    /// Seed the same private stack under its original incarnation and the
    /// actual caller's frame/options receipt through duplication and cleanup.
    pub(super) fn seed_original_error_stack_using(
        &self,
        original: *mut TclObj,
        extra: &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
        retain: &dyn Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        extra()?;
        let reached = (|| -> Result<(), tcl_cmd_core::CmdError> {
            let (mut header, protocol, mut ticket) = {
                let stack = self.error_stack.borrow();
                (
                    stack.header.as_ref().map(obj::Owned::as_ptr),
                    stack.protocol,
                    std::rc::Rc::clone(&stack.incarnation),
                )
            };
            let protocol = protocol.ok_or(ValueError::CommandProtocolUnavailable(
                "private error-stack header",
            ))?;
            let original_header = header.ok_or(ValueError::CommandProtocolUnavailable(
                "private error-stack header",
            ))?;
            obj::check_native_liveness(original_header)?;
            obj::check_native_liveness(original)?;
            let header_lifetime = obj::NativeObjectLifetime::retain(original_header);
            let original_lifetime = obj::NativeObjectLifetime::retain(original);
            extra()?;
            self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            // The native worker duplicates its shared private header before
            // converting the supplied operand, which may be that same header.
            if obj::is_shared(original_header) {
                let duplicate = obj::Owned::fresh(obj::duplicate_with_stage_check(
                    original_header,
                    &mut || {
                        extra()?;
                        self.check_original_error_stack_projection(
                            scope.currency(),
                            header,
                            &ticket,
                        )
                        .map_err(tcl_cmd_core::CmdError::from)
                    },
                    &mut |first: &tcl_cmd_core::CmdError| {
                        retain(first.clone());
                    },
                )?);
                crate::list::duplicate_native_backing(
                    original_header,
                    duplicate.as_ptr(),
                    protocol,
                );
                extra()?;
                self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
                let retired = {
                    let mut stack = self.error_stack.borrow_mut();
                    let retired = stack.header.replace(duplicate);
                    stack.renew_incarnation();
                    header = stack.header.as_ref().map(obj::Owned::as_ptr);
                    ticket = std::rc::Rc::clone(&stack.incarnation);
                    retired
                };
                drop(retired);
                extra()?;
                self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            }
            let members = crate::list::list_elements_native_using(
                original,
                protocol,
                &|| {
                    extra()?;
                    self.check_original_error_stack_projection(scope.currency(), header, &ticket)
                        .map_err(tcl_cmd_core::CmdError::from)
                },
                &|error| retain(error.into()),
            );
            extra()?;
            self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            obj::check_native_liveness(original_lifetime.as_ptr())?;
            let members = members?;
            if members.len() % 2 != 0 {
                return Err(
                    ValueError::CommandProtocolUnavailable("validated error-stack pairs").into(),
                );
            }
            let current = || {
                extra()?;
                self.check_original_error_stack_projection(scope.currency(), header, &ticket)
                    .map_err(tcl_cmd_core::CmdError::from)
            };
            let refuse = |error: ValueError| retain(error.into());
            let value = crate::list::replace_elements_native_using(
                header.expect("checked existing header"),
                &members,
                protocol,
                &current,
                &refuse,
            )?;
            current()?;
            let retired = {
                let mut stack = self.error_stack.borrow_mut();
                let retired = stack.header.replace(value);
                stack
                    .metadata
                    .adopt(vec![(); members.len()])
                    .expect("validated pairs");
                stack.renew_incarnation();
                header = stack.header.as_ref().map(obj::Owned::as_ptr);
                ticket = std::rc::Rc::clone(&stack.incarnation);
                retired
            };
            drop(retired);
            drop(original_lifetime);
            drop(header_lifetime);
            extra()?;
            self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            Ok(())
        })();
        let reached = reached.map_err(retain);
        scope.currency().ensure_current_or_refuse()?;
        extra()?;
        reached
    }

    fn check_original_error_stack_projection(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        header: Option<*mut TclObj>,
        ticket: &std::rc::Rc<()>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let current = {
            let stack = self.error_stack.borrow();
            stack.header.as_ref().map(obj::Owned::as_ptr) == header
                && std::rc::Rc::ptr_eq(ticket, &stack.incarnation)
        };
        if !current {
            return Err(self.clone().refuse_completion_value_access(
                ValueError::CommandProtocolUnavailable("changed original error-stack projection"),
            ));
        }
        if let Some(header) = header {
            obj::check_native_liveness(header)
                .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        }
        Ok(())
    }

    pub(super) fn publish_carried_return_options(
        &self,
        mut options: NativeReturnOptions,
    ) -> (std::rc::Rc<()>, Option<obj::NativeObjectLifetime>) {
        options.renew_incarnation();
        let ticket = std::rc::Rc::clone(&options.incarnation);
        let lifetime = options.header().map(obj::NativeObjectLifetime::retain);
        let retired = self.return_options.replace(options);
        drop(retired);
        (ticket, lifetime)
    }

    pub(super) fn check_carried_return_options_publication(
        &self,
        ticket: &std::rc::Rc<()>,
        lifetime: &Option<obj::NativeObjectLifetime>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.check_entered_native_operation()?;
        if !std::rc::Rc::ptr_eq(ticket, &self.return_options.borrow().incarnation) {
            return Err(self.clone().refuse_completion_value_access(
                ValueError::CommandProtocolUnavailable(
                    "carried return options changed during retirement",
                ),
            ));
        }
        if let Some(original) = lifetime {
            obj::check_native_liveness(original.as_ptr())
                .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        }
        Ok(())
    }

    /// Read the existing carried option roles without a return-options borrow
    /// across Dict/key getters. The current header and backing remain exact;
    /// allocation-only leases add no native member or header references.
    pub(crate) fn pending_return_option_objects_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Vec<ReturnOptionPair<obj::Owned>>, tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let (header, protocol, keys, other, ticket) = {
            let options = self.return_options.borrow();
            (
                options.header(),
                options.protocol,
                options.keys.clone(),
                std::rc::Rc::clone(&options.other),
                std::rc::Rc::clone(&options.incarnation),
            )
        };
        let (Some(header), Some(protocol)) = (header, protocol) else {
            return Ok(other.as_ref().clone());
        };
        obj::check_native_liveness(header)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let original = obj::NativeObjectLifetime::retain(header);
        let reached = crate::dict::native_dict_pairs_using(
            header,
            protocol,
            &|| self.check_original_return_option_header(currency, header, &ticket),
            &|error| self.clone().refuse_completion_value_access(error),
        );
        currency.ensure_current_or_refuse()?;
        self.check_original_return_option_header(currency, header, &ticket)?;
        let pairs = reached?;
        let backing = crate::dict::original_dictionary_backing(header)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let lifetimes = pairs
            .iter()
            .map(|&(key, value)| {
                obj::check_native_liveness(key)?;
                obj::check_native_liveness(value)?;
                Ok((
                    obj::NativeObjectLifetime::retain(key),
                    obj::NativeObjectLifetime::retain(value),
                ))
            })
            .collect::<Result<Vec<_>, ValueError>>()
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let mut converted = Vec::with_capacity(pairs.len());
        for &(key, value) in &pairs {
            let reached = crate::dict::native_object_bytes(key, protocol);
            currency.ensure_current_or_refuse()?;
            self.check_original_return_option_header(currency, header, &ticket)?;
            crate::dict::original_dictionary_pairs_current(header, &backing, &pairs)
                .map_err(|error| self.clone().refuse_completion_value_access(error))?;
            obj::check_native_liveness(key)
                .map_err(|error| self.clone().refuse_completion_value_access(error))?;
            obj::check_native_liveness(value)
                .map_err(|error| self.clone().refuse_completion_value_access(error))?;
            let bytes =
                reached.map_err(|error| self.clone().refuse_completion_value_access(error))?;
            converted.push((key, value, bytes));
        }
        let values = keys
            .iter()
            .filter_map(|key| {
                converted.iter().find(|(_, _, bytes)| bytes == key).map(
                    |&(original, value, ref bytes)| ReturnOptionPair {
                        key: obj::Owned::retain(original),
                        value: obj::Owned::retain(value),
                        key_bytes: bytes.clone(),
                    },
                )
            })
            .collect();
        drop(lifetimes);
        drop(original);
        self.check_original_return_option_header(currency, header, &ticket)?;
        Ok(values)
    }

    fn check_original_return_option_header(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        original: *mut TclObj,
        ticket: &std::rc::Rc<()>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        obj::check_native_liveness(original)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        if !{
            let options = self.return_options.borrow();
            options.header() == Some(original) && std::rc::Rc::ptr_eq(ticket, &options.incarnation)
        } {
            return Err(self.clone().refuse_completion_value_access(
                ValueError::CommandProtocolUnavailable(
                    "original return options replaced during getter",
                ),
            ));
        }
        Ok(())
    }

    /// Retain the genuine current private stack role under the caller's original
    /// operation. This is a query hold, not a protective conversion reference.
    pub(crate) fn original_error_stack_value_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<obj::Owned, tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let original = {
            self.error_stack
                .borrow()
                .header
                .as_ref()
                .map(obj::Owned::as_ptr)
        };
        let original = original.ok_or_else(|| {
            self.clone()
                .refuse_completion_value_access(ValueError::CommandProtocolUnavailable(
                    "current original error-stack header",
                ))
        })?;
        obj::check_native_liveness(original)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let original = obj::Owned::retain(original);
        currency.ensure_current_or_refuse()?;
        Ok(original)
    }

    /// Duplicate the actual private return-options header before overlaying live
    /// metadata. Retaining its members alone loses dictionary storage history.
    pub(crate) fn duplicate_original_return_options(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Option<(obj::Owned, NativeStringProtocol)>, tcl_runtime_api::NativeExecutionError>
    {
        currency.ensure_current_or_refuse()?;
        let (original, protocol, ticket) = {
            let options = self.return_options.borrow();
            (
                options.header(),
                options.protocol,
                std::rc::Rc::clone(&options.incarnation),
            )
        };
        let (Some(original), Some(protocol)) = (original, protocol) else {
            return Ok(None);
        };
        self.check_original_return_option_header(currency, original, &ticket)?;
        let lifetime = obj::NativeObjectLifetime::retain(original);
        let backing = crate::dict::original_dictionary_backing(original)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let duplicate = obj::Owned::fresh(obj::duplicate(original));
        let reached = self
            .check_original_return_option_header(currency, original, &ticket)
            .and_then(|()| {
                crate::dict::original_dictionary_backing_current(original, &backing)
                    .map_err(|error| self.clone().refuse_completion_value_access(error))
            });
        if let Err(first) = reached {
            drop(duplicate);
            drop(lifetime);
            currency.ensure_current_or_refuse()?;
            return Err(first);
        }
        drop(lifetime);
        self.check_original_return_option_header(currency, original, &ticket)?;
        Ok(Some((duplicate, protocol)))
    }
}

#[cfg(test)]
mod c9_error_root_generation_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_prepared_stack_tests;

#[cfg(test)]
mod original_carried_option_tests;
