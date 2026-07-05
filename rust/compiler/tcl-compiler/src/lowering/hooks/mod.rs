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

//! Per-command lowering specialisations, one file per command.
//!
//! Each submodule exposes a `try_lower_<name>` entry point that takes
//! a [`LoweringCommand`](crate::lowering_hooks::LoweringCommand) and
//! returns a [`Statement`](crate::ir::Statement). The shared
//! dispatcher in [`crate::lowering_hooks::try_lower_hook`] routes
//! command names to the matching submodule.
//!
//! Split out from the `crate::lowering_hooks` module so each
//! command's logic lives in its own file.

pub mod control;
pub mod incr;
