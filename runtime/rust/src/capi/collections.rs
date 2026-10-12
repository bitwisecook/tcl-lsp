// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C API collection/table reads retain header lifetime and entered currency.

use super::*;
use crate::interp::native_operation_currency::{NativeOperationCurrency, NativeOperationScope};
use crate::list::append_list_element;
use crate::obj::TclObjType;
use tcl_cmd_core::prefix::{self, Resolution};
use tcl_cmd_core::CmdError;
use tcl_registry::native_index_lookup::NativeIndexLookupProtocol;
use tcl_core_types::NativeIndexLookupFlags;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

struct Operation {
    interpreter: Interp,
    scope: NativeOperationScope,
    lifetimes: Vec<obj::NativeObjectLifetime>,
}

impl Operation {
    fn enter(interpreter: Option<&Interp>, originals: &[*mut TclObj]) -> Result<Self, CmdError> {
        let interpreter = interpreter.ok_or(ValueError::CommandProtocolUnavailable(
            "missing original C API collection interpreter issuer",
        ))?;
        let scope =
            NativeOperationScope::enter(interpreter).map_err(CmdError::from_execution_refusal)?;
        let lifetimes = originals
            .iter()
            .map(|&original| {
                obj::check_native_liveness(original)?;
                Ok(obj::NativeObjectLifetime::retain(original))
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let operation = Self {
            interpreter: interpreter.clone(),
            scope,
            lifetimes,
        };
        operation.check()?;
        Ok(operation)
    }

    fn currency(&self) -> &NativeOperationCurrency {
        self.scope.currency()
    }

    fn check(&self) -> Result<(), CmdError> {
        self.currency()
            .ensure_current_or_refuse()
            .map_err(CmdError::from_execution_refusal)?;
        for original in &self.lifetimes {
            obj::check_native_liveness(original.as_ptr())?;
        }
        Ok(())
    }

    fn protocol(&self) -> Result<NativeStringProtocol, CmdError> {
        self.check()?;
        self.interpreter
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or_else(|| {
                ValueError::CommandProtocolUnavailable("original C API String protocol").into()
            })
    }

    fn string(&self, original: *mut TclObj) -> Result<Vec<u8>, CmdError> {
        let result = crate::dict::native_object_bytes(original, self.protocol()?);
        self.check()?;
        Ok(result?)
    }

    fn list_protocol(&self) -> Result<NativeStringProtocol, CmdError> {
        let protocol = self.protocol()?;
        let pointers: Vec<_> = self
            .lifetimes
            .iter()
            .map(obj::NativeObjectLifetime::as_ptr)
            .collect();
        let result = self.interpreter.associate_native_jim_arguments(&pointers);
        self.check()?;
        result?;
        Ok(protocol)
    }

    fn publish(&mut self, error: CmdError) -> c_int {
        let error = self.check().err().unwrap_or(error);
        let guest = error.native_execution_refusal().is_none();
        self.interpreter.report_cmd_error(error);
        if guest && self.check().is_ok() {
            self.interpreter.note_c_api_error();
        }
        TCL_ERROR
    }
}

unsafe fn enter(interpreter: *mut Interp, originals: &[*mut TclObj]) -> Result<Operation, c_int> {
    // SAFETY: each public facade forwards its original nullable interpreter contract.
    Operation::enter(unsafe { interpreter.as_ref() }, originals).map_err(|error| {
        if let Some(interpreter) = unsafe { interpreter.as_mut() } {
            interpreter.report_cmd_error(error);
        }
        TCL_ERROR
    })
}

/// Probe the original List view with an explicit interpreter issuer. Allocation
/// leases preserve rc0/COW decisions; the returned window stays owned by the header.
///
/// # Errors
/// Returns the actual first Host refusal, missing issuer/protocol/liveness, or
/// selected native list syntax failure without publishing guest state.
///
/// # Safety
/// `original` is a live runtime allocation; its caller keeps that header alive
/// after this function returns while using the borrowed element window.
pub unsafe fn probe_list_elements(
    interpreter: Option<&Interp>,
    original: *mut TclObj,
) -> Result<(*mut *mut TclObj, TclSize), CmdError> {
    let operation = Operation::enter(interpreter, &[original])?;
    elements(&operation, original)
}

fn elements(
    operation: &Operation,
    original: *mut TclObj,
) -> Result<(*mut *mut TclObj, TclSize), CmdError> {
    let protocol = operation.list_protocol()?;
    let result = list::elements_raw_native_with_currency(original, protocol, operation.currency());
    operation.check()?;
    let (elements, length) = result?;
    let length = TclSize::try_from(length)
        .map_err(|_| ValueError::CommandProtocolUnavailable("C API List output width"))?;
    operation.check()?;
    Ok((elements, length))
}

pub(super) unsafe fn get_elements(
    interpreter: *mut Interp,
    original: *mut TclObj,
    count: *mut TclSize,
    output: *mut *mut *mut TclObj,
) -> c_int {
    let mut operation = match unsafe { enter(interpreter, &[original]) } {
        Ok(operation) => operation,
        Err(status) => return status,
    };
    match elements(&operation, original) {
        Ok((elements, length)) => {
            // SAFETY: writable actual TclSize and borrowed-pointer outputs.
            unsafe {
                count.write(length);
                output.write(elements);
            }
            TCL_OK
        }
        Err(error) => operation.publish(error),
    }
}

pub(super) unsafe fn length(
    interpreter: *mut Interp,
    original: *mut TclObj,
    output: *mut TclSize,
) -> c_int {
    let mut operation = match unsafe { enter(interpreter, &[original]) } {
        Ok(operation) => operation,
        Err(status) => return status,
    };
    let result = (|| {
        let protocol = operation.list_protocol()?;
        // The actual abstract Length hook is independent of GetElements cache creation.
        let abstract_length = crate::native_arithseries::length_in(original, protocol);
        operation.check()?;
        let length = if let Some(length) = abstract_length? {
            TclSize::try_from(length)
                .map_err(|_| ValueError::CommandProtocolUnavailable("C API List output width"))?
        } else {
            elements(&operation, original)?.1
        };
        operation.check()?;
        Ok::<_, CmdError>(length)
    })();
    match result {
        Ok(length) => {
            unsafe { output.write(length) };
            TCL_OK
        }
        Err(error) => operation.publish(error),
    }
}

pub(super) unsafe fn append(
    interpreter: *mut Interp,
    original: *mut TclObj,
    member: *mut TclObj,
) -> c_int {
    let mut operation = match unsafe { enter(interpreter, &[original, member]) } {
        Ok(operation) => operation,
        Err(status) => return status,
    };
    let result = (|| {
        if obj::is_shared(original) {
            return Err(CmdError::new(
                "Tcl_ListObjAppendElement called with shared object",
            ));
        }
        let protocol = operation.list_protocol()?;
        let result = list::append_native_element_with_currency(
            original,
            member,
            protocol,
            operation.currency(),
        );
        operation.check()?;
        Ok(result?)
    })();
    match result {
        Ok(()) => TCL_OK,
        Err(error) => operation.publish(error),
    }
}

#[derive(Clone, Copy)]
struct IndexRep {
    protocol: NativeIndexLookupProtocol,
    table: *const c_void,
    offset: TclSize,
    index: usize,
    entry: *const c_char,
}
extern "C" fn free_index(original: *mut TclObj) {
    // SAFETY: this exact Index descriptor owns one box, not the caller's table.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(original) as usize as *mut IndexRep
        ))
    };
}
extern "C" fn duplicate_index(original: *mut TclObj, duplicate: *mut TclObj) {
    let backing = index_rep(original);
    let copy = Box::new(IndexRep {
        protocol: backing.protocol,
        table: backing.table,
        offset: backing.offset,
        index: backing.index,
        entry: backing.entry,
    });
    // SAFETY: exact original Index backing and a fresh destination header.
    unsafe {
        (*duplicate).type_ptr = &TCL_INDEX_TYPE;
        (*duplicate).internal_rep = Box::into_raw(copy) as usize as u64;
    }
}
static TCL_INDEX_TYPE: TclObjType = TclObjType {
    name: c"index".as_ptr(),
    free_int_rep_proc: Some(free_index),
    dup_int_rep_proc: Some(duplicate_index),
    update_string_proc: None,
    set_from_any_proc: None,
};
fn index_rep(original: *mut TclObj) -> IndexRep {
    // SAFETY: copy read-only origin fields from the exact live header backing.
    // The external table remains under its original caller lifetime contract.
    unsafe { *(obj::internal_rep(original) as usize as *const IndexRep) }
}
fn index_protocol(operation: &Operation) -> Result<NativeIndexLookupProtocol, CmdError> {
    operation.check()?;
    operation
        .interpreter
        .native_invocation_dialect()
        .native_index_lookup_protocol()
        .ok_or_else(|| {
            ValueError::CommandProtocolUnavailable("original C API Index protocol").into()
        })
}
fn index_spelling(
    operation: &Operation,
    original: *mut TclObj,
) -> Result<Option<Vec<u8>>, CmdError> {
    operation.check()?;
    if obj::obj_type_ptr(original) != &TCL_INDEX_TYPE {
        return Ok(None);
    }
    let rep = index_rep(original);
    if rep.protocol != index_protocol(operation)? {
        return Err(ValueError::CommandProtocolUnavailable("foreign C API Index origin").into());
    }
    // SAFETY: retained original table entry under the caller's static-table contract.
    Ok(Some(
        unsafe { CStr::from_ptr(rep.entry) }.to_bytes().to_vec(),
    ))
}

