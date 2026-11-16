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

//! The policy types an execution engine selects: indices, variable lookup and storage, parameters, the package protocol, interpreters and `Ternary`.

use super::TclVersion;

/// Native `concat` semantics, including dependence on object representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConcatPolicy {
    /// The selected C release owns pure/canonical List eligibility, original
    /// header copying, hash-head fallback and string-result construction.
    Tcl(TclVersion),
    /// Jim can return list concatenation or string concatenation depending on
    /// the operands' internal representations. String values alone are insufficient.
    JimRepresentationSensitive,
}

/// Container index syntax, independent of numeral spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexGrammar {
    /// Tcl 8.4: integers and abbreviated `end` with a subtractive integer.
    Tcl84,
    /// Tcl 8.5 and later: one additive or subtractive integer connector.
    TclModern,
    /// Jim: safe integer expressions; variable and script requests are rejected.
    Jim,
}

/// Integer conversion performed by a native container index parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexIntegerWidth {
    /// C Tcl 8 accepts signed and unsigned 32-bit integers, then wraps.
    Tcl32,
    /// C Tcl 9 uses signed pointer-size integers on the supported 64-bit host.
    Tcl64,
    /// Jim encodes absolute and end-relative indices in a signed 32-bit slot.
    Jim32,
}

/// Grammar of the regular-expression commands' start offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegexStartGrammar {
    /// Tcl 8.4 accepts an integer, without end-relative or index arithmetic.
    Integer,
    /// Later C Tcl and Jim use their native container-index parser.
    Index,
}

/// The complete native policy needed to parse a container index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexSyntax {
    /// Integer spelling accepted by the selected release.
    pub numbers: crate::NumberSyntax,
    /// Native index expression protocol.
    pub grammar: IndexGrammar,
    /// Native conversion width, independently of numeral spelling.
    pub width: IndexIntegerWidth,
    /// Whether `e` and `en` abbreviate `end` (C Tcl 8 only).
    pub end_abbreviations: bool,
}

impl IndexSyntax {
    /// Whether one `lindex` operand is a list of successive indices.
    ///
    /// C Tcl splits the operand into a path; Jim evaluates it as one index.
    #[must_use]
    pub const fn lindex_argument_is_path(self) -> bool {
        !matches!(self.grammar, IndexGrammar::Jim)
    }

    /// Regex offset grammar, measured independently of container indices.
    #[must_use]
    pub const fn regex_start_grammar(self) -> RegexStartGrammar {
        if matches!(self.grammar, IndexGrammar::Tcl84) {
            RegexStartGrammar::Integer
        } else {
            RegexStartGrammar::Index
        }
    }

    /// Container index policy for a selected C Tcl release.
    #[must_use]
    pub const fn for_version(version: TclVersion) -> Self {
        Self {
            numbers: version.number_syntax(),
            grammar: if matches!(version, TclVersion::V8_4) {
                IndexGrammar::Tcl84
            } else {
                IndexGrammar::TclModern
            },
            end_abbreviations: !matches!(version, TclVersion::V9_0 | TclVersion::V9_1),
            width: if matches!(version, TclVersion::V9_0 | TclVersion::V9_1) {
                IndexIntegerWidth::Tcl64
            } else {
                IndexIntegerWidth::Tcl32
            },
        }
    }
}

/// Whether `lset` may replace the element immediately after the current list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListSetBounds {
    /// An index must designate an existing element, as in Tcl 8.4/8.5 and Jim.
    ExistingElement,
    /// The list length is also accepted and appends, as in Tcl 8.6 and later.
    AppendAtEnd,
}

impl ListSetBounds {
    /// Check the resolved integer index without changing index-word grammar.
    #[must_use]
    pub fn accepts(self, index: i64, length: usize) -> bool {
        usize::try_from(index)
            .is_ok_and(|index| index < length || (self == Self::AppendAtEnd && index == length))
    }
}

/// Variable namespace/activation lookup policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableLookupPolicy {
    /// Tcl qualified names address namespace cells.
    Tcl,
    /// Jim treats relative qualified spellings as activation-local names.
    Jim,
}

