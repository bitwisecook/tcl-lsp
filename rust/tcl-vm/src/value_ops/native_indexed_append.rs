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
//! Its owning result stays with `CmdCore`; append transports retain allocation
//! safety only and cannot introduce an extra native reference or COW stage.

use crate::{
    interp::{
        Vm, native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
    },
    value::{NativeObjectLifetimeLease, Value},
};
use std::{cell::RefCell, rc::Rc};
use tcl_cmd_core::{
    CmdError,
    native_append::{self, NativeAppendObjects},
};
use tcl_syntax::{
    native_string::{NativeStringProtocol, NativeStringStorageIdentity},
    value::ValueError,
};

#[derive(Clone)]
struct PrivateValue {
    header: NativeObjectLifetimeLease,
    argument: usize,
}
struct PrivateObjects<'a> {
    numeric: OriginalNumericOps<'a>,
    originals: [NativeObjectLifetimeLease; 2],
    protocol: NativeStringProtocol,
    failure: RefCell<Option<CmdError>>,
}
impl PrivateObjects<'_> {
    fn check(&self, value: &PrivateValue) -> Result<(), CmdError> {
        self.numeric.current_for_value(value.header.value())?;
        if self.originals[value.argument]
            .value()
            .native_object_identity()
            != value.header.value().native_object_identity()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "private indexed append original header",
            )
            .into());
        }
        Ok(())
    }
    fn stage<T>(
        &self,
        _value: &PrivateValue,
        reached: Result<T, CmdError>,
    ) -> Result<T, ValueError> {
        if let Err(error) = &reached {
            self.numeric.retain_failure(error);
            if self.failure.borrow().is_none() {
                *self.failure.borrow_mut() = Some(error.clone());
            }
        }
        self.numeric.current().map_err(|_| unwind())?;
        reached.map_err(|_| unwind())
    }
    fn protocol(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
    ) -> Result<(), ValueError> {
        self.stage(value, self.check(value))?;
        if protocol != self.protocol {
            return Err(unwind());
        }
        Ok(())
    }
    fn finish(&self, reached: Result<(), ValueError>) -> Result<(), CmdError> {
        self.numeric.current()?;
        if let Some(error) = self.failure.borrow_mut().take() {
            return Err(error);
        }
        reached.map_err(Into::into)
    }
}
fn unwind() -> ValueError {
    ValueError::CommandProtocolUnavailable("original private indexed append")
}
impl NativeAppendObjects for PrivateObjects<'_> {
    type Value = PrivateValue;
    fn snapshot(
        &self,
        value: &PrivateValue,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        self.stage(value, self.check(value))?;
        Ok(value.header.value().native_object_snapshot())
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
            .stage(
                receiver,
                self.check(receiver).and_then(|()| self.check(source)),
            )
            .is_err()
        {
            return;
        }
        crate::value::VmAppendObjects
            .duplicate_into(receiver.header.value(), source.header.value());
    }
    fn duplicate_into_checked(
        &self,
        receiver: &PrivateValue,
        source: &PrivateValue,
        outer: &mut impl FnMut() -> Result<(), ValueError>,
    ) -> Result<(), ValueError> {
        outer()?;
        let reached = receiver
            .header
            .value()
            .native_duplicate_into_checked(source.header.value(), &mut || {
                self.check(receiver).and_then(|()| self.check(source))
            });
        self.stage(receiver, reached)?;
        outer()
    }
    fn string(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        self.protocol(value, protocol)?;
        self.stage(value, self.numeric.string(value.header.value(), protocol))
    }
    fn unicode(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError> {
        self.protocol(value, protocol)?;
        self.stage(
            value,
            value.header.value().native_unicode_units_checked(
                protocol,
                &mut || self.check(value),
                &mut || self.numeric.string(value.header.value(), protocol),
            ),
        )
    }
    fn decode_unicode_source(
        &self,
        protocol: NativeStringProtocol,
        bytes: &[u8],
    ) -> Result<Rc<[u32]>, ValueError> {
        self.numeric.current().map_err(|_| unwind())?;
        if protocol != self.protocol {
            return Err(unwind());
        }
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
        self.protocol(value, protocol)?;
        self.stage(
            value,
            value.header.value().native_append_string_checked(
                protocol,
                resident,
                count,
                unicode,
                &mut || self.check(value),
            ),
        )
    }
    fn set_binary(
        &self,
        value: &PrivateValue,
        protocol: NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), ValueError> {
        self.protocol(value, protocol)?;
        self.stage(
            value,
            value
                .header
                .value()
                .native_append_binary_checked(protocol, bytes, &mut || self.check(value)),
        )
    }
    fn new_string(&self, _bytes: Rc<[u8]>) -> PrivateValue {
        unreachable!("private append always has its original fresh result receiver")
    }
}
pub(super) fn append_object(vm: &mut Vm, result: &Value, source: &Value) -> Result<(), CmdError> {
    let scope = NativeOperationScope::enter(vm)?;
    let numeric = OriginalNumericOps::in_original_scope(vm, &scope)?;
    let recipe = numeric
        .dialect()
        .native_object_append_protocol(None)
        .filter(|purpose| purpose.recipe().string_protocol().tcl_version().is_some())
        .ok_or(ValueError::CommandProtocolUnavailable(
            "private original C append issuer",
        ))?
        .recipe();
    let result = PrivateValue {
        header: result.native_lifetime_lease(),
        argument: 0,
    };
    let source = PrivateValue {
        header: source.native_lifetime_lease(),
        argument: 1,
    };
    let objects = PrivateObjects {
        numeric,
        originals: [result.header.clone(), source.header.clone()],
        protocol: recipe.string_protocol(),
        failure: RefCell::new(None),
    };
    let reached = native_append::append_private_object_checked(
        &objects,
        recipe,
        &result,
        &source,
        &mut || objects.stage(&result, objects.numeric.current()),
    );
    objects.finish(reached)
}
pub(super) fn append_unicode(vm: &mut Vm, result: &Value, units: &[u32]) -> Result<(), CmdError> {
    let scope = NativeOperationScope::enter(vm)?;
    let numeric = OriginalNumericOps::in_original_scope(vm, &scope)?;
    let recipe = numeric
        .dialect()
        .native_object_append_protocol(None)
        .filter(|purpose| purpose.recipe().string_protocol().tcl_version().is_some())
        .ok_or(ValueError::CommandProtocolUnavailable(
            "private original C Unicode append issuer",
        ))?
        .recipe();
    let result = PrivateValue {
        header: result.native_lifetime_lease(),
        argument: 0,
    };
    let objects = PrivateObjects {
        numeric,
        originals: [result.header.clone(), result.header.clone()],
        protocol: recipe.string_protocol(),
        failure: RefCell::new(None),
    };
    let reached =
        native_append::append_unicode_units_checked(&objects, recipe, &result, units, &mut || {
            objects.stage(&result, objects.numeric.current())
        });
    objects.finish(reached)
}
