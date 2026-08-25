// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed static outcomes of a registry-declared command-prefix expression.
//!
//! The registry owns the vocabulary. Consumers attach source spans and prove a
//! concrete expression has one of these outcomes by following
//! [`crate::Traits::BUILDS_COMMAND_PREFIX`] and
//! [`crate::Traits::WRAPS_COMMAND_PREFIX`].

/// The dispatch model represented by a statically recoverable command prefix.
///
/// This is deliberately not a boolean: current-object callbacks have two
/// different visibility models, and wrappers must preserve that distinction.
/// Future class systems can extend the closed vocabulary without each
/// consumer inventing its own callback classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandPrefixTarget {
    /// A normal static command head such as `[list ::pkg::callback arg]`.
    DirectCommandHead,
    /// `[list [self] method ...]`, dispatched externally through an object command.
    CurrentObjectExternalMethod,
    /// `[list my method ...]`, dispatched through the current object's method frame.
    CurrentObjectInternalMethod,
}
