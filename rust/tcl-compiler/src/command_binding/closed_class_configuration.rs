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

//! A stock class definer applied to an existing source class by members that
//! only declare.

use super::{
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceNativeInvocation,
};
use tcl_registry::{InvocationFacts, ObjectDispatchLayer};

/// The definition a closed class configuration evaluates, for the deferred
/// walk of the method bodies it declares, and the class definition it
/// configures as it stood before the configuration ran.
pub(super) struct ClosedClassConfiguration {
    pub(super) grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    pub(super) body: super::ExecutedScriptSource,
    pub(super) parameters: tcl_dialect::ParameterGrammar,
    pub(super) class: super::SourceCommandTarget,
    pub(super) receipt: super::ClassDefinitionReceipt,
    pub(super) bounded: Option<super::module_bindings::BoundedClassConstruction>,
    pub(super) tainted: bool,
    pub(super) tainted_all: bool,
}

impl SourceCommandBindings {
    /// `oo::define Class {…}` or `oo::define Class member …` on a class this
    /// source declared, where every member only declares (a method, a
    /// variable, a relation). Such a definition changes that class's member
    /// tables and nothing else: no command is published, moved or deleted
    /// and no member body runs, so it completes without entering the script
    /// as an ordinary body.
    pub(super) fn closed_class_configuration(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<ClosedClassConfiguration> {
        let script_layout =
            super::own_object_configuration::own_object_definition_layout(native, facts)?;
        if state.has_opaque_domain()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || !native.target.registry_backed
            || !native.target.prepended.is_empty()
            || native.target.implementation_generation != 0
        {
            return None;
        }
        let target = super::own_object_configuration::configuration_target(
            facts,
            ObjectDispatchLayer::Class,
        )?;
        // A class reached through a substituted word (`oo::define $c …`) is a
        // relation a rename cannot follow; only a written name is closed.
        if !matches!(
            native.words.get(target + 1),
            Some(crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. })
        ) {
            return None;
        }
        let name = native.invocation.arguments().literal_at(target)?;
        let proof = super::source_binding(state, name, &context.namespace_identity());
        let class_target = proof.proved_target()?;
        let identity = class_target.identity.as_ref()?;
        let class = state.class_definitions.get(identity)?;
        let dialect = state.baseline.dialect?;
        let construction = context.registry.native_default_construction_grammar(
            &class.factory,
            dialect,
            context.realm(),
        )?;
        let grammar = context
            .registry
            .get_for_surface(
                &native.target.command,
                dialect
                    .authoring_query()
                    .map(|query| query.with_realm(context.realm())),
            )?
            .definition_body?;
        if grammar.family != construction.family
            || !state.default_construction_dependencies_hold(construction)
        {
            return None;
        }
        let body = super::own_object_configuration::own_object_definition(
            script_layout,
            native,
            grammar,
            state,
            context.config,
        )?;
        state
            .definition_has_only_deferred_members(body.text.try_text().ok()?, construction, context)
            .then_some(())?;
        Some(ClosedClassConfiguration {
            grammar,
            body,
            parameters: dialect.parameter_grammar()?,
            class: class_target.clone(),
            receipt: class.clone(),
            bounded: state.bounded_constructions.get(identity).cloned(),
            tainted: state
                .tainted_object_dispatch
                .contains(&class_target.command),
            tainted_all: state.tainted_object_dispatch.contains("*"),
        })
    }

    /// Give the declared method bodies their deferred entries, and keep the
    /// configured class's definition with the methods it now declares.
    pub(super) fn register_closed_class_configuration(
        &mut self,
        configuration: &ClosedClassConfiguration,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        state.retain_configured_class_definition(configuration, context);
        self.register_original_method_bodies(
            configuration.grammar,
            &configuration.body,
            state.baseline.dialect,
            configuration.parameters,
            context,
        );
    }
}
