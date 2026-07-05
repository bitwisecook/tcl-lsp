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

"""Parse structured ``# tcl-lsp: stub`` comments and ``.tcl.stubs`` files.

Two delivery mechanisms:

1. **External stub files** — ``<dialect>.tcl.stubs``
   A standalone file containing only stub definitions (one per line,
   no ``#`` prefix required).  Loaded at workspace init or via
   ``package require``-style discovery.

2. **Inline stubs** — bracketed by markers in a ``.tcl`` source file::

       # tcl-lsp: stubs-begin
       # tcl-lsp: stub my_cmd {arg1:var body:body} -loop
       # tcl-lsp: stub my_eval {script:body} -barrier
       # tcl-lsp: stubs-end

   Stubs declared inside the markers are scoped to the enclosing file.
   Outside the markers, ``# tcl-lsp: stub`` lines are ignored by the
   analyser (they must be inside a stubs block).

Stub syntax (identical in both files and inline blocks)::

    stub <command-name> {arg1:role arg2 ?optArg:role?} ?flags...?

Argument roles (after the ``:``):

    body      — Tcl script body (recursively analysed)
    expr      — Expression (expr sub-language)
    var       — Variable name written by the command
    var_read  — Variable name read without modification
    name      — Symbolic name (proc name, namespace name)
    pattern   — Pattern or regex
    channel   — Channel identifier
    value     — Generic value (default when no role specified)

Flags:

    -barrier      — Command creates a dynamic barrier (like eval)
    -loop         — Command has a loop body
    -pure         — Command is side-effect-free
    -mutator      — Command mutates state
    -unsafe       — Command is unsafe
    -scope_alias  — Command creates a scope alias (like upvar)

A ``-loop`` stub is lowered to a real loop (so the loop variable stays
defined and the body is analysed in iteration order) when the call has
the single-iterator ``... varName:var collection body:body`` shape — a
leading subcommand word or option flags are tolerated.  Other shapes,
and non-loop ``body:body`` arguments, are treated as barriers: nested
commands are still checked, but the body's variable writes are not
threaded into the surrounding data flow (the command's body scope is
unknown, so assuming it runs in the caller's scope could be wrong).

Examples::

    # tcl-lsp: stubs-begin
    # tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
    # tcl-lsp: stub my_eval {script:body} -barrier
    # tcl-lsp: stub with_var {varName:var value} -mutator -scope_alias
    # tcl-lsp: stubs-end

External stub file (e.g. ``synopsys.tcl.stubs``)::

    stub foreach_in_collection {varName:var collection body:body} -loop
    stub define_design_lib {libName:name path}
    stub redirect {?-file? filename:value body:body}

Expression function and operator stubs::

    # tcl-lsp: stubs-begin
    # tcl-lsp: stub expr-func sizeof 1
    # tcl-lsp: stub expr-func clamp 3
    # tcl-lsp: stub expr-op contains 2
    # tcl-lsp: stub expr-op starts_with 2
    # tcl-lsp: stubs-end

In ``.tcl.stubs`` files::

    stub expr-func sizeof 1
    stub expr-op contains 2
"""

from __future__ import annotations

import contextlib
import contextvars
import re
from pathlib import Path
from typing import TYPE_CHECKING

from compiler.registry.stub_types import StubArgDef, StubCommandDef, StubExprDef
from shared.diagnostic import Range

if TYPE_CHECKING:
    from collections.abc import Iterable

# Workspace-level stub commands discovered from external ``.tcl.stubs``
# files.  Unlike inline stubs (scanned per-source), these apply to every
# file analysed under the active scope.  They feed the signature overlay
# and the compile cache fingerprint, but NOT the per-file shadow check
# (W116/W117) — their ``range`` points into a different document, so
# attaching diagnostics to the file under analysis would be wrong.
_ambient_cmd_stubs_var: contextvars.ContextVar[tuple[StubCommandDef, ...]] = contextvars.ContextVar(
    "tcl_lsp_ambient_cmd_stubs", default=()
)


def ambient_cmd_stubs() -> tuple[StubCommandDef, ...]:
    """Return the workspace stub commands active in the current scope."""
    return _ambient_cmd_stubs_var.get()


