// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Counted engine publication and completion views over actual interpreter owners.

use super::{TCL_ERROR, TCL_OK, define_unit_original};
use crate::interp::{Interp, ObjCommand, RenameOutcome, new_string};
use crate::obj::{TclObj, incr_ref_count};
use core::ffi::{c_int, c_void};

/// The original producer's interpreter and still-installed command generation.
/// The qualified address owns one object reference. The caller releases it.
/// Its bytes are a replay address, not permission to restore a retired generation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EngineCommandReceipt {
    /// Actual original interpreter owner.
    pub owner: u64,
    /// Actual interpreter within that owner.
    pub interpreter: u64,
    /// Installed never-reused command generation.
    pub generation: u64,
    /// Independently owned counted publication address.
    pub qualified: *mut TclObj,
}

/// Original command identity for retention without a borrowed address object.
/// The same issuing interpreter must still own the installed generation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EngineCommandIdentity {
    /// Process-unique issuing runtime owner.
    pub owner: u64,
    /// Original interpreter within that owner.
    pub interpreter: u64,
    /// Never-reused installed command generation.
    pub generation: u64,
}

impl From<EngineCommandReceipt> for EngineCommandIdentity {
    fn from(receipt: EngineCommandReceipt) -> Self {
        Self {
            owner: receipt.owner,
            interpreter: receipt.interpreter,
            generation: receipt.generation,
        }
    }
}

fn original_bytes<'a>(
    interp: &mut Interp,
    bytes: *const u8,
    length: c_int,
) -> Result<&'a [u8], c_int> {
    let Ok(length) = usize::try_from(length) else {
        interp.refuse_host_command("engine counted input has a negative length");
        return Err(TCL_ERROR);
    };
    if length == 0 {
        return Ok(b"");
    }
    if bytes.is_null() {
        interp.refuse_host_command("engine counted input has no original bytes");
        return Err(TCL_ERROR);
    }
    // SAFETY: every ABI entry requires its original counted input to remain live.
    Ok(unsafe { core::slice::from_raw_parts(bytes, length) })
}

unsafe fn issue_receipt(
    interp: &mut Interp,
    generation: u64,
    out: *mut EngineCommandReceipt,
) -> c_int {
    if interp.host_refusal_pending() {
        return TCL_ERROR;
    }
    let address = {
        let names = interp.namespaces();
        names
            .native_command_slot_at_node(generation)
            .map(|(namespace, simple)| names.command_fqn_at(namespace, &simple))
    };
    let Some(address) = address else {
        interp.refuse_host_command("engine publication no longer has its installed command");
        return TCL_ERROR;
    };
    let original = interp.native_callable_interpreter();
    let qualified = new_string(&address);
    // SAFETY: fresh original receipt address; caller adopts its one reference.
    unsafe {
        incr_ref_count(qualified);
        out.write(EngineCommandReceipt {
            owner: original.owner,
            interpreter: original.interpreter,
            generation,
            qualified,
        });
    }
    TCL_OK
}

/// Create a C object command from counted input and return its actual receipt.
/// The existing native C publication recipe selects input extent and namespace.
///
/// # Safety
/// Interpreter, counted bytes, callback and output cell must be live. The
/// callback remains callable while installed. `out` is written only on success.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_create_command_counted(
    interp: *mut Interp,
    name: *const u8,
    length: c_int,
    procedure: Option<crate::interp::TclObjCmdProc>,
    client: *mut c_void,
    out: *mut EngineCommandReceipt,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return TCL_ERROR;
    }
    let Some(procedure) = procedure else {
        interp.refuse_host_command("engine publication has no original callback");
        return TCL_ERROR;
    };
    if out.is_null() {
        interp.refuse_host_command("engine publication has no receipt cell");
        return TCL_ERROR;
    }
    let Ok(name) = original_bytes(interp, name, length) else {
        return TCL_ERROR;
    };
    let Some(generation) =
        interp.create_obj_command(name, ObjCommand::new(procedure, client, None))
    else {
        return TCL_ERROR;
    };
    // SAFETY: actual installer selected the generation and caller supplies out.
    unsafe { issue_receipt(interp, generation, out) }
}

