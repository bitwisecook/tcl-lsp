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

//! Conditional CFG and lattice facts for an authentic original child body.

use super::{CompilationUnit, FunctionLatticeInputs, FunctionUnit, ModuleTraceFacts, UnitDialect};
use tcl_registry::CommandRegistry;

impl CompilationUnit {
    fn conditional_original_body_input(
        &self,
        registry: &CommandRegistry,
        original_function: &FunctionUnit,
        #[cfg(test)] trace_enabled: bool,
        #[cfg(test)] trace: &impl Fn(&str, bool),
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        let module = &self.ir_module;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "conditional_body.document",
                module.source == tcl_lexer::SourceImage::document(&self.source),
            );
        }
        if module.source != tcl_lexer::SourceImage::document(&self.source) {
            return None;
        }
        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module);
        #[cfg(test)]
        if trace_enabled {
            trace("conditional_body.module_metadata", metadata.is_some());
        }
        metadata?;
        let input = module.source_metadata_input.as_ref()?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "conditional_body.input_config",
                input.lexer_config() == module.lexer_config,
            );
        }
        if input.lexer_config() != module.lexer_config {
            return None;
        }
        #[cfg(test)]
        if trace_enabled {
            trace(
                "conditional_body.retained",
                module.retained_source_bindings.is_some(),
            );
        }
        let positioned = module
            .retained_source_bindings
            .as_deref()?
            .positioned_for_module(module, registry);
        #[cfg(test)]
        if trace_enabled {
            trace("conditional_body.positioned", positioned.is_some());
        }
        positioned?;
        let function_metadata =
            original_function.invocation_metadata_context_for_module(registry, module);
        #[cfg(test)]
        if trace_enabled {
            trace(
                "conditional_body.function_metadata",
                function_metadata.is_some(),
            );
        }
        function_metadata?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "conditional_body.original_function",
                self.all_body_function_units()
                    .any(|owned| std::ptr::eq(owned, original_function)),
            );
        }
        if !self
            .all_body_function_units()
            .any(|owned| std::ptr::eq(owned, original_function))
        {
            return None;
        }
        Some(input)
    }

    /// Build conditional CFG/lattice facts for an authentic original child body.
    /// The actual CU function and original parent vector select the source;
    /// report labels supply no body, namespace or entered-frame assumptions.
    pub(crate) fn conditional_original_body_unit(
        &self,
        registry: &CommandRegistry,
        original_function: &FunctionUnit,
        parent: &crate::ir::CommandTokens,
        argument: usize,
    ) -> Option<FunctionUnit> {
        // naming.source.original-collection-body-owner
        // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
        #[cfg(test)]
        let trace_enabled =
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
        #[cfg(test)]
        let trace = |stage: &str, passed: bool| {
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
                let offset = parent.argv.first().map(|span| span.start());
                eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset:?} passed={passed}");
            }
        };
        let module = &self.ir_module;
        let input = self.conditional_original_body_input(
            registry,
            original_function,
            #[cfg(test)]
            trace_enabled,
            #[cfg(test)]
            &trace,
        )?;
        let script =
            crate::lowering::Lowerer::original_body_for_module(module, registry, parent, argument);
        #[cfg(test)]
        if trace_enabled {
            trace("conditional_body.script", script.is_some());
        }
        let script = script?;
        script.executed_source.as_ref()?;
        script.namespace_context.as_ref()?;
        let config = tcl_lexer::SourceSegmentationPurpose::NestedScript
            .config(original_function.source_lexer_config());
        let child_input = input.for_nested_source();
        let cfg = crate::cfg_builder::build_analysis_body(
            "::conditional_original_body",
            &script,
            crate::ir::ExecutionNamespace::RuntimeSelected,
            registry,
            config,
            child_input.context_registry(),
        );
        let known_classes = self.retained_source_class_names(registry);
        #[cfg(test)]
        if trace_enabled {
            trace("conditional_body.class_facts", known_classes.is_some());
        }
        let known_classes = known_classes?;
        let context = crate::value_transfer::AnalysisContextKey::for_module(
            &self.command_mutations,
            registry,
        )
        .with_deferred_writes(module.deferred_writes.clone());
        Some(FunctionUnit::build_conditional_original_body(
            cfg,
            FunctionLatticeInputs {
                dialect: UnitDialect {
                    registry,
                    config,
                    source_metadata_input: Some(&child_input),
                },
                param_constants: None,
                known_classes,
                trace_facts: ModuleTraceFacts {
                    traced_variables: &module.traced_variables,
                    has_dynamic_variable_trace: module.has_dynamic_variable_trace,
                    deferred_writes: &module.deferred_writes,
                },
                analysis_context: &context,
                command_trust: &self.command_mutations,
                // A child script is not the original enclosing event body.
                event_body: None,
            },
        ))
    }
}