@contextlib.contextmanager
def ambient_stub_scope(cmd_stubs: "Iterable[StubCommandDef]"):
    """Make *cmd_stubs* the ambient workspace stubs for the ``with`` block.

    In-process callers (the LSP diagnostics pipeline and its thread
    fallbacks) use this so :func:`ambient_cmd_stubs` resolves to the
    workspace's external stubs.  Restored on exit — safe to nest.
    """
    token = _ambient_cmd_stubs_var.set(tuple(cmd_stubs))
    try:
        yield
    finally:
        _ambient_cmd_stubs_var.reset(token)


def set_ambient_stubs(cmd_stubs: "Iterable[StubCommandDef]") -> None:
    """Set the ambient workspace stubs without a restore token.

    For subprocess-pool workers, which do not share ContextVars with the
    parent process and re-establish per-request state on every call (the
    same pattern as ``configure_signatures``)."""
    _ambient_cmd_stubs_var.set(tuple(cmd_stubs))


_VALID_ROLES: frozenset[str] = frozenset(
    {"body", "expr", "var", "var_read", "name", "pattern", "channel", "value"}
)

_VALID_FLAGS: frozenset[str] = frozenset(
    {"-barrier", "-loop", "-pure", "-mutator", "-unsafe", "-scope_alias"}
)

# Matches a stub definition line (with or without ``# tcl-lsp:`` prefix).
# An optional subcommand word may appear between the command name and the
# braced argument list: ``stub db eval {sql script:body}``.  The
# subcommand cannot start with ``{`` (which would be the args list).
_STUB_RE = re.compile(
    r"stub\s+(\S+)(?:\s+([^\s{][^\s]*))?\s+\{([^}]*)\}(.*)",
    re.IGNORECASE,
)

# Matches an expr function/operator stub line.
_EXPR_STUB_RE = re.compile(
    r"stub\s+expr-(func|op)\s+(\S+)(?:\s+(\d+))?",
    re.IGNORECASE,
)

# Markers for inline stub blocks.
_STUBS_BEGIN_RE = re.compile(r"#\s*tcl-lsp:\s*stubs-begin", re.IGNORECASE)
_STUBS_END_RE = re.compile(r"#\s*tcl-lsp:\s*stubs-end", re.IGNORECASE)


def _strip_comment_prefix(line: str) -> str:
    """Strip ``#`` prefix and optional ``tcl-lsp:`` prefix from a line."""
    text = line.lstrip("# \t")
    lower = text.lower()
    if lower.startswith("tcl-lsp:"):
        text = text[len("tcl-lsp:") :].strip()
    return text


def parse_stub_line(line: str, line_range: Range) -> StubCommandDef | None:
    """Parse a single command stub definition line.

    Accepts both bare ``stub ...`` (from .tcl.stubs files) and
    ``# tcl-lsp: stub ...`` (from inline blocks) forms.
    Returns ``None`` for expr stubs (use :func:`parse_expr_stub_line`).
    """
    text = _strip_comment_prefix(line)

    m = _STUB_RE.match(text)
    if m is None:
        return None

    cmd_name = m.group(1)
    sub_name = m.group(2)
    args_str = m.group(3).strip()
    flags_str = m.group(4).strip()

    # ``stub expr-func`` / ``stub expr-op`` are expression stubs and
    # handled by :func:`parse_expr_stub_line`.  The regex would
    # otherwise match ``expr-func`` as the command name with
    # ``<name>`` as the subcommand word — bail out so the caller can
    # dispatch correctly.
    if cmd_name.lower().startswith("expr-"):
        return None

    args = _parse_args(args_str)
    if args is None:
        return None

    flags = _parse_flags(flags_str)

    return StubCommandDef(
        name=cmd_name,
        subcommand=sub_name,
        args=tuple(args),
        range=line_range,
        barrier="-barrier" in flags,
        loop="-loop" in flags,
        pure="-pure" in flags,
        mutator="-mutator" in flags,
        unsafe="-unsafe" in flags,
        scope_alias="-scope_alias" in flags,
    )


def parse_expr_stub_line(line: str, line_range: Range) -> StubExprDef | None:
    """Parse a single expr function/operator stub line.

    Syntax: ``stub expr-func <name> ?arity?``
            ``stub expr-op <name> ?arity?``
    """
    text = _strip_comment_prefix(line)

    m = _EXPR_STUB_RE.match(text)
    if m is None:
        return None

    kind = "function" if m.group(1).lower() == "func" else "operator"
    name = m.group(2)
    arity = int(m.group(3)) if m.group(3) else (1 if kind == "function" else 2)

    return StubExprDef(
        name=name,
        kind=kind,
        arity=arity,
        range=line_range,
    )


