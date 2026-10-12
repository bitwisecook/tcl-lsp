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

//! Checked native append into a private indexed String result.
//! Allocation-only handles leave its genuine fresh-zero cleanup role with the
//! shared command owner. No native +1 or COW decision surrounds source effects.

use crate::{
    interp::{native_arguments::OriginalArguments, Interp},
    obj::{self, TclObj},
};
use std::{cell::RefCell, rc::Rc};
use tcl_cmd_core::{
    native_append::{self, NativeAppendObjects},
    CmdError,
};
use tcl_syntax::{
    native_string::{NativeStringProtocol, NativeStringStorageIdentity},
    value::ValueError,
};

#[derive(Clone)]
struct PrivateValue {
    header: obj::NativeObjectLifetime,
    argument: usize,
}
struct PrivateObjects {
    interpreter: Interp,
    arguments: OriginalArguments,
    protocol: NativeStringProtocol,
    failure: RefCell<Option<CmdError>>,
}
impl PrivateObjects {
    fn check(&self, value: &PrivateValue) -> Result<(), CmdError> {
        self.arguments
            .check_original_for(&self.interpreter, value.argument, value.header.as_ptr())
    }
    fn stage<T>(&self, reached: Result<T, CmdError>) -> Result<T, ValueError> {
        if let Err(error) = &reached {
            if let Some(first) = error.native_execution_refusal() {
                self.interpreter
                    .clone()
                    .refuse_native_execution(first.clone());
            }
            if self.failure.borrow().is_none() {
                *self.failure.borrow_mut() = Some(error.clone());
            }
        }
        self.arguments
            .check_for(&self.interpreter)
            .map_err(|_| unwind())?;
        reached.map_err(|_| unwind())
    }
    fn protocol(&self, protocol: NativeStringProtocol) -> Result<(), CmdError> {
        self.arguments.check_for(&self.interpreter)?;
        if protocol != self.protocol {
            return Err(
                ValueError::CommandProtocolUnavailable("private indexed append protocol").into(),
            );
        }
        Ok(())
    }
    fn finish(&self, reached: Result<(), ValueError>) -> Result<(), CmdError> {
        self.arguments.check_for(&self.interpreter)?;
        if let Some(error) = self.failure.borrow_mut().take() {
            return Err(error);
        }
        reached.map_err(Into::into)
    }
}
fn unwind() -> ValueError {
    ValueError::CommandProtocolUnavailable("original private indexed append")
}
impl NativeAppendObjects for PrivateObjects {
    type Value = PrivateValue;
    fn snapshot(
        &self,
        value: &PrivateValue,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        self.stage(self.check(value))?;
        self.stage(obj::native_object_snapshot(value.header.as_ptr()).map_err(Into::into))
    }
    fn prepare_receiver(
        &self,
        _value: &PrivateValue,
        _protocol: NativeStringProtocol,
    ) -> PrivateValue {
        unreachable!("private append has no copy-on-write entry")
    }
    fn duplicate_into(&self, receiver: &PrivateValue, source: &PrivateValue) {
        if self
            .stage(self.check(receiver).and_then(|()| self.check(source)))
            .is_err()
        {
            return;
        }
        obj::duplicate_into(receiver.header.as_ptr(), source.header.as_ptr());
    }
    fn duplicate_into_checked(
        &self,
        receiver: &PrivateValue,
        source: &PrivateValue,
        outer: &mut impl FnMut() -> Result<(), ValueError>,
    ) -> Result<(), ValueError> {
        outer()?;
        let reached = obj::duplicate_into_with_stage_check(
            receiver.header.as_ptr(),
            source.header.as_ptr(),
            &mut || self.check(receiver).and_then(|()| self.check(source)),
            &mut |error: &CmdError| {
                let _ = self.stage::<()>(Err(error.clone()));
            },
        );
        self.stage(reached)?;
        outer()
    }
    fn string(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        self.stage(self.protocol(protocol).and_then(|()| self.check(value)))?;
        self.stage(self.arguments.bytes(value.argument))
    }
    fn unicode(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError> {
        self.stage(self.protocol(protocol).and_then(|()| self.check(value)))?;
        self.stage(obj::native_unicode_units_checked(
            value.header.as_ptr(),
            protocol,
            &mut || self.check(value),
            &mut || self.arguments.bytes(value.argument),
        ))
    }
    fn decode_unicode_source(
        &self,
        protocol: NativeStringProtocol,
        bytes: &[u8],
    ) -> Result<Rc<[u32]>, ValueError> {
        self.stage(self.protocol(protocol))?;
        let version = protocol.tcl_version().ok_or(unwind())?;
        Ok(Rc::from(
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_storage_units(
                bytes,
                tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
            ),
        ))
    }
    fn set_string(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), ValueError> {
        self.stage(self.protocol(protocol).and_then(|()| self.check(value)))?;
        self.stage(obj::set_native_append_string_checked(
            value.header.as_ptr(),
            protocol,
            resident,
            count,
            unicode,
            &mut || self.check(value),
        ))
    }
    fn set_binary(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), ValueError> {
        self.stage(self.protocol(protocol).and_then(|()| self.check(value)))?;
        let recipe = self
            .interpreter
            .native_invocation_dialect()
            .byte_array_string_recipe(None)
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or(unwind())?;
        self.stage(crate::bytearray::set_native_append_bytes_checked(
            value.header.as_ptr(),
            &bytes,
            recipe,
            &mut || self.check(value),
        ))
    }
    fn new_string(&self, _bytes: Rc<[u8]>) -> PrivateValue {
        unreachable!("private append always has its original fresh result receiver")
    }
}
fn objects(
    interpreter: &Interp,
    result: *mut TclObj,
    source: *mut TclObj,
) -> Result<PrivateObjects, CmdError> {
    let arguments = OriginalArguments::enter(interpreter, &[result, source])?;
    let protocol = interpreter
        .native_invocation_dialect()
        .native_string_protocol()
        .filter(|protocol| protocol.tcl_version().is_some())
        .ok_or(ValueError::CommandProtocolUnavailable(
            "private original C append issuer",
        ))?;
    Ok(PrivateObjects {
        interpreter: interpreter.clone(),
        arguments,
        protocol,
        failure: RefCell::new(None),
    })
}
pub(super) fn append_object(
    interpreter: &mut Interp,
    result: *mut TclObj,
    source: *mut TclObj,
) -> Result<(), CmdError> {
    let objects = objects(interpreter, result, source)?;
    let result = PrivateValue {
        header: obj::NativeObjectLifetime::retain(result),
        argument: 0,
    };
    let source = PrivateValue {
        header: obj::NativeObjectLifetime::retain(source),
        argument: 1,
    };
    let recipe = interpreter
        .native_invocation_dialect()
        .native_object_append_protocol(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "private original C append recipe",
        ))?
        .recipe();
    let reached = native_append::append_private_object_checked(
        &objects,
        recipe,
        &result,
        &source,
        &mut || objects.stage(objects.arguments.check_for(interpreter)),
    );
    objects.finish(reached)
}
pub(super) fn append_unicode(
    interpreter: &mut Interp,
    result: *mut TclObj,
    units: &[u32],
) -> Result<(), CmdError> {
    let objects = objects(interpreter, result, result)?;
    let result = PrivateValue {
        header: obj::NativeObjectLifetime::retain(result),
        argument: 0,
    };
    let recipe = interpreter
        .native_invocation_dialect()
        .native_object_append_protocol(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "private original C Unicode append recipe",
        ))?
        .recipe();
    let reached =
        native_append::append_unicode_units_checked(&objects, recipe, &result, units, &mut || {
            objects.stage(objects.arguments.check_for(interpreter))
        });
    objects.finish(reached)
}
