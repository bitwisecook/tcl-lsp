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

"""Bytecoded codegen for ``regexp``."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def _regexp_to_glob(pattern: str) -> str | None:
    """Convert a simple regexp pattern to a glob pattern for strmatch.

    Returns the glob string, or None if the pattern is too complex.
    Handles:
    - ``{hello}`` → ``*hello*`` (unanchored)
    - ``{^abc}`` → ``abc*`` (left-anchored)
    - ``{abc$}`` → ``*abc`` (right-anchored)
    - ``{^abc$}`` → ``abc`` (fully anchored)
    """
    # Strip braces if present
    if pattern.startswith("{") and pattern.endswith("}"):
        pattern = pattern[1:-1]
    if not pattern:
        return None
    # Check for special regex chars (beyond ^ and $)
    _REGEX_SPECIAL = set(r".+*?[](){}|\\")
    left_anchor = pattern.startswith("^")
    right_anchor = pattern.endswith("$")
    core = pattern
    if left_anchor:
        core = core[1:]
    if right_anchor:
        core = core[:-1]
    if not core:
        return None
    # Reject if the core contains any regex-special characters
    if any(c in _REGEX_SPECIAL for c in core):
        return None
    # Build glob
    if left_anchor and right_anchor:
        return core
    if left_anchor:
        return core + "*"
    if right_anchor:
        return "*" + core
    return "*" + core + "*"


def codegen_regexp(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``regexp`` to specialised opcodes."""
    if not (2 <= len(args) <= 4):
        return False
    # Check for -nocase: regexp -nocase ?--? pattern string
    # tclsh converts simple -nocase regexps to strmatch +1.
    nocase = False
    rargs = list(args)
    if rargs and rargs[0] == "-nocase":
        nocase = True
        rargs.pop(0)
    if rargs and rargs[0] == "--":
        rargs.pop(0)
    if len(rargs) == 2 and nocase:
        glob = _regexp_to_glob(rargs[0])
        if glob is not None:
            emitter._push_lit(glob)
            emitter._emit_value(rargs[1])
            emitter._emit(Op.STR_MATCH, 1)
            emitter._emit(Op.POP)
            return True
    # Simple regexp: regexp ?--? pattern string
    if len(rargs) == 2 and not nocase:
        for a in args:
            emitter._emit_value(a)
        # tclsh encodes argc + 1 as the regexp operand
        emitter._emit(Op.REGEXP, len(args) + 1)
        emitter._emit(Op.POP)
        return True
    return False


def register() -> None:
    from compiler.registry import REGISTRY

    REGISTRY.register_codegen("regexp", codegen_regexp)