def parse_stubs_file(
    path: Path,
) -> tuple[list[StubCommandDef], list[StubExprDef]]:
    """Parse a ``.tcl.stubs`` file and return all valid stub definitions.

    Each non-blank, non-comment line is expected to be a stub definition.
    Lines starting with ``#`` that are not stub definitions are ignored
    as comments.

    Returns a tuple of (command_stubs, expr_stubs).
    """
    from shared.tokens import SourcePosition

    cmd_stubs: list[StubCommandDef] = []
    expr_stubs: list[StubExprDef] = []
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return cmd_stubs, expr_stubs

    for lineno, line in enumerate(text.splitlines()):
        stripped = line.strip()
        if not stripped or (stripped.startswith("#") and "stub " not in stripped.lower()):
            continue
        pos = SourcePosition(line=lineno, character=0, offset=0)
        line_range = Range(start=pos, end=pos)

        # Try expr stub first (expr-func / expr-op).
        expr_stub = parse_expr_stub_line(stripped, line_range)
        if expr_stub is not None:
            expr_stubs.append(expr_stub)
            continue

        # Try command stub.
        stub = parse_stub_line(stripped, line_range)
        if stub is not None:
            cmd_stubs.append(stub)

    return cmd_stubs, expr_stubs


def scan_source_for_stubs(
    source: str,
) -> tuple[list[StubCommandDef], list[StubExprDef]]:
    """Pre-scan source text for inline stubs blocks.

    Finds ``# tcl-lsp: stubs-begin`` / ``stubs-end`` markers and parses
    all stub definitions between them.  Multiple blocks are supported.

    This is a line-based scan independent of the Tcl parser, so it works
    even before lexing.  Stubs outside a begin/end block are ignored.
    """
    from shared.tokens import SourcePosition

    cmd_stubs: list[StubCommandDef] = []
    expr_stubs: list[StubExprDef] = []
    in_block = False

    for lineno, line in enumerate(source.splitlines()):
        stripped = line.strip()
        if not stripped.startswith("#"):
            continue

        if is_stubs_begin(stripped):
            in_block = True
            continue
        if is_stubs_end(stripped):
            in_block = False
            continue

        if not in_block:
            continue

        if "stub" not in stripped.lower():
            continue

        pos = SourcePosition(line=lineno, character=0, offset=0)
        line_range = Range(start=pos, end=pos)

        expr_stub = parse_expr_stub_line(stripped, line_range)
        if expr_stub is not None:
            expr_stubs.append(expr_stub)
            continue

        cmd_stub = parse_stub_line(stripped, line_range)
        if cmd_stub is not None:
            cmd_stubs.append(cmd_stub)

    return cmd_stubs, expr_stubs


def is_stubs_begin(comment: str) -> bool:
    """Return ``True`` if the comment is a ``# tcl-lsp: stubs-begin`` marker."""
    return bool(_STUBS_BEGIN_RE.search(comment))


def is_stubs_end(comment: str) -> bool:
    """Return ``True`` if the comment is a ``# tcl-lsp: stubs-end`` marker."""
    return bool(_STUBS_END_RE.search(comment))


def _parse_args(args_str: str) -> list[StubArgDef] | None:
    """Parse the argument list from a stub definition."""
    if not args_str:
        return []

    result: list[StubArgDef] = []
    for token in args_str.split():
        optional = False
        name = token

        # Handle optional args: ?argName? or ?argName:role?
        if name.startswith("?") and name.endswith("?"):
            optional = True
            name = name[1:-1]

        # Handle role annotation: argName:role
        if ":" in name:
            parts = name.split(":", 1)
            arg_name = parts[0]
            role = parts[1].lower()
            if role not in _VALID_ROLES:
                return None
        else:
            arg_name = name
            role = "value"

        if not arg_name:
            return None

        result.append(StubArgDef(name=arg_name, role=role, optional=optional))

    return result


def _parse_flags(flags_str: str) -> set[str]:
    """Parse flag arguments from a stub definition."""
    flags: set[str] = set()
    for token in flags_str.split():
        if token in _VALID_FLAGS:
            flags.add(token)
    return flags
