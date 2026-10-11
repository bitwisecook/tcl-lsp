// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original private List/Dictionary headers; saved state retains headers, not child copies.

use crate::{obj, obj::TclObj};
use tcl_cmd_core::return_options::ReturnOptionPair;
#[cfg(test)]
use tcl_runtime_api::error_stack::ErrorStackValueError;
use tcl_runtime_api::error_stack::{ErrorStack, ErrorStackFrame};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

#[derive(Clone, Default)]
pub(super) struct NativeReturnOptions {
    header: Option<obj::Owned>,
    protocol: Option<NativeStringProtocol>,
    keys: Vec<Vec<u8>>,
    other: std::rc::Rc<Vec<ReturnOptionPair<obj::Owned>>>,
}
impl NativeReturnOptions {
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
        })
    }
    /// The original instruction retains its merged operand before conversion.
    /// The supplied check belongs to that actual entered operation, not a new
    /// Dict or source issuer. No key getter runs under interpreter state borrows.
    pub(super) fn from_original_checked(
        original: *mut TclObj,
        protocol: NativeStringProtocol,
        check: &dyn Fn() -> Result<(), tcl_runtime_api::NativeExecutionError>,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        check()?;
        obj::check_native_liveness(original)?;
        let header = obj::Owned::retain(original);
        let reached = crate::dict::native_dict_pairs_using(
            original,
            protocol,
            &|| check().map_err(tcl_cmd_core::CmdError::from),
            &tcl_cmd_core::CmdError::from,
        );
        check()?;
        let pairs = reached?;
        let backing = crate::dict::original_dictionary_backing(original)?;
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
            .collect::<Result<Vec<_>, ValueError>>()?;
        let mut keys = Vec::new();
        for &(key, value) in &pairs {
            obj::check_native_liveness(key)?;
            obj::check_native_liveness(value)?;
            let key_lifetime = obj::NativeObjectLifetime::retain(key);
            let reached = crate::dict::native_object_bytes(key, protocol);
            check()?;
            crate::dict::original_dictionary_pairs_current(original, &backing, &pairs)?;
            obj::check_native_liveness(key_lifetime.as_ptr())?;
            let key = reached?;
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
        })
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
    /// `TclGetInnerContext` owns its List independently of the `errorStack` reference.

    /// Append actual producer objects; no child is materialized or reconstructed.

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
        let mut replacement = NativeErrorStack::default();
        replacement.protocol = protocol;
        replacement.header = protocol
            .map(|protocol| obj::Owned::fresh(crate::list::new_list_obj_native(&[], protocol)));
        replacement.inner_context = protocol.map(|protocol| {
            std::rc::Rc::new(std::cell::RefCell::new(obj::Owned::fresh(
                crate::list::new_list_obj_native(&[], protocol),
            )))
        });
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
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
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
            self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            // The native worker duplicates its shared private header before
            // converting the supplied operand, which may be that same header.
            if obj::is_shared(original_header) {
                let duplicate = obj::Owned::fresh(obj::duplicate(original_header));
                crate::list::duplicate_native_backing(
                    original_header,
                    duplicate.as_ptr(),
                    protocol,
                );
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
                self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            }
            let members = crate::list::list_elements_native_using(
                original,
                protocol,
                &|| self.check_original_error_stack_projection(scope.currency(), header, &ticket),
                &|error| self.clone().refuse_completion_value_access(error),
            );
            self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            obj::check_native_liveness(original_lifetime.as_ptr())?;
            let members = members?;
            if members.len() % 2 != 0 {
                return Err(
                    ValueError::CommandProtocolUnavailable("validated error-stack pairs").into(),
                );
            }
            let current =
                || self.check_original_error_stack_projection(scope.currency(), header, &ticket);
            let refuse = |error| self.clone().refuse_completion_value_access(error);
            let value = crate::list::replace_elements_native_checked(
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
            self.check_original_error_stack_projection(scope.currency(), header, &ticket)?;
            Ok(())
        })();
        scope.currency().ensure_current_or_refuse()?;
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

    /// Read the existing carried option roles without a return-options borrow
    /// across Dict/key getters. The current header and backing remain exact;
    /// allocation-only leases add no native member or header references.
    pub(crate) fn pending_return_option_objects_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Vec<ReturnOptionPair<obj::Owned>>, tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let (header, protocol, keys, other) = {
            let options = self.return_options.borrow();
            (
                options.header(),
                options.protocol,
                options.keys.clone(),
                std::rc::Rc::clone(&options.other),
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
            &|| self.check_original_return_option_header(currency, header),
            &|error| self.clone().refuse_completion_value_access(error),
        );
        currency.ensure_current_or_refuse()?;
        self.check_original_return_option_header(currency, header)?;
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
            self.check_original_return_option_header(currency, header)?;
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
        currency.ensure_current_or_refuse()?;
        Ok(values)
    }

    fn check_original_return_option_header(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        original: *mut TclObj,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        obj::check_native_liveness(original)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        if self.return_options.borrow().header() != Some(original) {
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
        let (original, protocol) = {
            let options = self.return_options.borrow();
            (options.header(), options.protocol)
        };
        let (Some(original), Some(protocol)) = (original, protocol) else {
            return Ok(None);
        };
        self.check_original_return_option_header(currency, original)?;
        let lifetime = obj::NativeObjectLifetime::retain(original);
        let backing = crate::dict::original_dictionary_backing(original)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let duplicate = obj::Owned::fresh(obj::duplicate(original));
        let reached = self
            .check_original_return_option_header(currency, original)
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
        currency.ensure_current_or_refuse()?;
        Ok(Some((duplicate, protocol)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    #[test]
    fn private_headers_save_without_duplicate_member_references() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let recipe = tcl_registry::InvocationDialect::for_version(version)
                .native_error_objects_protocol()
                .unwrap();
            let key = obj::Owned::fresh(obj::new_string_bytes(b"-custom"));
            let value = obj::Owned::fresh(obj::new_string_bytes(b"original\0\xff"));
            let key_ptr = key.as_ptr();
            let value_ptr = value.as_ptr();
            let options = NativeReturnOptions::new(
                vec![ReturnOptionPair {
                    key,
                    value,
                    key_bytes: b"-custom".to_vec(),
                }],
                Some(recipe),
            )
            .unwrap();
            let header = options.header().unwrap();
            // SAFETY: these objects are borrowed from the live private Dict owner.
            unsafe {
                assert_eq!((*header).ref_count, 1);
                assert_eq!((*key_ptr).ref_count, 1);
                assert_eq!((*value_ptr).ref_count, 1);
            }
            let saved = options.clone();
            unsafe {
                assert_eq!((*header).ref_count, 2);
                assert_eq!((*value_ptr).ref_count, 1);
            }
            drop(options);
            unsafe {
                assert_eq!((*header).ref_count, 1);
                assert_eq!((*value_ptr).ref_count, 1);
            }
            let snapshot = saved.pairs();
            assert_eq!(snapshot[0].key.as_ptr(), key_ptr);
            assert_eq!(snapshot[0].value.as_ptr(), value_ptr);
            drop(snapshot);
            if !recipe.has_error_stack() {
                continue;
            }
            let mut stack = NativeErrorStack::default();
            stack.configure(Some(recipe));
            let header = stack.header.as_ref().unwrap().as_ptr();
            assert!(obj::obj_type_ptr(header).is_null());
            stack.begin_inner(b"INNER".to_vec(), b"original".to_vec());
            assert_eq!(stack.header.as_ref().unwrap().as_ptr(), header);
            let parts =
                crate::list::list_elements_native_checked(header, recipe.strings()).unwrap();
            let child = parts[1];
            let saved = stack.clone();
            unsafe {
                assert_eq!((*header).ref_count, 2);
                assert_eq!((*child).ref_count, 1);
            }
            stack.mark_reset();
            stack.begin_inner(b"INNER".to_vec(), b"callback".to_vec());
            assert_ne!(stack.header.as_ref().unwrap().as_ptr(), header);
            unsafe {
                assert_eq!((*header).ref_count, 1);
                assert_eq!((*child).ref_count, 1);
            }
            stack = saved;
            assert_eq!(stack.header.as_ref().unwrap().as_ptr(), header);
            assert_eq!(stack.entries()[1], b"original");
        }
    }
    #[test]
    fn explicit_stack_preserves_original_member_primary_without_string_getters() {
        let recipe = tcl_registry::InvocationDialect::for_version(TclVersion::V9_0)
            .native_error_objects_protocol()
            .unwrap();
        let member = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[obj::new_string_bytes(b"original")],
            recipe.strings(),
        ));
        let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
        let supplied = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[tag.as_ptr(), member.as_ptr()],
            recipe.strings(),
        ));
        let mut stack = NativeErrorStack::default();
        stack.configure(Some(recipe));
        let identity = stack.header.as_ref().unwrap().as_ptr();
        stack.adopt_original(supplied.as_ptr()).unwrap();
        assert_eq!(stack.header.as_ref().unwrap().as_ptr(), identity);
        assert!(!obj::has_string_rep(member.as_ptr()));
        let parts = crate::list::list_elements_native_checked(identity, recipe.strings()).unwrap();
        assert_eq!(parts[1], member.as_ptr());
        let getter = stack.value();
        assert_eq!(getter.as_ptr(), identity);
        stack
            .adopt(vec![b"INNER".to_vec(), b"second".to_vec()])
            .unwrap();
        assert_ne!(stack.header.as_ref().unwrap().as_ptr(), identity);
        let parts =
            crate::list::list_elements_native_checked(getter.as_ptr(), recipe.strings()).unwrap();
        assert_eq!(parts[1], member.as_ptr());
    }
}

