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

//! The `argparse` package command definitions.

use crate::spec::CommandSpec;

// The `argparse` command itself lives in `argparse/command.rs`; naming the
// submodule for its contents (rather than repeating the package name) lets the
// directory grow more command files without the parent/child name collision
// (`clippy::module_inception`).
mod command;

pub use command::{ELEMENT_SWITCHES, ELEMENT_SWITCHES_WITH_ARGS};

/// All command specs contributed by the `argparse` package.
#[must_use]
pub fn argparse_command_specs() -> Vec<CommandSpec> {
    vec![command::spec()]
}
