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

//! Native source-file handler grammar and operand selection, independent
//! of file availability.

use crate::InvocationDialect;
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
            Self::EncodingOrNoPackage => "?-encoding encoding? fileName",
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

    /// The exact option table the native handler matches the first word
    /// against for an argument count it accepts but [`Self::select_original`]
    /// rejects: a miss there is `bad option`, not `wrong # args`.
    #[must_use]
    pub const fn option_table(self, count: usize) -> Option<&'static [&'static str]> {
        match (self, count) {
            (Self::Encoding | Self::EncodingOrNoPackage, 3) => Some(&["-encoding"]),
            (Self::EncodingOrNoPackage, 2) => Some(&["-nopkg"]),
            _ => None,
        }
    }

    /// Native wrong-argument usage, without adding unsupported option forms.
    #[must_use]
    pub const fn synopsis(self) -> &'static str {
        match self {
            Self::Filename => "source fileName",
            Self::Encoding => "source ?-encoding name? fileName",
            Self::EncodingOrNoPackage => "source ?-encoding encoding? fileName",
        }
    }
}

/// Explicit host installation of an authored file-source syntax recipe.
/// This supplies no physical getter, filesystem or compiled-body permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalSourceFileProvider {
    /// Logical F5 Tcl8.4 filename-only source handling.
    Tcl84CoreSimulation,
}

/// Pure selected file syntax and channel default, without operation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalSourceFileRecipe {
    grammar: SourceFileGrammar,
    protocol: tcl_cmd_core::encoding::NativeSourceFileProtocol,
}

impl LogicalSourceFileRecipe {
    /// Selected logical argument grammar.
    #[must_use]
    pub const fn grammar(self) -> SourceFileGrammar {
        self.grammar
    }
    /// Selected logical channel default, without conversion or file authority.
    #[must_use]
    pub const fn protocol(self) -> tcl_cmd_core::encoding::NativeSourceFileProtocol {
        self.protocol
    }
}

impl InvocationDialect {
    /// Describe an explicitly supplied logical file recipe independently of native selection.
    #[must_use]
    pub fn logical_source_file_recipe(
        self,
        provider: LogicalSourceFileProvider,
    ) -> Option<LogicalSourceFileRecipe> {
        match provider {
            LogicalSourceFileProvider::Tcl84CoreSimulation => {
                self.authored_f5_tcl84_core()?;
                Some(LogicalSourceFileRecipe {
                    grammar: SourceFileGrammar::Filename,
                    protocol: tcl_cmd_core::encoding::NativeSourceFileProtocol::CSystem(
                        TclVersion::V8_4,
                    ),
                })
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
