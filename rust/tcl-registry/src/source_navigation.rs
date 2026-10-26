// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-navigation operands from the actual authored Registry schema.

mod static_display;
pub use static_display::{StaticTclSourceDisplay, static_tcl_source_display};

/// Conditional source-query algebra from a selected authored descriptor.
/// No current package, provided value, load success or Native handler follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePackageQuery {
    /// Query the already-provided version of one package; never its setter.
    ProvidedVersion,
    /// A requirement that may load code; distinct from a read-only query.
    RequiredVersion,
    /// Compare one version against one or more requirement expressions.
    VersionSatisfies,
}

/// Exact selected effective operand range, excluding the command head and
/// subcommand selector. Source values and execution need independent owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePackageQueryInvocation {
    /// Selected descriptor algebra, not a display-name classification.
    pub query: SourcePackageQuery,
    /// Complete query operands, in the original effective post-head space.
    pub arguments: std::ops::Range<usize>,
}

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

/// Classification of the actual selected source package operation.
/// Missing selector/operand/cardinality is unavailable, not a non-reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePackageReferenceSelection {
    /// Selected require or version-supplying provide naming operand.
    Reference(SourcePackageReference),
    /// Complete selected non-reference operation, including provide-name query.
    NotReference,
    /// Source selector, options or cardinality cannot establish the operation.
    Unavailable,
}

impl crate::resolved_invocation::ResolvedInvocation<'_, '_> {
    /// Reborrow the actual selected subcommand/form query geometry. A selected
    /// form with no query explicitly withholds the parent's source algebra.
    /// This is a conditional source model only, never a package-state read.
    #[must_use]
    pub fn authored_source_package_query(&self) -> Option<SourcePackageQueryInvocation> {
        let query = if let Some(form) = self.selected.form {
            form.source_package_query?
        } else {
            self.selected.sub?.source_package_query?
        };
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let first = self.semantics.argument_offset;
        if !self
            .semantics
            .arity
            .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        match query {
            SourcePackageQuery::ProvidedVersion if count == first.checked_add(1)? => {}
            SourcePackageQuery::RequiredVersion if count == first.checked_add(1)? => {
                // The shared option walk must prove that this is a package
                // operand, not an option-only/malformed require invocation.
                if self.authored_source_navigation_operand()?
                    != (SourceNavigationOperand::Package { argument: first })
                {
                    return None;
                }
            }
            SourcePackageQuery::VersionSatisfies if count >= first.checked_add(2)? => {}
            _ => return None,
        }
        Some(SourcePackageQueryInvocation {
            query,
            arguments: first..count,
        })
    }

