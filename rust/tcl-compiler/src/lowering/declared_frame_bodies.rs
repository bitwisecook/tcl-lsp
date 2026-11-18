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

//! Each called procedure's body, lowered again in its own declared frame.

use super::{IrulesExecutionContext, Lowerer, Procedure};
use crate::ir::{ProcedureBodyKind, Script};
use crate::var_resolve::VariableExecutionFrame;

impl Lowerer<'_> {
    /// Lower again, in its own declared frame, the body of every named
    /// procedure whose commands the walk bound in a caller's activation, or a
    /// join of several: the body a transfer summary describes, whatever the
    /// caller. The second lowering runs against a snapshot of the module, so
    /// it registers nothing; only the bodies are kept, in
    /// `Module::declared_frame_bodies`.
    pub(super) fn retain_declared_frame_bodies(&mut self) {
        let (Some(input), Some(bindings)) = (
            self.module.source_metadata_input.clone(),
            self.module_source_bindings.clone(),
        ) else {
            return;
        };
        let candidates = self
            .module
            .procedures
            .iter()
            .filter(|(_, procedure)| procedure.body_kind == ProcedureBodyKind::Named)
            .filter_map(|(id, procedure)| {
                let frame =
                    crate::command_binding::SourceOriginalVariableFrame::from_authored_procedure(
                        procedure, &input,
                    )?
                    .frame()
                    .clone();
                (!runs_in(&procedure.body, &frame)).then(|| (*id, procedure.clone(), frame))
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return;
        }
        // The second lowering shares no cache and leaves no state behind:
        // a body lowered under the view is never one the build reuses.
        let snapshot = self.module.clone();
        let integrity = self.optimiser_lowering_integrity.take();
        let body_cache = self.body_cache.take();
        let syntax_cache = self.source_body_syntax_cache.take();
        let original_cache = self.original_body_cache.take();
        let aliases = self.aliases.clone();
        let directives = self.namespace_directive_sites.clone();
        let imports = self.namespace_imports.clone();
        let exports = self.namespace_exports.clone();
        let instance_vars = self.class_instance_vars.clone();
        let when_counts = self.when_counts.clone();
        let body_units = self.body_unit_count;
        let enclosing = self.source_bindings.take();
        let registering = std::mem::replace(&mut self.suppress_proc_register, true);
        let mut bodies = std::collections::BTreeMap::new();
        for (id, procedure, frame) in candidates {
            let (Some(source), Some(namespace), Some(text)) = (
                procedure.body.executed_source.as_deref().cloned(),
                procedure.body.namespace_context.as_deref().cloned(),
                procedure.body_source.as_deref(),
            ) else {
                continue;
            };
            let Ok(length) = u32::try_from(text.len()) else {
                continue;
            };
            let span = tcl_lexer::Span::new(procedure.body_offset, procedure.body_offset + length);
            let Some(view) = bindings.declared_frame_view(&frame, span) else {
                continue;
            };
            self.source_bindings = Some(view.into());
            let body = self
                .in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                    lowerer.lower_original_body(source, Some(&namespace))
                });
            if runs_in(&body, &frame) {
                bodies.insert(id, Procedure { body, ..procedure });
            }
        }
        self.source_bindings = enclosing;
        self.suppress_proc_register = registering;
        self.module = snapshot;
        self.optimiser_lowering_integrity = integrity;
        self.body_cache = body_cache;
        self.source_body_syntax_cache = syntax_cache;
        self.original_body_cache = original_cache;
        self.aliases = aliases;
        self.namespace_directive_sites = directives;
        self.namespace_imports = imports;
        self.namespace_exports = exports;
        self.class_instance_vars = instance_vars;
        self.when_counts = when_counts;
        self.body_unit_count = body_units;
        self.module.declared_frame_bodies = bodies;
    }
}

/// Whether every source command of `body` was bound in `frame`.
fn runs_in(body: &Script, frame: &VariableExecutionFrame) -> bool {
    body.statements
        .iter()
        .filter(|statement| statement.synthetic_marker().is_none())
        .all(|statement| {
            body.retained_source_tokens_for_statement(statement)
                .and_then(|tokens| tokens.source_binding.as_ref())
                .is_some_and(|binding| &binding.variable_frame == frame)
        })
}
