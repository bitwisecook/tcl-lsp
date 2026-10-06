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

//! Case-list operand projection shared by source execution and possible regions.

use crate::body_execution::BodyOperand;
use crate::{CaseListSpec, CommandRegistry, InvocationArguments, InvocationFacts};

/// Possible selected scripts, retaining list element rather than fake argv positions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CaseBodyOperands {
    /// Original effective argument and optional decoded list element.
    pub bodies: Vec<BodyOperand>,
    /// Literal match-pattern locations from the same accepted clause grammar.
    /// These are data origins, not executable scripts or evaluated reads.
    pub patterns: Vec<BodyOperand>,
    /// Layout depends on a dynamic subject that may also select a leading option.
    /// Callers retain an unknown selection/error continuation independently.
    pub selection_unknown: bool,
    /// A normal successor may be selected without entering any listed body.
    /// Only the descriptor's exhaustive fallback removes this alternative.
    pub no_match_possible: bool,
}

impl CaseListSpec {
    /// Project possible bodies through this descriptor's existing option and
    /// clause grammar. Unknown subject bytes are never replaced with a literal.
    /// A two-operand subject/list shape may enter these bodies, while an opaque
    /// option selection remains a separate residual. No matching is predicted.
    #[must_use]
    pub fn possible_body_operands(
        self,
        arguments: InvocationArguments<'_>,
        options: &[&crate::hover::OptionSpec],
    ) -> Option<CaseBodyOperands> {
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| arguments.literal_at(index))
            .collect::<Vec<_>>();
        let layout = self.invocation_values(
            &values,
            options,
            arguments
                .dialect()
                .and_then(crate::InvocationDialect::authoring_query),
        )?;
        let list_index = layout.clause_list_index;
        let inline_start = layout.inline_clause_start;
        let selection_unknown = layout.subject_index.is_some_and(|index| {
            values[index].is_none()
                && !values[..index].contains(&self.end_options_option)
                && self
                    .option_scan_reserved_for_arguments(
                        arguments,
                        arguments
                            .dialect()
                            .and_then(crate::InvocationDialect::authoring_query),
                        2,
                    )
                    .is_none_or(|reserved| count != 2 || reserved != 2)
        });
        let no_match_possible;
        let patterns;
        let bodies = if let Some(argument) = list_index {
            let dialect = arguments.dialect()?;
            let elements = dialect.word_values.split_list(values[argument]?).ok()?;
            if elements.is_empty() {
                return None;
            }
            let literals = elements.iter().map(AsRef::as_ref).collect::<Vec<_>>();
            let clauses = self.inline_clauses(&literals, 0)?;
            if self
                .fallthrough_body
                .is_some_and(|marker| literals.last().copied() == Some(marker))
            {
                return None;
            }
            no_match_possible = !self.clauses_are_exhaustive(&literals, &clauses);
            patterns = clauses
                .iter()
                .map(|clause| BodyOperand {
                    argument,
                    list_element: Some(clause.pattern_index),
                })
                .collect();
            clauses
                .into_iter()
                .filter_map(|clause| {
                    let element = clause.body_index?;
                    (self.fallthrough_body != Some(literals[element])).then_some(BodyOperand {
                        argument,
                        list_element: Some(element),
                    })
                })
                .collect()
        } else {
            let start = inline_start?;
            let literals = values[start..]
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()?;
            let clauses = self.inline_clauses(&literals, 0)?;
            no_match_possible = !self.clauses_are_exhaustive(&literals, &clauses);
            patterns = clauses
                .iter()
                .map(|clause| BodyOperand {
                    argument: start + clause.pattern_index,
                    list_element: None,
                })
                .collect();
            clauses
                .into_iter()
                .filter_map(|clause| {
                    let relative = clause.body_index?;
                    (self.fallthrough_body != Some(literals[relative])).then_some(BodyOperand {
                        argument: start + relative,
                        list_element: None,
                    })
                })
                .collect()
        };
        Some(CaseBodyOperands {
            bodies,
            patterns,
            selection_unknown,
            no_match_possible,
        })
    }

    fn clauses_are_exhaustive(
        self,
        literals: &[&str],
        clauses: &[crate::spec::InlineCaseClause],
    ) -> bool {
        clauses.iter().enumerate().any(|(index, clause)| {
            self.exhaustive_keyword_patterns
                .contains(&literals[clause.pattern_index])
                && self.is_keyword_pattern(literals[clause.pattern_index], index, clauses.len())
        })
    }
}

