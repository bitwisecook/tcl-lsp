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

// I/O + channel stubs for Tcl 8.4–9.0 core commands we haven't
// implemented.  Each stub raises ``unsupported command: <name>``
// through :func:`tcl_stubs.unsupported`.
//
// **Reachability** — these ``pub export fn tcl_cmd_X`` symbols are
// direct WASM imports on the Python codegen side.  Each export name
// is referenced from the ``WasmRuntimeImport`` field on a
// ``CommandSpec`` under ``dialects/tcl/``.  The
// compiled WASM emits direct calls to these exports, so deleting
// any one of them breaks module instantiation for scripts that use
// that command.  Keep them in lock-step with the specs; the parity
// gate (``scripts/check_wasm_command_parity.py``) enforces this.
//
// Coverage today (everything else has a real impl elsewhere):
//   - fileevent, socket — needs an event loop
//
// ``chan`` dispatches through ``cmds/chan.zig:eval_chan`` (no direct
// codegen import); see issue #270.
//
// ``puts`` lives in tcl_io.zig; ``flush`` / ``open`` / ``close`` /
// ``read`` / ``gets`` / ``eof`` / ``fblocked`` / ``tell`` / ``seek``
// / ``fcopy`` live in tcl_chan.zig (real WASI-backed implementations).

const stubs = @import("tcl_stubs.zig");
const chan = @import("../io/tcl_chan.zig");

/// ``flush ?channelId?`` — drain the channel's per-write buffer
/// to its underlying fd.  With no argument (``fd == 0``) the call
/// is a no-op so the codegen's "flush stdout after a top-level
/// puts" emission doesn't trap.  Without a real flush path,
/// ``-buffering full`` writes would never reach the host stream
/// before ``close`` ran.
pub export fn tcl_cmd_flush(fd: i32) i32 {
    return chan.flush_chan_id(fd);
}

pub export fn tcl_cmd_fileevent(fd: i32, mode: i32) i32 {
    _ = fd;
    _ = mode;
    stubs.unsupported("fileevent");
    return 0;
}

pub export fn tcl_cmd_socket(host: i32, port: i32) i32 {
    _ = host;
    _ = port;
    stubs.unsupported("socket");
    return 0;
}
