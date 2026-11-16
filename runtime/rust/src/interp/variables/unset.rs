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

//! `unset`: removing a variable or an element, the array teardown, and the unset callbacks a destroyed cell fires.

use super::super::{
    drop_fresh, native_variable_observers, native_variable_teardown, new_string, obj_bytes, Interp,
    VarTeardownCallback,
};
use tcl_runtime_api::guard::GuardDomain;

impl Interp {
    /// `unset name` — returns whether it existed.
    pub(crate) fn var_unset(&mut self, name: &[u8]) -> bool {
        if self.observed_name_policy_selected() {
            return self
                .observed_variable_unset(name, self.frames.borrow().current_level())
                .unwrap_or(false);
        }
        let Ok(input) = self.combined_variable_input(name) else {
            return false;
        };
        let root = input.root().selected().to_vec();
        let element = input.element().map(|element| element.selected().to_vec());
        self.remove_captured_byte_variable(
            native_variable_teardown::OriginalVariableLocation {
                namespace: self.current_ns.get(),
                level: None,
            },
            &root,
            element.as_deref(),
            name,
            true,
            false,
        )
        .unwrap_or(false)
    }

    pub(in super::super) fn finish_array_unset(
        &mut self,
        name: &[u8],
        base: &[u8],
        home: Option<crate::vars::TraceHome>,
        selected: crate::frame::RetainedArrayCell,
        current: impl Fn(&Interp) -> Result<(), tcl_runtime_api::NativeExecutionError>,
    ) -> Result<bool, tcl_runtime_api::NativeExecutionError> {
        let mut first = current(self).err();
        let native_error = home
            .as_ref()
            .and_then(|home| self.native_error_variable_at(home));
        let native_precision = home
            .as_ref()
            .is_some_and(|home| self.native_precision_trace_at(home));
        // naming.variable.original-undefined-array-member-retirement
        // docs/design/analysis/name-resolution-proofs/variable-original-undefined-array-member-retirement.md
        // The selected generation keeps original physical entries. Software
        // refusals stop later callbacks while its mandatory owners still retire.
        let elements = selected.search_keys().unwrap_or_default();
        if let Some(home) = home.as_ref() {
            let access = self.trace_access(name, base, None, home, true);
            let root = self.cell_unset_traces(home, None, &access.reported, b"");
            self.detach_destroyed_trace_group(home, None);
            if first.is_none() {
                self.fire_unset_callbacks(root);
                if let Err(error) = current(self) {
                    first.get_or_insert(error);
                }
            }
            for element in &elements {
                let Some(member_home) = home.for_selected_array_member(&selected, element) else {
                    continue;
                };
                let callbacks =
                    self.cell_unset_traces(&member_home, Some(element), &access.reported, element);
                let registrations = self.destroyed_trace_group_ids(&member_home, Some(element));
                self.detach_destroyed_trace_ids(&registrations);
                let preserve_definition =
                    self.native_c_variable_name_protocol()
                        .is_some_and(|protocol| {
                            protocol.element_unset_preserves_definition_during_trace()
                        });
                let trace = selected.begin_member_retirement(
                    element,
                    preserve_definition,
                    first.is_none() && !callbacks.is_empty(),
                );
                selected.retire_member(element);
                if let Err(error) = current(self) {
                    first.get_or_insert(error);
                }
                if first.is_none() {
                    self.fire_unset_callbacks(callbacks);
                    if let Err(error) = current(self) {
                        first.get_or_insert(error);
                    }
                }
                drop(trace);
                if let Err(error) = current(self) {
                    first.get_or_insert(error);
                }
                let object_table = self
                    .native_c_variable_name_protocol()
                    .is_some_and(|protocol| protocol.element_table_retains_original());
                selected.finish_member_retirement(element, object_table);
                if let Err(error) = current(self) {
                    first.get_or_insert(error);
                }
            }
        }
        if let Err(error) = selected.finish_destruction_with_check(|| current(self)) {
            first.get_or_insert(error);
        }
        if first.is_none()
            && native_precision
            && home
                .as_ref()
                .is_some_and(|home| self.native_precision_trace_at(home))
        {
            self.install_native_precision_trace_at(name);
            if let Err(error) = current(self) {
                first.get_or_insert(error);
            }
        }
        if first.is_none() {
            if let Some(variable) = native_error {
                self.install_native_error_variable_trace(variable);
                if let Err(error) = current(self) {
                    first.get_or_insert(error);
                }
            }
        }
        drop(selected);
        if let Err(error) = current(self) {
            first.get_or_insert(error);
        }
        first.map_or(Ok(true), Err)
    }

    fn destroyed_trace_group_ids(
        &self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
    ) -> Vec<u64> {
        let cell = self.variable_trace_scope(home, element);
        self.traces
            .borrow()
            .traces
            .iter()
            .filter(|trace| cell.owns_registration(trace))
            .map(|trace| trace.id)
            .collect()
    }

