// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-navigation operands from the actual authored Registry schema.

/// Exact post-head ordinal in one source declaration schema. This supplies
/// source-card geometry only, without native loader, filesystem, package
/// identity, command presence or successful execution authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceNavigationOperand {
    /// Source filename after the descriptor option prefix; no native
    /// loader or runtime option grammar is selected.
    File {
        /// Original argument ordinal, excluding the command head.
        argument: usize,
    },
    /// Package requirement after the selected member and option prefix.
    Package {
        /// Original argument ordinal, excluding the command head.
        argument: usize,
    },
}
/// Source package-reference purpose, independent of package installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePackageReferenceKind {
    /// A selected package requirement with an original name operand.
    Require,
    /// A selected provision with both original name and version operands.
    Provide,
}

/// Effective source operand from the actual selected package grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePackageReference {
    /// Requirement or version-supplying provision, never a provision query.
    pub kind: SourcePackageReferenceKind,
    /// Exact effective post-head package-name ordinal.
    pub argument: usize,
}

impl crate::resolved_invocation::ResolvedInvocation<'_, '_> {
    /// Original namespace-name and sole trailing body operands from the
    /// selected source vocabulary. This describes same-invocation source
    /// syntax, without an entered namespace, frame or executed body.
    #[must_use]
    pub fn authored_source_namespace_body_arguments(&self) -> Option<(usize, usize)> {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        use crate::{ArgRole, BodyInterpreter, ScriptTiming, Traits};
        if !self.semantics.traits.contains(Traits::DECLARES_NAMESPACE)
            || self.semantics.body_interpreter != BodyInterpreter::Current
        {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let indices = |role| {
            roles
                .iter()
                .filter(|(_, current)| *current == role)
                .map(|(argument, _)| self.semantics.argument_offset + usize::from(*argument))
                .collect::<Vec<_>>()
        };
        let names = indices(ArgRole::NamespaceName);
        let bodies = indices(ArgRole::Body);
        let ([name], [body]) = (names.as_slice(), bodies.as_slice()) else {
            return None;
        };
        (*body + 1 == count
            && *name < *body
            && self.authored_source_script_timing_at(*body) == Some(ScriptTiming::SameInvocation))
        .then_some((*name, *body))
    }

    /// Original post-head rule/procedure operand from the selected authored
    /// user-procedure schema and option grammar. This is a report candidate
    /// ordinal only; it supplies no owning rule, lookup or successful call.
    #[must_use]
    pub fn authored_source_rule_procedure_operand(&self) -> Option<usize> {
        // Implementation contract: naming.consumer.original-rule-reference-candidates
        // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset;
        if !self
            .semantics
            .traits
            .contains(crate::Traits::INVOKES_USER_PROC)
            || !self
                .semantics
                .arity
                .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        let prefix = self
            .semantics
            .options
            .leading_word_count(arguments.slice_from(offset))?;
        let argument = offset.checked_add(prefix)?;
        (argument < count).then_some(argument)
    }

    /// Source package geometry from the selected descriptor, complete count
    /// and option walk. A `provide NAME` query supplies no provision evidence.
    #[must_use]
    pub fn authored_source_package_reference(&self) -> Option<SourcePackageReference> {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        use crate::hooks::AnalyserHookId;
        match self.semantics.analyser_hook? {
            AnalyserHookId::PackageRequire => {
                let SourceNavigationOperand::Package { argument } =
                    self.authored_source_navigation_operand()?
                else {
                    return None;
                };
                Some(SourcePackageReference {
                    kind: SourcePackageReferenceKind::Require,
                    argument,
                })
            }
            AnalyserHookId::PackageProvide => {
                let offset = self.semantics.argument_offset;
                let count = self.words.arguments().exact_argv_len()?;
                (count == offset.checked_add(2)?
                    && self
                        .semantics
                        .arity
                        .accepts(self.argument_count_for_arity()?))
                .then_some(SourcePackageReference {
                    kind: SourcePackageReferenceKind::Provide,
                    argument: offset,
                })
            }
            _ => None,
        }
    }

    /// Select navigation geometry through the actual hook/member, source
    /// cardinality and shared option walk. Missing selectors or expansion
    /// withdraw this advice. Interpreter capability remains independent.
    #[must_use]
    pub fn authored_source_navigation_operand(&self) -> Option<SourceNavigationOperand> {
        use crate::hooks::AnalyserHookId;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset;
        if !self
            .semantics
            .arity
            .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        match self.semantics.analyser_hook? {
            AnalyserHookId::Source => {
                if offset == 0 && count == 1 {
                    return Some(SourceNavigationOperand::File { argument: 0 });
                }
                let prefix = self
                    .semantics
                    .options
                    .leading_word_count(arguments.slice_from(offset))?;
                let argument = offset.checked_add(prefix)?;
                (count == argument.checked_add(1)?)
                    .then_some(SourceNavigationOperand::File { argument })
            }
            AnalyserHookId::PackageRequire => {
                let prefix = self
                    .semantics
                    .options
                    .leading_word_count(arguments.slice_from(offset))?;
                let argument = offset.checked_add(prefix)?;
                (argument < count).then_some(SourceNavigationOperand::Package { argument })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationWord;

    #[test]
    fn authored_path_roles_preserve_namespace_body_and_assignment_layouts() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let actual =
            crate::model::ingress::context_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let registry = actual.commands();
        let select = |head, args: &[&str]| {
            let schema = crate::model::resolve_invocation_in_context(
                registry,
                Some(actual.context()),
                head,
                args,
            )
            .unwrap();
            (
                schema.authored_source_namespace_body_arguments(),
                schema.authored_source_assignment_arguments(),
            )
        };
        assert_eq!(
            select("namespace", &["eval", "N", "set x /tmp"]).0,
            Some((1, 2))
        );
        assert!(
            select("namespace", &["inscope", "N", "set x /tmp"])
                .0
                .is_none()
        );
        assert!(
            select("namespace", &["eval", "N", "set", "x", "/tmp"])
                .0
                .is_none()
        );
        assert_eq!(select("set", &["name", "/tmp"]).1, Some(vec![(0, Some(1))]));
        assert_eq!(
            select("variable", &["a", "/A", "b"]).1,
            Some(vec![(0, Some(1)), (2, None)])
        );
        assert_eq!(select("append", &["name", "/tmp"]).1, Some(vec![]));
    }

    #[test]
    fn authored_navigation_operands_preserve_actual_templates_and_option_ordinals() {
        // Implementation contract: naming.vendor.original-source-navigation
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-navigation.md
        let context = crate::model::ingress::static_context_for("f5-iapps");
        let select = |head, args: &[InvocationWord<'_>]| {
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                crate::InvocationWords::structured(InvocationWord::Literal(head), args),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .and_then(|selected| selected.authored_source_navigation_operand())
        };
        assert_eq!(
            select("source", &[InvocationWord::Literal("a.tcl")]),
            Some(SourceNavigationOperand::File { argument: 0 })
        );
        assert_eq!(
            select(
                "source",
                &[
                    InvocationWord::Literal("-encoding"),
                    InvocationWord::Literal("utf-8"),
                    InvocationWord::Literal("a.tcl")
                ]
            ),
            None
        );
        assert_eq!(
            select(
                "package",
                &[
                    InvocationWord::Literal("require"),
                    InvocationWord::Literal("-exact"),
                    InvocationWord::Literal("P"),
                    InvocationWord::Literal("1.0")
                ]
            ),
            Some(SourceNavigationOperand::Package { argument: 2 })
        );
        assert_eq!(
            select(
                "package",
                &[InvocationWord::Literal("require"), InvocationWord::Dynamic]
            ),
            None
        );
        assert_eq!(select("source", &[InvocationWord::Expanded]), None);
        let tmm = crate::model::ingress::static_context_for("f5-irules");
        let args = [InvocationWord::Literal("a.tcl")];
        assert!(
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                tmm.commands(),
                Some(tmm.context()),
                crate::InvocationWords::structured(InvocationWord::Literal("source"), &args),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .is_none()
        );
    }
    #[test]
    fn original_rule_operand_uses_actual_traits_options_and_cardinality() {
        // Implementation contract: naming.consumer.original-rule-reference-candidates
        // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
        let context = crate::model::ingress::static_context_for("f5-irules");
        let select = |head, arguments: &[InvocationWord<'_>]| {
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                crate::InvocationWords::structured(InvocationWord::Literal(head), arguments),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .and_then(|selected| selected.authored_source_rule_procedure_operand())
        };
        assert_eq!(
            select(
                "call",
                &[InvocationWord::Literal("Lib::one"), InvocationWord::Dynamic]
            ),
            Some(0)
        );
        assert_eq!(
            select(
                "call",
                &[
                    InvocationWord::Literal("-debug"),
                    InvocationWord::Literal("Lib::one")
                ]
            ),
            Some(1)
        );
        assert_eq!(select("call", &[InvocationWord::Expanded]), None);
        assert_eq!(select("call", &[]), None);
        assert_eq!(select("puts", &[InvocationWord::Literal("Lib::one")]), None);
    }

    #[test]
    fn original_package_reference_distinguishes_provision_query_and_selected_options() {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let select = |args: &[InvocationWord<'_>]| {
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                crate::InvocationWords::structured(InvocationWord::Literal("package"), args),
                tcl_dialect::model::InvocationRealm::InterpreterRuntime,
            )
            .resolved()
            .and_then(|schema| schema.authored_source_package_reference())
        };
        assert_eq!(
            select(&[
                InvocationWord::Literal("provide"),
                InvocationWord::Literal("csv")
            ]),
            None
        );
        assert_eq!(
            select(&[
                InvocationWord::Literal("provide"),
                InvocationWord::Literal("csv"),
                InvocationWord::Literal("1")
            ]),
            Some(SourcePackageReference {
                kind: SourcePackageReferenceKind::Provide,
                argument: 1
            })
        );
        assert_eq!(
            select(&[
                InvocationWord::Literal("require"),
                InvocationWord::Literal("-exact"),
                InvocationWord::Literal("csv"),
                InvocationWord::Literal("1")
            ]),
            Some(SourcePackageReference {
                kind: SourcePackageReferenceKind::Require,
                argument: 2
            })
        );
        assert_eq!(
            select(&[
                InvocationWord::Literal("provide"),
                InvocationWord::Literal("csv"),
                InvocationWord::Expanded
            ]),
            None
        );
    }
}
