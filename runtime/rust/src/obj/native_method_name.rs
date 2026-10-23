// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native TclOO method-name primary sharing an original engine call-chain owner.
use super::*;
use crate::cmd_oo::native_method_cache::NativeMethodChain;
use std::rc::Rc;
extern "C" fn free(value: *mut TclObj) {
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut Rc<NativeMethodChain>
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, copy: *mut TclObj) {
    let chain = unsafe { &*(internal_rep(source) as usize as *const Rc<NativeMethodChain>) };
    change_type(
        copy,
        &TYPE,
        Box::into_raw(Box::new(Rc::clone(chain))) as usize as u64,
    );
}
pub(crate) static TYPE: TclObjType = TclObjType {
    name: c"TclOO method name".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
pub(crate) fn is_cached(value: *mut TclObj) -> bool {
    core::ptr::eq(obj_type_ptr(value), &TYPE)
}
pub(crate) fn chain(value: *mut TclObj) -> Option<Rc<NativeMethodChain>> {
    if !is_cached(value) {
        return None;
    }
    Some(Rc::clone(unsafe {
        &*(internal_rep(value) as usize as *const Rc<NativeMethodChain>)
    }))
}
pub(crate) fn clear(value: *mut TclObj) {
    if is_cached(value) {
        change_type(value, core::ptr::null(), 0);
    }
}
pub(crate) fn install(
    value: *mut TclObj,
    chain: Rc<NativeMethodChain>,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    check: impl Fn() -> Result<(), tcl_cmd_core::CmdError>,
) -> Result<(), tcl_cmd_core::CmdError> {
    check()?;
    check_native_liveness(value)?;
    let result = crate::dict::native_object_bytes(value, protocol);
    check()?;
    result?;
    // Retirement is reached before later cache adoption, including auxiliary
    // header-cache free callbacks. The original world is never recaptured.
    let retired = discard_native_internal_representation(value);
    check()?;
    retired?;
    change_type(value, &TYPE, Box::into_raw(Box::new(chain)) as usize as u64);
    check()
}
