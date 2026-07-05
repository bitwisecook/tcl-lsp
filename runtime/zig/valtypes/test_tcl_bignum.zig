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

// Wrapper that pulls the inline ``test`` blocks of
// ``valtypes/tcl_bignum.zig`` into the Zig test runner.  The build
// graph collects ``test_*.zig`` files automatically (see
// ``build.zig::collectTestFiles``); inline tests inside source files
// only run when an importer reaches them, so this thin shim lets
// ``zig build test`` execute the bignum unit tests without forcing
// every other importer to host them transitively.

comptime {
    _ = @import("tcl_bignum.zig");
}
