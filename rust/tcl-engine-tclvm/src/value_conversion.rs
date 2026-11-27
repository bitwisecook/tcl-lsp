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

//! Conversion between interface values and VM values: scalars, versions, storage and scalar caches.

use super::{check_projection, projection_error};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_engine_api::{EngineError, Value};
use tcl_vm::Vm;

/// Convert an interface value to the VM's representation.
///
/// A Dictionary retains its member objects without a string round-trip.
// The actual VM receipt spans recursive transport and mandatory cleanup.
pub(super) struct ActualScalarImport<'a> {
    pub(super) vm: RefCell<&'a mut Vm>,
    pub(super) original: &'a tcl_vm::NativeScalarIngress,
}

impl ActualScalarImport<'_> {
    pub(super) fn check(&self) -> Result<(), EngineError> {
        self.vm
            .borrow_mut()
            .check_original_scalar_ingress(self.original)
            .map_err(EngineError::execution_failure)
    }
    pub(super) fn scalar(
        &self,
        facts: &tcl_engine_api::NativeScalarCache,
        resident: Option<(
            Rc<[u8]>,
            tcl_syntax::native_string::NativeStringStorageIdentity,
        )>,
    ) -> Result<tcl_vm::Value, EngineError> {
        self.vm
            .borrow_mut()
            .import_original_native_scalar(self.original, facts, resident)
            .map_err(EngineError::execution_failure)
    }
}

pub(super) fn import_vm_scalar(
    facts: &tcl_engine_api::NativeScalarCache,
    dialect: tcl_native_model::InvocationDialect,
    resident: Option<(
        Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<tcl_vm::Value, EngineError> {
    if let Some(actual) = actual {
        return actual.scalar(facts, resident);
    }
    let (cache, origin) = import_scalar_cache(facts);
    tcl_vm::Value::from_native_scalar_cache_with_storage(cache, origin, dialect, resident)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

#[cfg(test)]
pub(super) fn to_vm_value(
    value: &Value,
    dialect: tcl_native_model::InvocationDialect,
) -> Result<tcl_vm::Value, EngineError> {
    to_vm_value_using(value, dialect, None)
}

pub(super) fn to_vm_value_using(
    value: &Value,
    dialect: tcl_native_model::InvocationDialect,
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<tcl_vm::Value, EngineError> {
    if let Some(actual) = actual {
        actual.check()?;
    }
    let refusal = |error: &dyn std::fmt::Display| EngineError::ExecutionRefusal(error.to_string());
    let reached = (|| {
        Ok(match value {
            Value::Empty => tcl_vm::Value::string(String::new()),
            Value::Str(text) => tcl_vm::Value::from_string_bytes(text.as_bytes()),
            Value::StringBytes(bytes) => tcl_vm::Value::from_string_bytes(bytes.as_ref()),
            Value::ByteArray(bytes) => {
                tcl_vm::Value::from_native_byte_array(Rc::clone(bytes), dialect)
                    .map_err(|error| refusal(&error))?
            }
            Value::NativeScalar(cache) => import_vm_scalar(cache, dialect, None, actual)?,
            Value::Resident {
                value,
                string,
                storage,
            } => {
                let storage =
                    tcl_syntax::scalar_getter::carrier::checked_storage(*storage, string.len())
                        .map_err(|error| refusal(&error))?;
                if let Value::NativeScalar(cache) = value.as_ref() {
                    import_vm_scalar(cache, dialect, Some((Rc::clone(string), storage)), actual)?
                } else {
                    to_vm_value_using(value, dialect, actual)?
                        .with_resident_string_bytes_and_storage(Rc::clone(string), storage)
                        .map_err(|error| refusal(&error))?
                }
            }
            Value::Int(number) => tcl_vm::Value::int(*number),
            Value::Double(number) => tcl_vm::Value::double(*number),
            Value::List(items) => tcl_vm::Value::native_list_constructor(
                items
                    .iter()
                    .map(|value| to_vm_value_using(value, dialect, actual))
                    .collect::<Result<_, _>>()?,
                dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal("List import updater issuer unavailable".into())
                })?,
            ),
            Value::Dict(entries) => tcl_vm::Value::native_dictionary_constructor(
                entries
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            to_vm_value_using(key, dialect, actual)?,
                            to_vm_value_using(value, dialect, actual)?,
                        ))
                    })
                    .collect::<Result<_, EngineError>>()?,
                None,
                dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal(
                        "Dictionary import updater issuer unavailable".into(),
                    )
                })?,
            )
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
        })
    })();
    if let Some(actual) = actual {
        actual.check()?;
    }
    reached
}

/// Convert a VM value to the interface's representation.
///
/// Deliberately string-shaped. The VM's value is dual-rep, and asking it "are
/// you *really* a list" is not a question — it is a *conversion*, which
/// installs a list intrep on every string that happens to parse as one and
/// changes what the body's own later use of that value costs. The host parses
/// what its protocol expects; the boundary reports what the body produced.
pub(super) fn import_version(version: tcl_engine_api::NativeCVersion) -> tcl_dialect::TclVersion {
    tcl_syntax::scalar_getter::carrier::import_version(version)
}

pub(super) fn export_version(version: tcl_dialect::TclVersion) -> tcl_engine_api::NativeCVersion {
    tcl_syntax::scalar_getter::carrier::export_version(version)
}