pub(super) unsafe fn get_index(
    interpreter: *mut Interp,
    original: *mut TclObj,
    table: *const c_void,
    offset: TclSize,
    noun: *const c_char,
    flags: c_int,
    output: *mut c_void,
) -> c_int {
    let originals = if original.is_null() {
        Vec::new()
    } else {
        vec![original]
    };
    let mut operation = match unsafe { enter(interpreter, &originals) } {
        Ok(operation) => operation,
        Err(status) => return status,
    };
    let result = (|| {
        let protocol = index_protocol(&operation)?;
        let selected_flags = NativeIndexLookupFlags {
            exact: flags & TCL_EXACT != 0,
            null_ok: flags & TCL_NULL_OK != 0,
            temporary_table: flags & TCL_INDEX_TEMP_TABLE != 0,
        };
        protocol.validate_flags(selected_flags)?;
        let pointer_width = TclSize::try_from(core::mem::size_of::<*const c_char>())
            .expect("pointer width fits TclSize");
        let offset = if offset < pointer_width {
            if protocol.version() >= tcl_dialect::TclVersion::V9_0 {
                return Err(CmdError::new(format!(
                    "Invalid struct offset value {offset}."
                )));
            }
            pointer_width
        } else {
            offset
        };
        if !selected_flags.temporary_table
            && !original.is_null()
            && obj::obj_type_ptr(original) == &TCL_INDEX_TYPE
        {
            let rep = index_rep(original);
            if rep.protocol != protocol {
                return Err(
                    ValueError::CommandProtocolUnavailable("foreign C API Index origin").into(),
                );
            }
            if rep.table == table && rep.offset == offset {
                return TclSize::try_from(rep.index).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("C API Index output width").into()
                });
            }
        }
        let entries = unsafe { read_table(table, offset) };
        let names: Vec<&[u8]> = entries.iter().map(|(name, _)| name.as_slice()).collect();
        let what = unsafe { c_bytes(noun) };
        let original_bytes = if original.is_null() {
            if !selected_flags.null_ok {
                return Err(ValueError::CommandProtocolUnavailable(
                    "missing original C API Index object",
                )
                .into());
            }
            Vec::new()
        } else {
            operation.string(original)?
        };
        let key = protocol.selected_input(&original_bytes);
        let resolution = prefix::scan(&names, key, selected_flags.exact);
        let index = match resolution {
            _ if key.is_empty() && selected_flags.null_ok => return Ok(-1),
            Resolution::Exact(index) | Resolution::UniquePrefix(index) => index,
            Resolution::Ambiguous | Resolution::NoMatch => {
                let ambiguous = matches!(resolution, Resolution::Ambiguous);
                let message = if selected_flags.null_ok {
                    let mut message = if ambiguous {
                        b"ambiguous ".to_vec()
                    } else {
                        b"bad ".to_vec()
                    };
                    message.extend_from_slice(&what);
                    message.extend_from_slice(b" \"");
                    message.extend_from_slice(key);
                    message.extend_from_slice(b"\": must be ");
                    for (position, name) in names.iter().enumerate() {
                        if position > 0 {
                            message.extend_from_slice(b", ");
                        }
                        message.extend_from_slice(name);
                    }
                    message.extend_from_slice(b", or \"\"");
                    message
                } else {
                    prefix::bad_key_message(&names, &what, key, ambiguous)
                };
                return Err(CmdError::with_error_code_bytes(
                    message,
                    protocol.error_code(&what, key),
                ));
            }
        };
        operation.check()?;
        let output_index = TclSize::try_from(index)
            .map_err(|_| ValueError::CommandProtocolUnavailable("C API Index output width"))?;
        if !selected_flags.temporary_table && !original.is_null() {
            let backing = Box::new(IndexRep {
                protocol,
                table,
                offset,
                index,
                entry: entries[index].1,
            });
            if !obj::obj_type_ptr(original).is_null() {
                obj::discard_native_internal_representation(original)?;
                operation.check()?;
            }
            obj::change_type(
                original,
                &TCL_INDEX_TYPE,
                Box::into_raw(backing) as usize as u64,
            );
        }
        operation.check()?;
        Ok(output_index)
    })();
    match result {
        Ok(index) => {
            if let Err(error) = operation.check() {
                return operation.publish(error);
            }
            if !output.is_null() {
                unsafe {
                    match flags & 0b1_0110 {
                        2 => output.cast::<u8>().write(index as u8),
                        4 => output.cast::<u16>().write(index as u16),
                        16 => output.cast::<i64>().write(index as i64),
                        _ => output.cast::<i32>().write(index as i32),
                    }
                }
            }
            TCL_OK
        }
        Err(error) => operation.publish(error),
    }
}

