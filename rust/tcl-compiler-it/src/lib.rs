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

//! No library code: `tcl-compiler`'s tests that need `tcl-spectcl` or
//! `tcl-engine-tclvm`, both of which depend on `tcl-compiler`. Keeping them
//! out of `tcl-compiler`'s own dev-dependencies means an edit to
//! `tcl-compiler` rebuilds only `tcl-compiler` before its unit tests link.
//!
//! Its integration tests that read the command registry live here too, and
//! install the shipped surfaces before their first read, so `tcl-compiler`
//! itself needs no surface crate to test. Their fixtures stay under
//! `tcl-compiler/tests`, which other crates' tests share.
