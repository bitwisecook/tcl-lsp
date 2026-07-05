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

// WASI I/O helpers: itoa, fd_write_all, puts.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const read_i32 = obj.read_i32;
const read_i64 = obj.read_i64;
const obj_ensure_string = obj.obj_ensure_string;
const TYPE_STRING = obj.TYPE_STRING;
const OBJ_TYPE_TAG = obj.OBJ_TYPE_TAG;
const OBJ_STR_PTR = obj.OBJ_STR_PTR;
const OBJ_STR_LEN = obj.OBJ_STR_LEN;
const OBJ_INT_CACHE = obj.OBJ_INT_CACHE;

// Re-export ``itoa`` from tcl_obj (it lives there to avoid circular deps).
// The canonical implementation renders an integer *without* a trailing
// newline; callers that want one (``tcl_cmd_puts``) append it
// explicitly via ``fd_write_all(1, "\n", 1)`` after writing the digits.
pub const itoa = obj.itoa;

pub fn fd_write_all(fd: i32, data: [*]const u8, len: u32) void {
    const iov = [_]std.os.wasi.ciovec_t{.{
        .base = data,
        .len = len,
    }};
    var written: usize = 0;
    _ = std.os.wasi.fd_write(@intCast(fd), &iov, 1, &written);
}

// Internal: emit the rendered string of *value* to stdout, with or
// without a trailing newline.  ``tcl_cmd_puts`` / ``tcl_cmd_puts_
// nonewline`` share this helper.  ``itoa`` renders digits without a
// newline so we can append (or skip) it uniformly at the end,
// matching Tcl's ``puts`` / ``puts -nonewline`` contract.
fn puts_raw(value: i32, want_newline: bool) void {
    if (value == 0) {
        if (want_newline) fd_write_all(1, "\n", 1);
        return;
    }
    const s = obj_ensure_string(value);
    if (s.len > 0) {
        fd_write_all(1, @ptrFromInt(s.ptr), s.len);
    }
    if (want_newline) fd_write_all(1, "\n", 1);
}

// Exported: puts — write value to stdout via WASI fd_write.
pub export fn tcl_cmd_puts(value: i32) i32 {
    puts_raw(value, true);
    return 0;
}

// Exported: puts -nonewline — write value without appending a newline.
pub export fn tcl_cmd_puts_nonewline(value: i32) i32 {
    puts_raw(value, false);
    return 0;
}
