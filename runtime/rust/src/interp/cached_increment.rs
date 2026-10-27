// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pure eligibility before the original named increment enters any effects.

use super::Interp;
use crate::obj::{self, TclObj};
use tcl_syntax::{number::Number, scalar_getter::NativeScalarGetterProtocol};

impl Interp {
    pub(crate) fn cached_increment_name_matches(
        &self,
        name: &[u8],
        expected: *mut TclObj,
        delta: *mut TclObj,
        old: i64,
        sum: i64,
    ) -> bool {
        if old.checked_add(1) != Some(sum)
            || self.pending_delete.get()
            || self.current_ns.get() != crate::namespace::GLOBAL
            || !self
                .frames
                .try_borrow()
                .is_ok_and(|frames| frames.cached_increment_is_root())
            || !self
                .observed_names
                .try_borrow()
                .is_ok_and(|names| names.is_none())
        {
            return false;
        }
        let Some(names) = self.native_c_variable_name_protocol() else {
            return false;
        };
        if names.version() != tcl_dialect::TclVersion::V9_0 {
            return false;
        }
        let recipe = tcl_syntax::naming::NativeNameProtocol::C(names.version());
        let projected = recipe.variable_root_input(name);
        if projected.selected() != name
            || projected.qualification() != tcl_syntax::naming::NativeNameQualification::Unqualified
            || names.parsed_array_parts(name).is_some()
        {
            return false;
        }
        let quiet = self.traces.try_borrow().is_ok_and(|traces| {
            traces.traces.is_empty()
                && traces.cmd_traces.is_empty()
                && traces.step_active.is_empty()
                && traces.firing_exec_traces.is_empty()
                && traces.firing_cmd_traces.is_empty()
                && traces.exec_firing == 0
                && (traces.env.is_none() || name != b"env")
        });
        if !quiet {
            return false;
        }
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_scalar_getter_protocol()
        else {
            return false;
        };
        if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V9_0)
            || !cached_int_matches(expected, old, protocol)
            || !cached_int_matches(delta, 1, protocol)
        {
            return false;
        }
        self.namespaces.try_borrow().is_ok_and(|namespaces| {
            namespaces
                .original_variable_table_at(crate::namespace::GLOBAL)
                .and_then(|table| table.cached_increment_scalar_identity(name, expected))
                .is_some_and(|identity| {
                    self.native_error_cells.try_borrow().is_ok_and(|cells| {
                        cells.iter().all(|(_, registered)| *registered != identity)
                    }) && self.native_precision_cell.get() != Some(identity)
                })
        })
    }
}

fn cached_int_matches(
    value: *mut TclObj,
    expected: i64,
    protocol: NativeScalarGetterProtocol,
) -> bool {
    if obj::check_native_liveness(value).is_err() {
        return false;
    }
    if obj::native_scalar_descriptor_origin(value)
        .is_some_and(|origin| Some(origin) != protocol.tcl_version())
    {
        return false;
    }
    let Ok(Some(cache)) = obj::native_scalar_cache(value) else {
        return false;
    };
    let Some(conversion) = protocol.increment_number_preflight(Some(&cache), None, false) else {
        return false;
    };
    conversion.cache().is_none()
        && matches!(conversion.outcome(), Ok(Number::Int(actual)) if *actual == expected)
}
