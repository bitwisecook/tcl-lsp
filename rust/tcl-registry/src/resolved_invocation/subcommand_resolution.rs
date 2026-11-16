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

//! The outcome of resolving a subcommand word: resolved, ambiguous, unknown or unresolved, borrowed and owned.

use super::ResolvedInvocation;
use crate::invocation_words::InvocationWordKind;

/// A subcommand matched by [`crate::CommandRegistry::resolve_invocation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedSubcommand<'w> {
    /// The first argument word exactly as supplied by the caller.
    pub spelling: &'w str,
    /// The registry's canonical subcommand name.
    pub canonical_name: &'static str,
}

/// The typed outcome of resolving the first post-head word as a subcommand.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubcommandResolutionKind {
    /// The source word equals the canonical subcommand spelling.
    Exact,
    /// The source word is a registry-approved unique prefix.
    UniquePrefix,
    /// The source word prefixes several registry subcommands.
    Ambiguous,
    /// The source word names no registry subcommand.
    Unknown,
    /// The source word is not a known literal, so no subcommand lookup was
    /// attempted.
    Indeterminate,
}

/// The subcommand-resolution outcome for a command invocation.
///
/// A command without subcommands, or an invocation with no first argument to
/// resolve, is [`Self::NotApplicable`].  Unknown and ambiguous words remain
/// visible to common analysis rather than being silently treated as an
/// ordinary command form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubcommandResolution<'w> {
    /// There is no subcommand table or no first post-head word to query.
    NotApplicable,
    /// The source word exactly matches a declared subcommand.
    Exact(ResolvedSubcommand<'w>),
    /// The source word uniquely prefixes a declared subcommand.
    UniquePrefix(ResolvedSubcommand<'w>),
    /// The source word prefixes several declared subcommands.
    Ambiguous {
        /// The source word that was ambiguous.
        spelling: &'w str,
    },
    /// The source word names no declared subcommand.
    Unknown {
        /// The source word that was not in the table.
        spelling: &'w str,
    },
    /// The first post-head word is substituted, expanded, or opaque, so its
    /// runtime subcommand value cannot be selected from registry metadata.
    ///
    /// This outcome intentionally exposes only the source-word category, not
    /// a raw spelling that a consumer might confuse with the evaluated value.
    Indeterminate {
        /// Why the subcommand word cannot be looked up statically.
        word_kind: InvocationWordKind,
    },
}

impl<'w> SubcommandResolution<'w> {
    /// The resolution kind when a word was looked up in a subcommand table.
    #[must_use]
    pub const fn kind(self) -> Option<SubcommandResolutionKind> {
        match self {
            Self::NotApplicable => None,
            Self::Exact(_) => Some(SubcommandResolutionKind::Exact),
            Self::UniquePrefix(_) => Some(SubcommandResolutionKind::UniquePrefix),
            Self::Ambiguous { .. } => Some(SubcommandResolutionKind::Ambiguous),
            Self::Unknown { .. } => Some(SubcommandResolutionKind::Unknown),
            Self::Indeterminate { .. } => Some(SubcommandResolutionKind::Indeterminate),
        }
    }

    /// The matched canonical subcommand, for exact and unique-prefix cases.
    #[must_use]
    pub const fn resolved(self) -> Option<ResolvedSubcommand<'w>> {
        match self {
            Self::Exact(subcommand) | Self::UniquePrefix(subcommand) => Some(subcommand),
            Self::NotApplicable
            | Self::Ambiguous { .. }
            | Self::Unknown { .. }
            | Self::Indeterminate { .. } => None,
        }
    }

    /// Whether this outcome selected a subcommand descriptor.
    #[must_use]
    pub const fn is_resolved(self) -> bool {
        self.resolved().is_some()
    }

    /// Materialise this borrowed outcome for an owned consumer.
    #[must_use]
    pub fn into_owned(self) -> OwnedSubcommandResolution {
        match self {
            Self::NotApplicable => OwnedSubcommandResolution::NotApplicable,
            Self::Exact(subcommand) => OwnedSubcommandResolution::Exact {
                spelling: subcommand.spelling.to_owned(),
                canonical_name: subcommand.canonical_name.to_owned(),
            },
            Self::UniquePrefix(subcommand) => OwnedSubcommandResolution::UniquePrefix {
                spelling: subcommand.spelling.to_owned(),
                canonical_name: subcommand.canonical_name.to_owned(),
            },
            Self::Ambiguous { spelling } => OwnedSubcommandResolution::Ambiguous {
                spelling: spelling.to_owned(),
            },
            Self::Unknown { spelling } => OwnedSubcommandResolution::Unknown {
                spelling: spelling.to_owned(),
            },
            Self::Indeterminate { word_kind } => {
                OwnedSubcommandResolution::Indeterminate { word_kind }
            }
        }
    }
}

