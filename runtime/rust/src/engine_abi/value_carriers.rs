// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Physical selected producers for independently retained counted value facts.

use crate::capi::NativeScalarObjectAccessError;
use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
use core::ffi::c_int;
use tcl_core_types::{NativeScalarCache, NativeStringStorageIdentity};
use tcl_syntax::{scalar_getter::carrier, value::ValueError};

pub(crate) fn scalar(
    interp: &mut Interp,
    facts: &NativeScalarCache,
) -> Result<Owned, NativeScalarObjectAccessError> {
    let issuer = interp.native_scalar_object_issuer()?;
    let (cache, origin) = carrier::import_scalar(facts);
    let protocol = interp
        .native_invocation_dialect()
        .native_scalar_getter_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native scalar import",
        ))?;
    if origin.is_some_and(|origin| protocol.tcl_version() != Some(origin))
        || (matches!(
            cache,
            tcl_syntax::scalar_getter::NativeScalarCache::JimCoercedInteger(_)
        ) && !protocol.is_jim084())
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "foreign native scalar descriptor origin",
        )
        .into());
    }
    let original = Owned::fresh(obj::new_obj());
    obj::adopt_native_scalar_cache(original.as_ptr(), cache, protocol)?;
    obj::invalidate_string(original.as_ptr());
    interp.associate_native_jim_arguments(&[original.as_ptr()])?;
    obj::bind_scalar_object_context(original.as_ptr(), issuer)?;
    Ok(original)
}

pub(crate) fn byte_array(interp: &mut Interp, bytes: &[u8]) -> Result<Owned, ValueError> {
    interp
        .new_native_byte_array(bytes)
        .map(Owned::fresh)
        .map_err(|_| ValueError::CommandProtocolUnavailable("native byte-array producer"))
}

/// Bind an original object to the actual selected scalar engine and Host ABI.
/// A failed binding leaves the guest result and error code unchanged.
///
/// # Safety
/// Interpreter and original object must remain live throughout this operation.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_bind_scalar_object(
    interp: *mut Interp,
    original: *mut TclObj,
) -> c_int {
    // SAFETY: the caller supplies the original live interpreter.
    let interp = unsafe { &mut *interp };
    match interp.bind_native_scalar_object(original) {
        Ok(()) => 0,
        Err(error) => {
            // SAFETY: as above; this publishes only the original host refusal.
            unsafe { crate::capi::scalar_publish_access_error(interp, error) }
        }
    }
}

pub(crate) fn resident(
    original: &Owned,
    bytes: &[u8],
    storage: NativeStringStorageIdentity,
) -> Result<(), ValueError> {
    let storage = carrier::checked_storage(storage, bytes.len())
        .map_err(|_| ValueError::CommandProtocolUnavailable("native resident storage identity"))?;
    obj::invalidate_string(original.as_ptr());
    // SAFETY: the independent original has the explicitly validated recorded storage kind.
    unsafe {
        obj::set_native_updater_string_rep(
            original.as_ptr(),
            bytes,
            storage == tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty,
        )
    };
    Ok(())
}

fn refuse_value(interp: &mut Interp, error: ValueError) {
    if let Some(cause) = error.native_access_refusal() {
        interp.refuse_native_access(cause);
    } else {
        interp.refuse_host_command(error.to_string());
    }
}

fn bytes<'a>(interp: &mut Interp, pointer: *const u8, length: c_int) -> Option<&'a [u8]> {
    let Ok(length) = usize::try_from(length) else {
        interp.refuse_host_command("negative counted value transport length");
        return None;
    };
    if length == 0 {
        return Some(&[]);
    }
    if pointer.is_null() {
        interp.refuse_host_command("counted value transport is unavailable");
        return None;
    }
    // SAFETY: every exported entry requires the complete counted input to remain live.
    Some(unsafe { core::slice::from_raw_parts(pointer, length) })
}

/// Import full scalar facts through the actual selected descriptor producer.
/// Null retains a host cause outside the guest completion channel.
///
/// # Safety
/// Interpreter and complete counted encoded carrier must remain live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_new_scalar_carrier(
    interp: *mut Interp,
    input: *const u8,
    length: c_int,
) -> *mut TclObj {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return core::ptr::null_mut();
    }
    let Some(input) = bytes(interp, input, length) else {
        return core::ptr::null_mut();
    };
    let facts = match carrier::decode_scalar(input) {
        Ok(facts) => facts,
        Err(error) => {
            interp.refuse_host_command(error.to_string());
            return core::ptr::null_mut();
        }
    };
    match scalar(interp, &facts) {
        Ok(original) => original.into_raw(),
        Err(error) => {
            // SAFETY: this actual original interpreter remains live.
            unsafe { crate::capi::scalar_publish_access_error(interp, error) };
            core::ptr::null_mut()
        }
    }
}

