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

//! The call-frame stack and per-frame variable storage.
//!
//! Frame 0 is the global call level. Procedure frames own local name tables;
//! namespace variables instead belong to the stable namespace token's table.

use std::rc::Rc;
use std::sync::atomic::AtomicU64;

/// The lifetime of one actual frame, independent of its reusable stack level.
#[derive(Debug)]
pub(crate) struct ActivationIdentity {
    pub(crate) serial: u64,
}

impl ActivationIdentity {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let serial = tcl_runtime_api::checked_counter::allocate(&NEXT)
            .expect("activation identity space exhausted");
        Self { serial }
    }
}

use tcl_runtime_api::NsId;

use crate::value::Value;
use crate::vars::{StaticVariables, VarTable};
use tcl_core_types::VarId;

/// One call frame.
pub(crate) struct CallFrame {
    pub(crate) native_procedure_execution: Option<crate::command::NativeProcedureReference>,
    pub(crate) native_procedure_binding: Option<crate::command::NativeProcedureCommand>,
    pub(crate) native_c_procedure: Option<Rc<crate::command::ProcDef>>,
    pub(crate) native_local_names: Option<Rc<crate::literal_pool::NativeLocalNameTable>>,
    /// Jim changes this lookup ID on successful root unset; activation identity stays fixed.
    pub(crate) jim_lookup_id: std::cell::Cell<u64>,
    /// Actual current namespace object owned by this native Jim frame.
    pub(crate) jim_namespace: std::cell::RefCell<Option<(NsId, Value)>>,
    /// Successful local-command results retained until this real frame is freed.
    pub(crate) jim_local_commands: Vec<Value>,
    /// The sole lifetime owner for retained activation receipts.
    pub(crate) activation: Rc<ActivationIdentity>,
    /// Replacement command scheduled by native tailcall on this activation.
    pub tailcall: Option<crate::exec::TailcallReq>,
    /// Local variables by name.
    pub locals: VarTable,
    /// Ordered physical cells allocated by the procedure compiler.
    pub(crate) compiled_locals: Vec<(tcl_core_types::NameBytes, VarId)>,
    /// Reusable retained layout; physical cells belong only to this activation.
    pub(crate) compiled_local_layout:
        Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    /// Persistent bindings supplied by the selected native procedure.
    pub statics: Option<Rc<StaticVariables>>,
    /// The native namespace token in which this frame executes.
    #[allow(dead_code)]
    pub ns: NsId,
    /// Absolute frame level (0 = global).
    pub level: usize,
    /// The proc this frame belongs to (for `errorInfo`/`info level`); `None` at
    /// top level.
    pub proc_name: Option<String>,
    /// The invocation argv (proc name + args) — used by `info level N`.
    pub call_argv: Vec<Value>,
    /// Jim's actual frame owners, retired with this frame independently of
    /// the procedure declaration and the entered `EvalObj` Script pin.
    pub(crate) procedure_body: Option<Value>,
    pub(crate) procedure_parameters: Option<Value>,
    /// For a `namespace eval`/`inscope` body frame, the canonical namespace it
    /// runs in (no leading `::`; `""` = global). `None` for proc activations and
    /// the global frame. An unqualified variable accessed in such a frame is a
    /// *namespace* variable (`ns::name` in the global frame), not a local — see
    /// the shared variable resolver. This is what makes `uplevel`/`upvar` into
    /// a namespace-eval body resolve to namespace variables.
    pub ns_eval: Option<tcl_core_types::NameBytes>,
}

impl CallFrame {
    pub(crate) fn release_native_jim_owners(&mut self) {
        let parameters = self.procedure_parameters.take();
        let body = self.procedure_body.take();
        let namespace = self.jim_namespace.get_mut().take();
        tcl_runtime_api::jim_interpreter::release_jim_call_frame_objects(
            parameters, body, namespace,
        );
    }

    /// A fresh frame at `level` in namespace `ns`.
    pub fn new(level: usize, ns: NsId, proc_name: Option<String>, call_argv: Vec<Value>) -> Self {
        Self {
            native_c_procedure: None,
            native_procedure_execution: None,
            native_procedure_binding: None,
            native_local_names: None,
            jim_lookup_id: std::cell::Cell::new(ActivationIdentity::new().serial),
            jim_namespace: std::cell::RefCell::new(None),
            jim_local_commands: Vec::new(),
            activation: Rc::new(ActivationIdentity::new()),
            tailcall: None,
            locals: VarTable::new(),
            compiled_locals: Vec::new(),
            compiled_local_layout: None,
            procedure_body: None,
            procedure_parameters: None,
            statics: None,
            ns,
            level,
            proc_name,
            call_argv,
            ns_eval: None,
        }
    }
    /// Native named declaration slots precede dynamic bindings; duplicates remain.
    pub(crate) fn local_bindings(
        &self,
    ) -> impl Iterator<Item = (&tcl_core_types::NameBytes, &VarId)> {
        self.compiled_locals
            .iter()
            .enumerate()
            .filter_map(|(index, (name, cell))| {
                let named = self
                    .compiled_local_layout
                    .as_ref()
                    .is_none_or(|layout| layout.names.get(index).is_some_and(Option::is_some));
                named.then_some((name, cell))
            })
            .chain(self.locals.iter())
    }

    /// Consume every physical binding, including unnamed temporaries, for teardown.
    pub(crate) fn into_local_bindings(
        self,
    ) -> impl Iterator<Item = (tcl_core_types::NameBytes, VarId)> {
        self.compiled_locals.into_iter().chain(self.locals)
    }
}