impl VariableLookupPolicy {
    /// Root storage selected by this engine's variable lookup policy.
    #[must_use]
    pub const fn container_model(self) -> VariableContainerModel {
        match self {
            Self::Tcl => VariableContainerModel::DistinctArray,
            Self::Jim => VariableContainerModel::DictionaryValue,
        }
    }

    /// Alias binding selected by this engine's variable lookup policy.
    #[must_use]
    pub const fn link_binding(self) -> VariableLinkBinding {
        match self {
            Self::Tcl => VariableLinkBinding::StableCell,
            Self::Jim => VariableLinkBinding::SelectedFrameName,
        }
    }
}

/// Whether an array is a distinct variable kind or a dictionary value.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableContainerModel {
    /// C Tcl scalar and array roots are distinct; neither overwrites the other.
    #[default]
    DistinctArray,
    /// Jim array elements select entries of an ordinary dictionary-valued cell.
    DictionaryValue,
}

/// What a variable alias retains when its selected target is rebound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableLinkBinding {
    /// C Tcl retains the selected variable cell, including detached elements.
    StableCell,
    /// Jim retains a name and selected logical frame level, resolving on each
    /// access. Captured alias wrappers may reach a later activation at that
    /// level; they do not pin the original physical frame.
    SelectedFrameName,
}

/// Formal parameter parsing and activation protocol of the selected engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterGrammar {
    /// C Tcl scalar formals, trailing variadic args and positional defaults.
    Tcl,
    /// Jim name formals, middle variadic args and caller-reference parameters.
    Jim,
}

/// Compiler registration installed on a procedure's command header. Body
/// compilation is a separate phase and does not determine this registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeProcedureHeaderCompilation {
    /// The command header is dispatched generically.
    Absent,
    /// C Tcl installs its argument-evaluating no-op compiler.
    NoOp,
    /// Definition bytes, representation or native engine facts are unresolved.
    Unknown,
}

impl ParameterGrammar {
    /// Whether a byte-empty native body validates arity then returns without
    /// creating a frame or applying formal bindings.
    #[must_use]
    pub const fn skips_empty_body_activation(self) -> bool {
        matches!(self, Self::Jim)
    }
}

/// Package discovery and version negotiation protocol of the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageProtocol {
    /// C Tcl's versioned package database and deferred loaders.
    Tcl,
    /// Jim's direct file lookup; optional version words are ignored.
    Jim,
}

/// A direct package file type, distinct from deferred C Tcl index scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectPackageFileKind {
    /// A loadable native module.
    Native,
    /// A Tcl script evaluated directly in the global frame.
    Script,
}

/// One candidate in the engine's direct package search order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPackageFile {
    /// Portable path spelling passed to the host filesystem.
    pub path: String,
    /// Which loader must evaluate this file.
    pub kind: DirectPackageFileKind,
}

/// A direct package candidate retaining the engine's filesystem bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPackageFileBytes {
    /// Native path bytes, without Unicode replacement.
    pub path: Vec<u8>,
    /// Loader selected by the package protocol.
    pub kind: DirectPackageFileKind,
}

impl PackageProtocol {
    /// A C Tcl loader's non-error control completion becomes a package error.
    #[must_use]
    pub fn ifneeded_completion_error_bytes(
        self,
        package: &[u8],
        version: &[u8],
        code: i64,
    ) -> Option<Vec<u8>> {
        (self == Self::Tcl && code != 0 && code != 1).then(|| {
            let mut message = b"attempt to provide package ".to_vec();
            message.extend_from_slice(package);
            message.push(b' ');
            message.extend_from_slice(version);
            message.extend_from_slice(format!(" failed: bad return code: {code}").as_bytes());
            message
        })
    }

    /// Checked Unicode convenience for native loader-completion presentation.
    #[must_use]
    pub fn ifneeded_completion_error(
        self,
        package: &str,
        version: &str,
        code: i64,
    ) -> Option<String> {
        self.ifneeded_completion_error_bytes(package.as_bytes(), version.as_bytes(), code)
            .map(|bytes| String::from_utf8(bytes).expect("Unicode operands and ASCII presentation"))
    }