/// Install the ordinary original procedure and return its genuine receipt.
///
/// # Safety
/// Interpreter and original objects must be live; `out` writable.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_define_unit_receipt(
    interp: *mut Interp,
    name: *mut TclObj,
    params: *mut TclObj,
    body: *mut TclObj,
    out: *mut EngineCommandReceipt,
) -> c_int {
    // SAFETY: caller guarantees the live interpreter and original objects.
    let interp = unsafe { &mut *interp };
    if out.is_null() {
        interp.refuse_host_command("engine procedure publication has no receipt cell");
        return TCL_ERROR;
    }
    let Ok(generation) = define_unit_original(interp, name, params, body) else {
        return TCL_ERROR;
    };
    // SAFETY: installer returned the original generation and caller supplies out.
    unsafe { issue_receipt(interp, generation, out) }
}

/// Delete the command selected by the native C deletion input. Return 0 for a
/// deletion, -1 for absence, 1 for a host refusal. On deletion `out` owns the
/// exact selected pre-deletion receipt, never a later name relookup.
///
/// # Safety
/// Interpreter, counted input and output cell must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_delete_command_counted(
    interp: *mut Interp,
    name: *const u8,
    length: c_int,
    out: *mut EngineCommandReceipt,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return TCL_ERROR;
    }
    if out.is_null() {
        interp.refuse_host_command("engine retirement has no original receipt cell");
        return TCL_ERROR;
    }
    let Ok(original) = original_bytes(interp, name, length) else {
        return TCL_ERROR;
    };
    let name = tcl_core_types::c_string_extent(original);
    let Some(generation) = interp.resolve_cmd_token(name) else {
        return -1;
    };
    let mut captured = core::mem::MaybeUninit::uninit();
    // SAFETY: capture the real selected slot before any retirement callbacks.
    let status = unsafe { issue_receipt(interp, generation, captured.as_mut_ptr()) };
    if status != TCL_OK {
        return status;
    }
    // SAFETY: actual receipt producer initialised the owned original address.
    let captured = unsafe { captured.assume_init() };
    let deleted = matches!(interp.rename_command(name, b""), RenameOutcome::Deleted);
    if deleted && !interp.host_refusal_pending() {
        // SAFETY: caller supplies out; selected pre-retirement receipt is genuine.
        unsafe { out.write(captured) };
        TCL_OK
    } else {
        // SAFETY: no published receipt adopts the independent captured address.
        unsafe { crate::obj::decr_ref_count(captured.qualified) };
        if interp.host_refusal_pending() {
            TCL_ERROR
        } else {
            -1
        }
    }
}

/// Compare a retained receipt to this original interpreter and the currently
/// selected invocation. Reporting bytes alone cannot validate a generation.
///
/// # Safety
/// Interpreter and counted invocation bytes must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_command_receipt_current(
    interp: *mut Interp,
    owner: u64,
    interpreter: u64,
    generation: u64,
    name: *const u8,
    length: c_int,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return TCL_ERROR;
    }
    let Ok(name) = original_bytes(interp, name, length) else {
        return TCL_ERROR;
    };
    let actual = interp.native_callable_interpreter();
    if actual.owner == owner
        && actual.interpreter == interpreter
        && interp.resolve_cmd_token(name) == Some(generation)
    {
        TCL_OK
    } else {
        interp.refuse_host_command("engine command receipt is foreign, retired or replaced");
        TCL_ERROR
    }
}

