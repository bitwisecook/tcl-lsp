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

//! Registry-owned completion routes, including pending procedure unwinding.

use crate::completion::CompletionCode;

/// A pending return, retaining the native eventual code and remaining level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReturnCompletionRoute {
    /// Code released when the remaining return level reaches its boundary.
    pub eventual_code: CompletionCode,
    /// Native pending unwind level. Zero still denotes a pending return when
    /// the released code itself is `return`, as in Jim's return protocol.
    pub remaining_level: u64,
}

/// Completion of one proved invocation after its arguments have evaluated.
/// Unknown is explicit and includes both normal and abrupt continuations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationCompletionRoute {
    /// A concrete Tcl completion without pending return options.
    Tcl(CompletionCode),
    /// Finite authored code alternatives without pending return options.
    TclAlternatives(&'static [CompletionCode]),
    /// A pending return whose eventual code and level have been established.
    Return(ReturnCompletionRoute),
    /// Native scheduled tail invocation, retaining its catch-visible code.
    /// The target completion is released by the enclosing procedure boundary.
    Tailcall {
        /// C Tcl uses return (2); Jim uses eval (7).
        code: CompletionCode,
    },
    /// Tail scheduling or validation error; neither completes this script normally.
    TailcallOrError {
        /// Native scheduling code, independently of validation error (1).
        code: CompletionCode,
    },
    /// Immediate interpreter-process exit, outside catch/cleanup handling.
    ProcessExit,
    /// A dynamic return-code option either returns or fails its validation.
    ReturnOrError,
    /// Dynamic exit status either terminates or fails its validation.
    ExitOrError,
    /// Jim exit either emits catch-filterable native code 6 or validation error.
    CatchableExitOrError,
    /// Runtime-dependent normal/abrupt completion with unresolved residue.
    Unknown,
    /// An unresolved catchable non-OK completion; normal continuation is absent.
    UnknownAbrupt,
}

impl InvocationCompletionRoute {
    /// Whether execution can continue with the next statement in this script.
    #[must_use]
    pub const fn normal_possible(self) -> bool {
        match self {
            Self::TclAlternatives(codes) => {
                let mut index = 0;
                while index < codes.len() {
                    if matches!(codes[index], CompletionCode::Ok) {
                        return true;
                    }
                    index += 1;
                }
                false
            }
            _ => matches!(self, Self::Tcl(CompletionCode::Ok) | Self::Unknown),
        }
    }

    /// Whether execution can leave the current script before its next statement.
    #[must_use]
    pub const fn abrupt_possible(self) -> bool {
        match self {
            Self::TclAlternatives(codes) => {
                let mut index = 0;
                while index < codes.len() {
                    if !matches!(codes[index], CompletionCode::Ok) {
                        return true;
                    }
                    index += 1;
                }
                false
            }
            _ => !matches!(self, Self::Tcl(CompletionCode::Ok)),
        }
    }

    /// Concrete catch-visible code, excluding process exit and dynamic unions.
    #[must_use]
    pub const fn immediate_code(self) -> Option<CompletionCode> {
        match self {
            Self::Tcl(code) | Self::Tailcall { code } => Some(code),
            Self::TclAlternatives(codes) => {
                if codes.len() == 1 {
                    Some(codes[0])
                } else {
                    None
                }
            }
            Self::Return(_) => Some(CompletionCode::Return),
            Self::ProcessExit
            | Self::TailcallOrError { .. }
            | Self::ReturnOrError
            | Self::ExitOrError
            | Self::CatchableExitOrError
            | Self::Unknown
            | Self::UnknownAbrupt => None,
        }
    }

    /// Expand authored finite domains before attaching execution states.
    /// Each abrupt alternative then retains its own exact code at a boundary.
    #[must_use]
    pub fn alternatives(self) -> Vec<Self> {
        match self {
            Self::TclAlternatives(codes) => codes.iter().copied().map(Self::Tcl).collect(),
            Self::TailcallOrError { code } => {
                vec![Self::Tailcall { code }, Self::Tcl(CompletionCode::Error)]
            }
            route => vec![route],
        }
    }