    /// Original namespace-name and sole trailing body operands from the
    /// selected source vocabulary. This describes same-invocation source
    /// syntax, without an entered namespace, frame or executed body.
    #[must_use]
    pub fn authored_source_namespace_body_arguments(&self) -> Option<(usize, usize)> {
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

    /// Exact post-head name, version and script ordinals from the selected
    /// package-registration source schema. These conditional data coordinates
    /// establish neither an installed package nor execution of its deferred body.
    #[must_use]
    pub fn authored_source_package_registration(&self) -> Option<(usize, usize, usize)> {
        use crate::{ArgRole, hooks::AnalyserHookId};
        if self.semantics.analyser_hook != Some(AnalyserHookId::PackageIfneeded)
            || !self
                .semantics
                .arity
                .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete || count != self.semantics.argument_offset.checked_add(3)? {
            return None;
        }
        let ordinal = |wanted| {
            let mut selected =
                roles
                    .iter()
                    .filter(|(_, role)| *role == wanted)
                    .map(|&(position, _)| {
                        self.semantics
                            .argument_offset
                            .checked_add(usize::from(position))
                    });
            let first = selected.next()??;
            (first < count && selected.next().is_none()).then_some(first)
        };
        let name = ordinal(ArgRole::Name)?;
        let version = ordinal(ArgRole::Value)?;
        let body = ordinal(ArgRole::Body)?;
        (name != version && name != body && version != body).then_some((name, version, body))
    }

    /// Source package geometry from the selected descriptor, complete count
    /// and option walk. A `provide NAME` query supplies no provision evidence.
    #[must_use]
    pub fn authored_source_package_reference(&self) -> Option<SourcePackageReference> {
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

    /// Preserve unavailable source operation selection for inventory consumers.
    /// This classifies selected metadata only, without runtime package absence.
    #[must_use]
    pub fn authored_source_package_reference_selection(&self) -> SourcePackageReferenceSelection {
        use crate::hooks::AnalyserHookId;
        use crate::resolved_invocation::SubcommandResolution;
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return SourcePackageReferenceSelection::Unavailable;
        }
        if let Some(reference) = self.authored_source_package_reference() {
            return SourcePackageReferenceSelection::Reference(reference);
        }
        match self.semantics.analyser_hook {
            Some(AnalyserHookId::PackageRequire) => SourcePackageReferenceSelection::Unavailable,
            Some(AnalyserHookId::PackageProvide) => {
                let query = self.words.arguments().exact_argv_len()
                    == self.semantics.argument_offset.checked_add(1)
                    && self
                        .argument_count_for_arity()
                        .is_some_and(|count| self.semantics.arity.accepts(count));
                if query {
                    SourcePackageReferenceSelection::NotReference
                } else {
                    SourcePackageReferenceSelection::Unavailable
                }
            }
            _ => SourcePackageReferenceSelection::NotReference,
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
    fn original_package_registration_ordinals_keep_query_and_expansion_distinct() {
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let select = |arguments: &[InvocationWord<'_>]| {
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                crate::InvocationWords::structured(InvocationWord::Literal("package"), arguments),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .and_then(|selected| selected.authored_source_package_registration())
        };
        assert_eq!(
            select(&[
                InvocationWord::Literal("ifneeded"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("1.0"),
                InvocationWord::Dynamic
            ]),
            Some((1, 2, 3))
        );
        assert_eq!(
            select(&[
                InvocationWord::Literal("ifneeded"),
                InvocationWord::Literal("p"),
                InvocationWord::Literal("1.0")
            ]),
            None
        );
        assert_eq!(
            select(&[
                InvocationWord::Literal("ifneeded"),
                InvocationWord::Expanded,
                InvocationWord::Literal("1.0"),
                InvocationWord::Dynamic
            ]),
            None
        );
        assert_eq!(
            select(&[
                InvocationWord::Literal("provide"),
                InvocationWord::Literal("p"),
                InvocationWord::Literal("1.0")
            ]),
            None
        );
        let mut malformed = crate::CommandRegistry::build_default();
        malformed.insert(crate::CommandSpec {
            name: "register",
            arity: crate::Arity::exact(3),
            analyser_hook: Some(crate::hooks::AnalyserHookId::PackageIfneeded),
            arg_roles: &[
                (0, crate::ArgRole::Name),
                (0, crate::ArgRole::Value),
                (0, crate::ArgRole::Body),
            ],
            ..crate::CommandSpec::DEFAULT
        });
        let arguments = [
            InvocationWord::Literal("p"),
            InvocationWord::Literal("1.0"),
            InvocationWord::Dynamic,
        ];
        assert_eq!(
            malformed
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        InvocationWord::Literal("register"),
                        &arguments
                    ),
                    None,
                )
                .resolved()
                .and_then(|schema| schema.authored_source_package_registration()),
            None
        );
    }

    #[test]
    fn original_package_reference_distinguishes_provision_query_and_selected_options() {
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

#[cfg(test)]
mod target_survey_query_tests {
    use super::*;
    use crate::{Arity, CommandRegistry, CommandSpec, SubCommand};

    #[test]
    fn selected_builtin_queries_keep_complete_ordinals_and_options() {
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let select = |args: &[&str]| {
            crate::model::resolve_invocation_in_context(
                context.commands(),
                Some(context.context()),
                "package",
                args,
            )
            .and_then(|schema| schema.authored_source_package_query())
        };
        assert_eq!(
            select(&["provide", "Tcl"]),
            Some(SourcePackageQueryInvocation {
                query: SourcePackageQuery::ProvidedVersion,
                arguments: 1..2,
            })
        );
        assert_eq!(
            select(&["require", "Tcl"]),
            Some(SourcePackageQueryInvocation {
                query: SourcePackageQuery::RequiredVersion,
                arguments: 1..2,
            })
        );
        assert_eq!(
            select(&["vsatisfies", "8.6", "8.5", "9"]),
            Some(SourcePackageQueryInvocation {
                query: SourcePackageQuery::VersionSatisfies,
                arguments: 1..4,
            })
        );
        for args in [
            vec!["provide", "Tcl", "8.6"],
            vec!["require", "-exact"],
            vec!["require", "-exact", "Tcl"],
            vec!["require"],
            vec!["provide"],
            vec!["vsatisfies", "8.6"],
        ] {
            assert!(select(&args).is_none(), "{args:?}");
        }
        let expanded = [
            crate::InvocationWord::Literal("provide"),
            crate::InvocationWord::Expanded,
        ];
        assert!(
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("package"),
                    &expanded
                ),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .and_then(|schema| schema.authored_source_package_query())
            .is_none()
        );
    }

    #[test]
    fn selected_concrete_form_cannot_inherit_parent_query_by_display_name() {
        const FORMS: &[crate::forms::SubCommandForm] = &[crate::forms::SubCommandForm {
            name: "provide",
            arity: Arity::exact(1),
            ..crate::forms::SubCommandForm::DEFAULT
        }];
        const MEMBERS: &[SubCommand] = &[SubCommand {
            name: "provide",
            arity: Arity::exact(1),
            subcommand_forms: FORMS,
            source_package_query: Some(SourcePackageQuery::ProvidedVersion),
            ..SubCommand::DEFAULT
        }];
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "survey-form-fixture",
            subcommands: MEMBERS,
            ..CommandSpec::DEFAULT
        });
        let schema = crate::model::resolve_invocation_in_context(
            &registry,
            None,
            "survey-form-fixture",
            &["provide", "Tcl"],
        )
        .unwrap();
        assert!(schema.form.is_some());
        assert!(schema.authored_source_package_query().is_none());
    }
}
