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

//! Error type for the BIG-IP object model and config parser.
//!
//! Every fallible public function in this crate reports failure as a
//! [`BigipError`]. Each variant names the subsystem that raised it and carries
//! the human-facing message; the CLI renders it as `error: {err}` (the
//! `Display` text is exactly the carried message). Callers that only need the
//! string form can use `err.to_string()`.

/// A typed error covering every fallible operation in the BIG-IP crate.
///
/// The variant names the subsystem the failure came from, so callers can match
/// on the category; the wrapped string is the ready-to-display detail message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BigipError {
    /// A PCAPNG block stream was malformed, truncated, or could not be
    /// serialised (for example a non-length-preserving packet rewrite).
    #[error("{0}")]
    Pcapng(String),

    /// A `--schema` trailer overlay was malformed, missing a required key, or
    /// named an unknown `ip_fields` kind.
    #[error("{0}")]
    Schema(String),

    /// A graph export request named an unsupported output format.
    #[error("{0}")]
    Graph(String),

    /// An object-graph grep request used conflicting modes, an out-of-range
    /// bound, an invalid direction, or an unparseable regex / CIDR pattern.
    #[error("{0}")]
    Grep(String),

    /// A redaction map could not be parsed, a target CIDR could not be
    /// allocated, or a redaction option was invalid.
    #[error("{0}")]
    Redact(String),

    /// A PCAPNG name-resolution enrichment could not proceed (for example the
    /// input was libpcap rather than PCAPNG).
    #[error("{0}")]
    Enrich(String),
}

impl BigipError {
    /// Build a [`BigipError::Pcapng`] from any displayable detail.
    pub(crate) fn pcapng(message: impl Into<String>) -> Self {
        BigipError::Pcapng(message.into())
    }

    /// Build a [`BigipError::Schema`] from any displayable detail.
    pub(crate) fn schema(message: impl Into<String>) -> Self {
        BigipError::Schema(message.into())
    }

    /// Build a [`BigipError::Graph`] from any displayable detail.
    pub(crate) fn graph(message: impl Into<String>) -> Self {
        BigipError::Graph(message.into())
    }

    /// Build a [`BigipError::Grep`] from any displayable detail.
    pub(crate) fn grep(message: impl Into<String>) -> Self {
        BigipError::Grep(message.into())
    }

    /// Build a [`BigipError::Redact`] from any displayable detail.
    pub(crate) fn redact(message: impl Into<String>) -> Self {
        BigipError::Redact(message.into())
    }

    /// Build a [`BigipError::Enrich`] from any displayable detail.
    pub(crate) fn enrich(message: impl Into<String>) -> Self {
        BigipError::Enrich(message.into())
    }
}