/// Compatibility entry for generations already authenticated in this interpreter.
/// This vector carries no foreign-owner proof. New counted consumers use
/// `tcl_engine_restrict_original_receipts`. Malformed Unicode inputs refuse.
///
/// # Safety
/// Interpreter, allowed original list and counted generation vector are live;
/// every generation must originate from this same interpreter.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_restrict_receipts(
    interp: *mut Interp,
    allowed: *mut TclObj,
    generations: *const u64,
    count: c_int,
) -> c_int {
    // SAFETY: caller guarantees the live interpreter and original list.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return TCL_ERROR;
    }
    let Ok(count) = usize::try_from(count) else {
        interp.refuse_host_command("engine restriction has a negative receipt count");
        return TCL_ERROR;
    };
    let generations = if count == 0 {
        &[]
    } else if generations.is_null() {
        interp.refuse_host_command("engine restriction has no original receipts");
        return TCL_ERROR;
    } else {
        // SAFETY: compatibility caller retains this interpreter's counted generations.
        unsafe { core::slice::from_raw_parts(generations, count) }
    };
    restrict_original_generations(interp, allowed, generations)
}

fn restrict_original_generations(
    interp: &mut Interp,
    allowed: *mut TclObj,
    generations: &[u64],
) -> c_int {
    let names = match interp.restriction_unicode_names(allowed) {
        Ok(names) => names,
        Err(error) => {
            interp.refuse_restriction_input(error);
            return TCL_ERROR;
        }
    };
    let words = names.iter().map(String::as_str).collect::<Vec<_>>();
    if interp.restrict_to_tokens(&words, generations).is_ok() {
        TCL_OK
    } else {
        TCL_ERROR
    }
}

/// Retain full original command identities from the actual issuing interpreter.
/// Foreign or retired receipts and malformed Unicode inputs refuse before any
/// command-table effects. Deletion callbacks retain the first host cause.
///
/// # Safety
/// Interpreter, original list and the complete counted identity vector are live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_restrict_original_receipts(
    interp: *mut Interp,
    allowed: *mut TclObj,
    identities: *const EngineCommandIdentity,
    count: c_int,
) -> c_int {
    // SAFETY: caller retains the original interpreter.
    let interp = unsafe { &mut *interp };
    if interp.admit_restriction_purpose().is_err() {
        return TCL_ERROR;
    }
    let Ok(count) = usize::try_from(count) else {
        interp.refuse_host_command("engine restriction has a negative receipt count");
        return TCL_ERROR;
    };
    let identities = if count == 0 {
        &[]
    } else if identities.is_null() {
        interp.refuse_host_command("engine restriction has no original identities");
        return TCL_ERROR;
    } else {
        // SAFETY: caller supplies all counted repr(C) original identity headers.
        unsafe { core::slice::from_raw_parts(identities, count) }
    };
    let actual = interp.native_callable_interpreter();
    if identities.iter().any(|identity| {
        identity.owner != actual.owner
            || identity.interpreter != actual.interpreter
            || interp
                .namespaces()
                .native_command_slot_at_node(identity.generation)
                .is_none()
    }) {
        interp.refuse_host_command("engine restriction receipt is foreign, retired or replaced");
        return TCL_ERROR;
    }
    let generations = identities
        .iter()
        .map(|identity| identity.generation)
        .collect::<Vec<_>>();
    restrict_original_generations(interp, allowed, &generations)
}

/// Whether a reached original host cause prevents every guest object getter.
///
/// # Safety
/// Interpreter must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_host_refusal_pending(interp: *mut Interp) -> c_int {
    // SAFETY: caller guarantees the original live interpreter.
    c_int::from(unsafe { &*interp }.host_refusal_pending())
}

/// Presentation of the retained host cause as an independently owned counted
/// string. It supplies no guest completion or original metadata authority.
///
/// # Safety
/// Interpreter must be live. Caller releases a non-null result once.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_host_refusal_text(interp: *mut Interp) -> *mut TclObj {
    // SAFETY: caller guarantees the original live interpreter.
    let Some(error) = unsafe { &*interp }.native_execution_refusal() else {
        return core::ptr::null_mut();
    };
    let object = new_string(error.to_string().as_bytes());
    // SAFETY: fresh presentation object; caller adopts the one owned reference.
    unsafe { incr_ref_count(object) };
    object
}

