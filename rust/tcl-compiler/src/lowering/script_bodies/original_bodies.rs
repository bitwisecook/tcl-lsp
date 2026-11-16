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

//! Lowering an original body in its retained namespace and source context, and retaining an unchanged body slice.

use super::super::{Lowerer, SourceBodyMode};
use crate::command_binding::RetainedSourceBodyInventory;
use crate::ir::{CommandTokens, Module, Script, Statement};
use tcl_registry::CommandRegistry;

pub(in super::super) fn unavailable_original_body_script(
    base_offset: u32,
    len: usize,
    reason: &str,
) -> Script {
    let end = base_offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
    Script::from_statements(vec![Statement::Barrier {
        span: tcl_lexer::Span::new(base_offset, end),
        reason: reason.to_owned(),
        command: String::new(),
        canonical_command: None,
        args: Vec::new(),
        tokens: None,
    }])
}

impl<'r> Lowerer<'r> {
    #[inline(never)]
    fn selected_source_bindings_boxed_in_context(
        &self,
        source: &crate::command_binding::ExecutedScriptSource,
        namespace: &crate::command_binding::SourceNamespaceKey,
    ) -> Option<Box<RetainedSourceBodyInventory>> {
        self.source_bindings
            .as_ref()?
            .selected_source_in_context(source, namespace)
            .map(Box::new)
    }

    /// Lower an original body in its retained namespace incarnation. The
    /// presentation label is independent of command lookup and replay context.
    pub(in super::super) fn lower_executed_script_in_context(
        &mut self,
        source: crate::command_binding::ExecutedScriptSource,
        namespace: &crate::command_binding::SourceNamespaceKey,
    ) -> Script {
        #[cfg(any(test, debug_assertions))]
        let started = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        #[cfg(any(test, debug_assertions))]
        let trace = |phase: &str| {
            if let Some(started) = started {
                eprintln!(
                    "LOWER_ORIGINAL_BODY_PHASE bytes={} base={} phase={phase} scope=body-inclusive ms={}",
                    source.text.len(),
                    source.base(),
                    started.elapsed().as_millis()
                );
            }
        };
        let (Ok(text), Some(bindings)) = (
            source.try_text(),
            self.selected_source_bindings_boxed_in_context(&source, namespace),
        ) else {
            #[cfg(any(test, debug_assertions))]
            trace("unavailable-projection");
            let mut script = unavailable_original_body_script(
                source.base(),
                source.text.len(),
                "original body source admission is unavailable",
            );
            script.executed_source = Some(std::sync::Arc::new(source));
            script.namespace_context = Some(Box::new(namespace.clone()));
            return script;
        };
        #[cfg(any(test, debug_assertions))]
        trace("selected-projection");
        let implicit_math_invocations =
            bindings.math_invocations_for_script(self.registry, &source);
        #[cfg(any(test, debug_assertions))]
        trace("math-inventory");
        let expression_preparations = bindings.expression_preparations_for_script(&source);
        #[cfg(any(test, debug_assertions))]
        trace("expression-inventory");
        let label = namespace.display().unwrap_or_default();
        let enclosing_channel = std::mem::replace(&mut self.source_channel, source.text.channel());
        // Keep the authentic pre-selection owner. A namespace projection memo
        // can filter observations but cannot issue written source geometry.
        let word_source = self.source_bindings.as_ref().and_then(|owner| {
            let input = self
                .invocation_metadata_context()?
                .source_analysis_input()?;
            match self.structured_source_parent.as_deref() {
                Some(parent) => owner.original_body_written_word_owner_at_parent(
                    &source,
                    self.config,
                    self.registry,
                    input,
                    parent,
                ),
                None => owner.original_body_written_word_owner(
                    &source,
                    self.config,
                    self.registry,
                    input,
                ),
            }
        });
        let enclosing_word_source =
            std::mem::replace(&mut self.original_body_word_source, word_source);
        let mut script =
            self.lower_into_script_with_boxed_bindings(text, source.base(), &label, bindings);
        self.original_body_word_source = enclosing_word_source;
        #[cfg(any(test, debug_assertions))]
        trace("lowered");
        self.source_channel = enclosing_channel;
        script.implicit_math_invocations = implicit_math_invocations;
        script.expression_preparations = expression_preparations;
        script.executed_source = Some(std::sync::Arc::new(source));
        script.namespace_context = Some(Box::new(namespace.clone()));
        script
    }

