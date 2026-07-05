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

//! Standalone helper functions for the optimiser.
//!
//! Broken into focused sub-modules so each pass pulls in exactly
//! the helpers it needs:
//!
//! - [`naming`] — namespace / proc-name resolution.
//! - [`literals`] — literal parsing + Tcl-source rendering.
//! - [`select`] — overlap-aware optimisation selection (the
//!   `manager`'s final output filter).

pub mod expr_simplify;
pub mod literals;
pub mod naming;
pub mod select;
pub mod spans;
pub mod tokens;
