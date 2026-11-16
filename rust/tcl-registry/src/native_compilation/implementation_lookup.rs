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

//! The command implementation a compiler hook depends on: hook presence, the lookup it performs and its prerequisites.

use super::introspection_grammars::{
    INFO_COROUTINE_IMPLEMENTATION, INFO_EXISTS_IMPLEMENTATION, INFO_LEVEL_IMPLEMENTATION,
    NAMESPACE_CODE_IMPLEMENTATION, NAMESPACE_CURRENT_IMPLEMENTATION,
    NAMESPACE_ORIGIN_IMPLEMENTATION, NAMESPACE_WHICH_IMPLEMENTATION,
};
use super::string_grammars::{STRING_EQUAL_IMPLEMENTATION, STRING_LENGTH_IMPLEMENTATION};
use super::{NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec};
use crate::InvocationDialect;
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;

/// Original private command used by an ensemble compiler.
/// Admission of this slot is an entry-table fact; the descriptor cannot prove
/// that a live interpreter still contains its original implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCompilerImplementationLookup {
    /// Original ensemble whose dispatch mapping must select this member slot.
    pub ensemble: &'static str,
    /// Literal ensemble member in the original mapping.
    pub member: &'static str,
    /// Fully qualified original implementation slot.
    pub slot: &'static str,
    /// Catalogue command carrying the implementation's semantic descriptors.
    pub command: &'static str,
    /// Semantic argument prefix, excluding the catalogue command head.
    pub prepended: &'static [&'static str],
}

