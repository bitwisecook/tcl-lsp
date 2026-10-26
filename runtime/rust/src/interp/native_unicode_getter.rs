// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original backend getter entry issued by selected NativeCore construction.
//! Currency authenticates continued use; it does not issue getter authority.

use super::{
    native_operation_currency::{NativeOperationScope, SavedNativeOperationCurrency},
    Interp,
};
use crate::obj;
use std::rc::Rc;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

type UnicodeGetter =
    fn(&Interp, &obj::NativeObjectLifetime, NativeStringProtocol) -> Result<Rc<[u32]>, ValueError>;

/// Not a dialect flag: this owns the actual backend getter entry and its issuer.
/// There is no public constructor or Clone implementation.
pub(super) struct NativeUnicodeGetterEntry {
    original_constructor: SavedNativeOperationCurrency,
    protocol: NativeStringProtocol,
    getter: UnicodeGetter,
}

fn unavailable() -> ValueError {
    ValueError::CommandProtocolUnavailable("selected native Unicode getter entry")
}

impl NativeUnicodeGetterEntry {
    /// Called only after this backend's actual selected constructor completed.
    pub(super) fn issue_after_native_constructor(
        interpreter: &Interp,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        if protocol.tcl_version().is_none() {
            return Err(unavailable());
        }
        let original_constructor = SavedNativeOperationCurrency::capture(interpreter);
        original_constructor
            .check(interpreter)
            .map_err(|_| unavailable())?;
        Ok(Self {
            original_constructor,
            protocol,
            getter: original_backend_unicode,
        })
    }

    fn read(
        &self,
        interpreter: &Interp,
        value: &obj::NativeObjectLifetime,
    ) -> Result<Rc<str>, ValueError> {
        self.original_constructor
            .check(interpreter)
            .map_err(|_| unavailable())?;
        let units = (self.getter)(interpreter, value, self.protocol)?;
        self.original_constructor
            .check(interpreter)
            .map_err(|_| unavailable())?;
        let text = if self
            .protocol
            .tcl_version()
            .is_some_and(|v| v <= tcl_dialect::TclVersion::V8_6)
        {
            let words = units
                .iter()
                .map(|&unit| u16::try_from(unit).map_err(|_| unavailable()))
                .collect::<Result<Vec<_>, _>>()?;
            char::decode_utf16(words)
                .map(|unit| unit.map_err(|_| unavailable()))
                .collect::<Result<String, _>>()?
        } else {
            units
                .iter()
                .map(|&unit| char::from_u32(unit).ok_or_else(unavailable))
                .collect::<Result<String, _>>()?
        };
        self.original_constructor
            .check(interpreter)
            .map_err(|_| unavailable())?;
        Ok(Rc::from(text))
    }
}

/// The actual Rust NativeCore backend getter; no external C ABI is claimed.
fn original_backend_unicode(
    interpreter: &Interp,
    value: &obj::NativeObjectLifetime,
    protocol: NativeStringProtocol,
) -> Result<Rc<[u32]>, ValueError> {
    let scope = NativeOperationScope::enter(interpreter).map_err(|first| {
        interpreter.clone().refuse_native_execution(first);
        unavailable()
    })?;
    let original = value;
    obj::check_native_liveness(original.as_ptr())?;
    let check = || {
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|_| unavailable())?;
        original.checked_ptr().map(|_| ())
    };
    check()?;
    let units = obj::native_unicode_units_checked(
        original.as_ptr(),
        protocol,
        &mut || check(),
        &mut || {
            interpreter.native_object_string_bytes_using(original.as_ptr(), &check, &|error| error)
        },
    )?;
    check()?;
    Ok(units)
}

impl Interp {
    pub(crate) fn has_native_character_view_entry(&self) -> bool {
        self.native_unicode_getter.borrow().is_some()
    }

    /// Separate checked view: the existing ValueOps APIs keep their signatures.
    /// The returned owned text outlives the original object and operation.
    /// Input custody is an actual allocation lease, not an arbitrary raw pointer.
    pub(crate) fn native_character_view(
        &mut self,
        value: &obj::NativeObjectLifetime,
    ) -> Result<Rc<str>, ValueError> {
        // Release the provider-slot borrow before any getter or callback.
        let entry = self
            .native_unicode_getter
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(unavailable)?;
        entry.read(self, value)
    }
}
