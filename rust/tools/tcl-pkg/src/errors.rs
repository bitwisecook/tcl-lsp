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

//! Error types for the `tclpkg` package manager.
//!
//! The CLI prints the
//! [`Display`] form verbatim, so the message composition (location prefixes,
//! `hint:` suffix, integrity expected/got detail) is stable and canonical.

use std::fmt;

/// Category tag for grouping errors by subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    TclPkg,
    Manifest,
    Resolver,
    Integrity,
    Registry,
    Fetch,
    Venv,
    Policy,
    Sandbox,
    Hook,
}

impl Category {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Category::TclPkg => "tclpkg",
            Category::Manifest => "manifest",
            Category::Resolver => "resolver",
            Category::Integrity => "integrity",
            Category::Registry => "registry",
            Category::Fetch => "fetch",
            Category::Venv => "venv",
            Category::Policy => "policy",
            Category::Sandbox => "sandbox",
            Category::Hook => "hook",
        }
    }
}

/// Base error type for every tclpkg-specific failure.
///
/// The variant carries a fully composed `message` plus an optional
/// `hint`; the `Display` impl renders the message, then
/// `\n  hint: <hint>` when present.
#[derive(Debug, Clone)]
pub struct TclPkgError {
    pub category: Category,
    pub message: String,
    pub hint: Option<String>,
}

impl TclPkgError {
    /// Construct a generic `tclpkg` error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            category: Category::TclPkg,
            message: message.into(),
            hint: None,
        }
    }

    /// Attach a recovery hint.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    #[must_use]
    pub fn with_category(mut self, category: Category) -> Self {
        self.category = category;
        self
    }

    /// Build a `ManifestError`: optional `path[:line]: ` prefix on the message.
    pub fn manifest(
        message: impl Into<String>,
        path: Option<&str>,
        line: Option<usize>,
        hint: Option<String>,
    ) -> Self {
        let mut location = String::new();
        if let Some(p) = path
            && !p.is_empty()
        {
            location.push_str(p);
            if let Some(l) = line {
                location.push(':');
                location.push_str(&l.to_string());
            }
            location.push_str(": ");
        }
        Self {
            category: Category::Manifest,
            message: format!("{location}{}", message.into()),
            hint,
        }
    }

    /// Build a `ResolutionError`: appends a `\n  via <chain>` trail.
    pub fn resolution(message: impl Into<String>, chain: &[String], hint: Option<String>) -> Self {
        let mut msg = message.into();
        if !chain.is_empty() {
            let joined = chain.join("\n  via ");
            msg = format!("{msg}\n  via {joined}");
        }
        Self {
            category: Category::Resolver,
            message: msg,
            hint,
        }
    }

    /// Build an `IntegrityError`: optional expected/got detail block.
    pub fn integrity(
        message: impl Into<String>,
        expected: Option<&str>,
        actual: Option<&str>,
        hint: Option<String>,
    ) -> Self {
        let mut msg = message.into();
        if let (Some(e), Some(a)) = (expected, actual) {
            msg = format!("{msg}\n  expected {e}\n  got      {a}");
        }
        Self {
            category: Category::Integrity,
            message: msg,
            hint,
        }
    }

    /// Build a `RegistryError`.
    pub fn registry(message: impl Into<String>, hint: Option<String>) -> Self {
        Self {
            category: Category::Registry,
            message: message.into(),
            hint,
        }
    }

    /// Build a `FetchError`.
    pub fn fetch(message: impl Into<String>) -> Self {
        Self {
            category: Category::Fetch,
            message: message.into(),
            hint: None,
        }
    }

    /// Build a `VenvError`.
    pub fn venv(message: impl Into<String>, hint: Option<String>) -> Self {
        Self {
            category: Category::Venv,
            message: message.into(),
            hint,
        }
    }
}

impl fmt::Display for TclPkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(hint) = &self.hint {
            write!(f, "{}\n  hint: {hint}", self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for TclPkgError {}

/// Convenience alias for results returning a [`TclPkgError`].
pub type Result<T> = std::result::Result<T, TclPkgError>;
