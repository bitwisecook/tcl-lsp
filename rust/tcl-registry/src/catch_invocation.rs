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

/// Purpose of the selected Jim completion-mask model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JimCaptureMaskPurpose {
    /// Observed original64 source grammar; this supplies no Native execution.
    MeasuredSource64,
    /// Existing supported Rust64 recipe, declining selectors outside its width.
    /// This does not prove portable signed-C shifts or bind an original build.
    SupportedRust64Bounded,
}

/// Leading selectors shared by Jim Catch and Try, before either command's tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JimCapturePrefix {
    /// Actual script ordinal in post-command argv.
    pub script_at: usize,
    /// Selected completion filter; its purpose remains independently explicit.
    pub ignored_codes: u64,
    /// Source observation and bounded native execution are distinct recipes.
    pub purpose: JimCaptureMaskPurpose,
}

impl JimCapturePrefix {
    /// Codes outside the mask's actual extent cannot bypass completion capture.
    #[must_use]
    pub fn ignores(self, code: i64) -> bool {
        mask_ignores(self.ignored_codes, code)
    }
}

fn mask_ignores(mask: u64, code: i64) -> bool {
    u32::try_from(code)
        .ok()
        .filter(|code| *code < 64)
        .is_some_and(|code| mask & (1_u64 << code) != 0)
}

/// Prefix validity without treating a missing value or protocol as a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimCapturePrefixSelection {
    /// Complete leading scan and an actual script operand.
    Valid(JimCapturePrefix),
    /// Original input fails before executing a body.
    Invalid,
    /// Source values or a selected numeric grammar remain unresolved.
    Unknown,
}

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
        mask_ignores(self.ignored_codes, code)
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
    let Some(length) = arguments.exact_argv_len() else {
        return CatchInvocationSelection::Unknown;
    };
    select_catch_invocation_by(length, dialect, |index| {
        Ok::<_, std::convert::Infallible>(
            arguments
                .native_bytes_at(index)
                .or_else(|| arguments.literal_at(index).map(str::as_bytes))
                .map(<[u8]>::to_vec),
        )
    })
    .unwrap_or_else(|error| match error {})
}

/// Select a runtime original-object capture, requesting String bytes only at
/// the positions inspected by the actual handler's leading-option scan.
/// Output names and a script following `--` remain uninspected original objects.
/// This source-grammar view retains measured64 wrapping without granting Native
/// execution; physical adapters use [`select_native_jim_catch_invocation`].
///
/// # Errors
/// The independently supplied original String getter can fail or refuse access.
pub fn select_original_catch_invocation<E>(
    length: usize,
    dialect: InvocationDialect,
    mut inspect: impl FnMut(usize) -> Result<Vec<u8>, E>,
) -> Result<CatchInvocationSelection, E> {
    select_catch_invocation_by(length, dialect, |index| inspect(index).map(Some))
}

fn select_catch_invocation_by<E>(
    length: usize,
    dialect: InvocationDialect,
    inspect: impl FnMut(usize) -> Result<Option<Vec<u8>>, E>,
) -> Result<CatchInvocationSelection, E> {
    use CatchInvocationSelection::{Invalid, Unknown, Valid};
    if let Some(arity) = dialect.catch_positional_arity() {
        if !arity.accepts(u16::try_from(length).unwrap_or(u16::MAX)) {
            return Ok(Invalid);
        }
        return Ok(Valid(CatchInvocation {
            script_at: 0,
            result_var_at: (length >= 2).then_some(1),
            options_var_at: (length >= 3).then_some(2),
            ignored_codes: 0,
            exit_policy: CatchExitPolicy::ProcessOutsideCapture,
        }));
    }
    if dialect.family() != Some(tcl_dialect::model::Family::Jim) {
        return Ok(Unknown);
    }
    let prefix = match select_jim_capture_prefix_by(
        length,
        dialect,
        JimCaptureMaskPurpose::MeasuredSource64,
        inspect,
    )? {
        Ok(JimCapturePrefixSelection::Valid(prefix)) => prefix,
        Ok(JimCapturePrefixSelection::Invalid) => return Ok(Invalid),
        Ok(JimCapturePrefixSelection::Unknown) | Err(_) => return Ok(Unknown),
    };
    Ok(Valid(jim_catch_tail(prefix, length)))
}

