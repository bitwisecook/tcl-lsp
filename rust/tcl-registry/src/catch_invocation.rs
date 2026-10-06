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

//! Dialect-aware capture-command argv grammar, independent of execution.

use crate::{InvocationArguments, InvocationDialect};

/// Native process termination relative to a catch activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatchExitPolicy {
    /// C Tcl terminates the interpreter process outside completion capture.
    ProcessOutsideCapture,
    /// The selected engine represents exit as a filterable native code.
    NativeCode(crate::completion::CompletionCode),
}

/// Observable ordering of the variable captures after a caught completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatchOutputOrder {
    /// The generic handler, Jim, and the C Tcl 8.5 compiler store the result first.
    ResultThenOptions,
    /// C Tcl 8.6 and later inline compilers store return options first.
    OptionsThenResult,
    /// The retained compiler protocol does not prove an output ordering.
    Unknown,
}

impl CatchOutputOrder {
    /// Actual lookup sequence, excluding captures the invocation did not request.
    #[must_use]
    pub fn indices(self, invocation: CatchInvocation) -> Option<[Option<usize>; 2]> {
        match self {
            Self::ResultThenOptions => Some([invocation.result_var_at, invocation.options_var_at]),
            Self::OptionsThenResult => Some([invocation.options_var_at, invocation.result_var_at]),
            Self::Unknown => None,
        }
    }
}

/// Selected source positions and completion filtering for a valid capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatchInvocation {
    /// Script position in post-command argv.
    pub script_at: usize,
    /// Optional result-variable position.
    pub result_var_at: Option<usize>,
    /// Optional return-options-variable position.
    pub options_var_at: Option<usize>,
    /// Jim completion codes passed through instead of captured.
    pub ignored_codes: u64,
    /// Actual engine's process-exit protocol, separate from filter masks.
    pub exit_policy: CatchExitPolicy,
}

impl CatchInvocation {
    /// Select output lookup order from the actual native compiler protocol.
    /// Each lookup occurs after the preceding output's observable write traces.
    /// C Tcl 8.5's inline compiler explicitly stores the result before options;
    /// the 8.6+ compiler reverses that order while the generic handler does not.
    #[must_use]
    pub fn output_order(
        self,
        dialect: InvocationDialect,
        compilation: crate::native_compilation::NativeCompilationSelection,
    ) -> CatchOutputOrder {
        use crate::native_compilation::NativeCompilationSelection as Selection;
        use tcl_dialect::{TclVersion, model::Family};
        if compilation == Selection::CompileError {
            return CatchOutputOrder::Unknown;
        }
        if self.result_var_at.is_none() || self.options_var_at.is_none() {
            return CatchOutputOrder::ResultThenOptions;
        }
        if dialect.native_family == Some(Family::Jim)
            || compilation == Selection::Generic
            || dialect
                .tcl_version
                .is_some_and(|version| version < TclVersion::V8_6)
        {
            return CatchOutputOrder::ResultThenOptions;
        }
        if dialect.native_family == Some(Family::Tcl)
            && dialect
                .tcl_version
                .is_some_and(|version| version >= TclVersion::V8_6)
            && matches!(compilation, Selection::Inline { .. })
        {
            return CatchOutputOrder::OptionsThenResult;
        }
        CatchOutputOrder::Unknown
    }

    /// Completion capture of the authored C Tcl lifecycle phases. Source
    /// operand positions are absent because the phase already evaluated.
    pub const CAPTURE_TCL_PHASE: Self = Self {
        script_at: 0,
        result_var_at: None,
        options_var_at: None,
        ignored_codes: 0,
        exit_policy: CatchExitPolicy::ProcessOutsideCapture,
    };