/// Create an original binary descriptor from independent exact binary bytes.
///
/// # Safety
/// Interpreter and counted binary input must remain live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_new_byte_array_carrier(
    interp: *mut Interp,
    input: *const u8,
    length: c_int,
) -> *mut TclObj {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return core::ptr::null_mut();
    }
    let Some(input) = bytes(interp, input, length) else {
        return core::ptr::null_mut();
    };
    match byte_array(interp, input) {
        Ok(original) => original.into_raw(),
        Err(error) => {
            refuse_value(interp, error);
            core::ptr::null_mut()
        }
    }
}

/// Attach recorded string bytes to an independently imported original payload.
/// Storage 0 is canonical empty, 1 allocated; unknown identity is refused.
///
/// # Safety
/// Interpreter, borrowed original and counted bytes must remain live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_adopt_resident_carrier(
    interp: *mut Interp,
    original: *mut TclObj,
    input: *const u8,
    length: c_int,
    storage: c_int,
) -> c_int {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return 1;
    }
    let Some(input) = bytes(interp, input, length) else {
        return 1;
    };
    if original.is_null() {
        interp.refuse_host_command("resident original payload is unavailable");
        return 1;
    }
    let storage = match storage {
        0 => NativeStringStorageIdentity::CanonicalEmpty,
        1 => NativeStringStorageIdentity::Allocated,
        _ => NativeStringStorageIdentity::Unknown,
    };
    match resident(&Owned::retain(original), input, storage) {
        Ok(()) => 0,
        Err(error) => {
            refuse_value(interp, error);
            1
        }
    }
}

/// Build a selected list or dictionary from counted original child objects.
///
/// # Safety
/// Interpreter and complete counted child vector must remain live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_new_sequence_carrier(
    interp: *mut Interp,
    children: *const *mut TclObj,
    count: c_int,
    dictionary: c_int,
) -> *mut TclObj {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return core::ptr::null_mut();
    }
    let Ok(count) = usize::try_from(count) else {
        interp.refuse_host_command("negative original child count");
        return core::ptr::null_mut();
    };
    let children = if count == 0 {
        &[]
    } else if children.is_null() {
        interp.refuse_host_command("original child vector is unavailable");
        return core::ptr::null_mut();
    } else {
        // SAFETY: caller retains the counted original child vector.
        unsafe { core::slice::from_raw_parts(children, count) }
    };
    if children.iter().any(|child| child.is_null()) {
        interp.refuse_host_command("original child object is unavailable");
        return core::ptr::null_mut();
    }
    let original = match dictionary {
        0 => Owned::fresh(interp.new_list_object(children)),
        1 if count % 2 == 0 => {
            let Some(recipe) = interp.native_invocation_dialect().native_string_materialization(Some(
                tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)) else {
                interp.refuse_host_command("dictionary import updater issuer unavailable"); return core::ptr::null_mut();
            };
            let pairs = children
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[key, value]| (*key, *value))
                .collect::<Vec<_>>();
            match crate::dict::new_dict_obj_native(&pairs, None, recipe.protocol()) {
                Ok(original) => Owned::fresh(original),
                Err(error) => {
                    refuse_value(interp, error);
                    return core::ptr::null_mut();
                }
            }
        }
        _ => {
            interp.refuse_host_command("invalid original sequence shape");
            return core::ptr::null_mut();
        }
    };
    original.into_raw()
}

pub(crate) fn complete(
    interp: &mut Interp,
    value: *mut TclObj,
    options: Option<*mut TclObj>,
    code: Code,
) -> Code {
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    if let Some(options) = options {
        let (mut ops, protocol) = match crate::return_options::NativeReturnOps::selected(interp) {
            Ok(selected) => selected,
            Err(error) => return interp.report_cmd_error(error),
        };
        let args = [
            Owned::fresh(crate::interp::new_string(b"-options")),
            Owned::retain(options),
            Owned::retain(value),
        ];
        let prepared = match tcl_cmd_core::return_options::prepare_return(
            &mut ops,
            protocol,
            &args,
            tcl_cmd_core::return_options::ReturnOptionsPurpose::InternalDictionary,
        ) {
            Ok(prepared) => prepared,
            Err(error) => return interp.report_cmd_error(error),
        };
        let published = crate::return_options::publish(interp, &mut ops, prepared);
        if interp.host_refusal_pending() {
            return Code::Error;
        }
        return if code == Code::Return {
            published
        } else {
            code
        };
    }
    interp.set_result(value);
    code
}

/// Settle original result/options with the explicit host completion code.
/// A null options pointer requests no options, except for Return.
///
/// # Safety
/// Interpreter and borrowed non-null result/options must remain live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_complete_original(
    interp: *mut Interp,
    value: *mut TclObj,
    options: *mut TclObj,
    code: c_int,
) -> c_int {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if value.is_null() {
        interp.refuse_host_command("original host result is unavailable");
        return 1;
    }
    let code = Code::from_int(code);
    let empty = (options.is_null() && code == Code::Return)
        .then(|| Owned::fresh(crate::interp::new_string(b"")));
    let options = (!options.is_null())
        .then_some(options)
        .or_else(|| empty.as_ref().map(Owned::as_ptr));
    c_int::try_from(complete(interp, value, options, code).as_int()).unwrap_or(1)
}

