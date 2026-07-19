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

"""Single-pass Tcl analyser.

Walks the token stream from TclLexer, builds a semantic model of the source:
scopes, proc definitions, variable definitions, and emits diagnostics for
detectable errors.
"""

from __future__ import annotations

import io
import logging
import re
from collections.abc import Sequence

from compiler.ir import (
    IRAssignConst,
    IRAssignValue,
    IRStatement,
)
from compiler.parsing.argv import widen_argv_tokens_to_word_spans
from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.green_tree import tokenise
from compiler.parsing.token_positions import token_content_base
from compiler.registry import REGISTRY
from shared.codes import diag
from shared.document_buffer import DocumentBuffer
from shared.naming import normalise_var_name
from shared.tokens import Token, TokenType

from ..semantic_model import (
    _NOQA_ALL,
    ParamDef,
    Range,
)

log = logging.getLogger(__name__)

_WORD_CLOSERS = {'"': '"', "{": "}", "[": "]"}


def _inclusive_word_end(source: str, tok_type: TokenType, start_off: int, end_off: int) -> int:
    """Inclusive offset of a word's last character in whole *source*.

    Internal ``Range`` ends are inclusive (the LSP layer makes them
    exclusive).  ``Token.end.offset`` points at the last *content* character;
    widen by one to swallow the closing delimiter the lexer omits from
    braced/quoted/bracketed words (and the ``}`` of a ``${var}``).  The check
    runs against the full source so a truncated statement span — which itself
    may drop the closer — does not hide it."""
    if tok_type in (TokenType.STR, TokenType.CMD, TokenType.ESC):
        if 0 <= start_off < len(source):
            closer = _WORD_CLOSERS.get(source[start_off])
            if closer and end_off + 1 < len(source) and source[end_off + 1] == closer:
                return end_off + 1
    elif (
        tok_type is TokenType.VAR
        and source[start_off : start_off + 2] == "${"
        and end_off + 1 < len(source)
        and source[end_off + 1] == "}"
    ):
        return end_off + 1
    return end_off


def _find_read_var_token(
    tokens: Sequence[Token], target: str, *, max_depth: int = 8
) -> Token | None:
    """Find the first ``$target`` read token, descending into ``[cmd]`` subs.

    A read-before-set can occur inside a command substitution (``[list ... $x]``,
    ``return [foo $x]``), where a flat scan of the statement's own words finds
    no matching ``VAR`` token and the diagnostic falls back to the whole
    statement.  Recurse into ``CMD`` words to locate the real read; braced
    ``{...}`` words are skipped because their ``$var`` is literal text, not a
    substitution at this scope.  Returned positions share the caller's base.
    """
    for tok in tokens:
        if tok.type is TokenType.VAR and normalise_var_name(tok.text) == target:
            return tok
    if max_depth <= 0:
        return None
    for tok in tokens:
        if tok.type is TokenType.CMD:
            base_offset, base_line, base_col = token_content_base(tok)
            sub_tokens, _ = tokenise(tok.text, base_offset, base_line, base_col)
            found = _find_read_var_token(sub_tokens, target, max_depth=max_depth - 1)
            if found is not None:
                return found
    return None


def narrow_to_variable(
    source: str,
    buffer: DocumentBuffer,
    stmt_range: Range,
    *,
    variable: str,
    kind: str,
) -> Range | None:
    """Narrow a whole-statement range to the token that concerns *variable*.

    Re-segments the statement's own source span (so the work is token-based,
    not text scanning) and locates:

    - ``"assigned_name"`` — the assignment target (second word of ``set`` etc.);
    - ``"read_var"`` — the ``$var`` reference being read;
    - ``"named_arg"`` — a bareword argument naming the variable (``unset x``).

    Returns *None* when the token cannot be located, so callers fall back to
    the statement range rather than emitting a worse one.
    """
    base = stmt_range.start.offset
    seg = source[base : stmt_range.end.offset + 1]
    cmds = segment_commands(seg)
    if not cmds or not cmds[0].argv:
        return None
    cmd = cmds[0]

    tok: Token | None = None
    if kind == "assigned_name":
        if len(cmd.argv) >= 2:
            tok = cmd.argv[1]
    elif kind == "read_var":
        target = normalise_var_name(variable)
        tok = _find_read_var_token(cmd.all_tokens, target)
    elif kind == "named_arg":
        target = normalise_var_name(variable)
        tok = next(
            (t for t in cmd.argv[1:] if normalise_var_name(t.text) == target),
            None,
        )
    if tok is None:
        return None

    start_off = base + tok.start.offset
    end_off = _inclusive_word_end(source, tok.type, start_off, base + tok.end.offset)
    return Range(
        start=buffer.offset_to_position(start_off),
        end=buffer.offset_to_position(end_off),
    )