/// Select body locations using the resolved descriptor, not source spelling.
#[must_use]
pub fn script_body_flow_in_registry(
    registry: &CommandRegistry,
    facts: &InvocationFacts,
    arguments: InvocationArguments<'_>,
) -> crate::script_body_flow::ScriptBodyFlow {
    if let Some(spec) = registry.get(&facts.canonical_command)
        && let Some(case_list) = spec.case_list
    {
        let options = spec.options.iter().collect::<Vec<_>>();
        return case_list
            .possible_body_operands(arguments, &options)
            .map_or_else(
                || crate::script_body_flow::ScriptBodyFlow::Unknown(Vec::new()),
                crate::script_body_flow::ScriptBodyFlow::CaseBodies,
            );
    }
    crate::script_body_flow::script_body_flow_for_invocation(facts, arguments)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dynamic_subject_layout_keeps_options_and_authored_no_match_paths() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let spec = registry.get("switch").unwrap();
        let options = spec.options.iter().collect::<Vec<_>>();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (clause_list, no_match) in [
            ("a {set x 1}", true),
            ("a {set x 1} default {set x 2}", false),
            ("default {set x 1} a {set x 2}", true),
        ] {
            let words = [
                crate::InvocationWord::Literal("--"),
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal(clause_list),
            ];
            let selection = spec
                .case_list
                .unwrap()
                .possible_body_operands(
                    InvocationArguments::structured(&words).with_dialect(dialect),
                    &options,
                )
                .unwrap();
            assert!(!selection.selection_unknown, "{clause_list}");
            assert_eq!(selection.no_match_possible, no_match, "{clause_list}");
            assert!(selection.bodies.iter().all(|body| body.argument == 2));
            assert!(
                selection
                    .patterns
                    .iter()
                    .all(|pattern| pattern.argument == 2)
            );
            assert_eq!(selection.patterns[0].list_element, Some(0));
            if clause_list.contains("default") {
                assert_eq!(selection.patterns[1].list_element, Some(2));
            }
        }
        let event_spec = crate::CaseListSpec::EXPECT;
        let words = ["default {set x 1}"];
        let selection = event_spec
            .possible_body_operands(
                InvocationArguments::literals(&words).with_dialect(dialect),
                &[],
            )
            .unwrap();
        assert!(
            selection.no_match_possible,
            "event keyword does not imply exhaustive input matching"
        );
    }

    #[test]
    fn dynamic_subject_list_bodies_keep_list_element_identity() {
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let spec = registry.get("switch").unwrap();
        let values = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("loud {set debug 1} default {set debug 0}"),
        ];
        let arguments = InvocationArguments::structured(&values).with_dialect(
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules()),
        );
        let selected = spec
            .case_list
            .unwrap()
            .possible_body_operands(arguments, &spec.options.iter().collect::<Vec<_>>())
            .unwrap();
        assert!(selected.selection_unknown);
        assert_eq!(
            selected.bodies,
            vec![
                BodyOperand {
                    argument: 1,
                    list_element: Some(1)
                },
                BodyOperand {
                    argument: 1,
                    list_element: Some(3)
                },
            ]
        );
    }
    #[test]
    fn malformed_and_fallthrough_lists_share_authored_clause_grammar() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let spec = registry.get("switch").unwrap();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for text in ["a {set x 1} dangling", "a -"] {
            let words = ["x", text];
            assert!(
                spec.case_list
                    .unwrap()
                    .possible_body_operands(
                        InvocationArguments::literals(&words).with_dialect(dialect),
                        &spec.options.iter().collect::<Vec<_>>()
                    )
                    .is_none()
            );
        }
        let words = ["x", "a - b {set x 1}"];
        let selection = spec
            .case_list
            .unwrap()
            .possible_body_operands(
                InvocationArguments::literals(&words).with_dialect(dialect),
                &spec.options.iter().collect::<Vec<_>>(),
            )
            .unwrap();
        assert_eq!(
            selection.bodies,
            vec![BodyOperand {
                argument: 1,
                list_element: Some(3)
            }]
        );
    }

    #[test]
    fn dynamic_two_argument_subject_uses_the_selected_optionless_protocol() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let spec = registry.get("switch").unwrap();
        let words = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("default {set x 1}"),
        ];
        for version in tcl_dialect::TclVersion::ALL {
            let selected = spec
                .case_list
                .unwrap()
                .possible_body_operands(
                    InvocationArguments::structured(&words)
                        .with_dialect(crate::InvocationDialect::for_version(version)),
                    &spec.options.iter().collect::<Vec<_>>(),
                )
                .unwrap();
            assert_eq!(
                selected.selection_unknown,
                version == tcl_dialect::TclVersion::V8_4,
                "{version:?}"
            );
            assert_eq!(selected.bodies.len(), 1);
        }
        assert!(
            spec.case_list
                .unwrap()
                .possible_body_operands(
                    InvocationArguments::structured(&words),
                    &spec.options.iter().collect::<Vec<_>>(),
                )
                .is_none()
        );
    }
}