    /// Cross one procedure return boundary. Namespace/eval activations must
    /// retain the route unchanged; allocating a frame alone does not unwind.
    ///
    /// ```
    /// use tcl_registry::completion::CompletionCode;
    /// use tcl_registry::completion_route::{InvocationCompletionRoute, ReturnCompletionRoute};
    /// let route = InvocationCompletionRoute::Return(ReturnCompletionRoute {
    ///     eventual_code: CompletionCode::Ok, remaining_level: 2,
    /// });
    /// assert!(!route.through_procedure_boundary().normal_possible());
    /// assert!(route.through_procedure_boundary().through_procedure_boundary().normal_possible());
    /// ```
    #[must_use]
    pub const fn through_procedure_boundary(self) -> Self {
        match self {
            Self::Return(route) if route.remaining_level > 1 => {
                Self::Return(ReturnCompletionRoute {
                    remaining_level: route.remaining_level - 1,
                    ..route
                })
            }
            Self::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::Return,
                ..
            }) => {
                // Releasing a native return code starts another pending return
                // with reset options. Jim retains code=return until this point;
                // C Tcl normalises it before the invocation produces its route.
                Self::Return(ReturnCompletionRoute {
                    eventual_code: CompletionCode::Ok,
                    remaining_level: 0,
                })
            }
            Self::Return(route) => Self::Tcl(route.eventual_code),
            Self::TclAlternatives(codes) => {
                let mut index = 0;
                while index < codes.len() {
                    if matches!(codes[index], CompletionCode::Return) {
                        return Self::Unknown;
                    }
                    index += 1;
                }
                self
            }
            Self::Tcl(CompletionCode::Return)
            | Self::ReturnOrError
            | Self::Tailcall { .. }
            | Self::TailcallOrError { .. } => Self::Unknown,
            route => route,
        }
    }

    /// Apply native procedure completion semantics after reducing a pending
    /// return. C rejects raw break/continue escaping a procedure; Jim propagates
    /// them. Codes released by a configured return keep their own identity.
    #[must_use]
    pub fn through_procedure_boundary_in(self, dialect: Option<crate::InvocationDialect>) -> Self {
        if let Self::TclAlternatives(codes) = self
            && codes
                .iter()
                .any(|code| matches!(code, CompletionCode::Break | CompletionCode::Continue))
            && dialect.map(crate::InvocationDialect::procedure_completion_policy)
                != Some(ProcedureCompletionPolicy::Jim)
        {
            return if self.normal_possible() {
                Self::Unknown
            } else {
                Self::UnknownAbrupt
            };
        }
        if matches!(
            self,
            Self::Tcl(CompletionCode::Break | CompletionCode::Continue)
        ) {
            return match dialect.map(crate::InvocationDialect::procedure_completion_policy) {
                Some(ProcedureCompletionPolicy::Tcl) => Self::Tcl(CompletionCode::Error),
                Some(ProcedureCompletionPolicy::Jim) => self,
                Some(ProcedureCompletionPolicy::Unknown) | None => Self::UnknownAbrupt,
            };
        }
        self.through_procedure_boundary()
    }
}

/// Native handling of raw loop completions escaping a procedure activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcedureCompletionPolicy {
    /// C Tcl converts raw break/continue to an outside-loop error.
    Tcl,
    /// Jim propagates raw break/continue to the caller.
    Jim,
    /// The selected runtime does not establish either protocol.
    Unknown,
}

impl crate::InvocationDialect {
    /// The actual engine's procedure boundary, independent of host surfaces.
    #[must_use]
    pub fn procedure_completion_policy(self) -> ProcedureCompletionPolicy {
        match self.family() {
            Some(tcl_dialect::model::Family::Jim) => ProcedureCompletionPolicy::Jim,
            Some(tcl_dialect::model::Family::Tcl) => ProcedureCompletionPolicy::Tcl,
            _ if self.tcl_version.is_some() => ProcedureCompletionPolicy::Tcl,
            _ => ProcedureCompletionPolicy::Unknown,
        }
    }
}

/// Native return command grammar. This is independent of host command surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReturnInvocationGrammar {
    LegacyTcl,
    OptionsTcl,
    Jim,
    Unknown,
}

