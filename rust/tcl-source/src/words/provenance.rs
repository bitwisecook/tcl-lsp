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

//! Node identities and the provenance of an IR site.

use tcl_lexer::Span;

// Command tokens

/// A stable, function-local identity for a semantic IR node.
///
/// A node ID is a structural path, not a source offset.  Rebase operations
/// therefore preserve it, which is essential for compilation-unit caching and
/// for eventual guard/deoptimisation maps.  The ID is intentionally local to a
/// executable-semantic function; a procedure name or other function key
/// supplies the enclosing identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(Vec<u32>);

impl NodeId {
    /// Create an ID from its deterministic structural path.
    ///
    /// This is public for IR transforms that preserve a node's identity. New
    /// source nodes should be allocated by the semantic-IR builder instead of
    /// manufacturing paths ad hoc.
    #[must_use]
    pub fn from_path(path: Vec<u32>) -> Self {
        Self(path)
    }

    /// Return the path segments that make up this function-local ID.
    #[must_use]
    pub fn path(&self) -> &[u32] {
        &self.0
    }
}

/// Why an IR site exists at a source location.
///
/// The source variant is used by the current lowering compatibility layer.
/// Transforming passes can retain the relationship to their source node(s)
/// through [`Self::Derived`] without inventing a misleading source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Provenance {
    /// Directly lowered from user-authored Tcl source.
    #[default]
    Source,
    /// Created by a semantic transform from one or more source nodes.
    Derived {
        /// Category of semantic transform that made this node.
        kind: DerivedKind,
        /// The source nodes that justify this derived node.
        origins: Vec<NodeId>,
    },
    /// A conservative compatibility placeholder for a hand-built or lossy
    /// IR node whose source origin cannot be established.
    Opaque,
}

/// The transform class for [`Provenance::Derived`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DerivedKind {
    /// Desugared from an equivalent source construct.
    Desugared,
    /// Produced by constant folding.
    Folded,
    /// Cloned into a caller by inlining.
    Inlined,
    /// Created by a guarded specialisation.
    Specialised,
    /// Created by another named semantic transform.
    Synthetic,
}

/// A source range together with the reason the IR associates work with it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSite {
    /// Byte span in the original source when known.
    pub span: Span,
    /// Origin relationship for this site.
    pub provenance: Provenance,
}

impl SourceSite {
    /// Build a direct user-source site.
    #[must_use]
    pub const fn source(span: Span) -> Self {
        Self {
            span,
            provenance: Provenance::Source,
        }
    }

    /// Build a transformed site while preserving its source relationship.
    #[must_use]
    pub fn derived(span: Span, kind: DerivedKind, origins: Vec<NodeId>) -> Self {
        Self {
            span,
            provenance: Provenance::Derived { kind, origins },
        }
    }

    /// Build an explicitly opaque compatibility site.
    #[must_use]
    pub const fn opaque(span: Span) -> Self {
        Self {
            span,
            provenance: Provenance::Opaque,
        }
    }
}
