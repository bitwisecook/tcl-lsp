// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-owned validation of relationships between literal arguments.
//!
//! Closed scalar sets are described by `closed_value_args`.  This descriptor
//! is the next rung: a command can validate a literal whose legal values
//! depend on another argument, or whose value is itself a Tcl list.  The
//! registry callback receives source-aware words, so a dynamic, expanded, or
//! incomplete invocation can decline rather than treating source spelling as
//! a runtime value.

use crate::InvocationArguments;

/// A registry callback which validates one relationship between literal
/// invocation arguments.
pub type LiteralArgumentValidator =
    for<'a> fn(InvocationArguments<'a>) -> LiteralArgumentValidation;

/// The typed outcome of a literal-argument validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralArgumentValidation {
    /// Every checked literal satisfies the declared relationship.
    Valid,
    /// A complete literal invocation violates the declared relationship.
    Invalid(LiteralArgumentIssue),
    /// The source does not prove enough to validate soundly.
    Abstain(LiteralValidationDecline),
}

/// Why a literal-argument validator deliberately produced no finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralValidationDecline {
    /// Expansion or a missing word makes the invocation's argv shape unknown.
    IncompleteInvocation,
    /// A required argument is substituted, expanded, or otherwise not a
    /// statically known Tcl value.
    NonLiteralArgument,
    /// The discriminator which selects the legal domain is itself invalid or
    /// ambiguous; the ordinary closed-value diagnostic owns that error.
    InvalidDiscriminator,
    /// The literal is not a complete, well-formed value in its declared
    /// grammar. Parse/recovery diagnostics own malformed source.
    MalformedLiteral,
}

/// One invalid literal argument, expressed independently of diagnostic or LSP
/// types so every consumer can render the same registry fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralArgumentIssue {
    /// Zero-based argument index after the command head.
    pub argument_index: usize,
    /// Registry-owned name for the argument/domain being checked.
    pub subject: &'static str,
    /// Why the value is invalid.
    pub reason: LiteralArgumentIssueReason,
    /// Exhaustive legal members for this selected argument domain.
    pub allowed_values: &'static [&'static str],
    /// Replacement Tcl *value*, when removing invalid members still leaves a
    /// valid value. The consumer is responsible for quoting it as one source
    /// word.
    pub replacement_value: Option<String>,
}

/// The shape of a registry-declared literal-argument violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralArgumentIssueReason {
    /// A required collection/string contains no member.
    Empty,
    /// These exact members are outside the selected legal set.
    InvalidMembers(Vec<String>),
}