    fn detach_destroyed_trace_ids(&mut self, registrations: &[u64]) {
        self.traces
            .borrow_mut()
            .traces
            .retain(|trace| !registrations.contains(&trace.id));
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    pub(in super::super) fn detach_destroyed_trace_group(
        &mut self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
    ) {
        let cell = self.variable_trace_scope(home, element);
        let mut traces = self.traces.borrow_mut();
        traces.traces.retain(|trace| !cell.owns_registration(trace));
        crate::env_trace::detach_destroyed_env_trace(&mut traces, home, element);
        drop(traces);
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    /// `unset name(key)` — returns whether it existed.
    /// [`Self::var_unset_elem`], with `reported` (when given) as the name the
    /// unset traces are told in every release — C's `Tcl_UnsetVar2` with the
    /// array and element passed apart, where 9.0 would otherwise report the
    /// `name(key)` spelling.
    pub(crate) fn var_unset_elem_reported(
        &mut self,
        name: &[u8],
        reported: Option<&[u8]>,
        key: &[u8],
    ) -> bool {
        let Some(reported) = reported else {
            return self.var_unset_elem(name, key);
        };
        if self.require_variable_name_protocol().is_err() {
            return false;
        }
        // `reported` names the same global array from the global frame (the
        // `env` re-sync's `Tcl_UnsetVar2(interp, "env", name, 0)`), and the
        // traces are told the array and element apart.
        let _ = name;
        self.unset_byte_variable_at(reported, reported, Some(key), 0, true)
    }

    pub(crate) fn var_unset_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        }
        let mut spelling = name.to_vec();
        spelling.push(b'(');
        spelling.extend_from_slice(key);
        spelling.push(b')');
        self.remove_captured_byte_variable(
            native_variable_teardown::OriginalVariableLocation {
                namespace: self.current_ns.get(),
                level: None,
            },
            name,
            Some(key),
            &spelling,
            true,
            false,
        )
        .unwrap_or(false)
    }

    /// The unset-trace callbacks a **cell** contributes when it is destroyed,
    /// newest-first — C walks the `Var`'s prepended trace list head to tail.
    /// Non-destructive: the caller's own sweep drops the traces.
    pub(in super::super) fn cell_unset_traces(
        &self,
        home: &crate::vars::TraceHome,
        elem: Option<&[u8]>,
        report_name: &[u8],
        report_elem: &[u8],
    ) -> Vec<VarTeardownCallback> {
        let cell = self.variable_trace_scope(home, elem);
        self.traces
            .borrow()
            .traces
            .iter()
            .rev()
            .filter(|trace| cell.owns_registration(trace) && cell.matches(trace, b"unset"))
            .map(|t| {
                (
                    report_name.to_vec(),
                    report_elem.to_vec(),
                    native_variable_observers::Callback::from_trace(t),
                    t.old_style,
                )
            })
            .collect()
    }

    /// Fire collected unset-trace callbacks as `command name {} unset`. Errors
    /// are ignored (an unset trace's result is discarded, as in C).
    pub(in super::super) fn fire_unset_callbacks(&mut self, victims: Vec<VarTeardownCallback>) {
        if self.host_refusal_pending() || victims.is_empty() {
            return;
        }
        let saved = match self.save_native_variable_trace_result(false) {
            Ok(saved) => saved,
            Err(_) => return,
        };
        for (name, elem, cmd, old_style) in victims {
            if self.check_entered_native_operation().is_err() {
                break;
            }
            if let native_variable_observers::Callback::Native(observer) = &cmd {
                self.call_native_variable_observer(observer.clone(), &name, &elem, "unset");
                if self.check_entered_native_operation().is_err() {
                    break;
                }
                continue;
            }
            let native_variable_observers::Callback::Script(cmd) = cmd else {
                unreachable!()
            };
            // A trace registered the deprecated way is called with the `rwua`
            // letter, not the operation name — the teardown path must honour
            // that exactly as the explicit-unset path does (`TraceVarProc`,
            // tclTrace.c 8.6.16:2002-2011).
            let op = tcl_cmd_core::trace::callback_op_word("unset", old_style);
            let args = self.new_list_object(&[
                new_string(&name),
                new_string(&elem),
                new_string(op.as_bytes()),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            if self.clear_return_options_checked().is_err() {
                break;
            }
            let saved_script = match self.save_native_variable_trace_result(true) {
                Ok(saved) => saved,
                Err(_) => break,
            };
            let _ = self.eval_str(&line);
            if let Some(saved_script) = saved_script {
                let _ = self.restore_native_variable_trace_result(saved_script);
            }
            if self.check_entered_native_operation().is_err() {
                break;
            }
        }
        if let Some(saved) = saved {
            let _ = self.restore_native_variable_trace_result(saved);
        }
    }
}
