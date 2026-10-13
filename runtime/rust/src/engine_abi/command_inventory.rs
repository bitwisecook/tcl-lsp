// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Callback-free visible table inventory for the engine's current instance.

use crate::interp::{new_string, Interp};
use crate::namespace::GLOBAL;
use crate::obj::{incr_ref_count, TclObj};
use tcl_core_types::ByteCommandSlot;
use tcl_runtime_api::command_inventory::{EngineCommandInventory, EngineCommandInventoryRow};
use tcl_syntax::naming::{native_command_root_source_spelling, NativeNameProtocol};

fn inventory(interp: &Interp) -> Option<EngineCommandInventory> {
    let dialect = interp.native_invocation_dialect();
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
        return None;
    }
    let version = dialect.tcl_version?;
    let protocol = NativeNameProtocol::for_tcl_version(version);
    let namespaces = interp.namespaces();
    let mut pending = vec![GLOBAL];
    let mut rows = Vec::new();
    while let Some(namespace) = pending.pop() {
        pending.extend(namespaces.children(namespace));
        let path = namespaces.native_c_command_context_path(namespace, version)?;
        for simple in interp.visible_command_names_in(namespace) {
            let generation = namespaces.command_generation(namespace, &simple)?;
            let slot = ByteCommandSlot::new(path.clone(), simple.clone().into());
            let root_operand = native_command_root_source_spelling(protocol, &slot)
                .filter(|operand| {
                    namespaces.home_of(GLOBAL, operand.as_bytes())
                        == Some((namespace, simple.clone()))
                        && interp
                            .resolve_dispatchable_with_generation(GLOBAL, operand.as_bytes())
                            .is_some_and(|(_, selected)| selected == Some(generation))
                })
                .map(String::into_bytes);
            rows.push(EngineCommandInventoryRow {
                namespace: u64::try_from(namespace).ok()?,
                generation,
                slot,
                root_operand,
            });
        }
    }
    Some(EngineCommandInventory { version, rows })
}

/// Return owned counted inventory data without evaluating Tcl or parsing reports.
/// Rows include visible imports/aliases; unavailable root operands remain explicit.
/// Numbers and coordinates are transport data, not installed-command receipts.
///
/// # Safety
/// `interp` must be live; the caller owns the returned object's one reference.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tcl_engine_visible_command_inventory(interp: *mut Interp) -> *mut TclObj {
    // SAFETY: the caller supplies its actual live interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return core::ptr::null_mut();
    }
    let Some(inventory) = inventory(interp) else {
        interp.refuse_host_command("visible command inventory has no selected C recipe");
        return core::ptr::null_mut();
    };
    let Some(size) = inventory
        .encoded_len()
        .and_then(|size| u64::try_from(size).ok())
    else {
        interp.refuse_host_command("visible command inventory exceeds counted transport");
        return core::ptr::null_mut();
    };
    if interp.charge_allocation(size).is_some() {
        return core::ptr::null_mut();
    }
    let Some(bytes) = inventory.encode() else {
        interp.refuse_host_command("visible command inventory exceeds counted transport");
        return core::ptr::null_mut();
    };
    let object = new_string(&bytes);
    // SAFETY: the new owned transport String remains live until caller release.
    unsafe { incr_ref_count(object) };
    object
}

#[cfg(test)]
#[path = "command_inventory_tests.rs"]
mod tests;