#[cfg(test)]
mod original_prepared_stack_tests {
    use super::*;
    use crate::interp::{default_host, ExceptionState, Interp, WeakInterp};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct RetiredMember {
        interpreter: WeakInterp,
        role: usize,
        mode: u8,
        arm: Rc<Cell<bool>>,
        events: Rc<RefCell<Vec<(usize, obj::TclSize)>>>,
        published: Rc<Cell<bool>>,
    }
    extern "C" fn retire(value: *mut TclObj) {
        let state =
            unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut RetiredMember) };
        if !state.arm.get() {
            return;
        }
        state
            .events
            .borrow_mut()
            .push((state.role, unsafe { (*value).ref_count }));
        let Some(mut interpreter) = state.interpreter.upgrade() else {
            return;
        };
        // A real member/header free may inspect the current physical roles;
        // neither the stack slot nor innerContext can be borrowed by its caller.
        let (header, inner) = {
            let stack = interpreter.error_stack.borrow_mut();
            (stack.original_header(), stack.original_inner_context())
        };
        state.published.set(header.is_some() && inner.is_some());
        if state.role == 1 || interpreter.native_execution_refusal().is_some() {
            return;
        }
        match state.mode {
            1 => {
                interpreter.refuse_host_command("original prepared stack first cause");
            }
            2 => {
                let saved = interpreter.runtime_context();
                let mut changed = saved.clone();
                changed.packages = vec![("original-prepared-stack".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&saved).unwrap();
            }
            3 => {
                let saved = interpreter
                    .save_native_variable_trace_result(true)
                    .unwrap()
                    .unwrap();
                interpreter.mark_error_stack_reset();
                interpreter
                    .restore_native_variable_trace_result(saved)
                    .unwrap();
            }
            _ => {}
        }
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalPreparedStackMember".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    fn native(name: &str) -> Interp {
        let environment = tcl_registry::model::ingress::resolve_known_environment(name).unwrap();
        Interp::with_native_core(
            default_host(),
            environment.unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn original(
        interpreter: &Interp,
        role: usize,
        mode: u8,
        arm: &Rc<Cell<bool>>,
        events: &Rc<RefCell<Vec<(usize, obj::TclSize)>>>,
        published: &Rc<Cell<bool>>,
    ) -> obj::Owned {
        let state = Box::new(RetiredMember {
            interpreter: interpreter.downgrade(),
            role,
            mode,
            arm: arm.clone(),
            events: events.clone(),
            published: published.clone(),
        });
        obj::Owned::fresh(obj::alloc_typed(
            &ORIGINAL,
            Box::into_raw(state) as usize as u64,
        ))
    }
    fn clear(interpreter: &Interp, arm: &Cell<bool>) {
        arm.set(false);
        let old = interpreter.error_stack.replace(NativeErrorStack::default());
        drop(old);
        let old = interpreter.exc.replace(ExceptionState::default());
        drop(old);
    }

    #[test]
    fn original_prepared_stack_members_retire_outside_borrows_and_stop_on_episode_aba() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software member frees/tickets; no external Native stack chronology is inferred.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for operation in 0..2 {
                for mode in 0..4 {
                    crate::counters::reset();
                    {
                        let mut interpreter = native(name);
                        interpreter.set_result_bytes(b"RETAINED RESULT");
                        let result = interpreter.result.get();
                        interpreter.error_line.set(37);
                        let arm = Rc::new(Cell::new(true));
                        let events = Rc::new(RefCell::new(Vec::new()));
                        let published = Rc::new(Cell::new(false));
                        let member = original(&interpreter, 0, mode, &arm, &events, &published);
                        let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
                        let protocol = interpreter
                            .native_invocation_dialect()
                            .native_error_objects_protocol()
                            .unwrap()
                            .strings();
                        let replacement = obj::Owned::fresh(crate::list::new_list_obj_native(
                            &[tag.as_ptr(), member.as_ptr()],
                            protocol,
                        ));
                        let old = {
                            let mut stack = interpreter.error_stack.borrow_mut();
                            stack.renew_incarnation();
                            if operation == 0 {
                                stack.metadata.begin_inner((), ());
                                stack.header.replace(replacement)
                            } else {
                                stack.metadata.mark_reset();
                                Some(stack.inner_context.as_ref().unwrap().replace(replacement))
                            }
                        };
                        drop(old);
                        drop(member);
                        drop(tag);
                        let scope =
                            super::super::native_operation_currency::NativeOperationScope::enter(
                                &interpreter,
                            )
                            .unwrap();
                        let operand = obj::Owned::fresh(obj::new_string_bytes(b"ORIGINAL OPERAND"));
                        let reached = if operation == 0 {
                            interpreter.adopt_error_stack_bytes_checked(&[
                                b"INNER".to_vec(),
                                b"NEW".to_vec(),
                            ])
                        } else {
                            interpreter.capture_original_return_instruction_context(
                                tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                                operand.as_ptr(), operand.as_ptr())
                        };
                        assert_eq!(
                            events.borrow().as_slice(),
                            &[(0, 0)],
                            "{name} op{operation} mode{mode}"
                        );
                        assert!(published.get());
                        assert_eq!(
                            reached.is_ok(),
                            mode == 0,
                            "{name} op{operation} mode{mode}"
                        );
                        assert_eq!(interpreter.result.get(), result);
                        assert_eq!(interpreter.error_line.get(), 37);
                        if mode == 0 {
                            scope.currency().ensure_current_or_refuse().unwrap();
                            let stack = interpreter.error_stack.borrow();
                            assert!(!stack.is_reset());
                            let header = stack.original_header().unwrap();
                            let members =
                                crate::list::list_elements_native_checked(header, protocol)
                                    .unwrap();
                            assert_eq!(members.len(), 2);
                            if operation == 1 {
                                let inner = stack.original_inner_context().unwrap();
                                assert_eq!(members[1], inner);
                                assert_eq!(unsafe { (*inner).ref_count }, 2);
                                let members =
                                    crate::list::list_elements_native_checked(inner, protocol)
                                        .unwrap();
                                assert_eq!(members.len(), 3);
                                assert_eq!(&members[1..], &[operand.as_ptr(), operand.as_ptr()]);
                                assert_eq!(unsafe { (*operand.as_ptr()).ref_count }, 3);
                            }
                        } else {
                            let first = interpreter.native_execution_refusal().unwrap();
                            assert_eq!(reached.unwrap_err(), first);
                            assert_eq!(interpreter.capture_original_return_instruction_context(
                                tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                                operand.as_ptr(), operand.as_ptr()).unwrap_err(), first);
                            assert_eq!(events.borrow().as_slice(), &[(0, 0)]);
                        }
                        clear(&interpreter, &arm);
                        drop(operand);
                        drop(scope);
                    }
                    assert_eq!(crate::counters::finalize(), 0);
                    assert_eq!(crate::counters::double_free_count(), 0);
                }
            }
        }
    }

    #[test]
    fn original_stack_configuration_publishes_roles_before_mandatory_header_retirement() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Generic configuration software publication and cleanup, not selected Reset/Setter order.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in 0..4 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    interpreter.set_result_bytes(b"PRIOR CONFIGURATION RESULT");
                    let result = interpreter.result.get();
                    let arm = Rc::new(Cell::new(true));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let published = Rc::new(Cell::new(false));
                    let header = original(&interpreter, 0, mode, &arm, &events, &published);
                    let inner = original(&interpreter, 1, mode, &arm, &events, &published);
                    let old = interpreter.error_stack.replace(NativeErrorStack {
                        header: Some(header),
                        inner_context: Some(Rc::new(RefCell::new(inner))),
                        ..NativeErrorStack::default()
                    });
                    drop(old);
                    let scope =
                        super::super::native_operation_currency::NativeOperationScope::enter(
                            &interpreter,
                        )
                        .unwrap();
                    let reached = interpreter.configure_native_error_stack_checked();
                    assert_eq!(
                        events.borrow().as_slice(),
                        &[(0, 0), (1, 0)],
                        "{name} mode{mode}"
                    );
                    assert!(published.get());
                    assert_eq!(reached.is_ok(), mode == 0);
                    assert_eq!(interpreter.result.get(), result);
                    assert!(interpreter.error_stack.borrow().original_header().is_some());
                    assert!(interpreter
                        .error_stack
                        .borrow()
                        .original_inner_context()
                        .is_some());
                    if mode == 0 {
                        scope.currency().ensure_current_or_refuse().unwrap();
                    } else {
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(reached.unwrap_err(), first);
                        assert_eq!(
                            interpreter
                                .configure_native_error_stack_checked()
                                .unwrap_err(),
                            first
                        );
                    }
                    clear(&interpreter, &arm);
                    drop(scope);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_stack_frame_append_keeps_existing_members_and_actual_frame_roles() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Source-only actual List/member roles; no public getter or frame-admission grant.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            crate::counters::reset();
            {
                let interpreter = native(name);
                let arm = Rc::new(Cell::new(true));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let inner = original(&interpreter, 0, 0, &arm, &events, &published);
                let frame = original(&interpreter, 1, 0, &arm, &events, &published);
                let scope = super::super::native_operation_currency::NativeOperationScope::enter(
                    &interpreter,
                )
                .unwrap();
                interpreter.capture_original_return_instruction_context(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                    inner.as_ptr(), inner.as_ptr()).unwrap();
                let header = interpreter.error_stack.borrow().original_header().unwrap();
                let protocol = interpreter
                    .native_invocation_dialect()
                    .native_error_objects_protocol()
                    .unwrap()
                    .strings();
                let before = crate::list::list_elements_native_checked(header, protocol).unwrap();
                interpreter
                    .log_original_error_stack_frame_checked(ErrorStackFrame::Call(frame.clone()))
                    .unwrap();
                let current = interpreter.error_stack.borrow().original_header().unwrap();
                assert_eq!(current, header);
                let after = crate::list::list_elements_native_checked(current, protocol).unwrap();
                assert_eq!(&after[..2], before.as_slice());
                assert_eq!(after.len(), 4);
                assert_eq!(after[3], frame.as_ptr());
                assert_eq!(unsafe { (*frame.as_ptr()).ref_count }, 2);
                assert!(events.borrow().is_empty());
                scope.currency().ensure_current_or_refuse().unwrap();
                clear(&interpreter, &arm);
                drop(frame);
                drop(inner);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}
