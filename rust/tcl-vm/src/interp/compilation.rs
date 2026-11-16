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

//! Runtime compilation: whether compiled bindings still hold, invalidation epochs and compiler policy stamps, procedure bodies, the native compilation entry, and the script compile cache.

use super::NativeCompilationEpochs;
use crate::compiled::NativeCompilerCacheEpoch;
use std::cell::RefCell;
use std::collections::HashMap;

mod binding_freshness;
mod invalidation;
mod native_entry;
mod native_prerequisites;
mod procedures;
mod script_compilation;

/// Trust state for bytecode that may bypass ordinary command lookup.
///
/// Registration is intentionally ignored while the interpreter installs its
/// own baseline. Once bootstrap ends, every live command-table mutation bumps
/// the binding generation checked by compiled artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommandSemantics {
    Bootstrapping,
    Tracking,
}

impl Default for NativeCompilationEpochs {
    fn default() -> Self {
        Self {
            cmd_resolve_cache: RefCell::new((0, HashMap::new())),
            cmd_epoch: std::cell::Cell::new(0),
            binding_epoch: std::cell::Cell::new(0),
            held_bindings: RefCell::new(Vec::new()),
            trace_deopt_epoch: std::cell::Cell::new(0),
            native_compiler_epoch: std::cell::Cell::new(NativeCompilerCacheEpoch::default()),
            native_namespace_epochs: HashMap::new(),
            native_ensemble_namespace_epochs: RefCell::new(HashMap::new()),
            command_semantics: std::cell::Cell::new(CommandSemantics::Bootstrapping),
        }
    }
}