    /// Native direct-file candidates. Jim checks its module before its script
    /// and consumes directory/package operands through its `CString` formatter.
    #[must_use]
    pub fn direct_files_bytes(
        self,
        directory: &[u8],
        package: &[u8],
        native_modules: bool,
    ) -> Vec<DirectPackageFileBytes> {
        if self != Self::Jim {
            return Vec::new();
        }
        let directory = tcl_core_types::c_string_extent(directory);
        let package = tcl_core_types::c_string_extent(package);
        let path = |extension: &[u8], omit_dot: bool| {
            let mut bytes = Vec::new();
            if !omit_dot || directory != b"." {
                bytes.extend_from_slice(directory);
                bytes.push(b'/');
            }
            bytes.extend_from_slice(package);
            bytes.extend_from_slice(extension);
            bytes
        };
        let mut files = Vec::new();
        if native_modules {
            files.push(DirectPackageFileBytes {
                path: path(b".so", false),
                kind: DirectPackageFileKind::Native,
            });
        }
        files.push(DirectPackageFileBytes {
            path: path(b".tcl", true),
            kind: DirectPackageFileKind::Script,
        });
        files
    }

    /// Unicode convenience sharing the native direct-search ordering.
    #[must_use]
    pub fn direct_files(
        self,
        directory: &str,
        package: &str,
        native_modules: bool,
    ) -> Vec<DirectPackageFile> {
        self.direct_files_bytes(directory.as_bytes(), package.as_bytes(), native_modules)
            .into_iter()
            .map(|file| DirectPackageFile {
                path: String::from_utf8(file.path)
                    .expect("Unicode path operands with ASCII separators"),
                kind: file.kind,
            })
            .collect()
    }
}

/// How namespace imports preserve their source binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamespaceImportBinding {
    /// C Tcl imports retain the source command token through renames.
    CommandToken,
    /// Jim imports dispatch the original source name at invocation time.
    SourceName,
}

/// Integer conversion performed by the process-exit command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessExitConversion {
    /// Tcl 8.x rejects magnitudes exceeding an unsigned 32-bit integer.
    Narrow32,
    /// Tcl 9 accepts a whole integer before host exit-code truncation.
    WholeInteger,
    /// Current Jim saturates an out-of-wide integer before host truncation.
    SaturatingWide,
}

/// Child interpreter creation and handle dispatch grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpreterProtocol {
    /// C Tcl's `interp create` and `interp alias` ensemble.
    Tcl,
    /// Jim's zero-argument factory and returned eval/delete/alias handle.
    JimHandles,
}

impl InterpreterProtocol {
    /// Subcommands implemented by Jim interpreter handles.
    pub const JIM_HANDLE_SUBCOMMANDS: &'static [&'static str] = &["eval", "delete", "alias"];
}

/// How a frame-crossing command locates an optional level operand.
/// Value parsing remains the dialect's independent numeral grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameLevelPresence {
    /// Consume a first word beginning with an ASCII digit or `#`.
    DigitOrHash,
    /// Consume a digit/`#` word or a non-negative parsed integer.
    DigitOrNonNegativeInteger,
    /// Consume a complete parsed integer, or any `#` word.
    IntegerOrHash,
    /// Consume one level word exactly when the argument count is odd.
    ArgumentParity,
}

/// A three-valued behaviour policy, so a non-Tcl profile (`f5-bigip`) and
/// the permissive unknown-dialect fallback are **inert** — "no opinion" —
/// rather than silently defaulted to one of the real behaviours
/// (dialect-profile-model.md §11.1). Consumers short-circuit on
/// [`Ternary::Inert`] instead of guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ternary {
    /// The behaviour applies (e.g. leading-zero integers read as octal).
    Yes,
    /// The behaviour does not apply (e.g. Tcl 9.x dropped bare-leading-zero
    /// octal, TIP 114/472).
    No,
    /// No opinion — the profile is not a Tcl runtime, or is the permissive
    /// unknown-dialect sink. Validators and const-folders abstain.
    Inert,
}

impl From<bool> for Ternary {
    /// A decided boolean as a [`Ternary`] — the inverse of
    /// [`Ternary::as_bool`], never producing [`Ternary::Inert`].
    fn from(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }
}

impl Ternary {
    /// The policy as an `Option<bool>`: `Inert` is `None`, so callers that
    /// already model "undecided" as `None` (the expr const-folder's octal
    /// input) consume it directly.
    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::Yes => Some(true),
            Self::No => Some(false),
            Self::Inert => None,
        }
    }
}
