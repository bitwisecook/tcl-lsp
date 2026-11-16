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

//! Variable access through the resolver: reads and stores, selected cells, constants and confinement, traces, unset, array targets and links.

use super::{native_operation_currency, Code, Interp};
use crate::frame::{Link, VarError};
use crate::namespace::NsId;

mod access;
mod array_targets;
mod selected_cells;
mod traces;
mod unset;

impl Interp {
    /// `variable tail` / `global tail` — link `tail` in the current frame to
    /// `target_ns::tail` (a no-op when the current context already is that var).
    pub(crate) fn make_variable(&mut self, target_ns: NsId, tail: &[u8]) -> Code {
        self.make_variable_mapped(target_ns, tail, tail)
    }

    /// Generic publication preserves adoption before old-role retirement.
    pub(crate) fn make_variable_mapped(
        &mut self,
        target_ns: NsId,
        local: &[u8],
        target: &[u8],
    ) -> Code {
        self.note_variable_link(local, target);
        let operation = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(operation) => operation,
            Err(first) => return self.refuse_native_execution(first),
        };
        if let Err(error) = self.require_variable_name_protocol() {
            return crate::builtins::var_error(self, local, error);
        }
        let publication = crate::vars::publish_variable_mapped(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            target_ns,
            local,
            target,
        );
        self.finish_generic_variable_link(publication, operation.currency(), local)
    }

    fn finish_generic_variable_link(
        &mut self,
        publication: crate::vars::PublishedVariableLink,
        currency: &native_operation_currency::NativeOperationCurrency,
        local: &[u8],
    ) -> Code {
        match publication.finish(self, currency) {
            Ok(()) => Code::Ok,
            Err(crate::frame::JimLinkStoreError::Command(error)) => self.report_cmd_error(error),
            Err(crate::frame::JimLinkStoreError::Variable(error)) => {
                crate::builtins::var_error(self, local, error)
            }
        }
    }

    /// Capture the selected target cell before installing an element alias.
    pub(crate) fn prepare_upvar_target(&mut self, target: &mut Link) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            return self.observed_refusal();
        }
        crate::vars::prepare_upvar_target(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            target,
        )
    }

    /// `upvar` — link `local` in the current frame to the resolved `target`.
    pub(crate) fn make_upvar(&mut self, target: Link, local: &[u8]) -> Code {
        self.note_variable_link(local, &target.name);
        let operation = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(operation) => operation,
            Err(first) => return self.refuse_native_execution(first),
        };
        if let Err(error) = self.require_variable_name_protocol() {
            return crate::builtins::var_error(self, local, error);
        }
        let publication = crate::vars::publish_upvar(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            target,
            local,
        );
        self.finish_generic_variable_link(publication, operation.currency(), local)
    }

    /// The storage resolver's lifetime check for an `upvar` alias.
    pub(crate) fn upvar_would_invert(&self, target: &Link, local: &[u8]) -> bool {
        crate::vars::upvar_would_invert(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            target,
            local,
        )
    }

    /// `upvar … target ns::tail` — install the link as namespace variable
    /// `home_ns::tail` (a qualified local name).
    pub(crate) fn make_upvar_in(&mut self, home_ns: NsId, tail: &[u8], target: Link) -> Code {
        self.note_variable_link(tail, &target.name);
        let operation = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(operation) => operation,
            Err(first) => return self.refuse_native_execution(first),
        };
        if let Err(error) = self.require_variable_name_protocol() {
            return crate::builtins::var_error(self, tail, error);
        }
        let publication = crate::vars::publish_upvar_in(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            home_ns,
            tail,
            target,
        );
        self.finish_generic_variable_link(publication, operation.currency(), tail)
    }
}
