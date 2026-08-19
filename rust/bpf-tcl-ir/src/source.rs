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

//! BPF-profile-pinned Tcl source lowering.

use tcl_compiler::Module;
use tcl_compiler::lowering::lower_to_ir_with_dialect;
use tcl_lexer::LexerConfig;
use tcl_registry::registry::CommandRegistry;

/// Lower BPF Tcl source under the BPF profile for every top-level and nested
/// body re-segmentation.
pub(crate) fn lower_bpf_source(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_with_dialect(
        source,
        registry,
        LexerConfig::for_dialect("bpf"),
        // The BPF profile as the registry itself resolved it — this crate
        // deliberately has no compile-time path to `tcl-dialect`, and reaches a
        // profile only through `CommandRegistry::profile()`.
        registry.profile(),
    )
}
