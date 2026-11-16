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

//! Command validity: arity, option relations and option words, command and subcommand availability, gated `expr` operators and malformed structure.

use super::*;

#[path = "validity/arity.rs"]
mod arity;
#[path = "validity/command_availability.rs"]
mod command_availability;
#[path = "validity/discouraged_commands.rs"]
mod discouraged_commands;
#[path = "validity/expr_operators.rs"]
mod expr_operators;
#[path = "validity/malformed_structure.rs"]
mod malformed_structure;
#[path = "validity/object_and_lambda_arity.rs"]
mod object_and_lambda_arity;
#[path = "validity/option_relations.rs"]
mod option_relations;
#[path = "validity/option_words.rs"]
mod option_words;
#[path = "validity/stub_shadows.rs"]
mod stub_shadows;
#[path = "validity/user_call_arity.rs"]
mod user_call_arity;
