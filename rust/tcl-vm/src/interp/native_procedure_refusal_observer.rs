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

//! Bounded WORK-only provenance of an already-returned body preparation failure.
//! Addresses identify held Rust allocations within this process only. Captured
//! entry fields are observations, not renewed currency or execution authority.
//! Full body bytes are observed only up to 1024; a truncated prefix grants no identity.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::ProcDef;

static ROWS: AtomicUsize = AtomicUsize::new(0);
const MAX_ROWS: usize = 16;
const MAX_NAME_BYTES: usize = 96;
const MAX_SOURCE_BYTES: usize = 1024;

pub(super) fn report(
    declaration: Option<&ProcDef>,
    source: &tcl_runtime_api::SourceImage,
    entry: &tcl_runtime_api::NativeCompilationEntry,
) {
    if std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() != Ok("first-decline-v1")
        || ROWS
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |rows| {
                (rows < MAX_ROWS).then_some(rows + 1)
            })
            .is_err()
    {
        return;
    }
    let name = declaration.map_or(&[][..], |declaration| declaration.name.as_bytes());
    let declaration_address = declaration.map(|value| std::ptr::from_ref(value) as usize);
    let body_value_slot_address =
        declaration.map(|value| std::ptr::from_ref(&value.body_src) as usize);
    let prefix = &name[..name.len().min(MAX_NAME_BYTES)];
    let source_prefix = &source.bytes()[..source.len().min(MAX_SOURCE_BYTES)];
    eprintln!(
        "ZZ_NATIVE_PROCEDURE_PREPARATION_REFUSAL declaration={declaration_address:?} body_value_slot={body_value_slot_address:?} name_len={} name_complete={} name_prefix={prefix:?} source_bytes_address={:p} source_len={} source_complete={} source_prefix={source_prefix:?} interpreter={:?} captured_epoch={} captured_namespace={} captured_execution_point={:?} captured_name_protocol={:?} captured_variable_protocol={:?}",
        name.len(),
        name.len() <= MAX_NAME_BYTES,
        source.bytes().as_ptr(),
        source.len(),
        source.len() <= MAX_SOURCE_BYTES,
        entry.interpreter,
        entry.epoch,
        entry.current_namespace,
        entry.execution_point,
        entry.name_protocol,
        entry.compiled_variable_protocol,
    );
}
