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

//! Verb handlers for the `tcl` CLI.
//!
//! Each handler resolves inputs via `tcl-cli-support`, drives the relevant Rust
//! engine crate, and writes output. Handlers return the intended process exit
//! code.

pub mod compile;
pub mod diag;
pub mod diagram;
pub mod diff;
pub mod docker;
pub mod explore;
pub mod graphs;
pub mod gui;
pub mod help;
pub mod highlight;
pub mod lookup;
pub mod minimize;
pub mod misc;
pub mod pkg;
pub mod pkg_discover;
pub mod registry;
pub mod spec;
pub mod transform;
pub mod venv;
