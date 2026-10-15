// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual retained Jim subcommand-table primary; it has no string updater.

use super::{TclObj, TclObjType};
use tcl_core_types::NativeJimSubcommandCache;
use tcl_syntax::value::ValueError;

extern "C" fn free(original: *mut TclObj) {
    // SAFETY: this exact descriptor owns the boxed retained table entry.
    unsafe {
        drop(Box::from_raw(
            super::internal_rep(original) as usize as *mut NativeJimSubcommandCache
        ));
    }
}

extern "C" fn duplicate(original: *mut TclObj, copy: *mut TclObj) {
    let retained = cache(original).expect("actual subcommand primary");
    super::change_type(
        copy,
        &TYPE,
        Box::into_raw(Box::new(retained)) as usize as u64,
    );
}

static TYPE: TclObjType = TclObjType {
    name: c"subcmd-lookup".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(crate) fn cache(original: *mut TclObj) -> Option<NativeJimSubcommandCache> {
    if !core::ptr::eq(super::obj_type_ptr(original), &TYPE) {
        return None;
    }
    // SAFETY: the exact descriptor owns this still-live representation.
    Some(unsafe {
        (&*(super::internal_rep(original) as usize as *const NativeJimSubcommandCache)).clone()
    })
}

pub(crate) fn install<E: From<ValueError>>(
    original: *mut TclObj,
    retained: NativeJimSubcommandCache,
    dialect: tcl_registry::InvocationDialect,
    check: &mut dyn FnMut() -> Result<(), E>,
) -> Result<(), E> {
    check()?;
    super::check_native_liveness(original)?;
    let table = dialect.native_jim_namespace_subcommand_table().ok_or(
        ValueError::CommandProtocolUnavailable("Jim original subcommand table issuer"),
    )?;
    if table.cached_entry(&retained).is_none() {
        return Err(ValueError::CommandProtocolUnavailable(
            "foreign original Jim subcommand table",
        )
        .into());
    }
    let mut current = || {
        check()?;
        super::check_native_liveness(original)?;
        if !super::has_string_rep(original) {
            return Err(
                ValueError::CommandProtocolUnavailable("Jim subcommand resident spelling").into(),
            );
        }
        Ok(())
    };
    current()?;
    let incoming = Box::into_raw(Box::new(retained));
    let reached = super::change_type_with_stage_check(
        original,
        &TYPE,
        incoming as usize as u64,
        &mut current,
    );
    if reached.is_err() {
        // SAFETY: a refused transfer leaves the incoming backing unadopted.
        drop(unsafe { Box::from_raw(incoming) });
    }
    reached
}
