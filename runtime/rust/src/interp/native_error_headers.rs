// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original private List/Dictionary headers; saved state retains headers, not child copies.

use crate::{obj, obj::TclObj};
use tcl_cmd_core::return_options::ReturnOptionPair;
use tcl_runtime_api::error_stack::{ErrorStack, ErrorStackFrame, ErrorStackValueError};
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
    fn replace_objects(&mut self, values: &[*mut TclObj]) {
        self.renew_incarnation();
        if let (Some(header), Some(protocol)) = (&self.header, self.protocol) {
            self.header = Some(
                crate::list::replace_elements_native(header.as_ptr(), values, protocol)
                    .expect("private native List header"),
            );
        }
    }
    fn replace_bytes(&mut self, values: &[Vec<u8>]) {
        let values: Vec<_> = values
            .iter()
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
            .collect();
        self.replace_objects(&values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>());
    }
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
    pub(super) fn begin_inner(&mut self, tag: Vec<u8>, context: Vec<u8>) -> bool {
        if !self.is_reset() {
            return false;
        }
        self.replace_bytes(&[tag.clone(), context.clone()]);
        self.metadata.begin_inner((), ())
    }
    /// `TclGetInnerContext` owns its List independently of the `errorStack` reference.
    pub(super) fn begin_instruction(
        &mut self,
        name: tcl_syntax::native_instruction_name::NativeInstructionName,
        operands: &[*mut TclObj],
    ) {
        if !self.is_reset() {
            return;
        }
        self.renew_incarnation();
        let protocol = self.protocol.expect("selected original innerContext");
        let instruction = obj::Owned::fresh(obj::native_instruction_name::fresh(name));
        let mut values = Vec::with_capacity(operands.len() + 1);
        values.push(instruction.as_ptr());
        values.extend_from_slice(operands);
        let context = crate::list::replace_elements_native(
            self.inner_context
                .as_ref()
                .expect("original interpreter innerContext")
                .borrow()
                .as_ptr(),
            &values,
            protocol,
        )
        .expect("original native innerContext List");
        let context_ptr = context.as_ptr();
        let retired = self
            .inner_context
            .as_ref()
            .expect("original innerContext role")
            .replace(context);
        drop(retired);
        let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
        self.replace_objects(&[tag.as_ptr(), context_ptr]);
        self.metadata.begin_inner((), ());
    }
    /// Append actual producer objects; no child is materialized or reconstructed.
    pub(super) fn log_original_frame(&mut self, frame: ErrorStackFrame<obj::Owned>) -> bool {
        if self.is_reset() {
            return false;
        }
        let (tag, value) = match frame {
            ErrorStackFrame::Unreported => return false,
            ErrorStackFrame::Redirect(value) => (b"UP".as_slice(), value),
            ErrorStackFrame::Call(value) => (b"CALL".as_slice(), value),
        };
        self.renew_incarnation();
        if let (Some(header), Some(protocol)) = (&self.header, self.protocol) {
            let tag = obj::Owned::fresh(obj::new_string_bytes(tag));
            self.header = Some(
                crate::list::append_native_elements(
                    Some(header.as_ptr()),
                    &[tag.as_ptr(), value.as_ptr()],
                    protocol,
                )
                .expect("private native List header"),
            );
        }
        self.metadata.push_pair((), ())
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
    pub(super) fn value(&self) -> obj::Owned {
        self.header
            .as_ref()
            .expect("selected private error-stack header")
            .clone()
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

    /// Duplicate the actual private return-options header before overlaying live
    /// metadata. Retaining its members alone loses dictionary storage history.
    pub(crate) fn duplicate_original_return_options(
        &self,
    ) -> Option<(obj::Owned, NativeStringProtocol)> {
        let options = self.return_options.borrow();
        let original = options.header()?;
        let protocol = options.protocol?;
        Some((obj::Owned::fresh(obj::duplicate(original)), protocol))
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
