// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current authored-body syntax for conditional taint diagnostics only.

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::CommandTokens;
use crate::registry_invocation::OriginalAuthoredSourceBodyScope;
use tcl_registry::CommandRegistry;

#[derive(Clone, Copy)]
pub(super) struct AuthoredBodyTaintContext<'a> {
    unit: &'a CompilationUnit,
    function: &'a FunctionUnit,
}

impl<'a> AuthoredBodyTaintContext<'a> {
    pub(super) fn capture(
        unit: &'a CompilationUnit,
        function: &'a FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let body = function.body_id?;
        if !crate::ssa::current_authored_analysis_function(unit, function, registry)
            || !function.has_original_semantic_value_owner()
            || !function.ssa.has_current_source_cfg_topology(&function.cfg)
        {
            return None;
        }
        let result = Self { unit, function };
        let procedure = unit.original_procedure_for_body(body)?;
        let card = unit
            .ir_module
            .original_authored_declaration_body_units
            .get(&body)?;
        if !card.matches_procedure(&unit.ir_module, procedure)
            || !result.matches_original_graph(registry)
        {
            return None;
        }
        Some(result)
    }

    fn matches_original_graph(self, registry: &CommandRegistry) -> bool {
        let Some(points) = self.function.ssa.point_contexts.as_ref() else {
            return false;
        };
        if self.function.ssa.blocks.len() != self.function.cfg.blocks.len()
            || self.function.ssa.blocks.iter().any(|(block, ssa)| {
                self.function.cfg.blocks.get(block).is_none_or(|cfg| {
                    ssa.statements.len() != cfg.statements.len()
                        || ssa
                            .statements
                            .iter()
                            .zip(&cfg.statements)
                            .any(|(ssa, cfg)| &ssa.statement != cfg)
                })
            })
        {
            return false;
        }
        let inventory = points.source_token_inventory();
        if self.function.cfg.blocks.iter().any(|(&block, cfg)| {
            cfg.statements.iter().enumerate().any(|(index, statement)| {
                statement.synthetic_marker().is_none() && !inventory.contains_key(&(block, index))
            })
        }) {
            return false;
        }
        !inventory.is_empty()
            && inventory.iter().all(|(&(block, index), tokens)| {
                self.function.cfg.source_input_tokens_at(block, index) == Some(tokens)
                    && self.scope_for_tokens(tokens, registry).is_some()
            })
    }

    pub(super) fn scope_for_tokens<'b>(
        &'b self,
        tokens: &'b CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<&'b OriginalAuthoredSourceBodyScope> {
        let body = self.function.body_id?;
        let module = &self.unit.ir_module;
        let retained = module.retained_source_bindings.as_deref()?;
        retained
            .owns_original_analysis_attachment(tokens)
            .then_some(())?;
        let binding = tokens.source_binding.as_ref()?;
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || binding.has_original_native_source_command()
            || binding.has_original_logical_source_advice()
        {
            return None;
        }
        let scope = if binding.has_original_source_layout_owner() {
            binding.original_authored_scalar_body_scope(tokens, registry)?
        } else {
            binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?
        };
        if !scope.matches_module(module)
            || !module
                .original_authored_declaration_body_units
                .get(&body)?
                .owns_original_source_scope(scope)
        {
            return None;
        }
        scope.original_words(tokens, registry)?;
        Some(scope)
    }

    pub(super) fn has_formal(self, ssa: &crate::ssa::SsaFunction, name: &str) -> bool {
        if !std::ptr::eq(ssa, &raw const self.function.ssa) {
            return false;
        }
        self.function.body_id.is_some_and(|body| {
            self.unit
                .ir_module
                .original_authored_declaration_body_units
                .get(&body)
                .is_some_and(|card| card.parameter_names().any(|formal| formal == name))
        })
    }
    pub(super) fn formals(
        self,
    ) -> Option<crate::registry_invocation::AuthoredProcedureFormalTopology> {
        let body = self.function.body_id?;
        let module = &self.unit.ir_module;
        module
            .original_authored_declaration_body_units
            .get(&body)?
            .original_taint_formals(module, self.unit.original_procedure_for_body(body)?)
    }

