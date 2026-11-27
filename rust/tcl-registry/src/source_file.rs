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

//! Native source-file argument selection, independent of file availability.

use crate::InvocationArguments;
use tcl_dialect::model::Family;
use tcl_native_model::InvocationDialect;
use tcl_native_model::source_file::{SourceFileGrammar, SourceFileOperands, SourceFileSelection};

/// Invalid conditional file-source argument layout for one selected authored
/// recipe. This supplies no handler identity, raised error or file effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceFileLayoutAbstention {
    descriptor: crate::CanonicalCommandDescriptorKey<'static>,
    dialect: InvocationDialect,
    grammar: SourceFileGrammar,
    count: usize,
    first: Option<Vec<u8>>,
}

impl AuthoredSourceFileLayoutAbstention {
    /// Same selected source recipe, actual dialect and original argv shape.
    /// Registry generation and original source ownership remain caller-owned.
    #[must_use]
    pub fn matches_invocation(&self, invocation: &crate::ResolvedInvocation<'_, '_>) -> bool {
        invocation.authored_source_file_layout_abstention().as_ref() == Some(self)
    }

    /// Actual source grammar that refused this exact argument shape.
    #[must_use]
    pub const fn grammar(&self) -> SourceFileGrammar {
        self.grammar
    }
}

impl crate::ResolvedInvocation<'_, '_> {
    /// Prove Invalid source-file layout independently from missing schema,
    /// unknown selectors and path values. Only authored syntax is observed;
    /// runtime dispatch, errors and completed file evaluation stay unknown.
    #[must_use]
    pub fn authored_source_file_layout_abstention(
        &self,
    ) -> Option<AuthoredSourceFileLayoutAbstention> {
        if self.semantics.body_execution
            != Some(crate::body_execution::BodyExecutionSpec::SourceFile)
            || !self.semantics.traits.contains(crate::Traits::SOURCES_FILE)
            || self.semantics.argument_offset != 0
        {
            return None;
        }
        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        let grammar = dialect.source_file_grammar()?;
        let count = arguments.exact_argv_len()?;
        let first = arguments.get(0).and_then(|word| match word {
            crate::InvocationWord::KnownBytes(bytes) => Some(bytes),
            crate::InvocationWord::Literal(value) => Some(value.as_bytes()),
            _ => None,
        });
        (grammar.select_original(count, first) == SourceFileSelection::Invalid).then(|| {
            AuthoredSourceFileLayoutAbstention {
                descriptor: self.descriptor_key(),
                dialect,
                grammar,
                count,
                first: first.map(<[u8]>::to_vec),
            }
        })
    }
}

/// Parse only reached, frozen argv through the actual native handler grammar.
#[must_use]
pub fn select(arguments: InvocationArguments<'_>) -> SourceFileSelection {
    use SourceFileSelection::Unknown;
    let Some(grammar) = arguments
        .dialect()
        .and_then(InvocationDialect::source_file_grammar)
    else {
        return Unknown;
    };
    select_with_grammar(arguments, grammar)
}

/// Source-navigation path candidate, without file availability or execution
/// authority. Missing native axes admit only layouts shared by every audited
/// file grammar; an option whose availability differs remains unknown.
#[must_use]
pub fn path_candidate(arguments: InvocationArguments<'_>) -> SourceFileSelection {
    if let Some(dialect) = arguments.dialect() {
        if let Some(grammar) = dialect.source_file_grammar() {
            return select_with_grammar(arguments, grammar);
        }
        // An unversioned Tcl or Jim interpreter follows one audited grammar.
        if !matches!(dialect.family(), Some(Family::Tcl | Family::Jim)) {
            return SourceFileSelection::Unknown;
        }
    }
    let first = select_with_grammar(arguments, SourceFileGrammar::Filename);
    if [
        SourceFileGrammar::Encoding,
        SourceFileGrammar::EncodingOrNoPackage,
    ]
    .into_iter()
    .all(|grammar| select_with_grammar(arguments, grammar) == first)
    {
        first
    } else {
        SourceFileSelection::Unknown
    }
}

