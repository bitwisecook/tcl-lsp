// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared provenance types for static TLS configuration adapters.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// One source assignment or directive considered by an adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigEvidence {
    /// One-based source line.
    pub line: u32,
    /// Source section or configuration context.
    pub section: String,
    /// Projected endpoint/context for which effectiveness was evaluated.
    pub target: Option<String>,
    /// Source directive or assignment name.
    pub directive: String,
    /// Literal, unexpanded source values.
    pub values: Vec<String>,
    /// Unexpanded source value in configured order.
    pub raw_value: String,
    /// Whether this occurrence contributed to the projected endpoint.
    pub effective: bool,
    /// Line of the occurrence that replaced this one, when known.
    pub overridden_by_line: Option<u32>,
}

/// A non-fatal limitation or preserved construct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigNotice {
    /// One-based source line.
    pub line: u32,
    /// Source section or configuration context.
    pub section: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Syntax that cannot be interpreted without guessing structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// One-based source line.
    pub line: u32,
    /// Human-readable parser detail.
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

impl Error for ConfigError {}