/// The owned form of [`SubcommandResolution`] retained by executable IR.
///
/// The non-resolved outcomes remain distinct so a later analysis can explain
/// why no subcommand-specific fact was available rather than treating every
/// case as an absent subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedSubcommandResolution {
    /// There is no subcommand table or no first post-head word to query.
    NotApplicable,
    /// A literal word exactly matched a declared subcommand.
    Exact {
        /// Literal source spelling.
        spelling: String,
        /// Registry canonical subcommand identity.
        canonical_name: String,
    },
    /// A literal word uniquely prefixed a declared subcommand.
    UniquePrefix {
        /// Literal source spelling.
        spelling: String,
        /// Registry canonical subcommand identity.
        canonical_name: String,
    },
    /// A literal word prefixed several declared subcommands.
    Ambiguous {
        /// Literal source spelling.
        spelling: String,
    },
    /// A literal word matched no declared subcommand.
    Unknown {
        /// Literal source spelling.
        spelling: String,
    },
    /// A substituted, expanded, or opaque word could not be looked up.
    Indeterminate {
        /// The source-word category that prevented lookup.
        word_kind: InvocationWordKind,
    },
}

impl OwnedSubcommandResolution {
    /// Return the canonical subcommand identity for resolved outcomes.
    #[must_use]
    pub fn canonical_name(&self) -> Option<&str> {
        match self {
            Self::Exact { canonical_name, .. } | Self::UniquePrefix { canonical_name, .. } => {
                Some(canonical_name)
            }
            Self::NotApplicable
            | Self::Ambiguous { .. }
            | Self::Unknown { .. }
            | Self::Indeterminate { .. } => None,
        }
    }
}

/// Why source-aware registry resolution could not select an invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationResolutionUnresolved<'w> {
    /// The command head is computed, expanded, or opaque, so it cannot be
    /// looked up in a literal registry table.
    ComputedHead {
        /// The source-word category that prevented lookup.
        word_kind: InvocationWordKind,
    },
    /// The command head was literal but no registry entry is available for
    /// the active dialect.
    UnknownLiteralHead {
        /// Literal source spelling.
        spelling: &'w str,
    },
}

/// The typed outcome of source-aware invocation resolution.
///
/// Exactly one of the two private fields is present. This deliberately uses a
/// compact result structure rather than boxing a large borrowed
/// [`ResolvedInvocation`] in an enum variant: structured resolution remains
/// allocation-free until an owned facts projection is requested.
#[derive(Debug, Clone, Copy)]
pub struct StructuredInvocationResolution<'r, 'w> {
    pub(crate) invocation: Option<ResolvedInvocation<'r, 'w>>,
    pub(crate) unresolved: Option<InvocationResolutionUnresolved<'w>>,
}

impl<'r, 'w> StructuredInvocationResolution<'r, 'w> {
    pub(crate) const fn from_unresolved(reason: InvocationResolutionUnresolved<'w>) -> Self {
        Self {
            invocation: None,
            unresolved: Some(reason),
        }
    }

    /// Return the borrowed resolved invocation, if registry selection
    /// succeeded.
    #[must_use]
    pub const fn resolved(self) -> Option<ResolvedInvocation<'r, 'w>> {
        self.invocation
    }

    /// Return the typed unresolved reason, if registry selection failed.
    #[must_use]
    pub const fn unresolved(self) -> Option<InvocationResolutionUnresolved<'w>> {
        self.unresolved
    }
}
