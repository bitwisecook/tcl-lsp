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

//! Bounded passive ordinary-world transitions. All fields describe captured
//! source-analysis data; neither opacity nor a pointer grants execution proof.

use super::SourceOutcomes;

pub(super) fn observe_command(
    segment: &crate::segmenter::SegmentedCommand,
    command: &SourceOutcomes,
    prefix_complete: bool,
) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_COMMAND_COMPLETION site={} prefix_complete={} command_complete={} normal={} abrupt={}",
            segment.span.start(),
            prefix_complete,
            command.normal_completion.is_some(),
            command.normal.is_some(),
            command.abrupt.len(),
        );
    }
}