/// Capture the original guest result and complete options with their real owners.
/// A host refusal produces no guest handles and leaves `out` untouched.
///
/// # Safety
/// Interpreter and properly aligned writable completion storage must remain live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_capture_original(
    interp: *mut Interp,
    code: c_int,
    out: *mut crate::codegen_abi::TclCompletionAbi,
) -> c_int {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return 1;
    }
    if out.is_null() {
        interp.refuse_host_command("completion output is unavailable");
        return 1;
    }
    let original = crate::state_traits::capture_completion(interp, Code::from_int(code));
    if interp.host_refusal_pending() {
        // SAFETY: original capture transferred both owned handles, possibly null.
        unsafe {
            obj::decr_ref_count(original.result);
            obj::decr_ref_count(original.options);
        }
        return 1;
    }
    // SAFETY: caller retains aligned writable output; both real captured references transfer.
    unsafe {
        out.write(crate::codegen_abi::TclCompletionAbi {
            code,
            result: original.result,
            options: original.options,
        });
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> Interp {
        Interp::with_native_core(
            Interp::new().host(),
            tcl_registry::model::resolve_environment("tcl8.6").unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn scalar_issuer_keeps_recorded_resident_spelling_and_refuses_foreign_origin() {
        // Software importer: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = core();
        let original = scalar(&mut interp, &NativeScalarCache::Integer(16)).unwrap();
        resident(&original, b"0x10", NativeStringStorageIdentity::Allocated).unwrap();
        assert_eq!(
            interp
                .native_object_string_bytes(original.as_ptr())
                .unwrap(),
            b"0x10"
        );
        assert!(
            resident(
                &original,
                b"different",
                NativeStringStorageIdentity::Unknown
            )
            .is_err()
        );
        assert_eq!(
            interp
                .native_object_string_bytes(original.as_ptr())
                .unwrap(),
            b"0x10"
        );
        let foreign = NativeScalarCache::WordBoolean {
            value: true,
            origin: tcl_core_types::NativeCVersion::V8_5,
        };
        let wire = carrier::encode_scalar(&foreign).unwrap();
        interp.set_result_bytes(b"EARLIER\0\xff");
        // SAFETY: counted carrier input and real original interpreter remain live.
        assert!(
            unsafe {
                tcl_engine_new_scalar_carrier(
                    &mut interp,
                    wire.as_ptr(),
                    i32::try_from(wire.len()).unwrap(),
                )
            }
            .is_null()
        );
        let first = interp.native_execution_refusal().unwrap();
        let mut out = crate::codegen_abi::TclCompletionAbi {
            code: 77,
            result: core::ptr::null_mut(),
            options: core::ptr::null_mut(),
        };
        // SAFETY: genuine writable output; Host must leave it untouched.
        assert_eq!(
            unsafe { tcl_engine_capture_original(&mut interp, 1, &mut out) },
            1
        );
        assert_eq!(out.code, 77);
        assert!(out.result.is_null() && out.options.is_null());
        assert_eq!(interp.native_execution_refusal().unwrap(), first);
    }

    #[test]
    fn original_completion_transport_retains_guest_options_and_binary_result() {
        // Software settlement: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = core();
        let value = Owned::fresh(crate::interp::new_string(b"RESULT\0\xff"));
        let options = Owned::fresh(crate::interp::new_string(
            b"-code 7 -level 0 -custom ORIGINAL",
        ));
        // SAFETY: all borrowed original objects remain live through publication.
        assert_eq!(
            unsafe {
                tcl_engine_complete_original(&mut interp, value.as_ptr(), options.as_ptr(), 7)
            },
            7
        );
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: genuine aligned writable output receives actual Guest owners.
        assert_eq!(
            unsafe { tcl_engine_capture_original(&mut interp, 7, out.as_mut_ptr()) },
            0
        );
        // SAFETY: successful Guest capture initialised both live owned handles.
        let mut out = unsafe { out.assume_init() };
        assert_eq!(out.code, 7);
        assert_eq!(
            interp.native_object_string_bytes(out.result).unwrap(),
            b"RESULT\0\xff"
        );
        let actual_options = interp.native_object_string_bytes(out.options).unwrap();
        assert!(
            actual_options
                .windows(16)
                .any(|bytes| bytes == b"-custom ORIGINAL")
        );
        // SAFETY: capture transferred exactly one reference per output.
        unsafe {
            crate::codegen_abi::tcl_completion_release(&mut out);
        }
        assert!(!interp.host_refusal_pending());
    }
}
