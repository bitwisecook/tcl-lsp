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

/// An inlined command body's instruction range and the `errorInfo` frame the
/// enclosing command (`eval`, a loop, `if`, `switch`) would add when its body
/// errors — the compiled analogue of C's per-command `CmdFrame`. Populated by
/// codegen when it folds a literal body inline (so the LSP keeps its inlined
/// view), consumed by the executor as an error unwinds (see
/// `tcl_bytecode::FunctionAsm::error_regions`).
#[derive(Debug, Clone)]
pub struct ErrorRegion {
    /// Start byte offset of the enclosing command's source span (inclusive). An
    /// instruction is covered when its `source_span` lies within `[start, end)`,
    /// so coverage is matched by source containment — robust to instruction
    /// reordering and exact across interleaved (non-body) instructions.
    pub start: u32,
    /// End byte offset of the enclosing command's source span (exclusive).
    pub end: u32,
    /// The frame the body adds before the command is logged.
    pub frame: ErrorRegionFrame,
    /// The enclosing command's surface text, for its `invoked from within "…"`
    /// frame (truncated to 150 bytes by the logger, as in C).
    pub cmd_text: tcl_lexer::SourceImage,
    /// One less than the body's first source line: a covered instruction's
    /// body-relative line is `instruction_line − line_base` (so the body frame
    /// reports a line relative to the body, not the whole module).
    pub line_base: u32,
    /// The enclosing command's own source line — the line its `invoked from
    /// within` frame contributes to any further-out (proc) frame.
    pub cmd_line: u32,
    /// Whether the release compiles the command in a procedure body, so the
    /// region adds no frame where its script runs as part of one.
    pub compiled_in_procedure: bool,
}

/// The `errorInfo` frame an [`ErrorRegion`] adds, `N` the failing command's
/// line within the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorRegionFrame {
    /// `("LABEL" body line N)`: `eval`, `foreach`, `while`, `for`, `lmap`.
    Body(String),
    /// `(TEXT line N)`: a `switch` arm's `"PATTERN" arm`.
    Line(String),
    /// `(TEXT)`: `"for" initial command`.
    Fixed(String),
    /// None: the command is logged alone (`if`).
    None,
}