    /// Select captured and propagated alternatives for one completion route.
    /// Pending return options survive propagation; C process exit bypasses all
    /// filters, while Jim's native exit code participates in its authored mask.
    #[must_use]
    pub fn route(
        self,
        route: crate::completion_route::InvocationCompletionRoute,
    ) -> CatchCompletionRoutes {
        use crate::completion::CompletionCode as Code;
        use crate::completion_route::InvocationCompletionRoute as Route;
        let mut result = CatchCompletionRoutes {
            captured: false,
            propagated: Vec::new(),
        };
        match route {
            Route::TailcallOrError { .. } => {
                for alternative in route.alternatives() {
                    let routed = self.route(alternative);
                    result.captured |= routed.captured;
                    result.propagated.extend(routed.propagated);
                }
            }
            Route::ProcessExit => result.propagated.push(route),
            Route::ExitOrError => {
                result.captured = !self.ignores(1);
                result.propagated.push(Route::ProcessExit);
                if self.ignores(1) {
                    result.propagated.push(Route::Tcl(Code::Error));
                }
            }
            Route::CatchableExitOrError => {
                for code in [Code::Error, Code::Other(6)] {
                    if self.ignores(code.as_int()) {
                        result.propagated.push(Route::Tcl(code));
                    } else {
                        result.captured = true;
                    }
                }
            }
            Route::ReturnOrError => {
                result.captured = !self.ignores(1) || !self.ignores(2);
                if self.ignores(1) || self.ignores(2) {
                    result.propagated.push(route);
                }
            }
            Route::Unknown | Route::UnknownAbrupt => {
                result.captured = true;
                if self.ignored_codes != 0 {
                    result.propagated.push(route);
                }
                if route == Route::Unknown
                    && self.exit_policy == CatchExitPolicy::ProcessOutsideCapture
                {
                    // An unresolved script includes exit independently of its
                    // catchable Tcl codes. A mask cannot intercept this edge.
                    result.propagated.push(Route::ProcessExit);
                }
            }
            Route::TclAlternatives(codes) => {
                for code in codes {
                    if self.ignores(code.as_int()) {
                        result.propagated.push(Route::Tcl(*code));
                    } else {
                        result.captured = true;
                    }
                }
            }
            Route::Tcl(code) | Route::Tailcall { code } => {
                if self.ignores(code.as_int()) {
                    result.propagated.push(route);
                } else {
                    result.captured = true;
                }
            }
            Route::Return(_) => {
                if self.ignores(2) {
                    result.propagated.push(route);
                } else {
                    result.captured = true;
                }
            }
        }
        result
    }

    /// Whether this completion bypasses the capture epilogue.
    #[must_use]
    pub fn ignores(self, code: i64) -> bool {
        u32::try_from(code)
            .ok()
            .filter(|code| *code < 64)
            .is_some_and(|code| self.ignored_codes & (1_u64 << code) != 0)
    }
}

/// Alternatives selected by a native catch completion filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchCompletionRoutes {
    /// Some catchable completion reaches the capture epilogue normally.
    pub captured: bool,
    /// Completions bypassing the epilogue with their return options retained.
    pub propagated: Vec<crate::completion_route::InvocationCompletionRoute>,
}

/// Argument validity without assuming dynamic argv words are literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatchInvocationSelection {
    /// A valid capture layout and completion filter.
    Valid(CatchInvocation),
    /// Arguments fail before script execution.
    Invalid,
    /// Runtime values or an unresolved dialect can select different layouts.
    Unknown,
}