/// Retain a reached host refusal outside guest `catch`/`try`, preserving an
/// earlier original cause and every earlier guest effect.
///
/// # Safety
/// Interpreter and counted reason bytes must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_refuse_host_counted(
    interp: *mut Interp,
    reason: *const u8,
    length: c_int,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    let Ok(reason) = original_bytes(interp, reason, length) else {
        return TCL_ERROR;
    };
    let Ok(reason) = core::str::from_utf8(reason) else {
        interp.refuse_host_command("host refusal reason is not an original Unicode description");
        return TCL_ERROR;
    };
    interp.refuse_host_command(reason);
    TCL_ERROR
}

/// Snapshot the original value through the producing interpreter's checked
/// string getter. Returned bytes own one fresh object; null carries no guest
/// object and requires the caller to read the retained host cause.
///
/// # Safety
/// Interpreter and original object must be live. Release a non-null result once.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_original_string_snapshot(
    interp: *mut Interp,
    original: *mut TclObj,
) -> *mut TclObj {
    // SAFETY: caller guarantees the original live interpreter and object.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return core::ptr::null_mut();
    }
    if original.is_null() {
        interp.refuse_host_command("engine string snapshot has no original object");
        return core::ptr::null_mut();
    }
    let bytes = match interp.native_object_string_bytes(original) {
        Ok(bytes) => bytes,
        Err(error) => {
            if let Some(cause) = error.native_access_refusal() {
                interp.refuse_native_access(cause);
            } else {
                interp
                    .refuse_host_command(format!("engine original string access failed: {error}"));
            }
            return core::ptr::null_mut();
        }
    };
    let snapshot = new_string(&bytes);
    // SAFETY: fresh exact counted string snapshot, caller owns its reference.
    unsafe { incr_ref_count(snapshot) };
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;
    use core::ffi::c_int;

    fn core(profile: &str) -> Interp {
        Interp::with_native_core(
            Interp::new().host(),
            tcl_registry::model::resolve_environment(profile).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    unsafe extern "C" fn original_callback(
        _: *mut c_void,
        interp: *mut Interp,
        _: c_int,
        _: *const *mut TclObj,
    ) -> c_int {
        // SAFETY: actual native dispatcher supplies the original interpreter.
        unsafe { &mut *interp }.set_result_bytes(b"ORIGINAL\0\xff");
        TCL_OK
    }

    fn create(interp: &mut Interp, name: &[u8]) -> EngineCommandReceipt {
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: actual original counted name, live callback and writable cell.
        assert_eq!(
            unsafe {
                tcl_engine_create_command_counted(
                    interp,
                    name.as_ptr(),
                    c_int::try_from(name.len()).unwrap(),
                    Some(original_callback),
                    core::ptr::null_mut(),
                    out.as_mut_ptr(),
                )
            },
            TCL_OK
        );
        // SAFETY: successful actual producer initialised this receipt.
        unsafe { out.assume_init() }
    }

    #[test]
    fn counted_command_publication_keeps_installed_context_and_native_extent() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = core(profile);
            let receipt = create(&mut interp, "::café\0retired suffix".as_bytes());
            // SAFETY: real publication transferred one owned reference.
            let qualified = unsafe { Owned::from_raw(receipt.qualified) };
            assert_eq!(
                crate::obj::bytes_of(qualified.as_ptr()),
                "::café".as_bytes()
            );
            assert_eq!(
                interp.resolve_cmd_token("::café".as_bytes()),
                Some(receipt.generation)
            );
            assert!(
                interp
                    .resolve_cmd_token("::café\0retired suffix".as_bytes())
                    .is_some()
            );
            let completion = interp.eval_completion("::café".as_bytes()).unwrap();
            assert_eq!(completion.result, b"ORIGINAL\0\xff");
            assert_eq!(receipt.owner, interp.native_callable_interpreter().owner);
            assert_eq!(
                receipt.interpreter,
                interp.native_callable_interpreter().interpreter
            );
        }
    }

    #[test]
    fn receipt_restriction_keeps_renamed_handler_and_drops_replacement_at_old_name() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = core("tcl8.6");
        let receipt = create(&mut interp, b"host");
        // SAFETY: adopt the real independent receipt address reference.
        let _qualified = unsafe { Owned::from_raw(receipt.qualified) };
        interp
            .eval_completion(b"rename host ::moved; proc host {} {set ::replacement_ran 1}")
            .unwrap();
        let allowed = Owned::fresh(new_string(b""));
        // SAFETY: actual list and one original installed generation are retained.
        assert_eq!(
            unsafe {
                tcl_engine_restrict_receipts(&mut interp, allowed.as_ptr(), &receipt.generation, 1)
            },
            TCL_OK
        );
        assert!(interp.resolve_cmd_token(b"host").is_none());
        assert_eq!(
            interp.resolve_cmd_token(b"::moved"),
            Some(receipt.generation)
        );
        assert_eq!(
            interp.eval_completion(b"::moved").unwrap().result,
            b"ORIGINAL\0\xff"
        );
        assert!(interp.var_get(b"::replacement_ran").is_err());
    }

    fn installed_generations(interp: &Interp) -> Vec<u64> {
        interp
            .namespaces()
            .native_command_generations()
            .into_iter()
            .filter(|generation| {
                interp
                    .namespaces()
                    .native_command_slot_at_node(*generation)
                    .is_some()
            })
            .collect()
    }

    #[test]
    fn original_restriction_identity_rejects_foreign_equal_generation_before_effects() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut issuing = core("tcl8.6");
        let receipt = create(&mut issuing, b"host");
        // SAFETY: real publication transferred one independently owned address.
        let _name = unsafe { Owned::from_raw(receipt.qualified) };
        let mut target = core("tcl8.6");
        let local = create(&mut target, b"host");
        // SAFETY: real target publication has its own independent address.
        let _local_name = unsafe { Owned::from_raw(local.qualified) };
        assert_eq!(receipt.generation, local.generation);
        assert_ne!(receipt.owner, local.owner);
        target.set_result_bytes(b"BEFORE\0\xff");
        let before = installed_generations(&target);
        let empty = Owned::fresh(new_string(b""));
        let identity = EngineCommandIdentity::from(receipt);
        // SAFETY: both original identity header and original list remain live.
        assert_eq!(
            unsafe {
                tcl_engine_restrict_original_receipts(&mut target, empty.as_ptr(), &identity, 1)
            },
            TCL_ERROR
        );
        let first = target.native_execution_refusal().unwrap();
        assert_eq!(installed_generations(&target), before);
        assert_eq!(target.result_bytes(), b"BEFORE\0\xff");
        // SAFETY: same refusal must prevent a later otherwise-valid restriction.
        assert_eq!(
            unsafe {
                tcl_engine_restrict_original_receipts(
                    &mut target,
                    empty.as_ptr(),
                    &EngineCommandIdentity::from(local),
                    1,
                )
            },
            TCL_ERROR
        );
        assert_eq!(target.native_execution_refusal(), Some(first));
        assert_eq!(installed_generations(&target), before);
    }

    #[test]
    fn original_restriction_identity_keeps_renamed_generation_and_refuses_retirement() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = core("tcl8.6");
        let receipt = create(&mut interp, b"host");
        // SAFETY: actual publication supplied one independently owned address.
        let _name = unsafe { Owned::from_raw(receipt.qualified) };
        interp
            .eval_completion(b"rename host moved; proc host {} {set ::replacement_ran 1}")
            .unwrap();
        let identity = EngineCommandIdentity::from(receipt);
        let allowed = Owned::fresh(new_string(b"rename"));
        // SAFETY: renamed original generation retains its issuing identity.
        assert_eq!(
            unsafe {
                tcl_engine_restrict_original_receipts(&mut interp, allowed.as_ptr(), &identity, 1)
            },
            TCL_OK
        );
        assert!(interp.resolve_cmd_token(b"host").is_none());
        assert_eq!(
            interp.eval_completion(b"moved").unwrap().result,
            b"ORIGINAL\0\xff"
        );
        interp.eval_completion(b"rename moved {}").unwrap();
        let before = installed_generations(&interp);
        // SAFETY: the retained original header is valid data but its token is retired.
        assert_eq!(
            unsafe {
                tcl_engine_restrict_original_receipts(&mut interp, allowed.as_ptr(), &identity, 1)
            },
            TCL_ERROR
        );
        assert_eq!(installed_generations(&interp), before);
    }

    #[test]
    fn command_receipt_guard_rejects_replacement_and_foreign_owner_before_effects() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = core("tcl8.6");
        let original = create(&mut interp, b"host");
        // SAFETY: adopt genuine original address ownership.
        let _original_name = unsafe { Owned::from_raw(original.qualified) };
        let mut foreign = core("tcl8.6");
        // SAFETY: genuine old receipt is checked against a different live producer.
        assert_eq!(
            unsafe {
                tcl_engine_command_receipt_current(
                    &mut foreign,
                    original.owner,
                    original.interpreter,
                    original.generation,
                    b"host".as_ptr(),
                    4,
                )
            },
            TCL_ERROR
        );
        assert!(foreign.host_refusal_pending());
        let replacement = create(&mut interp, b"host");
        // SAFETY: adopt genuine replacement address ownership.
        let _replacement_name = unsafe { Owned::from_raw(replacement.qualified) };
        assert_ne!(original.generation, replacement.generation);
        // SAFETY: same interpreter and written name cannot restore the old generation.
        assert_eq!(
            unsafe {
                tcl_engine_command_receipt_current(
                    &mut interp,
                    original.owner,
                    original.interpreter,
                    original.generation,
                    b"host".as_ptr(),
                    4,
                )
            },
            TCL_ERROR
        );
        assert!(interp.host_refusal_pending());
    }

    fn refuse(interp: &mut Interp, _: &[*mut TclObj]) -> crate::interp::Code {
        let reason = b"actual reached ABI refusal";
        // SAFETY: actual dispatcher supplies interpreter; original reason is live.
        unsafe { tcl_engine_refuse_host_counted(interp, reason.as_ptr(), 26) };
        crate::interp::Code::Error
    }

    #[test]
    fn counted_host_refusal_stays_outside_catch_and_nullable_guest_getters() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = core("tcl8.6");
        interp.register_builtin(b"refuse", refuse);
        let failure = interp
            .eval_completion(b"set ::before 1; catch {refuse}; set ::after 1")
            .unwrap_err();
        assert_eq!(Some(failure.clone()), interp.native_execution_refusal());
        assert!(interp.var_get(b"::before").is_ok());
        assert!(interp.var_get(b"::after").is_err());
        // SAFETY: original interpreter remains live; null is a transport placeholder.
        assert_eq!(unsafe { tcl_engine_host_refusal_pending(&mut interp) }, 1);
        assert!(
            unsafe { tcl_engine_original_string_snapshot(&mut interp, core::ptr::null_mut()) }
                .is_null()
        );
        let text = unsafe { tcl_engine_host_refusal_text(&mut interp) };
        // SAFETY: host presentation transferred one real independent reference.
        let text = unsafe { Owned::from_raw(text) };
        assert_eq!(
            crate::obj::bytes_of(text.as_ptr()),
            b"actual reached ABI refusal"
        );
        assert_eq!(Some(failure), interp.native_execution_refusal());
    }
}