pub(super) fn import_storage(
    storage: tcl_engine_api::NativeStringStorageIdentity,
) -> tcl_syntax::native_string::NativeStringStorageIdentity {
    tcl_syntax::scalar_getter::carrier::import_storage(storage)
}

pub(super) fn export_storage(value: &tcl_vm::Value) -> tcl_engine_api::NativeStringStorageIdentity {
    tcl_syntax::scalar_getter::carrier::export_storage(
        value
            .resident_string_storage_identity()
            .unwrap_or(tcl_syntax::native_string::NativeStringStorageIdentity::Unknown),
    )
}

fn import_scalar_cache(
    cache: &tcl_engine_api::NativeScalarCache,
) -> (
    tcl_syntax::scalar_getter::NativeScalarCache,
    Option<tcl_dialect::TclVersion>,
) {
    tcl_syntax::scalar_getter::carrier::import_scalar(cache)
}

fn export_scalar_cache(
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    origin: Option<tcl_dialect::TclVersion>,
) -> Result<tcl_engine_api::NativeScalarCache, EngineError> {
    tcl_syntax::scalar_getter::carrier::export_scalar(cache, origin)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

pub(super) fn from_vm_object_value(value: &tcl_vm::Value) -> Result<Value, EngineError> {
    let payload = if let Some(bytes) = value.byte_array_representation() {
        Value::byte_array(bytes)
    } else if let Some(cache) = value.native_scalar_cache() {
        Value::NativeScalar(export_scalar_cache(
            cache,
            value.native_scalar_descriptor_origin(),
        )?)
    } else if value.has_list_representation() {
        if !matches!(
            value.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::List {
                canonical: true,
                ..
            }
        ) {
            return Err(EngineError::ExecutionRefusal(
                "native noncanonical list carrier is unavailable".into(),
            ));
        }
        let (items, _) = value.cached_list_representation().ok_or_else(|| {
            EngineError::ExecutionRefusal("native List carrier cache is unavailable".into())
        })?;
        Value::list(
            items
                .iter()
                .map(from_vm_object_value)
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        // Other primary caches need their own explicit carrier. This view may
        // not convert a stringless object to obtain a convenient representation.
        // A parsed variable name is a lookup cache over the word's own string
        // (a literal `a` an earlier `array set a` named), so the string is all
        // there is to carry.
        if !matches!(
            value.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
                | tcl_syntax::native_object::NativeObjectCacheSnapshot::ParsedVariableName { .. }
        ) {
            return Err(EngineError::ExecutionRefusal(
                "native object cache carrier is unavailable".into(),
            ));
        }
        let Some(bytes) = value.resident_string_bytes() else {
            return Err(EngineError::ExecutionRefusal(
                "native object cache carrier is unavailable".into(),
            ));
        };
        return Ok(Value::Empty.with_resident_string_storage(bytes, export_storage(value)));
    };
    Ok(if let Some(bytes) = value.resident_string_bytes() {
        payload.with_resident_string_storage(bytes, export_storage(value))
    } else {
        payload
    })
}

pub(super) fn from_vm_value_using(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    value: &tcl_vm::Value,
) -> Result<Value, EngineError> {
    vm.check_original_projected_value(original, value)
        .map_err(|error| projection_error(vm, original, error))?;
    if let Some(bytes) = value.byte_array_representation() {
        let payload = Value::byte_array(bytes);
        let result = if let Some(string) = value.resident_string_bytes() {
            payload.with_resident_string_storage(string, export_storage(value))
        } else {
            payload
        };
        check_projection(vm, original)?;
        return Ok(result);
    }
    if let Some(cache) = value.native_scalar_cache() {
        let payload = Value::NativeScalar(export_scalar_cache(
            cache,
            value.native_scalar_descriptor_origin(),
        )?);
        let string = vm
            .original_operand_native_raw_string(original, value)
            .map_err(|error| projection_error(vm, original, error))?;
        return Ok(payload.with_resident_string_storage(string, export_storage(value)));
    }
    vm.original_operand_native_raw_string(original, value)
        .map(Value::string_bytes)
        .map_err(|error| projection_error(vm, original, error))
}

#[cfg(test)]
pub(super) fn from_vm_value(
    value: &tcl_vm::Value,
    dialect: tcl_native_model::InvocationDialect,
) -> Result<Value, EngineError> {
    if let Some(bytes) = value.byte_array_representation() {
        let payload = Value::byte_array(bytes);
        return Ok(if let Some(string) = value.resident_string_bytes() {
            payload.with_resident_string_storage(string, export_storage(value))
        } else {
            payload
        });
    }
    if let Some(cache) = value.native_scalar_cache() {
        let payload = Value::NativeScalar(export_scalar_cache(
            cache,
            value.native_scalar_descriptor_origin(),
        )?);
        // Retain the bridge's string-valued host view while carrying the full
        // primary cache. Numeric string generation does not narrow that cache.
        let string = native_string_bytes(value, dialect)?;
        return Ok(payload.with_resident_string_storage(string, export_storage(value)));
    }
    Ok(Value::string_bytes(native_string_bytes(value, dialect)?))
}

#[cfg(test)]
pub(super) fn native_string_bytes(
    value: &tcl_vm::Value,
    dialect: tcl_native_model::InvocationDialect,
) -> Result<Rc<[u8]>, EngineError> {
    let protocol = dialect.native_string_protocol().ok_or_else(|| {
        EngineError::ExecutionRefusal(
            "native string materialisation protocol is unavailable".into(),
        )
    })?;
    value
        .native_string_bytes(protocol)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}