# iRules commands that are only valid at the top level of an iRule script.
# Derived from ``irules_top_level_only`` on CommandSpec at first use.
_IRULES_TOP_LEVEL_ONLY: frozenset[str] | None = None


def _irules_top_level_only() -> frozenset[str]:
    global _IRULES_TOP_LEVEL_ONLY  # noqa: PLW0603
    if _IRULES_TOP_LEVEL_ONLY is None:
        _IRULES_TOP_LEVEL_ONLY = REGISTRY.irules_top_level_only_commands()
    return _IRULES_TOP_LEVEL_ONLY


# Module-level registrations for codes emitted from class methods.
diag("E200", "Shimmer parse error — internal representation cannot be determined.", section="error")
diag("E101", "Syntax error — unclosed bracket.", section="error", internal=True)
diag("E103", "Syntax error — unexpected token.", section="error", internal=True)
diag(
    "H300",
    "Possible paste error — repeated assignment to same variable with same value.",
    section="hint",
)
diag(
    "I230",
    "Constant branch condition — the alternate branch is provably unreachable.",
    section="hint",
)
diag(
    "I231",
    "Constant switch arm condition — the arm is provably unreachable.",
    section="hint",
)
diag("W113", "Procedure shadows built-in command.", section="warning")
diag("IRULE5006", "Top-level-only command used inside a nested body.", section="irules")
diag(
    "IRULE5007", "Event-context command used at top level outside a `when` block.", section="irules"
)
diag("W116", "Stub command shadows built-in command.", section="warning")
diag("W117", "Stub expression definition shadows built-in function or operator.", section="warning")
diag("W124", "Invalid IP address literal.", section="warning")
diag(
    "W125",
    "Orphaned control-flow keyword used as standalone command.",
    section="warning",
)
diag("W126", "Non-channel value in channel argument position.", section="warning")
diag("W210", "Variable read before set.", section="variable")
diag("W211", "Variable set but never used.", section="variable")
diag(
    "W213",
    "Variable may not exist — use `unset -nocomplain` to suppress the error.",
    section="variable",
)
diag(
    "W214",
    "Unused proc parameter — argument is declared but never read in the procedure body.",
    section="variable",
)
diag(
    "W215",
    "Variable name unreachable via $-substitution (creatable via set/info exists/upvar but no $-form can read it).",
    section="variable",
)
diag(
    "W216",
    "Broken brace-form array element reference — ``${arr}(x)`` parses as scalar+literal, ``${arr($foo)}`` does not substitute the index.",
    section="variable",
)
diag("W220", "Dead store — variable set but overwritten before use.", section="variable")
diag(
    "IRULE4005",
    "Potential race — `static::` variable written outside `RULE_INIT` and read in another event.",
    section="irules_variable",
)
diag(
    "IRULE5005",
    "Direct proc invocation without `call` — use `call proc_name`.",
    section="irules",
)
diag(
    "W123",
    "Unresolved command — not found in registry, user procs, or `unknown` handler.",
    section="hint",
    default=False,
)

# Short names: d = Diagnostic, m = regex Match, r = Range,
# s = Scope, t = Token, p = ParamDef.


_UNUSED_VAR_RE = re.compile(r"Variable '([^']+)' is set but never used")

# Matches ``# tcl-lsp: disable=CODE1,CODE2`` (or ``=*``) at the start of a line,
# case-insensitive.  Commas and whitespace separate codes.
_FILE_DIRECTIVE_RE = re.compile(
    r"^\s*#\s*tcl-lsp\s*:\s*disable\s*=\s*([^\r\n]+?)\s*$",
    re.IGNORECASE,
)

# Cap on how many leading lines we scan for file-wide directives.  The loop
# also stops at the first non-comment, non-blank line, so this only matters
# for pathological files that are entirely comments.
_FILE_DIRECTIVE_SCAN_LINES = 100