impl NativeCompilationSpec {
    /// Original implementation prerequisites for this compiler purpose.
    /// An empty vector proves no additional slot is required by the authored
    /// descriptor; `None` retains an unavailable or unresolved protocol.
    /// Live command identity and every original map remain caller obligations.
    #[must_use]
    pub fn implementation_prerequisites(
        self,
        dialect: InvocationDialect,
    ) -> Option<Vec<NativeCompilerImplementationLookup>> {
        // naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        if let NativeCompilationGrammar::WithImplementationPath {
            compiler,
            lookups,
            implementation_from,
            monolithic_no_hook_before,
        } = self.grammar
        {
            if dialect.family() != Some(Family::Tcl)
                || lookups.is_empty()
                || compiler.operation != self.operation
                || self.body != compiler.body
                // A NULL compileProc does not select a compiled body. Its
                // runtime implementation can still enter a script object or
                // direct evaluation; both descriptors retain that same policy.
                || !(compiler.body == NativeBodyCompilation::Inherit
                    || compiler.grammar == NativeCompilationGrammar::NoHook)
                || !matches!(
                    compiler.grammar,
                    NativeCompilationGrammar::NoHook
                        | NativeCompilationGrammar::HookFrom(_)
                        | NativeCompilationGrammar::Regsub
                        | NativeCompilationGrammar::ClockReading(_)
                        | NativeCompilationGrammar::NamespaceWhich
                        | NativeCompilationGrammar::TclOoHelper(_)
                        | NativeCompilationGrammar::StringTrim { .. }
                        | NativeCompilationGrammar::StringMatch(_)
                        | NativeCompilationGrammar::NamedEnsembleInvocation { .. }
                )
                || !lookups.windows(2).all(|edges| edges[0].slot == edges[1].ensemble)
                || compiler
                    .implementation_lookup(dialect)
                    .is_some_and(|terminal| lookups.last() != Some(&terminal))
            {
                return None;
            }
            let version = dialect.tcl_version?;
            if version < implementation_from {
                return (monolithic_no_hook_before
                    && compiler.grammar == NativeCompilationGrammar::NoHook)
                    .then(Vec::new);
            }
            return Some(lookups.to_vec());
        }
        (self.grammar != NativeCompilationGrammar::Unresolved)
            .then(|| self.implementation_lookup(dialect).into_iter().collect())
    }

    /// Actual compiler-hook registration for the authored native engine point.
    /// Missing engine/version evidence retains uncertainty; handler semantics
    /// and the presence of a descriptor do not establish hook registration.
    #[must_use]
    pub fn compiler_hook_presence(self, dialect: InvocationDialect) -> Option<bool> {
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            self.implementation_prerequisites(dialect)?;
            return compiler.compiler_hook_presence(dialect);
        }
        if self.grammar == NativeCompilationGrammar::NoHook || dialect.family() == Some(Family::Jim)
        {
            return Some(false);
        }
        if dialect.family() != Some(Family::Tcl) {
            return None;
        }
        let version = dialect.tcl_version?;
        if matches!(self.grammar, NativeCompilationGrammar::StringInsert(_)) {
            return (version >= TclVersion::V9_0).then_some(true);
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::StringMap(_)
                | NativeCompilationGrammar::StringRange(_)
                | NativeCompilationGrammar::StringReplace(_)
        ) {
            // C85 registers these members with a NULL compileProc.
            return match version {
                TclVersion::V8_5 => Some(false),
                version => (version >= TclVersion::V8_6).then_some(true),
            };
        }
        Some(match self.grammar {
            NativeCompilationGrammar::NoHook => false,
            NativeCompilationGrammar::Regsub | NativeCompilationGrammar::ClockReading(_) => {
                version >= TclVersion::V8_6
            }
            NativeCompilationGrammar::MathOperator(operator) => version >= operator.first_version(),
            NativeCompilationGrammar::Dictionary { command, .. } => version >= command.hook_from(),
            NativeCompilationGrammar::HookFrom(first)
            | NativeCompilationGrammar::ProcedureHookFrom(first)
            | NativeCompilationGrammar::ArgumentListFrom(first)
            | NativeCompilationGrammar::ArgumentConcatFrom(first)
            | NativeCompilationGrammar::ArityFrom { first, .. }
            | NativeCompilationGrammar::NamedEnsembleInvocation {
                hook_from: first, ..
            } => version >= first,
            NativeCompilationGrammar::GlobalBindings
            | NativeCompilationGrammar::NamespaceVariableBindings
            | NativeCompilationGrammar::Upvar
            | NativeCompilationGrammar::InfoExists
            | NativeCompilationGrammar::ListAssignment
            | NativeCompilationGrammar::NamespaceLegacy
            | NativeCompilationGrammar::Switch => version >= TclVersion::V8_5,
            NativeCompilationGrammar::NamespaceUpvarBindings
            | NativeCompilationGrammar::NamespaceWhich
            | NativeCompilationGrammar::StringTrim { .. }
            | NativeCompilationGrammar::Array { .. }
            | NativeCompilationGrammar::Error
            | NativeCompilationGrammar::Throw
            | NativeCompilationGrammar::TclOoHelper(_)
            | NativeCompilationGrammar::CoroutineRelay
            | NativeCompilationGrammar::CoroutineYield
            | NativeCompilationGrammar::InfoCoroutine
            | NativeCompilationGrammar::LiteralUnset
            | NativeCompilationGrammar::InfoCommands
            | NativeCompilationGrammar::InfoLevel
            | NativeCompilationGrammar::Tailcall
            | NativeCompilationGrammar::ListRange
            | NativeCompilationGrammar::ListInsertion
            | NativeCompilationGrammar::ListReplacement
            | NativeCompilationGrammar::SubstitutionTemplate
            | NativeCompilationGrammar::NamespaceCurrent
            | NativeCompilationGrammar::NamespaceOrigin
            | NativeCompilationGrammar::NamespaceCode
            | NativeCompilationGrammar::FormatStringConcat
            | NativeCompilationGrammar::NamespaceString(_)
            | NativeCompilationGrammar::StringIsAscii(_) => version >= TclVersion::V8_6,
            NativeCompilationGrammar::Uplevel => version >= TclVersion::V9_1,
            NativeCompilationGrammar::Constant => version >= TclVersion::V9_0,
            NativeCompilationGrammar::StringOperand { operation, .. } => {
                version >= operation.hook_from()
            }
            _ => true,
        })
    }

    /// Additional original command identity required by this compiler grammar.
    /// C 8.5+ info is an ensemble whose exists compiler belongs to its private
    /// implementation command. C 8.4 and Jim have no such native dependency.
    #[must_use]
    pub fn implementation_lookup(
        self,
        dialect: InvocationDialect,
    ) -> Option<NativeCompilerImplementationLookup> {
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            self.implementation_prerequisites(dialect)?;
            return compiler.implementation_lookup(dialect);
        }
        if let NativeCompilationGrammar::NamespaceString(operation) = self.grammar {
            return implementation_version_matches(dialect, TclVersion::V8_6)
                .then_some(operation.implementation());
        }
        if let NativeCompilationGrammar::Array { lookup, .. } = self.grammar {
            return implementation_version_matches(dialect, TclVersion::V8_6).then_some(*lookup);
        }
        if self.grammar == NativeCompilationGrammar::NamespaceWhich {
            return implementation_version_matches(dialect, TclVersion::V8_6)
                .then_some(NAMESPACE_WHICH_IMPLEMENTATION);
        }
        if self.grammar == NativeCompilationGrammar::NamespaceUpvarBindings {
            return Self::namespace_upvar_implementation_lookup(dialect);
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::NamespaceCurrent
                | NativeCompilationGrammar::NamespaceOrigin
                | NativeCompilationGrammar::NamespaceCode
        ) {
            return implementation_version_matches(dialect, TclVersion::V8_6).then_some(match self
                .grammar
            {
                NativeCompilationGrammar::NamespaceOrigin => NAMESPACE_ORIGIN_IMPLEMENTATION,
                NativeCompilationGrammar::NamespaceCode => NAMESPACE_CODE_IMPLEMENTATION,
                _ => NAMESPACE_CURRENT_IMPLEMENTATION,
            });
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::StringIsAscii(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ) | NativeCompilationGrammar::StringEqual(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ) | NativeCompilationGrammar::StringLength(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ) | NativeCompilationGrammar::StringTrim {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                ..
            } | NativeCompilationGrammar::StringMatch(
                crate::native_string_compilation::NativeStringMatchScope::PublicMember,
            ) | NativeCompilationGrammar::StringMap(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | NativeCompilationGrammar::StringRange(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | NativeCompilationGrammar::StringReplace(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | NativeCompilationGrammar::StringInsert(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                ..
            }
        ) {
            return self.public_string_implementation_lookup(dialect);
        }
        if self.grammar == NativeCompilationGrammar::InfoCommands {
            return implementation_version_matches(dialect, TclVersion::V8_5).then_some(
                NativeCompilerImplementationLookup {
                    ensemble: "::info",
                    member: "commands",
                    slot: "::tcl::info::commands",
                    command: "info",
                    prepended: &["commands"],
                },
            );
        }
        if let NativeCompilationGrammar::NamedEnsembleInvocation {
            lookup,
            implementation_from,
            ..
        } = self.grammar
        {
            return implementation_version_matches(dialect, implementation_from).then_some(*lookup);
        }
        if let NativeCompilationGrammar::Dictionary {
            command,
            ensemble: true,
        } = self.grammar
        {
            return implementation_version_matches(dialect, TclVersion::V8_5)
                .then(|| command.lookup());
        }
        if !implementation_version_matches(dialect, TclVersion::V8_5) {
            return None;
        }
        match self.grammar {
            NativeCompilationGrammar::InfoExists => Some(INFO_EXISTS_IMPLEMENTATION),
            NativeCompilationGrammar::InfoLevel => Some(INFO_LEVEL_IMPLEMENTATION),
            NativeCompilationGrammar::InfoCoroutine => {
                implementation_version_matches(dialect, TclVersion::V8_6)
                    .then_some(INFO_COROUTINE_IMPLEMENTATION)
            }
            _ => None,
        }
    }

    fn namespace_upvar_implementation_lookup(
        dialect: InvocationDialect,
    ) -> Option<NativeCompilerImplementationLookup> {
        implementation_version_matches(dialect, TclVersion::V8_6).then_some(
            NativeCompilerImplementationLookup {
                ensemble: "::namespace",
                member: "upvar",
                slot: "::tcl::namespace::upvar",
                command: "namespace",
                prepended: &["upvar"],
            },
        )
    }

    fn public_string_implementation_lookup(
        self,
        dialect: InvocationDialect,
    ) -> Option<NativeCompilerImplementationLookup> {
        if matches!(
            self.grammar,
            NativeCompilationGrammar::StringEqual(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | NativeCompilationGrammar::StringLength(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            )
        ) {
            return implementation_version_matches(dialect, TclVersion::V8_5).then_some(
                if matches!(self.grammar, NativeCompilationGrammar::StringLength(_)) {
                    STRING_LENGTH_IMPLEMENTATION
                } else {
                    STRING_EQUAL_IMPLEMENTATION
                },
            );
        }
        if self.grammar
            == NativeCompilationGrammar::StringIsAscii(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            )
        {
            return implementation_version_matches(dialect, TclVersion::V8_5)
                .then_some(crate::native_string_is_compilation::STRING_IS_IMPLEMENTATION);
        }
        if let NativeCompilationGrammar::StringTrim {
            scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            operation,
        } = self.grammar
        {
            return implementation_version_matches(dialect, TclVersion::V8_5)
                .then_some(operation.lookup());
        }
        if self.grammar
            == NativeCompilationGrammar::StringMatch(
                crate::native_string_compilation::NativeStringMatchScope::PublicMember,
            )
        {
            return implementation_version_matches(dialect, TclVersion::V8_5)
                .then_some(crate::native_string_compilation::STRING_MATCH_IMPLEMENTATION);
        }
        if let NativeCompilationGrammar::StringOperand {
            scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            operation,
        } = self.grammar
        {
            return implementation_version_matches(dialect, TclVersion::V8_5)
                .then_some(*operation.implementation());
        }
        if self.grammar
            == NativeCompilationGrammar::StringRange(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            )
        {
            return (dialect.tcl_version >= Some(TclVersion::V8_6)
                && dialect.family() == Some(Family::Tcl))
            .then_some(crate::native_string_range_compilation::STRING_RANGE_IMPLEMENTATION);
        }
        if self.grammar
            == NativeCompilationGrammar::StringReplace(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            )
        {
            return (dialect.tcl_version >= Some(TclVersion::V8_6)
                && dialect.family() == Some(Family::Tcl))
            .then_some(crate::native_string_replace_compilation::STRING_REPLACE_IMPLEMENTATION);
        }
        if self.grammar
            == NativeCompilationGrammar::StringInsert(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            )
        {
            return (dialect.tcl_version >= Some(TclVersion::V9_0)
                && dialect.family() == Some(Family::Tcl))
            .then_some(crate::native_string_insert_compilation::STRING_INSERT_IMPLEMENTATION);
        }
        if self.grammar
            == NativeCompilationGrammar::StringMap(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            )
        {
            return (dialect.tcl_version >= Some(TclVersion::V8_6)
                && dialect.family() == Some(Family::Tcl))
            .then_some(crate::native_string_map_compilation::STRING_MAP_IMPLEMENTATION);
        }
        None
    }
}

fn implementation_version_matches(dialect: InvocationDialect, first: TclVersion) -> bool {
    dialect.family() == Some(Family::Tcl)
        && dialect.tcl_version.is_some_and(|version| version >= first)
}
