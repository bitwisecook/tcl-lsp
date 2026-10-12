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

//! Shared original source interpretation and its complete reuse boundary.

use std::sync::Arc;

use super::{
    ModuleCommandBindings, SourceAnalysisEntry, SourceAnalysisOptions, SourceCommandBindings,
};
use crate::ir::{Module, Procedure, TopLevelKind};
use crate::var_resolve::VariableExecutionFrame;
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Immutable command-effect projection of the interpretation used by lowering.
/// Changing any original input withdraws reuse and reaches the same source owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedSourceModuleBindings {
    source: tcl_lexer::SourceImage,
    entry: SourceAnalysisEntry,
    grammar: tcl_lexer::LexerConfig,
    namespace: String,
    native_namespace: Option<tcl_core_types::ByteNamespacePath>,
    namespace_context: Option<super::SourceNamespaceKey>,
    kind: TopLevelKind,
    registry: RegistrySemanticKey,
    observed: ModuleCommandBindings,
    positioned: Arc<SourceCommandBindings>,
    procedures: std::collections::BTreeMap<String, OriginalProcedureSource>,
}

/// Original header and body coordinates retained by the real Module producer.
/// This source signature supplies no entered procedure or native frame.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalProcedureSource {
    name: String,
    qualified_name: String,
    parameters: Vec<String>,
    parameters_text: String,
    declaration: tcl_lexer::Span,
    body_offset: u32,
    body_text: Option<String>,
    namespace: Option<Box<super::SourceNamespaceKey>>,
    body_source: Option<Arc<super::ExecutedScriptSource>>,
    statements: Vec<tcl_lexer::Span>,
}

impl OriginalProcedureSource {
    fn retain(procedure: &Procedure) -> Self {
        Self {
            name: procedure.name.clone(),
            qualified_name: procedure.qualified_name.clone(),
            parameters: procedure.params.clone(),
            parameters_text: procedure.params_raw.clone(),
            declaration: procedure.span,
            body_offset: procedure.body_offset,
            body_text: procedure.body_source.clone(),
            namespace: procedure.body.namespace_context.clone(),
            body_source: procedure.body.executed_source.clone(),
            statements: procedure
                .body
                .statements
                .iter()
                .map(crate::ir::Statement::span)
                .collect(),
        }
    }

    fn matches(&self, procedure: &Procedure) -> bool {
        self.name == procedure.name
            && self.qualified_name == procedure.qualified_name
            && self.parameters == procedure.params
            && self.parameters_text == procedure.params_raw
            && self.declaration == procedure.span
            && self.body_offset == procedure.body_offset
            && self.body_text == procedure.body_source
            && self.namespace == procedure.body.namespace_context
            && self.body_source == procedure.body.executed_source
            && self.statements.iter().copied().eq(procedure
                .body
                .statements
                .iter()
                .map(crate::ir::Statement::span))
    }
}

impl RetainedSourceModuleBindings {
    pub(crate) fn retain(
        bindings: &SourceCommandBindings,
        module: &Module,
        registry: &CommandRegistry,
    ) -> Option<Arc<Self>> {
        if bindings.root_origin.as_ref()?.source_image() != &module.source
            || bindings.lexer_config != Some(module.native_lexer_config())
            || bindings.compilation_scope != module.source_entry.compilation_scope
        {
            return None;
        }
        Some(Arc::new(Self {
            source: module.source.clone(),
            entry: module.source_entry.clone(),
            grammar: module.native_lexer_config(),
            namespace: module.top_level_namespace.clone(),
            native_namespace: module.native_namespace.clone(),
            namespace_context: module.top_level_namespace_context.clone(),
            kind: module.top_level_kind,
            registry: registry.snapshot().semantic_key(),
            observed: bindings.module_projection(module),
            positioned: Arc::new(bindings.clone()),
            procedures: module
                .procedures
                .iter()
                .map(|(name, procedure)| (name.clone(), OriginalProcedureSource::retain(procedure)))
                .collect(),
        }))
    }