fn select_with_grammar(
    arguments: InvocationArguments<'_>,
    grammar: SourceFileGrammar,
) -> SourceFileSelection {
    use SourceFileSelection::{Invalid, Selected, Unknown};
    let Some(count) = arguments.exact_argv_len() else {
        return Unknown;
    };
    let selected = |path_at, encoding_at, no_package| {
        Selected(SourceFileOperands {
            path_at,
            encoding_at,
            no_package,
        })
    };
    if count == 1 {
        return selected(0, None, false);
    }
    if grammar == SourceFileGrammar::Filename {
        return Invalid;
    }
    if count == 3 {
        return match arguments.literal_at(0) {
            Some("-encoding") => selected(2, Some(1), false),
            Some(_) => Invalid,
            None => Unknown,
        };
    }
    if count == 2 && grammar == SourceFileGrammar::EncodingOrNoPackage {
        return match arguments.literal_at(0) {
            Some("-nopkg") => selected(1, None, true),
            Some(_) => Invalid,
            None => Unknown,
        };
    }
    Invalid
}

/// Possible external source ingress from the shared filename grammar.
/// The operand retains its value knowledge and ordinal; no file availability,
/// successful dispatch or completed file evaluation is established.
pub const SOURCE_TRANSITIONS: crate::StateTransitionDescriptor = crate::StateTransitionDescriptor {
    logical_source_resolver: None,
    resolver: Some(source_transitions),
    argument_shape: crate::state_transition::StateTransitionArgumentShape::Positional,
    commit: crate::state_transition::StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    ..crate::StateTransitionDescriptor::EMPTY
};

fn source_transitions(arguments: InvocationArguments<'_>) -> crate::StateTransitions {
    let mut transitions = crate::StateTransitions::default();
    if let SourceFileSelection::Selected(operands) = path_candidate(arguments)
        && let Some(path) = crate::TransitionSubject::from_argument(arguments, operands.path_at)
    {
        transitions.push(crate::StateTransition::Package(
            crate::model::binding::PackageTransition::SourceLoad { path },
        ));
    }
    transitions
}

