// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional configuration-reference operands from selected source metadata.

use crate::resolved_invocation::{AuthoredSourceOptionBoundary, ResolvedInvocation};

/// Selected source grammar for a readonly configuration-reference candidate.
/// This supplies no native handler, object identity or accepted invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceConfigurationReferenceLayout {
    /// A three-word item/operator/data-group tail after the declared options.
    DataGroupComparison,
    /// The final data-group operand of `class lookup <key> <data-group>`.
    DataGroupLookup,
}

impl SourceConfigurationReferenceLayout {
    /// Complete declarative authoring vocabulary.
    pub const ALL: &'static [Self] = &[Self::DataGroupComparison, Self::DataGroupLookup];

    /// Exact declarative spelling used by native and loadable metadata.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DataGroupComparison => "DataGroupComparison",
            Self::DataGroupLookup => "DataGroupLookup",
        }
    }
}

/// Configuration-kind family, independent of runtime object lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationObjectFamily {
    /// Internal, external and file-backed data-group candidates.
    DataGroup,
}

/// One readonly source-reference ordinal from an actual selected descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceConfigurationReference {
    /// Effective post-head ordinal; original source geometry remains separate.
    pub argument: usize,
    /// Configuration family supplied by the selected descriptor.
    pub family: ConfigurationObjectFamily,
    /// The unknown item value can still be an option rather than data.
    pub selection_unknown: bool,
}

/// Comparison vocabulary shared by selected source layouts and the existing
/// literal compatibility resolver. This recognises syntax, not a live handler.
#[must_use]
pub fn is_data_group_comparison_operator(word: &str) -> bool {
    use tcl_syntax::expr::ast::BinOp;
    [
        BinOp::Contains,
        BinOp::StartsWith,
        BinOp::EndsWith,
        BinOp::StrEquals,
        BinOp::MatchesGlob,
        BinOp::MatchesRegex,
    ]
    .iter()
    .any(|operator| operator.as_str().eq_ignore_ascii_case(word))
}