pub(super) unsafe fn wrong_arguments(
    interpreter: *mut Interp,
    count: TclSize,
    argv: *const *mut TclObj,
    message: *const c_char,
) {
    let originals = unsafe { words(count, argv) };
    let mut operation = match unsafe { enter(interpreter, originals) } {
        Ok(operation) => operation,
        Err(_) => return,
    };
    let result = (|| {
        let protocol = operation
            .interpreter
            .native_invocation_dialect()
            .wrong_arguments_protocol(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C API wrong arguments",
            ))?;
        let mut text = b"wrong # args: should be \"".to_vec();
        for (position, &word) in originals.iter().enumerate() {
            let spelling = match index_spelling(&operation, word)? {
                Some(spelling) => spelling,
                None => operation.string(word)?,
            };
            append_list_element(&mut text, &spelling, true);
            if position + 1 < originals.len() || !message.is_null() {
                text.push(b' ');
            }
        }
        if !message.is_null() {
            text.extend_from_slice(&unsafe { c_bytes(message) });
        }
        text.push(b'"');
        operation.check()?;
        Ok::<_, CmdError>(CmdError::with_error_code_bytes(
            text,
            protocol.error_code().as_bytes(),
        ))
    })();
    let error = match result {
        Ok(error) | Err(error) => error,
    };
    operation.publish(error);
}

pub(super) unsafe fn set_error_code(interpreter: *mut Interp, original: *mut TclObj) {
    let mut operation = match unsafe { enter(interpreter, &[original]) } {
        Ok(operation) => operation,
        Err(_) => return,
    };
    if let Err(first) = operation.interpreter.set_original_c_error_code(original) {
        operation.publish(CmdError::from_execution_refusal(first));
    }
    if let Err(error) = operation.currency().ensure_current_or_refuse() {
        operation.interpreter.refuse_native_execution(error);
    }
}

#[cfg(test)]
mod tests;
