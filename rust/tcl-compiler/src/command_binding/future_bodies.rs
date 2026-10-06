// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Future script inventories kept separate from reached document dispatch.

use super::{
    Arc, CommandAllocationSite, DeferredSourceBody, ExecutedScriptSource, InvocationFacts,
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceInvocationBinding,
    SourceNativeInvocation, evaluated_script_realm, retained_script_operand,
};
use crate::var_resolve::VariableExecutionFrame;
use tcl_registry::CommandRegistry;

/// Conditional invocations of an explicitly selected future script entry.
/// These can withdraw caller assumptions; they establish no document execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFutureBodyInventory {
    registration: CommandAllocationSite,
    source: ExecutedScriptSource,
    frame: VariableExecutionFrame,
    bindings: Arc<SourceCommandBindings>,
}

/// One possible future caller, retained independently of document execution.
/// Consumers may withdraw caller assumptions but cannot license dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFutureCallSite {
    /// Actual registration instruction and source instance.
    pub registration: CommandAllocationSite,
    /// Actual evaluated future script, including its source mapping.
    pub source: ExecutedScriptSource,
    /// Command offset in that script's retained source instance.
    pub offset: u32,
    /// Exact possible binding and explicit unknown residual at future entry.
    pub binding: SourceInvocationBinding,
}

impl SourceFutureBodyInventory {
    /// Positioned assistance in this independent future entry. Absence of a
    /// retained point is uncertainty, not proof that a command is missing.
    #[must_use]
    pub fn invocation_at(&self, offset: u32) -> Option<SourceInvocationBinding> {
        self.bindings
            .dispatch_points
            .contains_key(&offset)
            .then(|| self.bindings.invocation_at_source("", offset))
    }

    /// Exact registration instruction, independent of its command spelling.
    #[must_use]
    pub fn registration(&self) -> &CommandAllocationSite {
        &self.registration
    }

    /// Evaluated script and its original or materialised source instance.
    #[must_use]
    pub fn source(&self) -> &ExecutedScriptSource {
        &self.source
    }

    /// Registry-selected future variable frame; no caller frame is implied.
    #[must_use]
    pub fn frame(&self) -> &VariableExecutionFrame {
        &self.frame
    }

    /// Possible dispatches in this future source, including lookup uncertainty.
    #[must_use]
    pub fn invocations(&self) -> Vec<(u32, SourceInvocationBinding)> {
        let end = self
            .source
            .base()
            .saturating_add(u32::try_from(self.source.text.len()).unwrap_or(u32::MAX));
        self.bindings
            .dispatch_points
            .range(self.source.base()..end)
            .map(|(&offset, _)| (offset, self.bindings.invocation_at_source("", offset)))
            .collect()
    }

    /// Compact caller-only projection; no world snapshots are copied.
    #[must_use]
    pub fn call_sites(&self) -> Vec<SourceFutureCallSite> {
        self.invocations()
            .into_iter()
            .map(|(offset, binding)| SourceFutureCallSite {
                registration: self.registration.clone(),
                source: self.source.clone(),
                offset,
                binding,
            })
            .collect()
    }
}

impl SourceCommandBindings {
    /// Independently analysed future entries; never reached wrapper effects.
    #[must_use]
    pub fn future_body_inventories(&self) -> &[SourceFutureBodyInventory] {
        &self.future_bodies
    }

