// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C variable-name descriptors and their original object owners.

use super::{Owned, TclObj, TclObjType};
use tcl_syntax::{
    native_variable_name::{
        NativeLocalVariableName, NativeParsedVariableElement, NativeParsedVariableName,
    },
    value::ValueError,
};

pub(crate) type Parsed = NativeParsedVariableName<Owned>;
pub(crate) type Local = NativeLocalVariableName<
    tcl_runtime_api::native_procedure_roles::NativeProcedureReference<crate::interp::ProcDef>,
    Owned,
>;

extern "C" fn free_parsed(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns this boxed primary.
    unsafe {
        drop(Box::from_raw(
            super::internal_rep(value) as usize as *mut Parsed
        ))
    };
}
extern "C" fn duplicate_parsed(source: *mut TclObj, target: *mut TclObj) {
    with_parsed(source, |cache| {
        install_parsed_unchecked(target, cache.duplicate().expect("native parsed-name shape"));
    })
    .expect("native parsed-name descriptor");
}
extern "C" fn update_parsed(value: *mut TclObj) {
    let bytes = with_parsed(value, |cache| {
        let (root, NativeParsedVariableElement::Bytes(element)) = cache
            .array
            .as_ref()
            .expect("native C8 parsed array updater")
        else {
            unreachable!("native C8 element")
        };
        cache
            .protocol
            .parsed_array_string(&super::bytes_of(root.as_ptr()), element)
            .expect("native C8 parsed array updater")
    })
    .expect("native parsed-name descriptor");
    // SAFETY: the selected descriptor owns the generated resident buffer.
    unsafe { super::set_native_updater_string_rep(value, &bytes, bytes.is_empty()) };
}
extern "C" fn free_local(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns this boxed primary.
    unsafe {
        drop(Box::from_raw(
            super::internal_rep(value) as usize as *mut Local
        ))
    };
}
extern "C" fn duplicate_local(source: *mut TclObj, target: *mut TclObj) {
    with_local(source, |cache| {
        let mut copy = cache.clone();
        if matches!(
            copy.owner,
            tcl_syntax::native_variable_name::NativeLocalVariableOwner::Name(None)
        ) {
            copy.owner = tcl_syntax::native_variable_name::NativeLocalVariableOwner::Name(Some(
                Owned::retain(source),
            ));
        }
        install_local_unchecked(target, copy);
    })
    .expect("native local-name descriptor");
}

static PARSED_C8: TclObjType = TclObjType {
    name: c"parsedVarName".as_ptr(),
    free_int_rep_proc: Some(free_parsed),
    dup_int_rep_proc: Some(duplicate_parsed),
    update_string_proc: Some(update_parsed),
    set_from_any_proc: None,
};
static PARSED_C9: TclObjType = TclObjType {
    name: c"parsedVarName".as_ptr(),
    free_int_rep_proc: Some(free_parsed),
    dup_int_rep_proc: Some(duplicate_parsed),
    update_string_proc: None,
    set_from_any_proc: None,
};
extern "C" fn update_local_c84(value: *mut TclObj) {
    let bytes = with_local(value, |cache| {
        let tcl_syntax::native_variable_name::NativeLocalVariableOwner::Procedure(owner) =
            &cache.owner
        else {
            unreachable!("C84 local-name owner");
        };
        owner
            .owner()
            .native_compiled_name(cache.index)
            .expect("live admitted C84 compiled declaration")
    })
    .expect("C84 local-name descriptor");
    // SAFETY: the authentic updater owns the original live header; C84 allocates even empty names.
    unsafe { super::set_native_updater_string_rep(value, &bytes, false) };
}
static LOCAL_C84: TclObjType = TclObjType {
    name: c"localVarName".as_ptr(),
    free_int_rep_proc: Some(free_local),
    dup_int_rep_proc: Some(duplicate_local),
    update_string_proc: Some(update_local_c84),
    set_from_any_proc: None,
};
static LOCAL: TclObjType = TclObjType {
    name: c"localVarName".as_ptr(),
    free_int_rep_proc: Some(free_local),
    dup_int_rep_proc: Some(duplicate_local),
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(crate) fn with_parsed<R>(value: *mut TclObj, borrow: impl FnOnce(&Parsed) -> R) -> Option<R> {
    let descriptor = super::obj_type_ptr(value);
    if !core::ptr::eq(descriptor, &PARSED_C8) && !core::ptr::eq(descriptor, &PARSED_C9) {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates this live boxed primary.
    Some(borrow(unsafe {
        &*(super::internal_rep(value) as usize as *const Parsed)
    }))
}
pub(crate) fn with_local<R>(value: *mut TclObj, borrow: impl FnOnce(&Local) -> R) -> Option<R> {
    if !core::ptr::eq(super::obj_type_ptr(value), &LOCAL)
        && !core::ptr::eq(super::obj_type_ptr(value), &LOCAL_C84)
    {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates this live boxed primary.
    Some(borrow(unsafe {
        &*(super::internal_rep(value) as usize as *const Local)
    }))
}
pub(crate) fn install_parsed(
    value: *mut TclObj,
    cache: Parsed,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), ValueError> {
    if dialect.native_variable_name_protocol() != Some(cache.protocol)
        || !super::has_string_rep(value)
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "native parsed variable-name origin/resident storage",
        ));
    }
    let valid = cache
        .array
        .as_ref()
        .is_none_or(|(_, element)| match element {
            NativeParsedVariableElement::Bytes(_) => cache.protocol.has_parsed_array_updater(),
            NativeParsedVariableElement::Object(_) => !cache.protocol.has_parsed_array_updater(),
        });
    if !valid {
        return Err(ValueError::CommandProtocolUnavailable(
            "native parsed variable-name parts",
        ));
    }
    install_parsed_unchecked(value, cache);
    Ok(())
}
fn install_parsed_unchecked(value: *mut TclObj, cache: Parsed) {
    let descriptor = if cache.protocol.has_parsed_array_updater() {
        &PARSED_C8
    } else {
        &PARSED_C9
    };
    super::change_type(
        value,
        descriptor,
        Box::into_raw(Box::new(cache)) as usize as u64,
    );
}
pub(crate) fn install_local(
    value: *mut TclObj,
    cache: Local,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), ValueError> {
    if dialect.native_variable_name_protocol() != Some(cache.protocol)
        || !super::has_string_rep(value)
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "native local variable-name origin/resident storage",
        ));
    }
    if cache.protocol.local_cache_owns_procedure()
        != matches!(
            cache.owner,
            tcl_syntax::native_variable_name::NativeLocalVariableOwner::Procedure(_)
        )
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "native local variable-name owner",
        ));
    }
    install_local_unchecked(value, cache);
    Ok(())
}
fn install_local_unchecked(value: *mut TclObj, cache: Local) {
    let descriptor = if cache.protocol.local_cache_owns_procedure() {
        &LOCAL_C84
    } else {
        &LOCAL
    };
    super::change_type(
        value,
        descriptor,
        Box::into_raw(Box::new(cache)) as usize as u64,
    );
}