/// Select the actual capture grammar of a supplied interpreter snapshot.
#[must_use]
pub fn select_catch_invocation(
    arguments: InvocationArguments<'_>,
    dialect: InvocationDialect,
) -> CatchInvocationSelection {
    use CatchInvocationSelection::{Invalid, Unknown, Valid};
    let Some(length) = arguments.exact_argv_len() else {
        return Unknown;
    };
    if let Some(arity) = dialect.catch_positional_arity() {
        if !arity.accepts(u16::try_from(length).unwrap_or(u16::MAX)) {
            return Invalid;
        }
        return Valid(CatchInvocation {
            script_at: 0,
            result_var_at: (length >= 2).then_some(1),
            options_var_at: (length >= 3).then_some(2),
            ignored_codes: 0,
            exit_policy: CatchExitPolicy::ProcessOutsideCapture,
        });
    }
    if dialect.family() != Some(tcl_dialect::model::Family::Jim) {
        return Unknown;
    }
    if length == 0 {
        return Invalid;
    }
    let mut script_at = 0;
    let mut ignored_codes = (1 << 5) | (1 << 6) | (1 << 7);
    while script_at < length - 1 {
        let Some(word) = arguments.literal_at(script_at) else {
            return Unknown;
        };
        if word == "--" {
            script_at += 1;
            break;
        }
        let Some(flag) = word.strip_prefix('-') else {
            break;
        };
        let (ignore, flag) = flag
            .strip_prefix("no")
            .map_or((false, flag), |flag| (true, flag));
        let numeric = flag.trim().parse::<u64>().ok();
        let code = numeric.or_else(|| {
            [
                "ok", "error", "return", "break", "continue", "signal", "exit", "eval",
            ]
            .iter()
            .position(|name| *name == flag)
            .map(|code| code as u64)
        });
        let Some(code) = code else {
            return Invalid;
        };
        // The supported Jim build uses a 64-bit completion mask. Its native
        // shifts wrap at that width; the oracle includes -no64 and -no128.
        let bit = 1_u64 << (code % 64);
        if ignore {
            ignored_codes |= bit;
        } else {
            ignored_codes &= !bit;
        }
        script_at += 1;
    }
    Valid(CatchInvocation {
        script_at,
        result_var_at: (script_at + 1 < length).then_some(script_at + 1),
        options_var_at: (script_at + 2 < length).then_some(script_at + 2),
        ignored_codes,
        exit_policy: CatchExitPolicy::NativeCode(crate::completion::CompletionCode::Other(6)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_script_preserves_the_actual_process_exit_protocol() {
        use crate::completion::CompletionCode as Code;
        use crate::completion_route::InvocationCompletionRoute as Route;
        let c = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let words = ["body"];
        for (dialect, wanted_exit) in [(c, true), (jim, false)] {
            let CatchInvocationSelection::Valid(selected) =
                select_catch_invocation(InvocationArguments::literals(&words), dialect)
            else {
                panic!("proved catch grammar");
            };
            let routed = selected.route(Route::Unknown);
            assert!(routed.captured);
            assert_eq!(routed.propagated.contains(&Route::ProcessExit), wanted_exit);
            let finite = selected.route(Route::TclAlternatives(&[Code::Ok, Code::Error]));
            assert!(finite.captured);
            assert!(finite.propagated.is_empty());
        }
        let words = ["-exit", "body"];
        let CatchInvocationSelection::Valid(selected) =
            select_catch_invocation(InvocationArguments::literals(&words), jim)
        else {
            panic!("Jim exit capture grammar");
        };
        let routed = selected.route(Route::CatchableExitOrError);
        assert!(routed.captured);
        assert!(routed.propagated.is_empty());
    }

    #[test]
    fn catches_follow_core_version_and_jim_option_layout() {
        let words = ["body", "result", "options", "ignored"];
        for release in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(release);
            assert_eq!(
                select_catch_invocation(InvocationArguments::literals(&words), dialect),
                CatchInvocationSelection::Invalid
            );
            let selected =
                select_catch_invocation(InvocationArguments::literals(&words[..3]), dialect);
            assert_eq!(
                matches!(selected, CatchInvocationSelection::Valid(_)),
                release >= tcl_dialect::TclVersion::V8_5
            );
        }
        let dialect = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let words = [
            "-noerror", "-exit", "--", "body", "result", "options", "ignored",
        ];
        let CatchInvocationSelection::Valid(selected) =
            select_catch_invocation(InvocationArguments::literals(&words), dialect)
        else {
            panic!("valid Jim capture");
        };
        assert_eq!(selected.script_at, 3);
        assert_eq!(selected.result_var_at, Some(4));
        assert_eq!(selected.options_var_at, Some(5));
        assert!(selected.ignores(1));
        assert!(!selected.ignores(6));
    }
}
#[test]
fn output_order_preserves_native_compiler_and_handler_differences() {
    use crate::native_compilation::{NativeCompilationGuard, NativeCompilationSelection};
    let invocation = CatchInvocation {
        result_var_at: Some(1),
        options_var_at: Some(2),
        ..CatchInvocation::CAPTURE_TCL_PHASE
    };
    let inline = NativeCompilationSelection::Inline {
        operation: crate::SemanticOperationId::StructuredLowering(
            crate::hooks::LoweringHookId::Catch,
        ),
        guard: NativeCompilationGuard::BeforeArguments,
    };
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let dialect = InvocationDialect::for_version(version);
        assert_eq!(
            invocation.output_order(dialect, NativeCompilationSelection::Generic),
            CatchOutputOrder::ResultThenOptions
        );
        let expected = if version == tcl_dialect::TclVersion::V8_5 {
            CatchOutputOrder::ResultThenOptions
        } else {
            CatchOutputOrder::OptionsThenResult
        };
        assert_eq!(invocation.output_order(dialect, inline), expected);
        assert_eq!(
            expected.indices(invocation),
            Some(if version == tcl_dialect::TclVersion::V8_5 {
                [Some(1), Some(2)]
            } else {
                [Some(2), Some(1)]
            })
        );
    }
    assert_eq!(
        invocation.output_order(
            InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            NativeCompilationSelection::Unknown,
        ),
        CatchOutputOrder::Unknown
    );
}
