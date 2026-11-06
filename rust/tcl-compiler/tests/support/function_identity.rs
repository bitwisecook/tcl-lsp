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

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};

pub trait OriginalFixtureFunctions {
    fn fixture_function(&self, display: &str) -> Option<&FunctionUnit>;
}

impl OriginalFixtureFunctions for CompilationUnit {
    fn fixture_function(&self, display: &str) -> Option<&FunctionUnit> {
        if display == "::top" {
            return (!self.ir_module.procedures.has_display(display))
                .then(|| self.function_for_body(None))
                .flatten();
        }
        let mut declarations = self.ir_module.procedures.iter().filter(|(_, procedure)| {
            procedure.qualified_name == display
                && procedure.declaration_site.as_ref().is_some_and(|site| {
                    site.source.source_image().bytes() == self.source.as_bytes()
                })
        });
        if let Some((&id, _)) = declarations.next() {
            assert!(
                declarations.next().is_none(),
                "authored fixture {display:?} ambiguous"
            );
            return self.function_for_body(Some(id));
        }

        // Original authored declarations have analysis-only body IDs. Their
        // retained source card validates the same-source namespace/name label;
        // this lookup does not publish a callable or make the body transformable.
        let registry = self.ir_module.resolved_registry();
        let mut original = self
            .ir_module
            .original_authored_declaration_body_units
            .keys()
            .filter_map(|&id| {
                let procedure = self.ir_module.body_units.get(&id)?;
                (self.ir_module.procedure_ids.owns(id)
                    && procedure.body_kind
                        == tcl_compiler::ir::ProcedureBodyKind::OriginalDeclaration
                    && procedure.publication.is_none()
                    && procedure.declaration_site.as_ref().is_some_and(|site| {
                        site.source.source_image().bytes() == self.source.as_bytes()
                    })
                    && self.ir_module.original_procedure_report_label(id, registry)
                        == Some(display))
                .then_some(id)
            });
        let id = original.next()?;
        assert!(
            original.next().is_none(),
            "authored fixture {display:?} ambiguous"
        );
        self.function_for_body(Some(id))
    }
}
