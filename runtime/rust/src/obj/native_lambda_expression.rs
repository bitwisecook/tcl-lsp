// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual anonymous C Proc and original namespace ownership in lambdaExpr.

use super::*;
use crate::interp::NativeCallableProcedure;
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;

pub(crate) struct LambdaExpression {
    pub(crate) interpreter: NativeInterpreterIdentity,
    pub(crate) procedure: NativeCallableProcedure,
    pub(crate) namespace: Owned,
}

extern "C" fn free(value: *mut TclObj) {
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut LambdaExpression
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, copy: *mut TclObj) {
    let original = unsafe { &*(internal_rep(source) as usize as *const LambdaExpression) };
    install(
        copy,
        LambdaExpression {
            interpreter: original.interpreter,
            procedure: original.procedure.duplicate_lambda_role(),
            namespace: original.namespace.clone(),
        },
    );
}
pub(crate) static TYPE: TclObjType = TclObjType {
    name: c"lambdaExpr".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
pub(crate) fn cached(
    value: *mut TclObj,
    interpreter: NativeInterpreterIdentity,
) -> Option<(NativeCallableProcedure, *mut TclObj)> {
    if !core::ptr::eq(obj_type_ptr(value), &TYPE) {
        return None;
    }
    let original = unsafe { &*(internal_rep(value) as usize as *const LambdaExpression) };
    (original.interpreter == interpreter)
        .then(|| (original.procedure.clone(), original.namespace.as_ptr()))
}
pub(crate) fn install(value: *mut TclObj, original: LambdaExpression) {
    change_type(
        value,
        &TYPE,
        Box::into_raw(Box::new(original)) as usize as u64,
    );
}
