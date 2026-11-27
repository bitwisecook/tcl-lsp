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

//! Source-key descriptors: the keys that name a namespace, a command, a source
//! instance and a variable cell during source analysis.
//!
//! These are descriptors, not proofs. They intern to `tcl-core-types`' `*Ref`
//! identities and carry no lookup, execution or binding authority. Binding
//! records that hold proofs, such as an invocation's binding, stay with the
//! binding layer.

pub mod command;
pub mod namespace;
pub mod origin;
pub mod place;
pub mod variable;
pub mod word;