    pub(super) fn register_future_body(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) {
        let Some(frame) = facts
            .body_execution
            .and_then(|spec| spec.deferred_entry_frame(native.invocation.arguments().dialect()?))
        else {
            return;
        };
        let tcl_registry::body_execution::DeferredBodyFrame::Global = frame;
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let registration = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.segment.span.start(),
        };
        for &(index, role) in &facts.arg_roles {
            if role != tcl_registry::ArgRole::Body {
                continue;
            }
            let argument = facts.argument_offset + usize::from(index);
            let Some(script) = retained_script_operand(
                argument,
                native.script_operands(),
                state,
                registration.offset,
                context.config,
            ) else {
                continue;
            };
            let root = DeferredSourceBody {
                realm: evaluated_script_realm(
                    argument..argument + 1,
                    native.script_operands(),
                    context,
                ),
                identity: format!(
                    "future:{}:{argument}",
                    registration.variable_storage_identity()
                ),
                implementation_generation: registration.offset,
                implementation_allocation: None,
                source_origin: Some(Arc::clone(&script.origin)),
                executed_script: Some((registration.clone(), argument, script.clone())),
                executed_word: None,
                source: {
                    let Ok(text) = script.try_text() else {
                        continue;
                    };
                    text.to_owned()
                },
                offset: script.base(),
                namespace: "::".to_owned(),
                namespace_key: state.native_root_namespace_key().unwrap_or_default(),
                event: None,
                receiver_method: false,
                future_frame: Some(VariableExecutionFrame::Global.with_namespace_identity(
                    state.native_root_namespace_key().unwrap_or_default(),
                )),
                parameters: Vec::new(),
                statics: None,
            };
            self.deferred.insert(root.implementation_id(), root);
        }
    }

    pub(super) fn walk_future_body(
        &mut self,
        root: &DeferredSourceBody,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        let (Some(frame), Some((registration, _, source))) =
            (&root.future_frame, &root.executed_script)
        else {
            return;
        };
        let mut state = incoming.clone();
        state.current_source_origin.clone_from(&root.source_origin);
        state.variable_frame = frame.clone();
        state.source_variables = Arc::new(state.source_variables.in_frame(frame));
        let variables = Arc::make_mut(&mut state.source_variables);
        variables.contents_world = crate::var_resolve::ContentsWorld::Unknown;
        variables.closed_array_roots.clear();
        variables.namespace_cells.closed_namespaces.clear();
        variables.contents_presence.clear();
        variables.contents_origins.clear();
        variables.contents_kinds.clear();
        variables.retain_literal_values(|_| false);
        variables.invalidate_shared_representations();
        let mut bindings = Self {
            declaration_preview_depth: 1,
            root_origin: root.source_origin.clone(),
            deferred: self.deferred.clone(),
            lexer_config: Some(config),
            ..Self::default()
        };
        let mut compilation = super::procedure_compilation(state.baseline.compilation_dialect());
        compilation.frame = tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode;
        let outcomes = bindings.walk_source(
            &root.source,
            root.offset,
            &mut state,
            &SourceExecutionContext {
                realm: root.realm,
                compilation,
                compilation_snapshot: None,
                selected_compilation: None,
                namespace: &root.namespace,
                frame,
                config,
                registry,
                depth: 0,
                invocation_offset: registration.offset,
                variable_read_owner: None,
                written_arguments: None,
                written_values: None,
                written_representations: None,
                written_objects: None,
                written_method_prefixes: None,
                written_variable_reads: None,
                namespace_key: Some(&root.namespace_key),
                expression_source: None,
            },
        );
        self.retain_deferred_outcomes(root, frame, outcomes);
        self.future_bodies.push(SourceFutureBodyInventory {
            registration: registration.clone(),
            source: source.clone(),
            frame: frame.clone(),
            bindings: Arc::new(bindings),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_callback_invocations_do_not_become_reached_document_points() {
        let source =
            "proc helper mode {return $mode}; after 0 {helper $::runtime_mode}; set after YES";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find("helper $::").unwrap()).unwrap();
        assert_ne!(
            bindings
                .invocation_at_source("helper", offset)
                .runtime_reachability(),
            super::super::SourceRuntimeReachability::Reached
        );
        let future = bindings.future_body_inventories();
        assert_eq!(future.len(), 1);
        assert_eq!(future[0].frame(), &VariableExecutionFrame::Global);
        assert_eq!(
            future[0].registration().offset,
            u32::try_from(source.find("after 0").unwrap()).unwrap()
        );
        assert!(future[0].invocations().iter().any(|(site, binding)| {
            *site == offset
                && (binding.unknown
                    || binding.targets.iter().any(|target| {
                        target.kind == super::super::BindingKind::Proc
                            && target.command == "::helper"
                    }))
        }));
    }
}