    /// Check the same exact original Module inputs without deriving a clone.
    /// This is a source-owner join, independently of any reached dispatch.
    pub(crate) fn matches_module(&self, module: &Module, registry: &CommandRegistry) -> bool {
        self.source == module.source
            && self.entry == module.source_entry
            && self.entry.metadata_context.source_analysis_input()
                == module.source_metadata_input.as_ref()
            && self.grammar == module.native_lexer_config()
            && self.namespace == module.top_level_namespace
            && self.native_namespace == module.native_namespace
            && self.namespace_context == module.top_level_namespace_context
            && self.kind == module.top_level_kind
            && self.registry == registry.snapshot().semantic_key()
    }

    /// Borrow the exact positioned interpretation from the original Lowerer.
    /// An absent or changed Module input is terminal for this projection;
    /// independent source interpretation has its own explicit ingress.
    pub(crate) fn positioned_for_module(
        &self,
        module: &Module,
        registry: &CommandRegistry,
    ) -> Option<&SourceCommandBindings> {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        if !self.matches_module(module, registry)
            || module.registry_snapshot.as_ref()?.semantic_key() != self.registry
            || !self
                .positioned
                .matches_original_source_image(&module.source, self.grammar)
        {
            return None;
        }
        Some(&self.positioned)
    }

    /// Retain an original procedure header, body coordinates and direct
    /// statement order under this same Lowerer-produced Module. Child statements
    /// still require their own original token and selected operation receipts.
    pub(crate) fn matches_original_procedure(&self, procedure: &Procedure) -> bool {
        self.procedures
            .get(&procedure.qualified_name)
            .is_some_and(|original| original.matches(procedure))
    }

    /// Authenticate the complete written source carrier under this Module's
    /// original interpretation. This supplies source ownership independently of
    /// compiler visitation, Native handler admission and read/store receipts.
    pub(crate) fn owns_original_tokens(&self, tokens: &crate::ir::CommandTokens) -> bool {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        if binding.original_written_words.is_none() || !tokens.original_words_match_argv_text() {
            return false;
        }
        let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
            return false;
        };
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let Some(snapshot) = binding.lookup_state.as_ref() else {
            return false;
        };
        config == self.grammar
            && site.source.source_image() == &self.source
            && self.positioned.root_origin.as_ref() == Some(&site.source)
            && snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            && snapshot.state.baseline == self.observed.baseline
    }

    pub(super) fn projection(
        &self,
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<ModuleCommandBindings> {
        if !self.matches_module(module, registry) || self.entry.options() != options {
            return None;
        }
        let mut observed = self.observed.clone();
        observed.has_redefined_procedures = !module.redefined_procedures.is_empty();
        Some(observed)
    }
}

impl SourceCommandBindings {
    pub(super) fn module_projection(&self, module: &Module) -> ModuleCommandBindings {
        let mut observed = self.final_state.as_ref().clone();
        for point in &self.points {
            observed.join(&point.state);
        }
        for future in &self.deferred_outcomes {
            observed.join(&future.state);
        }
        observed.root_boundary_bindings = Arc::clone(&self.final_state.bindings);
        observed.has_redefined_procedures = !module.redefined_procedures.is_empty();
        observed
    }
}

