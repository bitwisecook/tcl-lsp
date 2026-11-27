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

//! The canonical red-green concrete syntax tree (CST).
//!
//! The split mirrors
//! Roslyn / rust-analyzer:
//!
//! - [`green`] — the **position-independent** layer: a node knows only
//!   its *width* and its children, never an absolute offset.  Trivia is
//!   attached to the adjacent token, so a command is pure syntax while
//!   every inter-word byte still round-trips.
//! - [`red`] — overlays a green tree with an anchoring and resolves
//!   absolute positions lazily, reproducing the lexer's exact `Token`
//!   offsets / lines / columns.
//! - [`build`] — re-shapes the existing lexer stream into the tree (no
//!   second parser) via start-to-start tiling.
//! - [`segment`] — derives `SegmentedCommand` from the tree,
//!   byte-identically to the token-loop segmenter.
//! - [`descend`] — lazy descent into braced bodies / `[…]` subs as
//!   child CSTs anchored one byte past the opener.
//!
//! See `docs/design/compiler/syntax-tree.md`.

pub mod build;
pub mod descend;
pub mod green;
pub mod red;
pub mod segment;
