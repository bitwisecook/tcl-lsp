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

//! W250: instantiating an `oo::abstract` class.

use crate::analyser::state::Analyser;
use tcl_core_types::DiagCode;

impl Analyser {
    fn abstract_source_instantiation_diagnostic(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
        span: tcl_lexer::Span,
        #[cfg(test)] trace_state: (bool, &impl Fn(&str, Option<u32>, bool)),
    ) -> Option<crate::analyser::types::Diagnostic> {
        #[cfg(test)]
        let (trace_enabled, trace) = trace_state;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "w250.source_binding",
                Some(span.start()),
                tokens.source_binding.is_some(),
            );
        }
        let binding = tokens.source_binding.as_ref()?;
        let owned =
            binding.original_invocation_metadata_for_module(tokens, &cu.ir_module, registry);
        #[cfg(test)]
        if trace_enabled {
            trace(
                "w250.original_module_metadata",
                Some(span.start()),
                owned.is_some(),
            );
        }
        owned?;
        let image = &cu.ir_module.source;
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            binding.invocation_site()?.offset,
            cu.ir_module.lexer_config,
        );
        #[cfg(test)]
        if trace_enabled {
            trace(
                "w250.original_words",
                Some(span.start()),
                original.is_some(),
            );
        }
        let original = original?;
        let realm = self.result.retained_command_realm();
        #[cfg(test)]
        if trace_enabled {
            trace("w250.analysis_realm", Some(span.start()), realm.is_some());
        }
        let realm = realm?;
        let call = realm.original_source_constructor_call(input, &original);
        #[cfg(test)]
        if trace_enabled {
            trace(
                "w250.source_constructor",
                Some(span.start()),
                call.is_some(),
            );
        }
        let call = call?;
        let attempt = call.source_factory_attempt(&self.result);
        #[cfg(test)]
        if trace_enabled {
            trace("w250.source_attempt", Some(span.start()), attempt.is_some());
        }
        let attempt = attempt?;
        if !attempt
            .factory_traits()
            .contains(tcl_registry::Traits::ABSTRACT_CLASS_FACTORY)
        {
            return None;
        }
        let class_name = String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(
            attempt.class_declaration().source_slot(),
        ))
        .ok()?;
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::W250,
            span,
            format!("Instantiating abstract class '{class_name}' — use a concrete subclass"),
            super::super::types::Severity::Warning,
        ))
    }

    /// W250 — instantiating an `oo::abstract` class.
    ///
    /// `TclOO`'s `oo::abstract` removes `new` / `create` from the class, so
    /// `AbstractClass new` / `AbstractClass create obj` is a runtime error.
    /// Uses the genuine conditional source class incarnation and its original
    /// selected factory operation, including alias prefixes and nested calls.
    /// Constructor export, completion and Native entry remain independent;
    /// stale source owners and uncertain mutation refuse this projection.
    pub(in super::super) fn emit_abstract_instantiation_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.disabled_diagnostics.contains("W250") {
            return;
        }
        let Some(input) = self.result.resolved_input.as_ref() else {
            return;
        };
        let registry = input.borrowed_context_registry().commands();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
            registry,
            &cu.ir_module,
        );
        #[cfg(test)]
        let trace_enabled =
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
        #[cfg(test)]
        let trace = |stage: &str, offset: Option<u32>, passed: bool| {
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
                eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset:?} passed={passed}");
            }
        };
        #[cfg(test)]
        if trace_enabled {
            trace("w250.module_metadata", None, metadata.is_some());
        }
        let Some(metadata) = metadata else {
            return;
        };
        #[cfg(test)]
        if trace_enabled {
            trace(
                "w250.analysis_input",
                None,
                metadata.source_analysis_input() == Some(input),
            );
        }
        #[cfg(test)]
        if trace_enabled {
            trace(
                "w250.analysis_image",
                None,
                self.result
                    .matches_original_source_image(&cu.ir_module.source, cu.ir_module.lexer_config),
            );
        }
        if metadata.source_analysis_input() != Some(input)
            || !self
                .result
                .matches_original_source_image(&cu.ir_module.source, cu.ir_module.lexer_config)
        {
            return;
        }
        let diagnostic = |tokens: &crate::ir::CommandTokens, span: tcl_lexer::Span| {
            self.abstract_source_instantiation_diagnostic(
                cu,
                registry,
                input,
                tokens,
                span,
                #[cfg(test)]
                (trace_enabled, &trace),
            )
        };
        let mut diags = Vec::new();
        for fu in cu.analysable_body_function_units() {
            for statement in fu.cfg.blocks.values().flat_map(|block| &block.statements) {
                let Some(tokens) = statement.tokens() else {
                    continue;
                };
                if let Some(diag) = diagnostic(tokens, fu.abs_span(statement.span())) {
                    diags.push(diag);
                }
                // A normal assignment can remain a generic call. Lift its
                // original evaluated words through the shared substitution
                // owner; no string reparse or AssignValue-only branch can
                // preserve all nested invocation identities.
                for call in crate::word_subst::lifted_calls(Some(tokens), cu.ir_module.lexer_config)
                {
                    if let Some(tokens) = call.tokens
                        && let Some(diag) = diagnostic(&tokens, fu.abs_span(call.span))
                        && !diags.contains(&diag)
                    {
                        diags.push(diag);
                    }
                }
            }
        }

        self.result.diagnostics.extend(diags);
    }
}
