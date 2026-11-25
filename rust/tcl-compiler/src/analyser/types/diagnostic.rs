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

//! `Diagnostic`: one finding with its span, severity, fixes and owned subject.

use crate::irules_checks::CodeFix;
use tcl_core_types::{DiagCode, Severity};
use tcl_lexer::Span;

/// Source data-group equality with an item holding a signed scan result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedDataGroupComparison {
    /// Literal group reference selected by the registry.
    pub group: String,
    /// Unsigned keys representable by the scan field but outside its signed range.
    pub unsigned_keys: std::ops::RangeInclusive<u64>,
}

/// Diagnostic emitted by the analyser.
///
/// Carries a stable ``code`` (e.g. ``"W210"``), the source
/// [`Span`] the diagnostic anchors to, a one-line ``message``, a
/// [`Severity`], and optional [`CodeFix`] suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Optional typed subject from the emitting semantic owner.
    pub subject: Option<super::super::DiagnosticSubject>,
    /// Evidence a configuration owner can use to resolve a signed group comparison.
    pub signed_data_group_comparison: Option<Box<SignedDataGroupComparison>>,
    /// Stable W-/IRULE-coded identifier.
    pub code: DiagCode,
    /// Source span the diagnostic anchors to.
    pub span: Span,
    /// One-line user-facing message.
    pub message: String,
    /// Severity classifier.
    pub severity: Severity,
    /// Suggested fixes (zero or more).  Empty when no
    /// emitter-supplied fix is available.
    pub fixes: Vec<CodeFix>,
}

impl Diagnostic {
    /// Construct a diagnostic without a suggested fix.
    ///
    /// Emitters that can offer a source edit add it with
    /// [`Self::with_fix`] or [`Self::with_fixes`]. Keeping the empty list
    /// here makes the common, fix-less case explicit without repeating the
    /// representation detail at every emission site.
    #[must_use]
    pub fn new(code: DiagCode, span: Span, message: impl Into<String>, severity: Severity) -> Self {
        Self {
            code,
            span,
            message: message.into(),
            severity,
            fixes: Vec::new(),
            subject: None,
            signed_data_group_comparison: None,
        }
    }

    /// Retain the genuine semantic subject independently of reporting text.
    #[must_use]
    pub fn with_subject(mut self, subject: super::super::DiagnosticSubject) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Borrow the retained semantic subject, if the emitter could authenticate it.
    #[must_use]
    pub fn subject(&self) -> Option<&super::super::DiagnosticSubject> {
        self.subject.as_ref()
    }

    /// Selected package database key, without parsing the diagnostic message.
    #[must_use]
    pub fn required_package_key(
        &self,
    ) -> Option<&tcl_registry::native_package::NativePackageNameKey> {
        match self.subject()? {
            super::super::DiagnosticSubject::RequiredPackage(key) => Some(key),
            super::super::DiagnosticSubject::RegistrySource(source) => {
                source.required_package_key()
            }
            super::super::DiagnosticSubject::CallbackSourceArity(_)
            | super::super::DiagnosticSubject::ObjectSourceArity(_)
            | super::super::DiagnosticSubject::PrivateTclNamespace(_)
            | super::super::DiagnosticSubject::UserSourceArity(_)
            | super::super::DiagnosticSubject::CommandAvailability(_)
            | super::super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::super::DiagnosticSubject::DeclaredSource(_)
            | super::super::DiagnosticSubject::UnresolvedCommand(_)
            | super::super::DiagnosticSubject::UnresolvedMathFunction(_)
            | super::super::DiagnosticSubject::ConditionalInterpreterVisibility(_) => None,
        }
    }