fn jim_catch_tail(prefix: JimCapturePrefix, length: usize) -> CatchInvocation {
    CatchInvocation {
        script_at: prefix.script_at,
        result_var_at: (prefix.script_at + 1 < length).then_some(prefix.script_at + 1),
        options_var_at: (prefix.script_at + 2 < length).then_some(prefix.script_at + 2),
        ignored_codes: prefix.ignored_codes,
        exit_policy: CatchExitPolicy::NativeCode(crate::completion::CompletionCode::Other(6)),
    }
}

/// Scan actual original Jim operands using the current supported Rust64 mask.
/// The caller supplies its original String getter and retains its execution
/// owner. Selectors >=64 refuse; source-observed wrapping grants no execution.
///
/// # Errors
/// Original getter failures and unavailable native recipe purposes are terminal.
pub fn select_native_jim_capture_prefix<E: From<tcl_syntax::value::ValueError>>(
    length: usize,
    dialect: InvocationDialect,
    mut inspect: impl FnMut(usize) -> Result<Vec<u8>, E>,
) -> Result<JimCapturePrefixSelection, E> {
    let selected = select_jim_capture_prefix_by(
        length,
        dialect,
        JimCaptureMaskPurpose::SupportedRust64Bounded,
        |index| inspect(index).map(Some),
    )?;
    match selected.map_err(E::from)? {
        JimCapturePrefixSelection::Unknown => {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim capture prefix protocol",
            )
            .into())
        }
        selected => Ok(selected),
    }
}

/// Native Catch tail positions from the same bounded actual leading scan.
/// The returned layout supplies neither original headers nor execution authority.
///
/// # Errors
/// The actual String getter and bounded native prefix recipe may refuse.
pub fn select_native_jim_catch_invocation<E: From<tcl_syntax::value::ValueError>>(
    length: usize,
    dialect: InvocationDialect,
    inspect: impl FnMut(usize) -> Result<Vec<u8>, E>,
) -> Result<CatchInvocationSelection, E> {
    Ok(
        match select_native_jim_capture_prefix(length, dialect, inspect)? {
            JimCapturePrefixSelection::Valid(prefix) => {
                CatchInvocationSelection::Valid(jim_catch_tail(prefix, length))
            }
            JimCapturePrefixSelection::Invalid => CatchInvocationSelection::Invalid,
            JimCapturePrefixSelection::Unknown => {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Jim catch prefix protocol",
                )
                .into());
            }
        },
    )
}

