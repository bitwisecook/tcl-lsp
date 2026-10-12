// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original frame/receiver handoff to the existing physical append/Cat kernels.

use super::{RuntimeAppendObjects, RuntimeAppendValue, RuntimeDictionaryChecks};
use crate::obj::{self, TclObj};
use std::{cell::RefCell, rc::Rc};
use tcl_cmd_core::{native_append::NativeAppendObjects, native_cat::NativeCatObjects, CmdError};
use tcl_syntax::{
    native_string::{NativeStringProtocol, NativeStringStorageIdentity},
    value::ValueError,
};

pub(crate) struct OriginalRuntimeAppendObjects {
    base: RuntimeAppendObjects,
    original: Rc<RuntimeDictionaryChecks>,
    first: RefCell<Option<CmdError>>,
}
impl OriginalRuntimeAppendObjects {
    pub(super) fn new(base: RuntimeAppendObjects, original: Rc<RuntimeDictionaryChecks>) -> Self {
        Self {
            base,
            original,
            first: RefCell::new(None),
        }
    }
    pub(crate) fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.base.dialect
    }
    fn current(&self, value: *mut TclObj) -> Result<(), CmdError> {
        self.original.check()?;
        obj::check_native_liveness(value).map_err(|error| (self.original.refuse)(error))
    }
    fn retain(&self, error: CmdError) {
        let first = (self.original.retain)(error);
        if self.first.borrow().is_none() {
            *self.first.borrow_mut() = Some(first);
        }
    }
    fn stage<T>(&self, reached: Result<T, CmdError>) -> Result<T, ValueError> {
        if let Err(error) = &reached {
            self.retain(error.clone());
        }
        self.original.check().map_err(|error| {
            self.retain(error);
            unwind()
        })?;
        reached.map_err(|_| unwind())
    }
    fn value_error(&self, error: ValueError) -> CmdError {
        let error = (self.original.refuse)(error);
        self.retain(error.clone());
        error
    }
    pub(crate) fn finish<T>(&self, reached: Result<T, ValueError>) -> Result<T, CmdError> {
        // Operational errors have already been retained before genuine working
        // cleanup. The same original check transports that first cause.
        self.original.check()?;
        if let Some(first) = self.first.borrow_mut().take() {
            return Err(first);
        }
        reached.map_err(|error| self.value_error(error))
    }
    fn protocol(&self, protocol: NativeStringProtocol) -> Result<(), CmdError> {
        self.original.check()?;
        if self.base.dialect.native_string_protocol() != Some(protocol) {
            return Err(self.value_error(ValueError::CommandProtocolUnavailable(
                "original compiled append protocol",
            )));
        }
        Ok(())
    }
}
fn unwind() -> ValueError {
    ValueError::CommandProtocolUnavailable("original compiled dictionary append")
}
impl NativeAppendObjects for OriginalRuntimeAppendObjects {
    type Value = RuntimeAppendValue;
    fn snapshot(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        self.stage(self.current(value.as_ptr()))?;
        self.stage(
            self.base
                .snapshot(value)
                .map_err(|error| self.value_error(error)),
        )
    }
    fn prepare_receiver(
        &self,
        _value: &Self::Value,
        _protocol: NativeStringProtocol,
    ) -> Self::Value {
        unreachable!("original physical append enters fallible checked COW preparation")
    }
    fn prepare_receiver_checked(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
        check: &mut impl FnMut() -> Result<(), ValueError>,
    ) -> Result<Self::Value, ValueError> {
        self.stage(
            self.protocol(protocol)
                .and_then(|()| self.current(value.as_ptr())),
        )?;
        check()?;
        // The genuine sharing decision precedes acquisition of its working role.
        let prepared = if obj::is_shared(value.as_ptr()) {
            let raw = obj::duplicate_with_stage_check(
                value.as_ptr(),
                &mut || {
                    check()?;
                    self.stage(self.current(value.as_ptr()))
                },
                &mut |error| {
                    self.value_error(error.clone());
                },
            );
            let raw = self.stage(raw.map_err(|error| self.value_error(error)))?;
            crate::list::duplicate_native_backing(value.as_ptr(), raw, protocol);
            RuntimeAppendValue::retain(raw)
        } else {
            RuntimeAppendValue::retain(value.as_ptr())
        };
        self.stage(self.current(prepared.as_ptr()))?;
        check()?;
        Ok(prepared)
    }
    fn retain_append_failure(&self, first: &ValueError) {
        self.value_error(first.clone());
    }
    fn duplicate_into(&self, _receiver: &Self::Value, _source: &Self::Value) {
        unreachable!("original physical append enters checked duplication")
    }
    fn duplicate_into_checked(
        &self,
        receiver: &Self::Value,
        source: &Self::Value,
        check: &mut impl FnMut() -> Result<(), ValueError>,
    ) -> Result<(), ValueError> {
        self.stage(
            self.current(receiver.as_ptr())
                .and_then(|()| self.current(source.as_ptr())),
        )?;
        let reached = obj::duplicate_into_with_stage_check(
            receiver.as_ptr(),
            source.as_ptr(),
            &mut || {
                check()?;
                self.stage(
                    self.current(receiver.as_ptr())
                        .and_then(|()| self.current(source.as_ptr())),
                )
            },
            &mut |error| {
                self.value_error(error.clone());
            },
        );
        self.stage(reached.map_err(|error| self.value_error(error)))
    }
    fn string(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        self.stage(
            self.protocol(protocol)
                .and_then(|()| self.current(value.as_ptr())),
        )?;
        self.stage(self.original.bytes(value.as_ptr()))
    }
    fn unicode(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError> {
        self.stage(
            self.protocol(protocol)
                .and_then(|()| self.current(value.as_ptr())),
        )?;
        let reached = obj::native_unicode_units_checked(
            value.as_ptr(),
            protocol,
            &mut || self.current(value.as_ptr()),
            &mut || self.original.bytes(value.as_ptr()),
        );
        self.stage(reached)
    }
    fn decode_unicode_source(
        &self,
        protocol: NativeStringProtocol,
        bytes: &[u8],
    ) -> Result<Rc<[u32]>, ValueError> {
        self.stage(self.protocol(protocol))?;
        self.stage(
            self.base
                .decode_unicode_source(protocol, bytes)
                .map_err(|error| self.value_error(error)),
        )
    }
    fn set_string(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), ValueError> {
        self.stage(
            self.protocol(protocol)
                .and_then(|()| self.current(value.as_ptr())),
        )?;
        // Preserve the existing physical Unicode-constructor/duplicate route.
        if resident.is_none() && self.base.dialect.native_string_protocol() == Some(protocol) {
            if let Some(unicode) = &unicode {
                let raw = obj::new_native_unicode_obj(unicode.clone(), self.base.dialect)
                    .map_err(|error| self.value_error(error));
                let raw = self.stage(raw)?;
                let fresh = obj::Owned::fresh(raw);
                let reached = obj::duplicate_into_with_stage_check(
                    value.as_ptr(),
                    fresh.as_ptr(),
                    &mut || self.current(value.as_ptr()),
                    &mut |error| {
                        self.retain(error.clone());
                    },
                );
                let reached = self.stage(reached);
                drop(fresh);
                self.stage(self.current(value.as_ptr()))?;
                return reached;
            }
        }
        let reached = obj::set_native_append_string_checked(
            value.as_ptr(),
            protocol,
            resident,
            count,
            unicode,
            &mut || self.current(value.as_ptr()),
        );
        self.stage(reached)
    }
    fn set_binary(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), ValueError> {
        self.stage(
            self.protocol(protocol)
                .and_then(|()| self.current(value.as_ptr())),
        )?;
        let recipe = self
            .base
            .binary_recipe
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or_else(|| {
                self.value_error(ValueError::CommandProtocolUnavailable(
                    "native append binary updater recipe",
                ))
            });
        let recipe = self.stage(recipe)?;
        self.stage(crate::bytearray::set_native_append_bytes_checked(
            value.as_ptr(),
            &bytes,
            recipe,
            &mut || self.current(value.as_ptr()),
        ))
    }
    fn new_string(&self, bytes: Rc<[u8]>) -> Self::Value {
        self.base.new_string(bytes)
    }
}
impl NativeCatObjects for OriginalRuntimeAppendObjects {
    fn is_shared(&self, value: &Self::Value) -> bool {
        self.base.is_shared(value)
    }
    fn empty_binary(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        self.stage(
            self.protocol(protocol)
                .and_then(|()| self.current(value.as_ptr())),
        )?;
        let recipe = self
            .base
            .binary_recipe
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or_else(|| {
                self.value_error(ValueError::CommandProtocolUnavailable(
                    "native compiled concat binary issuer",
                ))
            });
        let recipe = self.stage(recipe)?;
        let version = protocol.tcl_version().ok_or_else(unwind)?;
        let reached = crate::bytearray::native_binary_bytes_checked(
            value.as_ptr(),
            tcl_registry::native_binary_value::NativeBinaryByteConversion::Narrow(
                version.string_character_model(),
            ),
            recipe,
            &mut || self.current(value.as_ptr()),
            &mut || {
                self.original
                    .bytes(value.as_ptr())
                    .map(|bytes| bytes.to_vec())
            },
        );
        self.stage(reached).map(Rc::from)
    }
    fn set_plain_string(&self, value: &Self::Value, bytes: Rc<[u8]>) -> Result<(), ValueError> {
        self.stage(self.current(value.as_ptr()))?;
        unsafe {
            obj::set_string_rep(value.as_ptr(), &bytes);
        }
        let reached = obj::discard_native_internal_representation_with_stage_check(
            value.as_ptr(),
            &mut || self.current(value.as_ptr()),
        );
        self.stage(reached)
    }
}