    /// Original unresolved name and source invocation, without message parsing.
    #[must_use]
    pub fn unresolved_command(&self) -> Option<&super::super::SourceUnresolvedCommandSubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::UnresolvedCommand(subject) => Some(subject),
            super::super::DiagnosticSubject::CallbackSourceArity(_)
            | super::super::DiagnosticSubject::ObjectSourceArity(_)
            | super::super::DiagnosticSubject::PrivateTclNamespace(_)
            | super::super::DiagnosticSubject::UserSourceArity(_)
            | super::super::DiagnosticSubject::CommandAvailability(_)
            | super::super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::super::DiagnosticSubject::DeclaredSource(_)
            | super::super::DiagnosticSubject::RegistrySource(_)
            | super::super::DiagnosticSubject::RequiredPackage(_)
            | super::super::DiagnosticSubject::UnresolvedMathFunction(_)
            | super::super::DiagnosticSubject::ConditionalInterpreterVisibility(_) => None,
        }
    }

    /// Original unresolved expression function, without a command-head issuer.
    #[must_use]
    pub fn unresolved_math_function(
        &self,
    ) -> Option<&super::super::SourceUnresolvedMathFunctionSubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::UnresolvedMathFunction(subject) => Some(subject),
            super::super::DiagnosticSubject::CallbackSourceArity(_)
            | super::super::DiagnosticSubject::ObjectSourceArity(_)
            | super::super::DiagnosticSubject::PrivateTclNamespace(_)
            | super::super::DiagnosticSubject::UserSourceArity(_)
            | super::super::DiagnosticSubject::CommandAvailability(_)
            | super::super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::super::DiagnosticSubject::DeclaredSource(_)
            | super::super::DiagnosticSubject::RegistrySource(_)
            | super::super::DiagnosticSubject::RequiredPackage(_)
            | super::super::DiagnosticSubject::UnresolvedCommand(_)
            | super::super::DiagnosticSubject::ConditionalInterpreterVisibility(_) => None,
        }
    }

    /// Conditional child source visibility and its explicit assumptions.
    /// This is not a selected hidden callable or an effects exclusion.
    #[must_use]
    pub fn conditional_interpreter_visibility(
        &self,
    ) -> Option<&super::super::ConditionalInterpreterVisibilitySubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::ConditionalInterpreterVisibility(subject) => {
                Some(subject)
            }
            super::super::DiagnosticSubject::CallbackSourceArity(_)
            | super::super::DiagnosticSubject::ObjectSourceArity(_)
            | super::super::DiagnosticSubject::PrivateTclNamespace(_)
            | super::super::DiagnosticSubject::UserSourceArity(_)
            | super::super::DiagnosticSubject::CommandAvailability(_)
            | super::super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::super::DiagnosticSubject::DeclaredSource(_)
            | super::super::DiagnosticSubject::RegistrySource(_)
            | super::super::DiagnosticSubject::RequiredPackage(_)
            | super::super::DiagnosticSubject::UnresolvedCommand(_)
            | super::super::DiagnosticSubject::UnresolvedMathFunction(_) => None,
        }
    }

    /// Original whole-command exclusion metadata, independent of reporting text.
    #[must_use]
    pub fn command_availability(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceCommandAvailability> {
        match self.subject.as_ref()? {
            super::super::DiagnosticSubject::CommandAvailability(subject) => Some(subject),
            _ => None,
        }
    }

    /// Original Registry source syntax retained independently of presentation.
    #[must_use]
    pub fn registry_source(&self) -> Option<&super::super::RegistrySourceDiagnosticSubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::RegistrySource(subject) => Some(subject),
            _ => None,
        }
    }

    /// Authored declaration purpose and original operands, without message parsing.
    #[must_use]
    pub fn declared_source(&self) -> Option<&super::super::DeclaredSourceDiagnosticSubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::DeclaredSource(subject) => Some(subject),
            _ => None,
        }
    }

    /// Original lexical object signature ownership, without receiver dispatch.
    #[must_use]
    pub fn object_source_arity(&self) -> Option<&super::super::ObjectSourceAritySubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::ObjectSourceArity(subject) => Some(subject),
            _ => None,
        }
    }

    /// Original private namespace source policy, without actual lookup authority.
    #[must_use]
    pub fn private_tcl_namespace(&self) -> Option<&super::super::SourcePrivateTclNamespaceSubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::PrivateTclNamespace(subject) => Some(subject),
            _ => None,
        }
    }

    /// Original conditional procedure signature, never an entered call proof.
    #[must_use]
    pub fn user_source_arity(&self) -> Option<&super::super::SourceUserCallAritySubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::UserSourceArity(subject) => Some(subject),
            _ => None,
        }
    }

    /// Conditional callback signature advice, without future dispatch proof.
    #[must_use]
    pub fn callback_source_arity(&self) -> Option<&super::super::SourceCallbackAritySubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::CallbackSourceArity(subject) => Some(subject),
            _ => None,
        }
    }

    /// Genuine registered-instance source metadata and its diagnostic purpose.
    #[must_use]
    pub fn registered_instance_source(
        &self,
    ) -> Option<&super::super::RegisteredInstanceSourceDiagnosticSubject> {
        match self.subject()? {
            super::super::DiagnosticSubject::RegisteredInstanceSource(subject) => Some(subject),
            _ => None,
        }
    }

    /// Attach one suggested edit.
    #[must_use]
    pub fn with_fix(mut self, fix: CodeFix) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Attach the complete set of suggested edits computed by an emitter.
    #[must_use]
    pub fn with_fixes(mut self, fixes: Vec<CodeFix>) -> Self {
        self.fixes = fixes;
        self
    }
}
