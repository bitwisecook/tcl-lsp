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
        string: &dyn Fn(
            *mut TclObj,
            &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
        ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>,
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
                    .save_native_variable_trace_result(false)
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
    fn original_prepared_stack_unavailable_releases_do_not_issue_or_retire_private_headers() {
        // naming.error.original-private-error-stack-header
        // docs/design/analysis/name-resolution-proofs/error-original-private-error-stack-header.md
        // These real releases lack the private stack; they cannot borrow the modern positive premise.
        for name in ["tcl8.4", "tcl8.5"] {
            crate::counters::reset();
            {
                let interpreter = native(name);
                let arm = Rc::new(Cell::new(true));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let member = original(&interpreter, 0, 0, &arm, &events, &published);
                let scope = super::super::native_operation_currency::NativeOperationScope::enter(
                    &interpreter,
                )
                .unwrap();
                let recipe = interpreter
                    .native_invocation_dialect()
                    .native_error_objects_protocol()
                    .unwrap();
                assert!(!recipe.has_error_stack());
                interpreter.configure_native_error_stack_checked().unwrap();
                interpreter.error_stack_log(b"UNAVAILABLE STACK").unwrap();
                assert!(interpreter.error_stack.borrow().original_header().is_none());
                assert!(interpreter
                    .error_stack
                    .borrow()
                    .original_inner_context()
                    .is_none());
                assert!(interpreter.error_stack.borrow().protocol.is_none());
                let refused = interpreter.seed_original_error_stack(member.as_ptr());
                assert!(refused.unwrap_err().native_access_refusal().is_some());
                assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
                assert_eq!(
                    obj::obj_type_ptr(member.as_ptr()),
                    &ORIGINAL as *const obj::TclObjType
                );
                assert!(events.borrow().is_empty());
                assert!(!published.get());
                scope.currency().ensure_current_or_refuse().unwrap();
                clear(&interpreter, &arm);
                drop(member);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }

    #[test]
    fn original_prepared_stack_members_retire_outside_borrows_and_stop_on_episode_aba() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software member frees/tickets; no external Native stack chronology is inferred.
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
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
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
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
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
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

#[cfg(test)]
mod original_carried_option_tests {
    use super::*;
    use crate::interp::native_operation_currency::{
        NativeOperationScope, SavedNativeOperationCurrency,
    };
    use crate::interp::{default_host, Code, ExceptionState, Interp, WeakInterp, GLOBAL};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use tcl_runtime_api::NativeExecutionError;

    struct Observer {
        interpreter: WeakInterp,
        action: u8,
        getter: bool,
        arm: Rc<Cell<bool>>,
        events: Rc<RefCell<Vec<(bool, obj::TclSize)>>>,
        published: Rc<Cell<bool>>,
        nested_refused: Rc<Cell<bool>>,
        level: Rc<Cell<usize>>,
    }
    fn observe(state: &Observer, value: *mut TclObj, getter: bool) {
        if !state.arm.get() {
            return;
        }
        state
            .events
            .borrow_mut()
            .push((getter, unsafe { (*value).ref_count }));
        let Some(mut interpreter) = state.interpreter.upgrade() else {
            return;
        };
        state.level.set(interpreter.frames.borrow().current_level());
        // A genuine callback mutably observes the slot outside the owning
        // writer's RefCell borrow. This is software transport, not Native order.
        state
            .published
            .set(interpreter.return_options.borrow_mut().header().is_none());
        if getter != state.getter {
            return;
        }
        match state.action {
            1 => {
                interpreter.refuse_host_command("first original carried-option callback");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("original-carried-option".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            3 => {
                let currency = SavedNativeOperationCurrency::capture(&interpreter);
                let saved = interpreter.take_return_options();
                let cleanup =
                    NativeOperationScope::enter_saved_cleanup(&currency, &interpreter).unwrap();
                interpreter
                    .restore_return_options_for_cleanup(saved, &cleanup)
                    .unwrap();
            }
            _ => {}
        }
        if !getter && state.action != 0 {
            // Mandatory cleanup must not issue a fresh nested entry after the
            // actual callback withdrew the inherited receipt.
            state
                .nested_refused
                .set(NativeOperationScope::enter(&interpreter).is_err());
        }
    }
    extern "C" fn update(value: *mut TclObj) {
        let state = unsafe { &*(obj::internal_rep(value) as usize as *const Observer) };
        unsafe { obj::set_native_updater_string_rep(value, b"-original", false) };
        observe(state, value, true);
    }
    extern "C" fn retire(value: *mut TclObj) {
        let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Observer) };
        observe(&state, value, false);
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalCarriedOption".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
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
        action: u8,
        getter: bool,
        arm: &Rc<Cell<bool>>,
        events: &Rc<RefCell<Vec<(bool, obj::TclSize)>>>,
        published: &Rc<Cell<bool>>,
        nested_refused: &Rc<Cell<bool>>,
    ) -> obj::Owned {
        let state = Box::new(Observer {
            interpreter: interpreter.downgrade(),
            action,
            getter,
            arm: arm.clone(),
            events: events.clone(),
            published: published.clone(),
            nested_refused: nested_refused.clone(),
            level: Rc::new(Cell::new(usize::MAX)),
        });
        obj::Owned::fresh(obj::alloc_typed(
            &ORIGINAL,
            Box::into_raw(state) as usize as u64,
        ))
    }
    fn cleanup(interpreter: &Interp, arm: &Cell<bool>) {
        arm.set(false);
        drop(
            interpreter
                .return_options
                .replace(NativeReturnOptions::default()),
        );
        drop(interpreter.exc.replace(ExceptionState::default()));
    }

    #[test]
    fn original_carried_option_retirement_stops_new_body_and_preserves_reached_publication() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software owning-member callbacks; no external C/Jim option retirement schedule is inferred.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for action in 0..3 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    let arm = Rc::new(Cell::new(false));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let published = Rc::new(Cell::new(false));
                    let nested = Rc::new(Cell::new(false));
                    let value = original(
                        &interpreter,
                        action,
                        false,
                        &arm,
                        &events,
                        &published,
                        &nested,
                    );
                    interpreter
                        .set_return_option_objects_checked(vec![ReturnOptionPair {
                            key: obj::Owned::fresh(obj::new_string_bytes(b"-original")),
                            value,
                            key_bytes: b"-original".to_vec(),
                        }])
                        .unwrap();
                    interpreter.set_result_bytes(b"PRIOR RESULT");
                    let prior = interpreter.result.get();
                    arm.set(true);
                    let code = interpreter.eval_str(b"set original_later REACHED");
                    assert_eq!(
                        events.borrow().as_slice(),
                        &[(false, 0)],
                        "{name} action{action}"
                    );
                    assert!(published.get());
                    assert!(interpreter.return_options.borrow().header().is_none());
                    assert_eq!(code, if action == 0 { Code::Ok } else { Code::Error });
                    let stored = crate::vars::get_at(
                        &interpreter.frames.borrow(),
                        &interpreter.namespaces.borrow(),
                        b"original_later",
                        0,
                    );
                    if action == 0 {
                        let stored =
                            stored.expect("ordinary option retirement reaches the actual body");
                        assert_eq!(obj::bytes_of(stored), b"REACHED");
                    } else {
                        assert_eq!(stored, None);
                        assert_eq!(interpreter.result.get(), prior);
                    }
                    if action == 0 {
                        assert!(interpreter.native_execution_refusal().is_none());
                    } else {
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(
                            matches!(first, NativeExecutionError::HostCommandRefusal(_)),
                            action == 1
                        );
                        assert!(nested.get());
                        assert_eq!(
                            interpreter.clear_return_options_checked().unwrap_err(),
                            first
                        );
                    }
                    cleanup(&interpreter, &arm);
                }
                assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_carried_option_key_getter_keeps_actual_roles_and_refuses_before_publication() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for action in 0..3 {
                crate::counters::reset();
                {
                    let interpreter = native(name);
                    interpreter
                        .set_return_options_checked(vec![(b"-prior".to_vec(), b"VALUE".to_vec())])
                        .unwrap();
                    let header = interpreter.return_options.borrow().header();
                    let arm = Rc::new(Cell::new(true));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let published = Rc::new(Cell::new(false));
                    let nested = Rc::new(Cell::new(false));
                    let key = original(
                        &interpreter,
                        action,
                        true,
                        &arm,
                        &events,
                        &published,
                        &nested,
                    );
                    let reached =
                        interpreter.set_return_option_objects_checked(vec![ReturnOptionPair {
                            key,
                            value: obj::Owned::fresh(obj::new_string_bytes(b"NEW VALUE")),
                            key_bytes: b"-original".to_vec(),
                        }]);
                    assert_eq!(events.borrow()[0], (true, 1), "{name} action{action}");
                    assert_eq!(reached.is_ok(), action == 0);
                    if action == 0 {
                        assert_ne!(interpreter.return_options.borrow().header(), header);
                    } else {
                        let error = match reached {
                            Err(error) => error,
                            Ok(()) => panic!("original key callback must refuse"),
                        };
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(error.native_execution_refusal(), Some(&first));
                        assert_eq!(interpreter.return_options.borrow().header(), header);
                    }
                    cleanup(&interpreter, &arm);
                }
                assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
            }
        }
    }

    #[test]
    fn original_carried_option_same_header_take_restore_cannot_revive_prepared_read() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // The retained Rust identity cannot wrap or be reused while held; no Native role is issued by it.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            crate::counters::reset();
            {
                let interpreter = native(name);
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let nested = Rc::new(Cell::new(false));
                let key = original(&interpreter, 3, true, &arm, &events, &published, &nested);
                let original_key = key.as_ptr();
                unsafe { obj::set_string_rep(original_key, b"-original") };
                interpreter
                    .set_return_option_objects_checked(vec![ReturnOptionPair {
                        key,
                        value: obj::Owned::fresh(obj::new_string_bytes(b"VALUE")),
                        key_bytes: b"-original".to_vec(),
                    }])
                    .unwrap();
                let (header, ticket) = {
                    let options = interpreter.return_options.borrow();
                    (options.header(), Rc::clone(&options.incarnation))
                };
                obj::invalidate_string(original_key);
                arm.set(true);
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                let reached = interpreter.pending_return_option_objects_checked(scope.currency());
                assert!(reached.is_err());
                assert_eq!(events.borrow().as_slice(), &[(true, 1)]);
                assert_eq!(interpreter.return_options.borrow().header(), header);
                assert!(!Rc::ptr_eq(
                    &ticket,
                    &interpreter.return_options.borrow().incarnation
                ));
                assert_eq!(unsafe { (*original_key).ref_count }, 1);
                let first = interpreter.native_execution_refusal().unwrap();
                assert_eq!(reached.err(), Some(first.clone()));
                assert_eq!(
                    scope.currency().ensure_current_or_refuse().unwrap_err(),
                    first
                );
                cleanup(&interpreter, &arm);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0, "{name}");
        }
    }

    #[test]
    fn original_saved_option_cleanup_finishes_result_restore_after_host_or_context_withdrawal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for action in 0..3 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    interpreter
                        .set_return_options_checked(vec![(b"-saved".to_vec(), b"SAVED".to_vec())])
                        .unwrap();
                    let saved_header = interpreter.return_options.borrow().header();
                    interpreter.set_result_bytes(b"SAVED RESULT");
                    let saved_result = interpreter.result.get();
                    let arm = Rc::new(Cell::new(false));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let published = Rc::new(Cell::new(false));
                    let nested = Rc::new(Cell::new(false));
                    let reached = interpreter.with_preserved_result_checked(|interp| {
                        let value =
                            original(interp, action, false, &arm, &events, &published, &nested);
                        interp
                            .set_return_option_objects_checked(vec![ReturnOptionPair {
                                key: obj::Owned::fresh(obj::new_string_bytes(b"-callback")),
                                value,
                                key_bytes: b"-callback".to_vec(),
                            }])
                            .unwrap();
                        interp.set_result_bytes(b"CALLBACK RESULT");
                        arm.set(true);
                        37u32
                    });
                    assert_eq!(events.borrow().as_slice(), &[(false, 0)]);
                    assert_eq!(interpreter.result.get(), saved_result);
                    assert_eq!(interpreter.return_options.borrow().header(), saved_header);
                    assert_eq!(reached.is_ok(), action == 0);
                    assert_eq!(interpreter.entered_native_operation_count(), 0);
                    if action == 0 {
                        assert_eq!(reached.unwrap(), 37);
                    } else {
                        assert!(nested.get());
                        assert_eq!(reached.err(), interpreter.native_execution_refusal());
                    }
                    cleanup(&interpreter, &arm);
                }
                assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
            }
        }
    }

    #[test]
    fn original_coroutine_option_handoff_renews_installed_and_parked_incarnations() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Mandatory physical handoff software currentness only, no coroutine Native chronology grant.
        let interpreter = native("tcl9.0");
        interpreter
            .set_return_options_checked(vec![(b"-live".to_vec(), b"LIVE".to_vec())])
            .unwrap();
        let live = Rc::clone(&interpreter.return_options.borrow().incarnation);
        let header = interpreter.return_options.borrow().header();
        let mut parked = crate::interp::CoroContext::fresh(GLOBAL);
        let parked_ticket = Rc::clone(&parked.return_options.incarnation);
        interpreter.swap_coro_ctx(&mut parked);
        assert_eq!(parked.return_options.header(), header);
        assert!(!Rc::ptr_eq(&live, &parked.return_options.incarnation));
        assert!(!Rc::ptr_eq(
            &parked_ticket,
            &interpreter.return_options.borrow().incarnation
        ));
        interpreter.swap_coro_ctx(&mut parked);
        assert_eq!(interpreter.return_options.borrow().header(), header);
        assert!(!Rc::ptr_eq(
            &live,
            &interpreter.return_options.borrow().incarnation
        ));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }

    #[test]
    fn original_procedure_option_body_start_refusal_still_tears_down_the_real_frame() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual stored procedure/entered frame and member free; no native procedure chronology inferred.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for action in 0..3 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    assert_eq!(
                        interpreter.eval_str(
                            b"proc original_option_proc {} {set ::original_body_effect REACHED}"
                        ),
                        Code::Ok
                    );
                    let command = obj::Owned::fresh(obj::new_string_bytes(b"original_option_proc"));
                    let (resolved, _) = interpreter
                        .resolve_original_command_at(GLOBAL, command.as_ptr())
                        .unwrap()
                        .unwrap();
                    let crate::interp::Command::Proc(binding) = resolved else {
                        panic!("the actual stored original procedure must be selected");
                    };
                    let arm = Rc::new(Cell::new(false));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let published = Rc::new(Cell::new(false));
                    let nested = Rc::new(Cell::new(false));
                    let value = original(
                        &interpreter,
                        action,
                        false,
                        &arm,
                        &events,
                        &published,
                        &nested,
                    );
                    let level = unsafe {
                        Rc::clone(
                            &(*(obj::internal_rep(value.as_ptr()) as usize as *const Observer))
                                .level,
                        )
                    };
                    interpreter
                        .set_return_option_objects_checked(vec![ReturnOptionPair {
                            key: obj::Owned::fresh(obj::new_string_bytes(b"-original")),
                            value,
                            key_bytes: b"-original".to_vec(),
                        }])
                        .unwrap();
                    let before_level = interpreter.frames.borrow().current_level();
                    let before_ns = interpreter.current_ns.get();
                    let before_depth = interpreter.recursion_depth.get();
                    let before_cmd_frames = interpreter.cmd_frames.borrow().len();
                    let scope = NativeOperationScope::enter(&interpreter).unwrap();
                    arm.set(true);
                    let code = interpreter.call_bound_proc(&binding, &[command.as_ptr()]);
                    assert_eq!(
                        events.borrow().as_slice(),
                        &[(false, 0)],
                        "{name} action{action}"
                    );
                    assert!(
                        level.get() > before_level,
                        "body-start retirement occurs in the genuine frame"
                    );
                    assert_eq!(interpreter.frames.borrow().current_level(), before_level);
                    assert_eq!(interpreter.current_ns.get(), before_ns);
                    assert_eq!(interpreter.recursion_depth.get(), before_depth);
                    assert_eq!(interpreter.cmd_frames.borrow().len(), before_cmd_frames);
                    let effect = crate::vars::get_at(
                        &interpreter.frames.borrow(),
                        &interpreter.namespaces.borrow(),
                        b"::original_body_effect",
                        0,
                    );
                    if action == 0 {
                        assert_eq!(code, Code::Ok);
                        assert_eq!(obj::bytes_of(effect.unwrap()), b"REACHED");
                        scope.currency().ensure_current_or_refuse().unwrap();
                    } else {
                        assert_eq!(code, Code::Error);
                        assert_eq!(effect, None);
                        assert!(nested.get());
                        assert_eq!(
                            scope.currency().ensure_current_or_refuse().err(),
                            interpreter.native_execution_refusal()
                        );
                    }
                    cleanup(&interpreter, &arm);
                    drop(scope);
                    drop(binding);
                }
                assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
            }
        }
    }
}