/// Apply the actual file-evaluation return boundary without opening a frame.
/// Raw break/continue and errors escape source unchanged. Jim's outer source
/// handler consumes any pending return left by its file evaluator as OK.
#[must_use]
pub fn completion_route(
    dialect: InvocationDialect,
    route: crate::completion_route::InvocationCompletionRoute,
) -> crate::completion_route::InvocationCompletionRoute {
    use crate::completion::CompletionCode;
    use crate::completion_route::InvocationCompletionRoute as Route;
    let Some(grammar) = dialect.source_file_grammar() else {
        return if matches!(route, Route::Return(_) | Route::Tcl(CompletionCode::Return)) {
            Route::UnknownAbrupt
        } else {
            route
        };
    };
    let reduced = route.through_procedure_boundary();
    if grammar == SourceFileGrammar::Filename
        && dialect.family() == Some(Family::Jim)
        && matches!(route, Route::Return(_) | Route::Tcl(CompletionCode::Return))
        && matches!(
            reduced,
            Route::Return(_) | Route::Tcl(CompletionCode::Return) | Route::Unknown
        )
    {
        Route::Tcl(CompletionCode::Ok)
    } else {
        reduced
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};
    use tcl_dialect::TclVersion;

    fn inspect_layout(
        version: TclVersion,
        head: &str,
        words: &[InvocationWord<'_>],
        check: impl FnOnce(&crate::ResolvedInvocation<'_, '_>),
    ) {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let context = crate::model::ingress::static_context_for_profile(profile);
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            InvocationWords::structured(InvocationWord::Literal(head), words)
                .with_dialect(InvocationDialect::for_version(version)),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        check(&resolution.resolved().unwrap());
    }

    #[test]
    fn authored_invalid_layout_retains_recipe_and_actual_grammar() {
        let encoding = [
            InvocationWord::Literal("-encoding"),
            InvocationWord::Literal("utf-8"),
            InvocationWord::Dynamic,
        ];
        for version in TclVersion::ALL {
            inspect_layout(version, "source", &encoding, |selected| {
                let receipt = selected.authored_source_file_layout_abstention();
                if version == TclVersion::V8_4 {
                    let receipt = receipt.unwrap();
                    assert_eq!(receipt.grammar(), SourceFileGrammar::Filename);
                    assert!(receipt.matches_invocation(selected));
                } else {
                    assert!(receipt.is_none(), "{version:?}");
                }
            });
            for words in [
                &[InvocationWord::Dynamic][..],
                &[InvocationWord::Expanded][..],
            ] {
                inspect_layout(version, "source", words, |selected| {
                    assert!(selected.authored_source_file_layout_abstention().is_none());
                });
            }
            inspect_layout(version, "list", &encoding, |selected| {
                assert!(selected.authored_source_file_layout_abstention().is_none());
            });
        }
    }

    #[test]
    fn authored_invalid_layout_refuses_unknown_selector_and_other_query() {
        let encoding = [
            InvocationWord::Literal("-encoding"),
            InvocationWord::Literal("utf-8"),
            InvocationWord::Dynamic,
        ];
        let mut receipt = None;
        inspect_layout(TclVersion::V8_4, "source", &encoding, |selected| {
            receipt = selected.authored_source_file_layout_abstention();
        });
        let receipt = receipt.unwrap();
        inspect_layout(TclVersion::V8_5, "source", &encoding, |selected| {
            assert!(!receipt.matches_invocation(selected));
        });
        let dynamic = [
            InvocationWord::Dynamic,
            InvocationWord::Dynamic,
            InvocationWord::Dynamic,
        ];
        inspect_layout(TclVersion::V8_5, "source", &dynamic, |selected| {
            assert!(selected.authored_source_file_layout_abstention().is_none());
        });
        inspect_layout(TclVersion::V8_4, "source", &dynamic, |selected| {
            assert!(!receipt.matches_invocation(selected));
        });
        let context = crate::model::ingress::static_context_for("tcl8.4");
        let unknown = InvocationDialect {
            tcl_version: None,
            ..InvocationDialect::for_version(TclVersion::V8_4)
        };
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            InvocationWords::structured(InvocationWord::Literal("source"), &encoding)
                .with_dialect(unknown),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let selected = resolution.resolved().unwrap();
        assert!(selected.authored_source_file_layout_abstention().is_none());
        assert!(!receipt.matches_invocation(&selected));
    }

    #[test]
    fn navigation_candidates_require_unanimous_or_selected_file_layout() {
        let filename = [InvocationWord::Dynamic];
        assert!(matches!(
            path_candidate(InvocationArguments::structured(&filename)),
            SourceFileSelection::Selected(SourceFileOperands { path_at: 0, .. })
        ));
        let encoding = [
            InvocationWord::Literal("-encoding"),
            InvocationWord::Dynamic,
            InvocationWord::Dynamic,
        ];
        let arguments = InvocationArguments::structured(&encoding);
        assert_eq!(path_candidate(arguments), SourceFileSelection::Unknown);
        assert!(matches!(
            path_candidate(
                arguments.with_dialect(InvocationDialect::for_version(TclVersion::V8_6))
            ),
            SourceFileSelection::Selected(SourceFileOperands { path_at: 2, .. })
        ));
        assert_eq!(
            path_candidate(
                arguments.with_dialect(InvocationDialect::for_version(TclVersion::V8_4))
            ),
            SourceFileSelection::Invalid
        );
        assert_eq!(
            path_candidate(InvocationArguments::structured(&[InvocationWord::Expanded])),
            SourceFileSelection::Unknown
        );
    }

    #[test]
    fn unversioned_tcl_navigation_uses_only_shared_file_layouts() {
        let unversioned = InvocationDialect {
            tcl_version: None,
            ..InvocationDialect::for_version(TclVersion::V8_6)
        };
        assert!(unversioned.source_file_grammar().is_none());
        let path = [InvocationWord::Literal("child.tcl")];
        let arguments = InvocationArguments::structured(&path).with_dialect(unversioned);
        assert!(matches!(
            path_candidate(arguments),
            SourceFileSelection::Selected(SourceFileOperands { path_at: 0, .. })
        ));
        assert_eq!(select(arguments), SourceFileSelection::Unknown);
        let encoding = [
            InvocationWord::Literal("-encoding"),
            InvocationWord::Literal("utf-8"),
            InvocationWord::Literal("child.tcl"),
        ];
        assert_eq!(
            path_candidate(InvocationArguments::structured(&encoding).with_dialect(unversioned)),
            SourceFileSelection::Unknown
        );
        let foreign = InvocationDialect {
            native_family: None,
            ..unversioned
        };
        assert_eq!(
            path_candidate(InvocationArguments::structured(&path).with_dialect(foreign)),
            SourceFileSelection::Unknown
        );
    }

    #[test]
    fn source_load_transitions_retain_only_selected_original_path_ordinals() {
        // Implementation contract: naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        for version in TclVersion::ALL {
            let words = [InvocationWord::Dynamic];
            let transitions = SOURCE_TRANSITIONS.resolve(
                InvocationArguments::structured(&words)
                    .with_dialect(InvocationDialect::for_version(version)),
            );
            let [fact] = transitions.facts() else {
                panic!("{version:?}: {transitions:?}");
            };
            let crate::StateTransition::Package(
                crate::model::binding::PackageTransition::SourceLoad { path },
            ) = &fact.transition
            else {
                panic!("{version:?}: {transitions:?}");
            };
            assert_eq!(path.argument_index(), Some(0));
            assert!(path.literal().is_none());
            let words = [
                InvocationWord::Literal("-encoding"),
                InvocationWord::Dynamic,
                InvocationWord::Dynamic,
            ];
            let transitions = SOURCE_TRANSITIONS.resolve(
                InvocationArguments::structured(&words)
                    .with_dialect(InvocationDialect::for_version(version)),
            );
            assert_eq!(
                transitions.facts().len(),
                usize::from(version != TclVersion::V8_4)
            );
            if let Some(fact) = transitions.facts().first() {
                let crate::StateTransition::Package(
                    crate::model::binding::PackageTransition::SourceLoad { path },
                ) = &fact.transition
                else {
                    panic!("{version:?}: {transitions:?}");
                };
                assert_eq!(path.argument_index(), Some(2));
            }
        }
        for words in [
            &[InvocationWord::Expanded][..],
            &[InvocationWord::Dynamic, InvocationWord::Dynamic][..],
        ] {
            assert_eq!(
                SOURCE_TRANSITIONS
                    .resolve(InvocationArguments::structured(words))
                    .facts()
                    .len(),
                0
            );
        }
    }

    #[test]
    fn native_file_layout_preserves_filename_dash_and_selected_options() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = InvocationDialect::for_version(version);
            let words = [InvocationWord::Literal("-encoding")];
            assert_eq!(
                select(
                    InvocationWords::structured(InvocationWord::Literal("source"), &words)
                        .with_dialect(dialect)
                        .arguments()
                ),
                SourceFileSelection::Selected(SourceFileOperands {
                    path_at: 0,
                    encoding_at: None,
                    no_package: false
                })
            );
            let words = [
                InvocationWord::Literal("-encoding"),
                InvocationWord::Literal("utf-8"),
                InvocationWord::Dynamic,
            ];
            let actual = select(
                InvocationWords::structured(InvocationWord::Literal("source"), &words)
                    .with_dialect(dialect)
                    .arguments(),
            );
            assert_eq!(
                actual,
                if version == TclVersion::V8_4 {
                    SourceFileSelection::Invalid
                } else {
                    SourceFileSelection::Selected(SourceFileOperands {
                        path_at: 2,
                        encoding_at: Some(1),
                        no_package: false,
                    })
                }
            );
        }
    }
    #[test]
    fn file_return_boundary_preserves_c_levels_and_jim_outer_absorption() {
        use crate::completion::CompletionCode;
        use crate::completion_route::{InvocationCompletionRoute as Route, ReturnCompletionRoute};
        let c = InvocationDialect::for_version(TclVersion::V8_6);
        let jim_profile = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let jim = InvocationDialect::of_profile(&jim_profile);
        let pending = Route::Return(ReturnCompletionRoute {
            eventual_code: CompletionCode::Error,
            remaining_level: 2,
        });
        assert_eq!(
            completion_route(c, pending),
            Route::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::Error,
                remaining_level: 1,
            })
        );
        assert_eq!(
            completion_route(jim, pending),
            Route::Tcl(CompletionCode::Ok)
        );
        for code in [
            CompletionCode::Break,
            CompletionCode::Continue,
            CompletionCode::Error,
        ] {
            for dialect in [c, jim] {
                assert_eq!(
                    completion_route(dialect, Route::Tcl(code)),
                    Route::Tcl(code)
                );
            }
        }
    }
}