fn select_jim_capture_prefix_by<E>(
    length: usize,
    dialect: InvocationDialect,
    purpose: JimCaptureMaskPurpose,
    mut inspect: impl FnMut(usize) -> Result<Option<Vec<u8>>, E>,
) -> Result<Result<JimCapturePrefixSelection, tcl_syntax::value::ValueError>, E> {
    use JimCapturePrefixSelection::{Invalid, Unknown, Valid};
    if dialect.family() != Some(tcl_dialect::model::Family::Jim) {
        return Ok(Ok(Unknown));
    }
    let Some(scalar) = dialect.native_scalar_getter_protocol() else {
        return Ok(Ok(Unknown));
    };
    if length == 0 {
        return Ok(Ok(Invalid));
    }
    let mut script_at = 0;
    let mut ignored_codes = (1_u64 << 5) | (1_u64 << 6) | (1_u64 << 7);
    while script_at < length - 1 {
        let Some(word) = inspect(script_at)? else {
            return Ok(Ok(Unknown));
        };
        let word = tcl_core_types::c_string_extent(&word);
        if word == b"--" {
            script_at += 1;
            break;
        }
        let Some(flag) = word.strip_prefix(b"-") else {
            break;
        };
        let (ignore, flag) = flag
            .strip_prefix(b"no")
            .map_or((false, flag), |flag| (true, flag));
        let Some(decimal) = scalar.jim_decimal_wide_probe(flag) else {
            return Ok(Ok(Unknown));
        };
        let code = match decimal {
            Ok(code) if code >= 0 => u64::try_from(code).ok(),
            _ => tcl_cmd_core::return_options::ReturnOptionsProtocol::Jim084
                .named_completion_code(flag)
                .and_then(|code| u64::try_from(code).ok()),
        };
        let Some(code) = code else {
            return Ok(Ok(Invalid));
        };
        if purpose == JimCaptureMaskPurpose::SupportedRust64Bounded && code >= 64 {
            return Ok(Err(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Jim capture mask shift outside supported Rust64 recipe",
                ),
            ));
        }
        // The source model retains the measured original build's wrapping. The
        // bounded native recipe never reaches this operation outside its width.
        let bit = 1_u64 << (code % 64);
        if ignore {
            ignored_codes |= bit;
        } else {
            ignored_codes &= !bit;
        }
        script_at += 1;
    }
    Ok(Ok(Valid(JimCapturePrefix {
        script_at,
        ignored_codes,
        purpose,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_jim_prefix_cells_keep_source_masks_separate_from_bounded_native_recipes() {
        let cases = include_str!(
            "../../../runtime/rust/tests/data/native_jim_try_leading_switches/cases-20261015.tsv"
        );
        let observations = include_str!(
            "../../../runtime/rust/tests/data/native_jim_try_leading_switches/observations-20261015.tsv"
        );
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let decode = |hex: &str| {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        for row in observations.lines().filter(|row| row.starts_with("OBS|")) {
            let fields = row.split('|').collect::<Vec<_>>();
            let case: usize = fields[1].parse().unwrap();
            let input = cases
                .lines()
                .nth(case)
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            let words = [
                decode(input[1]),
                b"body_probe".to_vec(),
                b"on".to_vec(),
                fields[2].as_bytes().to_vec(),
                b"message options".to_vec(),
                b"handler_probe".to_vec(),
            ];
            let source = select_original_catch_invocation(words.len(), jim, |index| {
                Ok::<_, std::convert::Infallible>(words[index].clone())
            })
            .unwrap();
            let mut inspected = Vec::new();
            let native = select_native_jim_capture_prefix(words.len(), jim, |index| {
                inspected.push(index);
                Ok::<_, tcl_syntax::value::ValueError>(words[index].clone())
            });
            match source {
                CatchInvocationSelection::Valid(source) => {
                    assert_eq!(fields[4], "1", "{row}");
                    assert_eq!(
                        source.ignores(fields[2].parse().unwrap()),
                        fields[5] == "0",
                        "{row}"
                    );
                    // The original build executes these cells; the bounded
                    // Native API conservatively declines its unbound shifts.
                    if matches!(case, 2 | 3 | 4 | 5 | 15) {
                        assert!(native.is_err(), "{row}");
                    } else {
                        let JimCapturePrefixSelection::Valid(native) = native.unwrap() else {
                            panic!("{row}")
                        };
                        assert_eq!(
                            native.purpose,
                            JimCaptureMaskPurpose::SupportedRust64Bounded
                        );
                        assert_eq!(native.script_at, source.script_at);
                        assert_eq!(native.ignored_codes, source.ignored_codes);
                    }
                }
                CatchInvocationSelection::Invalid => {
                    assert_eq!(fields[4], "0", "{row}");
                    assert_eq!(native.unwrap(), JimCapturePrefixSelection::Invalid);
                }
                CatchInvocationSelection::Unknown => {
                    panic!("captured literal input unresolved: {row}")
                }
            }
            if case == 20 {
                assert_eq!(inspected, [0, 1]);
            } else if case == 21 || case == 22 {
                assert_eq!(inspected, [0]);
            }
        }
    }

    #[test]
    fn jim_options_inspect_only_the_leading_original_bytes() {
        let dialect = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let words = [
            crate::InvocationWord::KnownBytes(b"-noerror\0tail"),
            crate::InvocationWord::KnownBytes(b"--"),
            crate::InvocationWord::KnownBytes(b"body\xff"),
            crate::InvocationWord::KnownBytes(b"r\xff"),
            crate::InvocationWord::KnownBytes(b"o\xed\xa0\x80"),
        ];
        let CatchInvocationSelection::Valid(selected) =
            select_catch_invocation(InvocationArguments::structured(&words), dialect)
        else {
            panic!("original output names are not option selectors");
        };
        assert_eq!(selected.script_at, 2);
        assert_eq!(selected.result_var_at, Some(3));
        assert_eq!(selected.options_var_at, Some(4));
        assert!(selected.ignores(1));
        let mut inspected = Vec::new();
        let inspected_selection = select_original_catch_invocation(5, dialect, |index| {
            inspected.push(index);
            Ok::<_, ()>(words[index].native_bytes().unwrap().to_vec())
        })
        .unwrap();
        assert_eq!(
            inspected_selection,
            CatchInvocationSelection::Valid(selected)
        );
        assert_eq!(inspected, [0, 1]);
        let encoded_zero = [
            crate::InvocationWord::KnownBytes(b"-noerror\xc0\x80tail"),
            crate::InvocationWord::KnownBytes(b"body"),
        ];
        assert_eq!(
            select_catch_invocation(InvocationArguments::structured(&encoded_zero), dialect),
            CatchInvocationSelection::Invalid
        );
    }

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
