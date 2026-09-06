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

use tcl_runtime_api::NsId;

use crate::value::Value;
use crate::vars::VarTable;

/// One call frame.
pub(crate) struct CallFrame {
    /// Local variables by name.
    pub locals: VarTable,
    /// The namespace this frame executes in (currently global-only).
    #[allow(dead_code)]
    pub ns: NsId,
    /// Absolute frame level (0 = global).
    pub level: usize,
    /// The proc this frame belongs to (for `errorInfo`/`info level`); `None` at
    /// top level.
    pub proc_name: Option<String>,
    /// The invocation argv (proc name + args) — used by `info level N`.
    pub call_argv: Vec<Value>,
    /// For a `namespace eval`/`inscope` body frame, the canonical namespace it
    /// runs in (no leading `::`; `""` = global). `None` for proc activations and
    /// the global frame. An unqualified variable accessed in such a frame is a
    /// *namespace* variable (`ns::name` in the global frame), not a local — see
    /// the shared variable resolver. This is what makes `uplevel`/`upvar` into
    /// a namespace-eval body resolve to namespace variables.
    pub ns_eval: Option<String>,
}

impl CallFrame {
    /// A fresh frame at `level` in namespace `ns`.
    pub fn new(level: usize, ns: NsId, proc_name: Option<String>, call_argv: Vec<Value>) -> Self {
        Self {
            locals: VarTable::new(),
            ns,
            level,
            proc_name,
            call_argv,
            ns_eval: None,
        }
    }
}