impl ResolvedInvocation<'_, '_> {
    /// Project a conditional reference from the actual selected subcommand.
    /// Ordinary unknown item values keep their slot; uncertain option prefixes,
    /// selectors, operator values and expansion cardinality decline. A result
    /// proves no accepted argv, object availability or successful execution.
    #[must_use]
    pub fn authored_source_configuration_reference_operand(
        &self,
    ) -> Option<AuthoredSourceConfigurationReference> {
        let layout = self
            .authored_source_descriptors()
            .subcommand?
            .source_configuration_reference?;
        match layout {
            SourceConfigurationReferenceLayout::DataGroupComparison => {
                let arguments = self.words.arguments();
                let count = arguments.exact_argv_len()?;
                if !self
                    .semantics
                    .arity
                    .accepts(self.argument_count_for_arity()?)
                {
                    return None;
                }
                let item = count.checked_sub(3)?;
                if item < self.semantics.argument_offset
                    || !arguments.get(item)?.has_exactly_one_argv_entry()
                {
                    return None;
                }
                let operator = item.checked_add(1)?;
                let argument = item.checked_add(2)?;
                if !is_data_group_comparison_operator(arguments.literal_at(operator)?)
                    || arguments.literal_at(argument)?.contains('\0')
                {
                    return None;
                }
                let scan = self.authored_source_diagnostic_options()?;
                if scan.options.iter().any(|option| {
                    !option.available
                        || option
                            .values
                            .as_ref()
                            .is_none_or(|values| values.end > item)
                }) {
                    return None;
                }
                let selection_unknown = match scan.boundary {
                    AuthoredSourceOptionBoundary::Positional(start) if start == item => false,
                    AuthoredSourceOptionBoundary::Terminator(terminator)
                        if terminator.checked_add(1) == Some(item) =>
                    {
                        false
                    }
                    AuthoredSourceOptionBoundary::Dynamic(start) if start == item => {
                        !arguments.get(start)?.proves_non_option()
                    }
                    _ => return None,
                };
                Some(AuthoredSourceConfigurationReference {
                    argument,
                    family: ConfigurationObjectFamily::DataGroup,
                    selection_unknown,
                })
            }
            SourceConfigurationReferenceLayout::DataGroupLookup => {
                let arguments = self.words.arguments();
                let count = arguments.exact_argv_len()?;
                let key = self.semantics.argument_offset;
                let argument = key.checked_add(1)?;
                if count != self.semantics.argument_offset.checked_add(2)?
                    || !self
                        .semantics
                        .arity
                        .accepts(self.argument_count_for_arity()?)
                    || !arguments.get(key)?.has_exactly_one_argv_entry()
                {
                    return None;
                }
                if arguments.literal_at(argument)?.contains('\0') {
                    return None;
                }
                Some(AuthoredSourceConfigurationReference {
                    argument,
                    family: ConfigurationObjectFamily::DataGroup,
                    selection_unknown: false,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationWord;

    fn project(args: &[InvocationWord<'_>]) -> Option<AuthoredSourceConfigurationReference> {
        let context = crate::model::ingress::static_context_for("f5-irules");
        crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(InvocationWord::Literal("class"), args),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .and_then(|selected| selected.authored_source_configuration_reference_operand())
    }

    #[test]
    fn selected_configuration_layout_keeps_unknown_item_and_original_ordinal() {
        use InvocationWord::{Dynamic, DynamicNonOption, Literal};
        for (args, argument, selection_unknown) in [
            (
                vec![Literal("match"), Dynamic, Literal("equals"), Literal("dg")],
                3,
                true,
            ),
            (
                vec![
                    Literal("match"),
                    Literal("--"),
                    Dynamic,
                    Literal("equals"),
                    Literal("dg"),
                ],
                4,
                false,
            ),
            (
                vec![
                    Literal("match"),
                    Literal("-nocase"),
                    Dynamic,
                    Literal("equals"),
                    Literal("dg"),
                ],
                4,
                true,
            ),
            (
                vec![
                    Literal("match"),
                    DynamicNonOption,
                    Literal("equals"),
                    Literal("dg"),
                ],
                3,
                false,
            ),
        ] {
            assert_eq!(
                project(&args),
                Some(AuthoredSourceConfigurationReference {
                    argument,
                    family: ConfigurationObjectFamily::DataGroup,
                    selection_unknown,
                })
            );
        }
        for operator in [
            "contains",
            "starts_with",
            "ends_with",
            "equals",
            "matches_glob",
            "matches_regex",
        ] {
            assert!(
                project(&[Literal("match"), Dynamic, Literal(operator), Literal("dg")]).is_some()
            );
        }
    }

    #[test]
    fn lookup_layout_keeps_literal_group_after_dynamic_key() {
        // Implementation contract: naming.consumer.original-source-attachment-candidates

        use InvocationWord::{Dynamic, Literal};
        assert_eq!(
            project(&[Literal("lookup"), Dynamic, Literal("uri_redirect_map")]),
            Some(AuthoredSourceConfigurationReference {
                argument: 2,
                family: ConfigurationObjectFamily::DataGroup,
                selection_unknown: false,
            })
        );
        assert_eq!(
            project(&[Literal("lookup"), Dynamic, Dynamic]),
            None,
            "a dynamic data-group operand is not a literal source candidate"
        );
        assert_eq!(project(&[Literal("lookup"), Dynamic]), None);
        assert_eq!(
            project(&[
                Literal("lookup"),
                Dynamic,
                Literal("uri_redirect_map"),
                Literal("extra")
            ]),
            None,
            "extra lookup arguments are outside the selected exact schema"
        );
        assert_eq!(
            project(&[
                Literal("lookup"),
                InvocationWord::Opaque,
                Literal("uri_redirect_map")
            ]),
            None,
            "an opaque key without exactly one argv entry cannot anchor the target"
        );
    }

    #[test]
    fn selected_configuration_layout_declines_unknown_control_and_cardinality() {
        use InvocationWord::{Dynamic, Expanded, Literal, Opaque};
        for args in [
            vec![Dynamic, Literal("x"), Literal("equals"), Literal("dg")],
            vec![Literal("search"), Dynamic, Literal("equals"), Literal("dg")],
            vec![Literal("match"), Dynamic, Dynamic, Literal("dg")],
            vec![Literal("match"), Dynamic, Literal("equals"), Dynamic],
            vec![Literal("match"), Dynamic, Literal("bogus"), Literal("dg")],
            vec![Literal("match"), Dynamic, Literal("equals")],
            vec![
                Literal("match"),
                Dynamic,
                Literal("equals"),
                Literal("dg"),
                Literal("extra"),
            ],
            vec![
                Literal("match"),
                Literal("-unknown"),
                Dynamic,
                Literal("equals"),
                Literal("dg"),
            ],
            vec![
                Literal("match"),
                Dynamic,
                Dynamic,
                Literal("equals"),
                Literal("dg"),
            ],
            vec![Literal("match"), Expanded, Literal("equals"), Literal("dg")],
            vec![Literal("match"), Opaque, Literal("equals"), Literal("dg")],
            vec![
                Literal("match"),
                Dynamic,
                Literal("equals"),
                Literal("dg\0tail"),
            ],
        ] {
            assert_eq!(project(&args), None, "{args:?}");
        }
    }
}
