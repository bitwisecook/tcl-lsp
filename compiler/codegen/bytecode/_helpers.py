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

"""Standalone helper functions for compile-time folding."""

from __future__ import annotations

from shared.tcl_list import tcl_list_split


def _split_list_simple(text: str) -> list[str]:
    """Split a constant Tcl list string into elements (compile-time only).

    Thin wrapper over the canonical :func:`shared.tcl_list.tcl_list_split`.
    """
    return tcl_list_split(text)


# Tcl 9.0 default trim characters — pushed when ``string trim`` is
# called without an explicit chars argument.  Includes ASCII whitespace,
# NUL, and all Unicode category Zs space separators.
_DEFAULT_TRIM_CHARS = (
    "\t\n\v\f\r "
    "\x00\x85\xa0"
    "\u1680\u180e"
    "\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a"
    "\u200b\u2028\u2029\u202f\u205f\u3000\ufeff"
)


def _tcl_string_hash(s: str) -> int:
    """Compute the Tcl string hash (matches ``Tcl_HashString``)."""
    h = 0
    for ch in s:
        h += (h << 3) + (ord(ch) & 0xFF)
        h &= 0xFFFFFFFF  # keep as uint32
    return h


def _tcl_hash_table_order(
    entries: list[tuple[str, str]],
) -> list[tuple[str, str]]:
    """Return jumpTable entries in Tcl hash-table iteration order.

    Tcl hash tables iterate buckets 0..N-1 and within each bucket
    entries are in LIFO order (most recently inserted first).
    The initial bucket count is 4.
    """
    num_buckets = 4
    # Grow buckets if needed (Tcl doubles when entries >= 3 * buckets).
    while len(entries) >= 3 * num_buckets:
        num_buckets *= 2
    # Assign entries to buckets in insertion order.
    buckets: dict[int, list[tuple[str, str]]] = {}
    for item in entries:
        bucket = _tcl_string_hash(item[0]) & (num_buckets - 1)
        buckets.setdefault(bucket, []).append(item)
    # Iterate buckets 0..N-1; within each bucket reverse (LIFO).
    result: list[tuple[str, str]] = []
    for b in range(num_buckets):
        if b in buckets:
            result.extend(reversed(buckets[b]))
    return result


def _tcl_list_element(s: str) -> str:
    """Format a string as a canonical Tcl list element (brace quoting)."""
    if not s:
        return "{}"
    # Characters that require quoting
    needs_quoting = False
    for ch in s:
        if ch in ' \t\n\r{}"\\;$[]':
            needs_quoting = True
            break
    if not needs_quoting:
        return s
    # Prefer brace quoting when the string has balanced braces
    # and does not end with a backslash
    if s[-1] != "\\" and s.count("{") == s.count("}"):
        return "{" + s + "}"
    # Fall back to backslash quoting
    out: list[str] = []
    for ch in s:
        if ch in ' \t\n\r{}"\\;$[]':
            out.append("\\")
        out.append(ch)
    return "".join(out)


def _parse_subst_template(
    template: str,
) -> list[tuple[str, str]] | None:
    """Parse a subst template into ``("lit", text)``, ``("var", name)``, and ``("cmd", "[...]")`` parts.

    Returns ``None`` if the template contains constructs we cannot inline.
    """
    parts: list[tuple[str, str]] = []
    i = 0
    n = len(template)
    buf: list[str] = []
    while i < n:
        ch = template[i]
        if ch == "[":
            if buf:
                parts.append(("lit", "".join(buf)))
                buf = []
            depth = 0
            start = i
            while i < n:
                if template[i] == "[":
                    depth += 1
                elif template[i] == "]":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                i += 1
            if depth != 0:
                return None
            parts.append(("cmd", template[start:i]))
            continue
        if ch == "\\":
            if i + 1 < n:
                next_ch = template[i + 1]
                # Apply Tcl backslash substitution for known escapes
                _bs_map = {
                    "a": "\a",
                    "b": "\b",
                    "f": "\f",
                    "n": "\n",
                    "r": "\r",
                    "t": "\t",
                    "v": "\v",
                    "\\": "\\",
                }
                buf.append(_bs_map.get(next_ch, next_ch))
                i += 2
            else:
                buf.append(ch)
                i += 1
            continue
        if ch == "$":
            if buf:
                parts.append(("lit", "".join(buf)))
                buf = []
            i += 1
            if i >= n:
                return None
            if template[i] == "=" and i + 1 < n and template[i + 1] == "{":
                end = template.find("}", i + 2)
                if end < 0:
                    return None
                parts.append(("scalar", template[i + 2 : end]))
                i = end + 1
            elif template[i] == "{":
                end = template.find("}", i + 1)
                if end < 0:
                    return None
                parts.append(("var", template[i + 1 : end]))
                i = end + 1
            else:
                start = i
                while i < n and (template[i].isalnum() or template[i] == "_"):
                    i += 1
                if i == start:
                    return None
                parts.append(("var", template[start:i]))
            continue
        buf.append(ch)
        i += 1
    if buf:
        parts.append(("lit", "".join(buf)))
    return parts if parts else None


def _regexp_to_glob(pattern: str) -> str | None:
    """Convert a simple anchored regex to an equivalent glob pattern.

    Handles:
    - ``{hello}`` → ``*hello*`` (unanchored)
    - ``{^abc}`` → ``abc*`` (left-anchored)
    - ``{abc$}`` → ``*abc`` (right-anchored)
    - ``{^abc$}`` → ``abc`` (fully anchored)
    """
    if pattern.startswith("{") and pattern.endswith("}"):
        pattern = pattern[1:-1]
    if not pattern:
        return None
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
    if any(c in _REGEX_SPECIAL for c in core):
        return None
    if left_anchor and right_anchor:
        return core
    if left_anchor:
        return core + "*"
    if right_anchor:
        return "*" + core
    return "*" + core + "*"
