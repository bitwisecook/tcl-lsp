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

//! The bodies run here are in `tcl_compiler::compilation_unit` behind `test-support`.

mod original_native_static_read_owner_tests {
    spectcl_seeded_tests!(tcl_compiler::compilation_unit::original_native_static_read_owner_tests:
        native_static_read_receipt_keeps_the_current_unit_allocation_owner,
        native_static_read_receipt_refuses_wrapped_and_relabelled_cells,
        direct_original_static_reads_keep_missing_and_other_body_diagnostics,
    );
}