    /// Project an original body from the same actual Module and parent vector.
    /// Only its retained child source and entry namespace select lowering.
    /// This readonly Script provides no new native compiler, frame or Normal
    /// receipt; absent child/origin/stage correspondence is terminal.
    pub(crate) fn original_body_for_module(
        module: &Module,
        registry: &'r CommandRegistry,
        parent: &CommandTokens,
        argument: usize,
    ) -> Option<Script> {
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
        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.module_metadata", metadata.is_some());
        }
        metadata?;
        let input = module.source_metadata_input.as_ref()?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "original_body.retained",
                module.retained_source_bindings.is_some(),
            );
        }
        let retained = module.retained_source_bindings.as_deref()?;
        let bindings = retained.positioned_for_module(module, registry);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.positioned", bindings.is_some());
        }
        let bindings = bindings?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "original_body.parent_vector",
                retained.owns_original_tokens(parent),
            );
        }
        if !retained.owns_original_tokens(parent) {
            return None;
        }
        let body_word = parent.argv.get(argument.checked_add(1)?)?;
        let source = bindings.executed_script_for_word(*body_word);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.selected_script", source.is_some());
        }
        let source = source?.clone();
        source.try_text().ok()?;
        let mut lowerer = Self::with_config(registry, module.lexer_config)
            .with_resolved_analysis_input(input.clone());
        lowerer.set_source_analysis_options(module.source_entry.options());
        lowerer.source_bindings = Some(bindings.clone().into());
        let namespace = lowerer.original_body_namespace_context(parent, argument, &source);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.namespace", namespace.is_some());
        }
        let namespace = namespace?;
        // Preserve the independently selected child stage and namespace issuer
        // before delegating the same original-body semantic owner.
        let selected = lowerer
            .source_bindings
            .as_ref()?
            .selected_source_in_context(&source, &namespace);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.stage", selected.is_some());
        }
        selected?;
        Some(lowerer.lower_original_body(source, Some(&namespace)))
    }

    /// Readonly Logical child lowering under the actual complete Module owner.
    /// Original body geometry and selected current/root source namespace stay
    /// independent of reached body-entry and Native compilation receipts.
    pub(crate) fn original_logical_body_for_module(
        module: &Module,
        registry: &'r CommandRegistry,
        parent: &CommandTokens,
        argument: usize,
    ) -> Option<Script> {
        let input = module.source_metadata_input.as_ref()?;
        if !input.has_logical_source_name_context() {
            return None;
        }
        let retained = module.retained_source_bindings.as_deref()?;
        let positioned = retained.positioned_for_module(module, registry)?;
        let body = crate::registry_invocation::original_logical_body_geometry_for_module(
            module, registry, parent, argument,
        )?;
        let namespace = crate::command_binding::original_logical_frame_body_namespace_for_module(
            module, registry, parent, &body,
        )?;
        let source = module.source.try_text().ok()?;
        let text = source.get(body.content_span().as_range())?;
        let mut lowerer = Self::with_config(registry, module.lexer_config)
            .with_resolved_analysis_input(input.clone());
        lowerer.set_source_analysis_options(module.source_entry.options());
        source.clone_into(&mut lowerer.source);
        lowerer.source_channel = module.source.channel();
        lowerer.source_bindings = Some(positioned.clone().into());
        lowerer.structured_source_parent = Some(Box::new(parent.clone()));
        lowerer.suppress_proc_register = true;
        lowerer.readonly_logical_body = SourceBodyMode::ReadOnly;
        let mut script = lowerer.lower_body(
            text,
            body.content_span().start(),
            &namespace.advisory_key()?,
        );
        script.namespace_context = Some(Box::new(namespace));
        Some(script)
    }

    /// Retain an original body without inventing a namespace when its entry
    /// context is unknown. Known contexts survive unsupported text/admission.
    pub(in super::super) fn lower_original_body(
        &mut self,
        source: crate::command_binding::ExecutedScriptSource,
        namespace: Option<&crate::command_binding::SourceNamespaceKey>,
    ) -> Script {
        if let Some(namespace) = namespace {
            return self.lower_executed_script_in_context(source, namespace);
        }
        let mut script = unavailable_original_body_script(
            source.base(),
            source.text.len(),
            "original body namespace context is unavailable",
        );
        script.executed_source = Some(std::sync::Arc::new(source));
        script
    }

    pub(in super::super) fn original_body_namespace_context(
        &self,
        tokens: &CommandTokens,
        argument: usize,
        source: &crate::command_binding::ExecutedScriptSource,
    ) -> Option<crate::command_binding::SourceNamespaceKey> {
        let bindings = self.source_bindings.as_ref()?;
        bindings.executed_script_entry_namespace_key_at(
            tokens.source_binding.as_ref()?.invocation_site()?,
            argument,
            source,
        )
    }

    /// Original retained contexts alone select a body inventory when its
    /// caller has no separate invocation/argument context receipt.
    pub(in super::super) fn lower_retained_script(
        &mut self,
        source: crate::command_binding::ExecutedScriptSource,
    ) -> Script {
        let namespace = self.source_bindings.as_ref().and_then(|bindings| {
            bindings
                .executed_script_namespace_context(&source)
                .or_else(|| bindings.compiled_script_namespace_context(&source))
                .or_else(|| bindings.declared_script_namespace_context(&source))
                .or_else(|| {
                    let parent = self.structured_source_parent.as_deref()?;
                    parent
                        .source_binding
                        .as_ref()?
                        .original_body_namespace_context(
                            self.registry,
                            self.invocation_metadata_context()?,
                            parent,
                            &source,
                        )
                })
        });
        self.lower_original_body(source, namespace.as_ref())
    }

    /// Retain an unchanged body slice independently of compiler admission.
    pub(super) fn retain_script_source(&self, mut script: Script, text: &str, base: u32) -> Script {
        if self.readonly_logical_body.is_read_only()
            || self.readonly_hosted_body.is_read_only()
            || self.readonly_authored_body.is_read_only()
            || self.readonly_native_declaration_body.is_read_only()
        {
            return script;
        }
        script.executed_source = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.source_origin())
            .cloned()
            .and_then(|origin| {
                crate::command_binding::ExecutedScriptSource::contiguous(origin, text, base)
            })
            .map(std::sync::Arc::new);
        if let Some(source) = &script.executed_source {
            script.implicit_math_invocations = self
                .source_bindings
                .as_ref()
                .map_or_else(Vec::new, |bindings| {
                    bindings.math_invocations_for_script(self.registry, source)
                });
            script.expression_preparations = self
                .source_bindings
                .as_ref()
                .map_or_else(Vec::new, |bindings| {
                    bindings.expression_preparations_for_script(source)
                });
        }
        script
    }
}
