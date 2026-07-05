# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Wire-format markers shared by the compiler and the VM.

These three constants are not user-visible: they're embedded inside literal
strings during compilation and stripped out again by the runtime. They exist
because the bytecode argument vector is just `list[str]`, so the compiler has
no other way to tell the runtime "this argument was originally braced" or
"this string contains raw ``$`` / ``[`` characters that must NOT be
substituted".

``_BRACE_OPEN`` / ``_BRACE_CLOSE`` mark a braced word. They use NUL bytes
plus a brace so no legitimate Tcl source can produce them by accident.

``_RAW_PREFIX`` marks a literal whose ``$`` / ``[`` characters came from
backslash escapes and must be pushed verbatim.
"""

from __future__ import annotations

_BRACE_OPEN = "\x00\x01{"
_BRACE_CLOSE = "}\x01\x00"
_RAW_PREFIX = "\x00\x02"

__all__ = ["_BRACE_OPEN", "_BRACE_CLOSE", "_RAW_PREFIX"]