    pub(super) fn sink_invocation(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::AuthoredSourceTaintInvocation> {
        let scope = self.scope_for_tokens(tokens, registry)?;
        crate::registry_invocation::AuthoredSourceTaintInvocation::capture_authored_body(
            tokens, scope, registry,
        )
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::taint::source_context::TaintSourceContext;

    const SOURCE: &str = "proc do_eval {x} { eval $x }\nset data [read $fd]\ndo_eval $data\n";

    fn fixture(source: &str, registry: &CommandRegistry) -> CompilationUnit {
        CompilationUnit::build_for(source, registry, false)
    }

    fn function(unit: &CompilationUnit) -> &FunctionUnit {
        let body = *unit
            .ir_module
            .original_authored_declaration_body_units
            .keys()
            .next()
            .unwrap();
        unit.function_for_body(Some(body)).unwrap()
    }

    #[test]
    fn original_sinking_helper_uses_only_current_authored_may_schema() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = fixture(SOURCE, registry);
        let function = function(&unit);
        let ordinary = TaintSourceContext::for_unit_function(registry, &unit, function);
        assert!(ordinary.metadata_context().is_none());
        assert!(!ordinary.allows_nominal_metadata());
        let source = TaintSourceContext::for_unit_may(registry, &unit, function);
        assert!(source.has_authored_body());
        assert!(source.metadata_context().is_none());
        assert!(!source.allows_nominal_metadata());
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        let procedure = unit
            .original_procedure_for_body(function.body_id.unwrap())
            .unwrap();
        assert!(procedure.publication.is_none());
        assert!(procedure.body.native_compilation_admission.is_none());
        let tokens = procedure
            .body
            .statements
            .iter()
            .find_map(|statement| {
                procedure
                    .body
                    .retained_source_tokens_for_statement(statement)
                    .filter(|tokens| tokens.synthetic.is_none())
            })
            .unwrap();
        let selected = source
            .authored_body_sink(tokens, registry)
            .expect("the current exact source body's possible invocation");
        assert_eq!(selected.command(), "eval");
        assert!(selected.uses_symbolic_body_reads());
        assert!(!selected.has_closed_scalar_read_context());
        let body = function.body_id.unwrap();
        assert!(matches!(
            TaintSourceContext::formals_for_unit_body(&unit, body, registry),
            Some(
                crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Authored(
                    _
                )
            )
        ));
        let dialect =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let warnings = crate::taint::find_taint_warnings_for_cu(&unit, registry, Some(dialect));
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == tcl_core_types::DiagCode::T100
                    && warning.sink_command == "eval")
        );
    }

    #[test]
    fn authored_may_context_refuses_foreign_stale_graph_profile_and_withdrawal() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = fixture(SOURCE, registry);
        let foreign = fixture(SOURCE, registry);
        assert!(AuthoredBodyTaintContext::capture(&unit, function(&unit), registry).is_some());
        assert!(AuthoredBodyTaintContext::capture(&unit, function(&foreign), registry).is_none());
        let mut stale = unit.clone();
        stale.source.push(' ');
        assert!(AuthoredBodyTaintContext::capture(&stale, function(&stale), registry).is_none());
        let mut changed = unit.clone();
        let body = function(&changed).body_id.unwrap();
        changed
            .body_units
            .get_mut(&body)
            .unwrap()
            .cfg
            .blocks
            .clear();
        assert!(
            AuthoredBodyTaintContext::capture(&changed, function(&changed), registry).is_none()
        );
        let mut missing_graph = unit.clone();
        missing_graph
            .body_units
            .get_mut(&body)
            .unwrap()
            .ssa
            .point_contexts = None;
        assert!(
            AuthoredBodyTaintContext::capture(&missing_graph, function(&missing_graph), registry)
                .is_none()
        );
        let mut withdrawn = unit.clone();
        withdrawn.ir_module.source_entry.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(None);
        assert!(
            AuthoredBodyTaintContext::capture(&withdrawn, function(&withdrawn), registry).is_none()
        );
        let other = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(AuthoredBodyTaintContext::capture(&unit, function(&unit), other).is_none());
        let actual = AuthoredBodyTaintContext::capture(&unit, function(&unit), registry).unwrap();
        assert!(!actual.has_formal(&function(&foreign).ssa, "x"));
    }

    #[test]
    fn authored_formal_ordinals_keep_defaults_rest_and_missing_grammar_unavailable() {
        use tcl_syntax::formal_params::FormalByteArgumentBinding as Binding;
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = fixture("proc p {x {optional DEFAULT} args} {eval $x}\n", registry);
        let body = function(&unit).body_id.unwrap();
        let Some(
            crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Authored(
                formals,
            ),
        ) = TaintSourceContext::formals_for_unit_body(&unit, body, registry)
        else {
            panic!("current common source formals");
        };
        let bindings = formals.bindings(1).unwrap();
        assert!(bindings.iter().any(|binding| matches!(
            binding,
            Binding::Value {
                parameter: 0,
                argument: 0
            }
        )));
        assert!(
            bindings
                .iter()
                .any(|binding| matches!(binding, Binding::Default { parameter: 1, .. }))
        );
        assert!(bindings.iter().any(|binding| matches!(
            binding,
            Binding::Rest {
                parameter: 2,
                len: 0,
                ..
            }
        )));
        assert!(formals.bindings(0).is_none());
        let disputed = fixture("proc p {&x} {eval $x}\n", registry);
        let disputed_body = function(&disputed).body_id.unwrap();
        assert!(TaintSourceContext::formals_for_unit_body(&disputed, disputed_body, registry)
            .is_none_or(|formals| match formals {
                crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Authored(formals) => formals.bindings(1).is_none(),
                _ => false,
            }));
    }
}