/// Actual entry frame shared by lowering and later original-source queries.
pub(crate) fn source_analysis_frame(
    entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    namespace: &str,
    kind: TopLevelKind,
) -> VariableExecutionFrame {
    use tcl_runtime_api::native_compilation::NativeCompilationFrame;
    if let Some(entry) = entry {
        let selected = entry
            .namespaces
            .iter()
            .find(|row| row.token == entry.current_namespace);
        let Some(selected) = selected.filter(|selected| {
            entry.closed
                && (!selected.visible
                    || entry
                        .namespaces
                        .iter()
                        .filter(|row| row.visible && row.path == selected.path)
                        .count()
                        == 1)
        }) else {
            return VariableExecutionFrame::Unknown;
        };
        let Ok(namespace_key) = entry
            .retained_namespace_context(selected.token)
            .map(super::SourceNamespaceKey::Native)
        else {
            return VariableExecutionFrame::Unknown;
        };
        let namespace = namespace_key.display().unwrap_or_default();
        let identity = format!(
            "runtime:{}:{}:{}:{}",
            entry.interpreter.owner,
            entry.interpreter.interpreter,
            entry.epoch,
            entry.current_namespace
        );
        let frame = match entry.frame {
            NativeCompilationFrame::Global => VariableExecutionFrame::Global,
            NativeCompilationFrame::Namespace => VariableExecutionFrame::NamespaceActivation {
                namespace,
                identity,
            },
            NativeCompilationFrame::Procedure => VariableExecutionFrame::Procedure {
                namespace,
                identity,
            },
            NativeCompilationFrame::Unknown => VariableExecutionFrame::Unknown,
        };
        return frame.with_namespace_identity(namespace_key);
    }
    if kind == TopLevelKind::ProcedureBody {
        VariableExecutionFrame::Procedure {
            namespace: namespace.to_owned(),
            identity: "::top".to_owned(),
        }
    } else if namespace == "::" {
        VariableExecutionFrame::Global
    } else {
        VariableExecutionFrame::Namespace(namespace.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "namespace eval N {proc p {} {return OK}}; N::p";

    fn lowered(registry: &CommandRegistry) -> Module {
        crate::lowering::lower_to_ir_with_config(
            SOURCE,
            registry,
            tcl_lexer::LexerConfig::default(),
        )
    }

    #[test]
    fn direct_original_source_tokens_do_not_require_compiler_visitation() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_known_environment(engine)
                .expect("each fixture uses a known engine environment")
                .analyser_profile();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let unit = crate::compilation_unit::CompilationUnit::build_for(
                "set x 0; llength $x; set x [expr {$x + 1}]",
                &registry,
                false,
            );
            let module = &unit.ir_module;
            let retained = module.retained_source_bindings.as_deref().unwrap();
            assert!(retained.matches_module(module, &registry), "{engine}");
            assert_eq!(
                module.source_entry.native_compilation.mode,
                tcl_registry::native_compilation::NativeCompilationMode::Direct,
                "{engine}",
            );
            assert!(module.source_entry.metadata_context.is_standalone());
            let script = &module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding.original_compiler_source(tokens).is_none(),
                "{engine}"
            );
            assert!(retained.owns_original_tokens(tokens), "{engine}");
            assert!(
                binding
                    .original_invocation_metadata_at_point(tokens, &registry)
                    .unwrap()
                    .is_none(),
                "{engine}",
            );
            let nested = crate::value_shapes::command_substitution_tokens(
                &tokens.words()[2],
                Some(tokens),
                binding.original_lexer_config_for_tokens(tokens).unwrap(),
            )
            .unwrap();
            let [expression] = nested.as_slice() else {
                panic!("the original fixture contains exactly one expression: {engine}");
            };
            assert!(
                expression
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_compiler_source(expression)
                    .is_none(),
                "{engine}",
            );
            assert!(retained.owns_original_tokens(expression), "{engine}");
        }
    }

    #[test]
    fn original_source_token_owner_refuses_missing_and_changed_carriers() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let unit = crate::compilation_unit::CompilationUnit::build_for("set x 0", &registry, false);
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_deref().unwrap();
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(retained.owns_original_tokens(tokens));
        for change in 0..10 {
            let mut changed = tokens.clone();
            match change {
                0 => changed.source_binding = None,
                1 => changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper),
                2 => {
                    changed.word_exprs.pop();
                }
                3 => {
                    changed.argv_texts.pop();
                }
                4 => *changed.argv_texts.last_mut().unwrap() = "OTHER".to_owned(),
                5 => {
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .original_written_words = None
                }
                6 => {
                    let binding = changed.source_binding.as_mut().unwrap();
                    binding.compiler_lookup_state = binding.lookup_state.take();
                }
                7 => {
                    let snapshot = Arc::make_mut(
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .lookup_state
                            .as_mut()
                            .unwrap(),
                    );
                    snapshot.state.current_source_origin = None;
                }
                8 => {
                    let snapshot = Arc::make_mut(
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .lookup_state
                            .as_mut()
                            .unwrap(),
                    );
                    snapshot.state.current_source_origin =
                        Some(Arc::new(super::super::SourceOriginId::authored_image(
                            tcl_lexer::SourceImage::native(module.source.bytes()),
                        )));
                }
                _ => {
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .dispatch_site
                        .as_mut()
                        .unwrap()
                        .offset += 1
                }
            }
            assert!(!retained.owns_original_tokens(&changed), "carrier {change}");
        }
        let mut missing_root = retained.clone();
        Arc::make_mut(&mut missing_root.positioned).root_origin = None;
        assert!(!missing_root.owns_original_tokens(tokens));
        for change in 0..3 {
            let mut changed = retained.clone();
            match change {
                0 => changed.grammar.strict_quoting = !changed.grammar.strict_quoting,
                1 => changed.grammar.leading_bom = tcl_lexer::LeadingBom::Skip,
                _ => changed.grammar.base_offset += 1,
            }
            assert!(
                !changed.owns_original_tokens(tokens),
                "configuration {change}"
            );
        }
        assert!(retained.owns_original_tokens(tokens));
    }

    #[test]
    fn original_source_token_owner_joins_the_complete_retained_baseline() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let unit = crate::compilation_unit::CompilationUnit::build_for("set x 0", &registry, false);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let retained = unit.ir_module.retained_source_bindings.as_deref().unwrap();
        let foreign = CommandRegistry::build_default();
        for change in 0..6 {
            let mut changed = tokens.clone();
            let snapshot = Arc::make_mut(
                changed
                    .source_binding
                    .as_mut()
                    .unwrap()
                    .lookup_state
                    .as_mut()
                    .unwrap(),
            );
            let baseline = Arc::make_mut(&mut snapshot.state.baseline);
            match change {
                0 => {
                    baseline.metadata_context =
                        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
                }
                1 => baseline.unknown_entry = !baseline.unknown_entry,
                2 => baseline.native_compilation.loop_depth += 1,
                3 => {
                    baseline.invocation_realm =
                        tcl_dialect::model::InvocationRealm::InterpreterRuntime
                }
                4 => baseline.registry_snapshot = Some(foreign.snapshot().semantic_key()),
                _ => {
                    baseline.native_compilation.mode =
                        tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject
                }
            }
            assert!(
                !retained.owns_original_tokens(&changed),
                "baseline {change}"
            );
        }
        assert!(retained.owns_original_tokens(tokens));
    }

    #[test]
    fn positioned_module_projection_shares_the_actual_producer_and_detaches_writes() {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = lowered(registry);
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let original = retained.positioned_for_module(&module, registry).unwrap();
        assert!(std::ptr::eq(original, retained.positioned.as_ref()));
        let sibling = module.clone();
        assert!(Arc::ptr_eq(
            &retained.positioned,
            &sibling
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .positioned,
        ));
        let mut mutable = original.clone();
        assert!(Arc::ptr_eq(&mutable.final_state, &original.final_state));
        assert!(mutable.points.shares_storage(&original.points));
        assert!(
            mutable
                .declaration_layouts
                .shares_storage(&original.declaration_layouts)
        );
        Arc::make_mut(&mut mutable.final_state).mark_opaque_binding_mutation();
        assert!(mutable.final_state.opaque_binding_mutation);
        assert!(!original.final_state.opaque_binding_mutation);
        assert!(!Arc::ptr_eq(&mutable.final_state, &original.final_state));
        assert!(std::ptr::eq(
            original,
            retained.positioned_for_module(&module, registry).unwrap(),
        ));
    }

    #[test]
    fn positioned_module_projection_refuses_each_changed_original_input() {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let original = lowered(registry);
        let retained = original.retained_source_bindings.as_ref().unwrap();
        assert!(
            retained
                .positioned_for_module(&original, registry)
                .is_some()
        );
        let mutations: &[fn(&mut Module)] = &[
            |module| module.source = tcl_lexer::SourceImage::document("return OTHER"),
            |module| module.source = tcl_lexer::SourceImage::native(module.source.bytes()),
            |module| module.lexer_config.strict_quoting = !module.lexer_config.strict_quoting,
            |module| module.top_level_namespace = "::Other".to_owned(),
            |module| {
                module.native_namespace = Some(tcl_core_types::ByteNamespacePath::from_segments([
                    b"Other".as_slice(),
                ]))
            },
            |module| {
                module.top_level_namespace_context =
                    Some(super::super::SourceNamespaceKey::authored("::Different"))
            },
            |module| module.top_level_kind = TopLevelKind::ProcedureBody,
            |module| module.source_entry.unknown_entry = !module.source_entry.unknown_entry,
            |module| {
                module.source_entry.compilation_scope =
                    tcl_runtime_api::SourceCompilationScope::EnteredSource
            },
            |module| module.source_entry.native_compilation.loop_depth += 1,
            |module| module.registry_snapshot = None,
        ];
        for (index, mutate) in mutations.iter().enumerate() {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert!(
                retained.positioned_for_module(&changed, registry).is_none(),
                "input {index}"
            );
        }
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        assert!(retained.positioned_for_module(&original, foreign).is_none());
        let mut replaced_snapshot = original.clone();
        replaced_snapshot.registry_snapshot = Some(foreign.snapshot());
        assert!(
            retained
                .positioned_for_module(&replaced_snapshot, registry)
                .is_none()
        );
    }

    #[test]
    fn shared_projection_equals_fresh_original_source_in_every_engine_profile() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
            let module = lowered(registry);
            assert!(module.retained_source_bindings.is_some(), "{engine}");
            let reused = ModuleCommandBindings::analyse(&module, registry);
            let mut fresh = module;
            fresh.retained_source_bindings = None;
            assert_eq!(
                reused,
                ModuleCommandBindings::analyse(&fresh, registry),
                "{engine}"
            );
        }
    }

    #[test]
    fn original_input_changes_withdraw_the_shared_projection() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let original = lowered(registry);
        let retained = original.retained_source_bindings.as_ref().unwrap();
        let mutations: &[fn(&mut Module)] = &[
            |module| module.source = tcl_lexer::SourceImage::document("return OTHER"),
            |module| {
                module.source = tcl_lexer::SourceImage::native(module.source.bytes());
            },
            |module| module.lexer_config.strict_quoting = !module.lexer_config.strict_quoting,
            |module| module.top_level_namespace = "::Other".to_owned(),
            |module| {
                module.top_level_namespace_context = Some(
                    super::super::SourceNamespaceKey::authored("::DifferentTypedNamespace"),
                );
            },
            |module| module.top_level_kind = TopLevelKind::ProcedureBody,
            |module| module.source_entry.unknown_entry = !module.source_entry.unknown_entry,
            |module| {
                module.source_entry.compilation_scope =
                    tcl_runtime_api::SourceCompilationScope::EnteredSource;
            },
            |module| module.source_entry.native_compilation.loop_depth += 1,
        ];
        for (index, mutate) in mutations.iter().enumerate() {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert!(
                retained
                    .projection(&changed, registry, changed.source_entry.options())
                    .is_none(),
                "input {index}"
            );
        }
        let other = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        assert!(
            retained
                .projection(&original, other, original.source_entry.options())
                .is_none()
        );
    }

    #[test]
    fn override_entry_contract_cannot_reuse_original_lowering() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = lowered(registry);
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let mut options = module.source_entry.options();
        options.unknown_entry = !options.unknown_entry;
        assert!(retained.projection(&module, registry, options).is_none());
    }
}
