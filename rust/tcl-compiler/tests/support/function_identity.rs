// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
        let (&id, _) = declarations.next()?;
        assert!(
            declarations.next().is_none(),
            "authored fixture {display:?} ambiguous"
        );
        self.function_for_body(Some(id))
    }
}