def parse_file_suppression(source: str) -> frozenset[str]:
    """Extract file-wide diagnostic suppression from top-of-file directives.

    Scans leading comment/blank lines for ``# tcl-lsp: disable=CODE1,CODE2``
    (or ``=*`` for all codes).  Stops at the first line that is neither blank
    nor a ``#`` comment.  Multiple directives accumulate.
    """
    codes: set[str] = set()
    for idx, line in enumerate(io.StringIO(source)):
        if idx >= _FILE_DIRECTIVE_SCAN_LINES:
            break
        raw = line.rstrip("\r\n")
        stripped = raw.strip()
        if not stripped:
            continue
        if not stripped.startswith("#"):
            break
        match = _FILE_DIRECTIVE_RE.match(raw)
        if not match:
            continue
        for token in match.group(1).replace(",", " ").split():
            token = token.strip()
            if token:
                codes.add(token)
    return frozenset(codes)


def parse_noqa_line_suppressions(source: str) -> dict[int, frozenset[str]]:
    """Scan source for ``# noqa`` comments and record next-line suppressions.

    For each ``# noqa`` / ``# noqa: CODE`` comment on line N, stores
    suppression codes for line N+1.  This handles two cases the
    command-level ``preceding_comment`` mechanism cannot reach:

    * A noqa comment at the tail of a brace body (orphaned — no following
      command in that scope), where the diagnostic fires on the immediately
      following ``} elseif …`` or similar line.
    * A noqa comment immediately before another comment line that itself
      generates a diagnostic (e.g. W115 on a ``\\``-continued comment).
    """
    result: dict[int, frozenset[str]] = {}
    for i, line in enumerate(source.split("\n")):
        stripped = line.strip()
        if not stripped.startswith("#"):
            continue
        lower = stripped.lower()
        noqa_pos = lower.find("noqa")
        if noqa_pos < 0:
            continue
        rest = stripped[noqa_pos + 4 :].strip()
        if rest.startswith(":"):
            codes: frozenset[str] = frozenset(c.strip() for c in rest[1:].split(",") if c.strip())
        else:
            codes = _NOQA_ALL
        next_line = i + 1
        existing = result.get(next_line)
        if existing is not None:
            result[next_line] = existing | codes
        else:
            result[next_line] = codes
    return result


def _possible_paste_fingerprint(stmt: IRStatement) -> tuple[str, str] | None:
    """Return (variable, static_value) for static assignments worth heuristic checks."""
    if isinstance(stmt, IRAssignConst):
        return (stmt.name, stmt.value.strip())

    if isinstance(stmt, IRAssignValue):
        value = stmt.value.strip()
        if not value:
            return None
        if "$" in value or "[" in value or "]" in value:
            return None
        return (stmt.name, value)

    return None


def _format_literal_for_message(value: str) -> str:
    """Keep heuristic diagnostic literals short and single-line."""
    display = value.replace("\n", "\\n")
    if len(display) > 40:
        return display[:37] + "..."
    return display


def _argv_with_word_spans(argv: list[Token], all_tokens: list[Token]) -> list[Token]:
    """Return argv tokens widened to each Tcl word's full token span."""
    return widen_argv_tokens_to_word_spans(argv, all_tokens)


def parse_param_list(param_str: str) -> list[ParamDef]:
    """Parse a Tcl proc argument list string into ParamDef objects.

    Handles:  "a b c"  and  "a {b default} c"
    """
    params: list[ParamDef] = []
    # Simple word-level parse -- braced items contain defaults
    i = 0
    text = param_str.strip()
    while i < len(text):
        # skip whitespace
        while i < len(text) and text[i] in " \t\n\r":
            i += 1
        if i >= len(text):
            break

        if text[i] == "{":
            # Braced parameter with possible default
            level = 1
            i += 1
            start = i
            while i < len(text) and level > 0:
                if text[i] == "{":
                    level += 1
                elif text[i] == "}":
                    level -= 1
                i += 1
            inner = text[start : i - 1].strip()
            parts = inner.split(None, 1)
            if len(parts) == 2:
                params.append(ParamDef(name=parts[0], has_default=True, default_value=parts[1]))
            elif len(parts) == 1:
                params.append(ParamDef(name=parts[0]))
        else:
            # Bare word
            start = i
            while i < len(text) and text[i] not in " \t\n\r":
                i += 1
            word = text[start:i]
            if word:
                params.append(ParamDef(name=word))

    return params
