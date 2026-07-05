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

"""W216 — broken brace-form variants of array element access.

Two related shapes both look like array-element access but parse
differently than the user intends:

1. ``${arr}(foo)`` -- the lexer ends the variable substitution at the
   ``}``, so this is scalar ``${arr}`` followed by literal ``(foo)``.
   No array access happens at all.

2. ``${arr($foo)}`` -- the brace form has no further substitution
   (Tcl(n): "There is no further substitution or modification to its
   character contents"), so the chars ``$foo`` inside the braces are
   *literal* -- the runtime looks up element ``$foo`` (the four-char
   string), not the value of the variable ``foo``.  Only the bare
   form ``$arr($foo)`` runs index substitution.

Both are flagged W216 with a quick-fix replacement.

Quick-fix selection:

- Prefer the **bare** form ``$name(idx)`` when ``name`` is
  bare-compatible (alnum / ``_`` / ``::``).  Bare form is the only
  ``$``-substitution syntax that runs index substitution at runtime.
- When ``name`` cannot live in bare form (hyphens, spaces, etc.) emit
  ``[set "name(idx)"]``: ``set``'s argument is parsed by the command
  parser, which performs ``$``-substitution -- so an indirected
  ``$foo`` inside the index still expands correctly at runtime.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from shared.naming import is_bare_var_name
from shared.tokens import SourcePosition, Token, TokenType

from ..semantic_model import CodeFix, Diagnostic, Range, Severity


def _position_with_offset(base: SourcePosition, delta: int) -> SourcePosition:
    """Shift a SourcePosition by *delta* characters within the same line.

    The W216 pattern ``${...}(...)`` is always intra-line at the start
    (``${`` and the var name share a line) and at most spans lines for
    the index content; for the start position we always shift by ``-2``
    on the same line."""
    return SourcePosition(
        line=base.line,
        character=base.character + delta,
        offset=base.offset + delta,
    )


def _offset_to_position(source: str, offset: int) -> SourcePosition:
    """Linear-scan offset → ``SourcePosition``.  Used for the (rare)
    end position of a W216 match where the index spans multiple lines."""
    line = 0
    last_nl = -1
    for i in range(min(offset, len(source))):
        if source[i] == "\n":
            line += 1
            last_nl = i
    char = offset - last_nl - 1
    return SourcePosition(line=line, character=char, offset=offset)


def _build_replacement(name: str, inner: str) -> str:
    """Render the safe replacement for an array element reference.

    Bare ``$name(idx)`` is the only ``$``-form that substitutes ``$``
    inside the index, so prefer it when ``name`` allows it.  Otherwise
    fall back to ``[set "name(idx)"]`` -- the command-parser substitutes
    ``$``-vars in ``set``'s argument, so indirection keeps working.

    The bare-vs-fallback decision uses :func:`is_bare_var_name`
    (centralised in ``shared.naming``) so the completion path
    and this diagnostic pass cannot drift from the lexer rule."""
    if is_bare_var_name(name):
        return f"${name}({inner})"
    return f'[set "{name}({inner})"]'


def _index_has_substitution(inner: str) -> bool:
    """Return True if ``inner`` contains a ``$`` or ``[`` that the user
    likely expects to substitute -- the trigger for the
    ``${arr($foo)}`` variant of W216.  We look at the raw text and
    skip over backslash escapes."""
    i = 0
    n = len(inner)
    while i < n:
        c = inner[i]
        if c == "\\" and i + 1 < n:
            i += 2
            continue
        if c == "$" or c == "[":
            return True
        i += 1
    return False


def _varname_word_indices(cmd_name: str, args: list[str]) -> list[int]:
    """Indices into *args* (0-based, so word index ``i+1``) that a command
    interprets as a **variable name** — the positions where the braced
    indirect-array idiom ``${name}(idx)`` is the correct, intended access
    rather than a typo for ``$name(idx)``.

    Kept deliberately local (not shared with the W212 ``_NAME_ARG_INDICES``
    table) so this AST-walk pass does not reach across into
    ``analyser.checks``; the shapes are simple and stable.
    """
    if cmd_name in ("set", "incr", "append", "lappend", "vwait"):
        return [0] if args else []
    if cmd_name == "unset":
        # unset ?-nocomplain? ?--? varName ?varName ...?
        start = 0
        for i, a in enumerate(args):
            if a == "--":
                start = i + 1
                break
            if a.startswith("-"):
                start = i + 1
                continue
            start = i
            break
        return list(range(start, len(args)))
    if cmd_name == "info":
        # info exists varName
        return [1] if len(args) >= 2 and args[0] == "exists" else []
    return []


def _is_brace_form(tok: Token) -> bool:
    """Return True when ``tok`` is a ``${name}`` (brace-form) VAR token.

    Mirrors ``compiler/optimiser/_propagation._is_braced_var_token``:
    bare ``$name`` spans ``len(name) + 1`` chars (one ``$``); brace
    ``${name}`` spans ``len(name) + 2`` (``${`` + name, with ``end``
    pointing at the last char of the name, before ``}``)."""
    if tok.type is not TokenType.VAR:
        return False
    span = tok.end.offset - tok.start.offset + 1
    return span > len(tok.text) + 1


class _AnalyserDiagBraceThenParenMixin(_Base):
    """W216 pass: detect broken brace-form array element references."""

    def _emit_w216_for_command(
        self,
        all_tokens: list[Token],
        argv: list[Token],
        argv_texts: list[str],
        source: str,  # accepted for the call-site but unused
    ) -> None:
        """Walk command tokens looking for the two W216 patterns.

        Token offsets are absolute relative to the original document
        source, so we always index into ``self._source`` rather than
        the inner-body ``source`` argument (which would be e.g. just
        the CMD-substitution body for nested commands and would
        out-of-bounds at the absolute token offsets).

        ``argv`` / ``argv_texts`` describe the command's word structure so
        Pattern (1) can recognise a ``${name}(idx)`` word sitting in a
        *variable-name* position (``set``/``unset``/… target).  There the
        construct is the legitimate indirect-array-element idiom, not a typo —
        see :func:`_varname_word_indices`."""
        del source  # see docstring -- always use ``self._source``.
        source = self._source
        # Word-start offsets that the command reads as a variable name; a
        # ``${name}(idx)`` Pattern-(1) match starting there is the indirect
        # idiom and must not fire W216.
        varname_word_starts: set[int] = set()
        if argv_texts:
            for ai in _varname_word_indices(argv_texts[0], argv_texts[1:]):
                wi = ai + 1
                if wi < len(argv):
                    varname_word_starts.add(argv[wi].start.offset)
        for t1 in all_tokens:
            if not _is_brace_form(t1):
                continue
            text = t1.text

            # Pattern (2) -- ``${arr($foo)}``: the VAR token's own text
            # contains ``(...)`` with ``$``/``[`` inside.  Tcl reads this
            # as variable named literally ``arr($foo)`` (no further
            # substitution), but the user almost always meant the bare
            # form which *does* substitute the index.
            if "(" in text and text.endswith(")"):
                paren_idx = text.index("(")
                name = text[:paren_idx]
                inner = text[paren_idx + 1 : -1]
                if name and _index_has_substitution(inner):
                    corrected = _build_replacement(name, inner)
                    start_pos = t1.start  # already at the leading ``$``
                    # Token end is the last char of the name+(idx) text;
                    # the closing ``}`` sits at end.offset + 1.  The
                    # analysis Range.end is *inclusive*, so point at
                    # the ``}`` itself.
                    end_pos = _offset_to_position(source, t1.end.offset + 1)
                    full_range = Range(start=start_pos, end=end_pos)
                    self.result.diagnostics.append(
                        Diagnostic(
                            range=full_range,
                            severity=Severity.WARNING,
                            code="W216",
                            message=(
                                f"``${{{name}({inner})}}`` does not substitute ``{inner}`` "
                                "(the brace form is documented to apply no further "
                                f"substitution to its content); use ``{corrected}`` "
                                "to access the array element with index substitution"
                            ),
                            fixes=(
                                CodeFix(
                                    range=full_range,
                                    new_text=corrected,
                                    description=f"Replace with ``{corrected}``",
                                ),
                            ),
                        )
                    )
                continue

            # Pattern (1) -- ``${arr}(foo)``: the VAR token ends at the
            # last char before ``}``, a literal ``}`` follows, then ``(``
            # opens a paren group.  Almost always a typo for the bare or
            # brace-array form.
            close_brace = t1.end.offset + 1
            if close_brace >= len(source) or source[close_brace] != "}":
                continue
            paren_start = close_brace + 1
            if paren_start >= len(source) or source[paren_start] != "(":
                continue
            paren_end = _find_matching_close_paren(source, paren_start)
            if paren_end is None:
                continue
            # In a variable-name position ``set ${token}(idx) …`` is the
            # indirect-array-element idiom (``token`` holds the array name),
            # not a broken ``$token(idx)``; suppress W216 there.  A
            # value-position ``puts ${arr}(x)`` still fires.
            if t1.start.offset in varname_word_starts:
                continue
            inner = source[paren_start + 1 : paren_end]
            corrected = _build_replacement(t1.text, inner)
            start_pos = t1.start  # already at the leading ``$``
            # Range.end is inclusive -- point at the ``)`` itself.
            end_pos = _offset_to_position(source, paren_end)
            full_range = Range(start=start_pos, end=end_pos)
            self.result.diagnostics.append(
                Diagnostic(
                    range=full_range,
                    severity=Severity.WARNING,
                    code="W216",
                    message=(
                        f"``${{{t1.text}}}({inner})`` is parsed as scalar "
                        f"``${{{t1.text}}}`` followed by literal text ``({inner})``; "
                        f"did you mean ``{corrected}`` for array element access?"
                    ),
                    fixes=(
                        CodeFix(
                            range=full_range,
                            new_text=corrected,
                            description=f"Replace with ``{corrected}``",
                        ),
                    ),
                )
            )


def _find_matching_close_paren(source: str, paren_start: int) -> int | None:
    """Find the offset of the ``)`` matching the ``(`` at ``paren_start``.

    Skips over balanced ``{...}`` literals, double-quoted strings, and
    backslash escapes so a ``)`` inside one of those doesn't mis-close
    the outer paren.  Returns ``None`` on malformed input."""
    depth = 1
    j = paren_start + 1
    in_quote = False
    n = len(source)
    while j < n and depth > 0:
        c = source[j]
        if in_quote:
            if c == "\\" and j + 1 < n:
                j += 2
                continue
            if c == '"':
                in_quote = False
            j += 1
            continue
        if c == '"':
            in_quote = True
            j += 1
            continue
        if c == "{":
            bd = 1
            j += 1
            while j < n and bd > 0:
                if source[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                if source[j] == "{":
                    bd += 1
                elif source[j] == "}":
                    bd -= 1
                j += 1
            continue
        if c == "\\" and j + 1 < n:
            j += 2
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        j += 1
    if depth != 0:
        return None
    return j - 1