impl ReturnInvocationGrammar {
    pub(crate) fn completion_code_policy(
        self,
        numbers: tcl_syntax::number::Numbers,
    ) -> crate::completion::CompletionCodePolicy {
        use crate::completion::CompletionCodePolicy as Policy;
        match self {
            Self::Jim => Policy::Jim,
            Self::LegacyTcl => Policy::Tcl8,
            Self::OptionsTcl => match numbers.syntax() {
                Some(tcl_dialect::NumberSyntax::Tcl90) => Policy::Tcl9,
                Some(_) => Policy::Tcl8,
                None => Policy::Unknown,
            },
            Self::Unknown => Policy::Unknown,
        }
    }
}

/// Mutable-state envelope of a native return after argv evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnStateEffect {
    /// Only result/return-option storage changes; no error globals are written.
    ResultAndCompletion,
    /// Error or unresolved options may materialise observable error globals.
    MayMaterialiseError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandRegistry, InvocationArguments, InvocationDialect};

    fn route(words: &[&str], dialect: InvocationDialect) -> InvocationCompletionRoute {
        CommandRegistry::build_default()
            .invocation_completion_route(
                "return",
                InvocationArguments::literals(words).with_dialect(dialect),
                dialect.authoring_query(),
            )
            .expect("core return descriptor")
    }

    #[test]
    fn return_state_envelope_preserves_error_observers_and_dynamic_results() {
        use crate::InvocationWord::{Dynamic, Literal};
        use crate::registry::native_return_state_effect;
        use crate::world_effect::WorldStateDomain;
        // Implementation contract: naming.registry.return-identity-effect-coverage
        // docs/design/analysis/name-resolution-proofs/return-identity-effect-coverage.md
        let registry = CommandRegistry::build_default();
        let selected = registry.get("return").expect("core return spec");
        let descriptor = selected
            .world_effects
            .expect("authored native return storage");
        let transitions = selected
            .state_transitions
            .expect("independent native return identity effects");
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            for words in [
                vec![Dynamic],
                vec![Literal("-code"), Literal("ok"), Dynamic],
                vec![
                    Literal("-code"),
                    Literal("ok"),
                    Literal("-errorinfo"),
                    Literal("unused"),
                    Dynamic,
                ],
            ] {
                let args = InvocationArguments::structured(&words).with_dialect(dialect);
                assert_eq!(
                    native_return_state_effect(args),
                    ReturnStateEffect::ResultAndCompletion
                );
                let effects = descriptor.resolve(args);
                assert!(!effects.requires_world_barrier());
                assert!(transitions.resolve(args).facts().is_empty());
                assert_eq!(
                    effects
                        .accesses()
                        .iter()
                        .map(|access| access.domain)
                        .collect::<Vec<_>>(),
                    vec![
                        WorldStateDomain::InterpreterResult,
                        WorldStateDomain::CompletionState
                    ]
                );
            }
            for words in [
                vec![Literal("-code"), Literal("error"), Dynamic],
                vec![Literal("-code"), Dynamic, Dynamic],
                vec![Literal("-code"), Literal("invalid"), Dynamic],
                vec![Literal("-code"), Literal("break"), Dynamic],
                vec![Literal("-level"), Dynamic, Dynamic],
            ] {
                let args = InvocationArguments::structured(&words).with_dialect(dialect);
                assert_eq!(
                    native_return_state_effect(args),
                    ReturnStateEffect::MayMaterialiseError
                );
                assert!(descriptor.resolve(args).requires_world_barrier());
                let unresolved = transitions.resolve(args);
                assert!(
                    crate::StateTransitionDomain::ALL
                        .iter()
                        .all(|domain| unresolved.widens(*domain))
                );
                assert!(unresolved.facts().iter().all(|fact| {
                    fact.commit == crate::StateTransitionCommit::MayCommitBeforeAbruptCompletion
                }));
            }
        }
        assert_eq!(
            native_return_state_effect(InvocationArguments::literals(&["value"])),
            ReturnStateEffect::MayMaterialiseError
        );
        let unavailable = transitions.resolve(InvocationArguments::literals(&["value"]));
        assert!(
            crate::StateTransitionDomain::ALL
                .iter()
                .all(|domain| unavailable.widens(*domain))
        );
    }

    #[test]
    fn authored_native_error_domains_stay_abrupt_at_procedure_boundaries() {
        let registry = CommandRegistry::build_default();
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (name, arguments) in [
            ("set", vec!["x", "1"]),
            ("incr", vec!["x"]),
            ("global", vec!["x"]),
            ("upvar", vec!["0", "x", "alias"]),
            ("proc", vec!["f", "", "return -code 7"]),
            ("rename", vec!["f", "g"]),
        ] {
            let route = registry
                .invocation_completion_route(
                    name,
                    InvocationArguments::literals(&arguments).with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .expect("authored native descriptor");
            assert!(route.normal_possible(), "{name}");
            assert!(route.abrupt_possible(), "{name}");
            assert_eq!(
                route.alternatives(),
                vec![
                    InvocationCompletionRoute::Tcl(CompletionCode::Ok),
                    InvocationCompletionRoute::Tcl(CompletionCode::Error),
                ],
                "{name}"
            );
            let error = route.alternatives()[1];
            assert!(
                !error
                    .through_procedure_boundary_in(Some(dialect))
                    .normal_possible()
            );
        }
    }

    #[test]
    fn tail_scheduling_uses_the_actual_native_code_and_activation() {
        use crate::{CommandRegistry, InvocationArguments, VariableAliasFrame};
        let registry = CommandRegistry::build_default();
        for (dialect, code, empty) in [
            (
                crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
                CompletionCode::Return,
                false,
            ),
            (
                crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                    tcl_dialect::model::Release::JIM_0_84,
                )),
                CompletionCode::Other(7),
                true,
            ),
        ] {
            let args = InvocationArguments::literals(&["target"]).with_dialect(dialect);
            let route = registry
                .invocation_completion_route("tailcall", args, None)
                .unwrap();
            assert_eq!(route, InvocationCompletionRoute::TailcallOrError { code });
            assert!(!route.normal_possible());
            assert_eq!(route.alternatives()[0].immediate_code(), Some(code));
            assert!(
                route.alternatives()[0]
                    .through_procedure_boundary_in(Some(dialect))
                    .normal_possible()
            );
            assert!(
                !route.alternatives()[1]
                    .through_procedure_boundary_in(Some(dialect))
                    .normal_possible()
            );
            assert_eq!(
                registry.invocation_completion_route_in_frame(
                    "tailcall",
                    args,
                    None,
                    VariableAliasFrame::Global
                ),
                Some(InvocationCompletionRoute::Tcl(CompletionCode::Error))
            );
            let route = registry
                .invocation_completion_route_in_frame(
                    "tailcall",
                    InvocationArguments::literals(&[]).with_dialect(dialect),
                    None,
                    VariableAliasFrame::Procedure,
                )
                .unwrap();
            assert_eq!(route.normal_possible(), empty);
        }
    }

    #[test]
    fn return_routes_retain_levels_and_legacy_grammar() {
        for release in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(release);
            let pending = route(&["-code", "return", "value"], dialect);
            assert_eq!(
                pending,
                InvocationCompletionRoute::Return(ReturnCompletionRoute {
                    eventual_code: CompletionCode::Ok,
                    remaining_level: 2,
                })
            );
            assert!(
                !pending
                    .through_procedure_boundary_in(Some(dialect))
                    .normal_possible()
            );
            assert!(
                pending
                    .through_procedure_boundary_in(Some(dialect))
                    .through_procedure_boundary_in(Some(dialect))
                    .normal_possible()
            );
            let modern = release >= tcl_dialect::TclVersion::V8_5;
            assert_eq!(
                route(&["-level", "0", "value"], dialect),
                InvocationCompletionRoute::Tcl(if modern {
                    CompletionCode::Ok
                } else {
                    CompletionCode::Error
                })
            );
            assert_eq!(
                route(&["-foo", "bar", "value"], dialect).immediate_code(),
                Some(if modern {
                    CompletionCode::Return
                } else {
                    CompletionCode::Error
                })
            );
        }
    }

    #[test]
    fn jim_keeps_return_code_until_its_procedure_boundary() {
        let dialect = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        for level in ["0", "1", "2"] {
            let pending = route(&["-code", "return", "-level", level, "value"], dialect);
            assert_eq!(
                pending,
                InvocationCompletionRoute::Return(ReturnCompletionRoute {
                    eventual_code: CompletionCode::Return,
                    remaining_level: level.parse().unwrap(),
                })
            );
        }
        assert_eq!(
            route(&["-foo", "bar", "value"], dialect),
            InvocationCompletionRoute::Tcl(CompletionCode::Error)
        );
        let pending = route(&["-code", "return", "-level", "0", "value"], dialect);
        assert_eq!(
            pending.through_procedure_boundary_in(Some(dialect)),
            InvocationCompletionRoute::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::Ok,
                remaining_level: 0,
            })
        );
        assert!(
            pending
                .through_procedure_boundary_in(Some(dialect))
                .through_procedure_boundary_in(Some(dialect))
                .normal_possible()
        );
    }

    #[test]
    fn dynamic_level_or_option_names_preserve_normal_continuation() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = CommandRegistry::build_default();
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for words in [
            vec![Literal("-level"), Dynamic, Literal("payload")],
            vec![
                Literal("-code"),
                Dynamic,
                Dynamic,
                Literal("0"),
                Literal("payload"),
            ],
        ] {
            let route = registry
                .invocation_completion_route(
                    "return",
                    InvocationArguments::structured(&words).with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .unwrap();
            assert!(
                route.normal_possible(),
                "dynamic options may select level zero: {words:?}"
            );
        }
        let words = [Literal("-code"), Dynamic, Literal("payload")];
        assert_eq!(
            registry.invocation_completion_route(
                "return",
                InvocationArguments::structured(&words).with_dialect(dialect),
                dialect.authoring_query()
            ),
            Some(InvocationCompletionRoute::ReturnOrError)
        );
    }

    #[test]
    fn procedure_boundaries_distinguish_raw_loop_codes_from_released_returns() {
        let c = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        for code in [CompletionCode::Break, CompletionCode::Continue] {
            let raw = InvocationCompletionRoute::Tcl(code);
            assert_eq!(
                raw.through_procedure_boundary_in(Some(c)),
                InvocationCompletionRoute::Tcl(CompletionCode::Error)
            );
            assert_eq!(raw.through_procedure_boundary_in(Some(jim)), raw);
            assert_eq!(
                raw.through_procedure_boundary_in(None),
                InvocationCompletionRoute::UnknownAbrupt
            );
            let configured = InvocationCompletionRoute::Return(ReturnCompletionRoute {
                eventual_code: code,
                remaining_level: 1,
            });
            assert_eq!(configured.through_procedure_boundary_in(Some(c)), raw);
            assert_eq!(configured.through_procedure_boundary_in(Some(jim)), raw);
        }
    }
}

