// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native source-file argument selection, independent of file availability.

use crate::{InvocationArguments, InvocationDialect};
use tcl_dialect::{TclVersion, model::Family};

/// Exact runtime grammar for the selected native source handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFileGrammar {
    /// C Tcl 8.4 and Jim: one filename, even when its bytes begin with a dash.
    Filename,
    /// C Tcl 8.5/8.6: one filename or exact -encoding/name/filename.
    Encoding,
    /// C Tcl 9: encoding form or exact -nopkg/filename, never combined.
    EncodingOrNoPackage,
}

impl SourceFileGrammar {
    /// Usage following the actual invoked command word.
    #[must_use]
    pub const fn usage_suffix(self) -> &'static str {
        match self {
            Self::Filename => "fileName",
            Self::Encoding => "?-encoding name? fileName",
            Self::EncodingOrNoPackage => "?-encoding name? fileName | -nopkg fileName",
        }
    }

    /// Select reached original object operands using the handler's `CString`
    /// selector comparisons. The filename itself retains its counted bytes.
    #[must_use]
    pub fn select_original(self, count: usize, first: Option<&[u8]>) -> SourceFileSelection {
        let selected = |path_at, encoding_at, no_package| {
            SourceFileSelection::Selected(SourceFileOperands {
                path_at,
                encoding_at,
                no_package,
            })
        };
        if count == 1 {
            return selected(0, None, false);
        }
        if self == Self::Filename {
            return SourceFileSelection::Invalid;
        }
        let Some(first) = first else {
            return SourceFileSelection::Unknown;
        };
        let first = tcl_core_types::c_string_extent(first);
        if count == 3 && first == b"-encoding" {
            return selected(2, Some(1), false);
        }
        if count == 2 && self == Self::EncodingOrNoPackage && first == b"-nopkg" {
            return selected(1, None, true);
        }
        SourceFileSelection::Invalid
    }

    /// Native wrong-argument usage, without adding unsupported option forms.
    #[must_use]
    pub const fn synopsis(self) -> &'static str {
        match self {
            Self::Filename => "source fileName",
            Self::Encoding => "source ?-encoding name? fileName",
            Self::EncodingOrNoPackage => {
                "source ?-encoding name? fileName | source -nopkg fileName"
            }
        }
    }
}

/// Selected actual file read operands, before any filesystem operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFileOperands {
    /// Effective argv index of the filename.
    pub path_at: usize,
    /// Explicit encoding operand, absent for the selected native default.
    pub encoding_at: Option<usize>,
    /// Whether native package initialisation is omitted.
    pub no_package: bool,
}

/// Selection preserves invalid grammar independently of unknown values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileSelection {
    /// A definite positional layout; filename contents may still be dynamic.
    Selected(SourceFileOperands),
    /// The native handler rejects argv before reading or executing a file.
    Invalid,
    /// Unknown native policy, expanded argc, or an unresolved selector.
    Unknown,
}

impl InvocationDialect {
    /// Audited native file grammar, without an assistance-profile fallback.
    #[must_use]
    pub fn source_file_grammar(self) -> Option<SourceFileGrammar> {
        match self.family()? {
            Family::Tcl => match self.tcl_version? {
                TclVersion::V8_4 => Some(SourceFileGrammar::Filename),
                TclVersion::V8_5 | TclVersion::V8_6 => Some(SourceFileGrammar::Encoding),
                TclVersion::V9_0 | TclVersion::V9_1 => Some(SourceFileGrammar::EncodingOrNoPackage),
            },
            Family::Jim
                if self.core_point.is_some_and(|point| {
                    point.release() == tcl_dialect::model::Release::JIM_0_84
                }) =>
            {
                Some(SourceFileGrammar::Filename)
            }
            _ => None,
        }
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
    if arguments.dialect().is_some() {
        return select(arguments);
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
