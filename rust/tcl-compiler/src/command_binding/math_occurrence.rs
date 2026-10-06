// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Purpose-separated original expression function occurrences.

use std::sync::Arc;

/// An original function occurrence, independently of actual dispatch authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceMathInvocation {
    /// Original expression source instance.
    pub origin: Arc<super::SourceOriginId>,
    /// Original function AST start in that source instance.
    pub site: u32,
    /// Function spelling from the checked original expression tree.
    pub function: String,
    purpose: SourceMathInvocationPurpose,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceMathInvocationPurpose {
    Reached(Box<super::SourceImplicitMathInvocation>),
    Conditional(Box<SourceConditionalMathInvocation>),
}

/// A checked expression occurrence requiring independent dispatch validation.
/// It contains no actual function registration, native header or CPP receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceConditionalMathInvocation {
    expression: super::expression_preparation::SourceConditionalExpressionEvaluation,
    arity: usize,
}

impl SourceConditionalMathInvocation {
    /// Exact original declaration or expression evaluation frame.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        self.expression.frame()
    }

    /// Original typed namespace owner, independently of printed names.
    #[must_use]
    pub fn namespace_context(&self) -> &super::SourceNamespaceKey {
        self.expression.namespace_context()
    }

    /// Original checked expression bytes and source identity.
    #[must_use]
    pub fn source(&self) -> &super::ExecutedScriptSource {
        self.expression.source()
    }

    /// Original AST argument count; it does not establish function identity.
    #[must_use]
    pub const fn arity(&self) -> usize {
        self.arity
    }
}

impl SourceMathInvocation {
    pub(crate) fn from_reached(proof: super::SourceImplicitMathInvocation) -> Self {
        Self {
            origin: Arc::clone(&proof.origin),
            site: proof.site,
            function: proof.function.clone(),
            purpose: SourceMathInvocationPurpose::Reached(Box::new(proof)),
        }
    }

    pub(super) fn from_conditional(
        expression: super::expression_preparation::SourceConditionalExpressionEvaluation,
        function: &str,
        site: u32,
        arity: usize,
    ) -> Self {
        Self {
            origin: Arc::clone(&expression.source().origin),
            site,
            function: function.to_owned(),
            purpose: SourceMathInvocationPurpose::Conditional(Box::new(
                SourceConditionalMathInvocation { expression, arity },
            )),
        }
    }

    /// Actual reached dispatch evidence. Conditional topology always declines.
    #[must_use]
    pub fn reached(&self) -> Option<&super::SourceImplicitMathInvocation> {
        match &self.purpose {
            SourceMathInvocationPurpose::Reached(proof)
                if self.origin == proof.origin
                    && self.site == proof.site
                    && self.function == proof.function
                    && proof.command_binding.as_ref().is_none_or(|binding| {
                        binding.runtime_reachability()
                            != super::SourceRuntimeReachability::Conditional
                    }) =>
            {
                Some(proof)
            }
            SourceMathInvocationPurpose::Reached(_)
            | SourceMathInvocationPurpose::Conditional(_) => None,
        }
    }

    /// Checked original topology with an unmet actual binding obligation.
    #[must_use]
    pub fn conditional(&self) -> Option<&SourceConditionalMathInvocation> {
        match &self.purpose {
            SourceMathInvocationPurpose::Conditional(proof)
                if self.origin == proof.expression.source().origin
                    && proof.expression.tree().function_calls().into_iter().any(
                        |(function, offset, arity)| {
                            function == self.function
                                && proof.expression.source().base().checked_add(offset)
                                    == Some(self.site)
                                && arity == proof.arity
                        },
                    ) =>
            {
                Some(proof)
            }
            SourceMathInvocationPurpose::Conditional(_)
            | SourceMathInvocationPurpose::Reached(_) => None,
        }
    }

    pub(crate) fn fixed_prerequisite(
        &self,
    ) -> Option<&tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite> {
        self.reached()?.fixed_prerequisite.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authored_calls(source: &str, profile: &str) -> Vec<SourceMathInvocation> {
        let selected = tcl_dialect::DialectProfile::find(profile).unwrap();
        let registry =
            tcl_registry::model::ingress::static_context_for_profile(selected).commands();
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(Some(selected)),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(selected)),
                ..Default::default()
            },
        );
        let original = super::super::ExecutedScriptSource::contiguous(
            Arc::clone(bindings.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        bindings.math_invocations_for_script(registry, &original)
    }

    #[test]
    fn conditional_math_occurrence_keeps_original_topology_without_native_authority() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let calls = authored_calls("expr {abs(-3)}", profile);
            assert_eq!(calls.len(), 1, "{profile}");
            let call = &calls[0];
            let conditional = call
                .conditional()
                .expect("missing actual entry retains a conditional obligation");
            assert!(call.reached().is_none());
            assert_eq!(call.site, 6);
            assert_eq!(call.function, "abs");
            assert_eq!(conditional.arity(), 1);
            assert_eq!(conditional.source().try_text().unwrap(), "abs(-3)");
            assert!(crate::math_function_binding::native_fold_dependency(call).is_none());
            let bindings = crate::math_function_binding::ExpressionMathBindings::for_origin(
                &calls,
                Some(conditional.source()),
                Some(call.site),
            );
            assert!(!bindings.proves_intrinsic("abs", 0));
            assert!(
                bindings
                    .resolved_call_for_value_analysis("abs", 0)
                    .is_none()
            );
            let mut relocated = call.clone();
            relocated.site += 1;
            assert!(relocated.conditional().is_none());
            assert!(relocated.reached().is_none());
            let mut changed = call.clone();
            changed.function = "sqrt".to_owned();
            assert!(changed.conditional().is_none());
        }
    }

    #[test]
    fn replaced_or_unknown_expression_handler_cannot_supply_function_topology() {
        for prefix in [
            "rename expr original; proc expr args {return custom}; ",
            "unknownfuture; ",
        ] {
            assert!(
                authored_calls(&format!("{prefix}expr {{abs(-3)}}"), "tcl9.0").is_empty(),
                "{prefix}"
            );
        }
    }
    #[test]
    fn actual_function_observations_do_not_inherit_declared_preview_issuer() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let registry =
                tcl_registry::model::ingress::static_context_for_profile(profile).commands();
            let native = crate::environment_ingress::captured_native_entry(profile);
            for entered in [false, true] {
                let source = if entered {
                    "proc f {} {expr {abs(-3)}}; f"
                } else {
                    "proc f {} {expr {abs(-3)}}"
                };
                let bindings = super::super::SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::for_profile(Some(profile)),
                    registry,
                    super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        native_entry: Some(&native),
                        ..Default::default()
                    },
                );
                let original = super::super::ExecutedScriptSource::contiguous(
                    Arc::clone(bindings.source_origin().unwrap()),
                    source,
                    0,
                )
                .unwrap();
                let calls = bindings.math_invocations_for_script(registry, &original);
                assert_eq!(calls.len(), 1, "{name}: entered={entered}");
                if entered {
                    assert!(calls[0].reached().is_some(), "{name}");
                    assert!(calls[0].conditional().is_none());
                } else {
                    assert!(calls[0].reached().is_none(), "{name}");
                    assert!(calls[0].conditional().is_some());
                    assert!(
                        crate::math_function_binding::native_fold_dependency(&calls[0]).is_none()
                    );
                }
            }
        }
    }
}