#[cfg(test)]
mod numeric_tests {
    use super::*;
    use crate::{CommandRegistry, InvocationArguments, InvocationDialect};

    #[test]
    fn native_completion_code_conversion_keeps_release_and_engine_differences() {
        let registry = CommandRegistry::build_default();
        for release in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(release);
            let words = ["-code", "-2147483649", "payload"];
            let route = registry
                .invocation_completion_route(
                    "return",
                    InvocationArguments::literals(&words).with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .unwrap();
            let expected = if release < tcl_dialect::TclVersion::V9_0 {
                InvocationCompletionRoute::Return(ReturnCompletionRoute {
                    eventual_code: CompletionCode::Other(i32::MAX),
                    remaining_level: 1,
                })
            } else {
                InvocationCompletionRoute::Tcl(CompletionCode::Error)
            };
            assert_eq!(route, expected, "{release:?}");
        }
        let dialect = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let words = ["-code", "4294967296", "payload"];
        assert_eq!(
            registry.invocation_completion_route(
                "return",
                InvocationArguments::literals(&words).with_dialect(dialect),
                dialect.authoring_query()
            ),
            Some(InvocationCompletionRoute::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::Ok,
                remaining_level: 1
            }))
        );
        let words = ["-code", "signal", "payload"];
        assert_eq!(
            registry.invocation_completion_route(
                "return",
                InvocationArguments::literals(&words).with_dialect(dialect),
                dialect.authoring_query()
            ),
            Some(InvocationCompletionRoute::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::Other(5),
                remaining_level: 1
            }))
        );
    }
    #[test]
    fn jim_static_definition_retains_normal_completion_under_contextual_arity() {
        let context = crate::model::ingress::static_context_for("jim");
        let registry = context.commands();
        let dialect = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let args = crate::InvocationArguments::literals(&["p", "", "{x OLD}", "return $x"])
            .with_dialect(dialect);
        let route = registry
            .invocation_completion_route("proc", args, None)
            .unwrap();
        assert!(route.normal_possible(), "{route:?}");
    }
}
