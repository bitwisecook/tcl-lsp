// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original instname longValue storage with native NULL free/duplicate hooks.

use super::{TclObj, TclObjType};
use tcl_dialect::TclVersion;
use tcl_syntax::native_instruction_name::{NativeInstructionName, NativeReturnInstructionName};
use tcl_syntax::native_string::NativeStringProtocol;

extern "C" fn update(object: *mut TclObj) {
    let name = cache(object).expect("exact native instname descriptor and opcode");
    // SAFETY: the selected immutable instruction table owns the updater bytes.
    unsafe { super::set_native_updater_string_rep(object, name.string_bytes(), false) };
}

const fn descriptor() -> TclObjType {
    TclObjType {
        name: c"instname".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    }
}
static TYPE86: TclObjType = descriptor();
static TYPE90: TclObjType = descriptor();
static TYPE91: TclObjType = descriptor();

pub(crate) fn cache(object: *mut TclObj) -> Option<NativeInstructionName> {
    let descriptor = super::obj_type_ptr(object);
    let version = if core::ptr::eq(descriptor, &TYPE86) {
        TclVersion::V8_6
    } else if core::ptr::eq(descriptor, &TYPE90) {
        TclVersion::V9_0
    } else if core::ptr::eq(descriptor, &TYPE91) {
        TclVersion::V9_1
    } else {
        return None;
    };
    [
        NativeReturnInstructionName::Syntax,
        NativeReturnInstructionName::Immediate,
    ]
    .into_iter()
    .filter_map(|kind| NativeInstructionName::for_return(NativeStringProtocol::C(version), kind))
    .find(|name| u64::from(name.opcode()) == super::internal_rep(object))
}

pub(crate) fn fresh(name: NativeInstructionName) -> *mut TclObj {
    let object = super::new_obj();
    let descriptor = match name.version() {
        TclVersion::V8_6 => &TYPE86,
        TclVersion::V9_0 => &TYPE90,
        TclVersion::V9_1 => &TYPE91,
        _ => unreachable!("selected native instname descriptor"),
    };
    super::invalidate_string(object);
    super::change_type(object, descriptor, u64::from(name.opcode()));
    object
}
