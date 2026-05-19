"""Signature types and argument roles for command metadata.

This module is deliberately free of registry imports so it can be used
by command definition files without introducing circular dependencies.
"""

from __future__ import annotations

import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import ClassVar


class ArgRole(Enum):
    """What role an argument plays in a command."""

    BODY = auto()  # Tcl script body -- recursively analysed
    EXPR = auto()  # Expression (expr sub-language)
    VAR_WRITE = auto()  # A variable name written by the command (set, regexp, scan, lassign)
    VAR_READ = auto()  # A variable name read without modification (info exists, array get)
    LOOP_VAR_LIST = (
        auto()
    )  # Whitespace-separated list of loop variable names (dict for/map ``{key val}``)
    PARAM_LIST = auto()  # Procedure parameter list
    NAME = auto()  # A symbolic name (proc name, namespace name)
    PATTERN = auto()  # Pattern or regex
    OPTION = auto()  # A switch/flag
    VALUE = auto()  # Generic value argument
    SUBCOMMAND = auto()  # The subcommand word (e.g. "length" in "string length")
    OPTION_TERMINATOR = auto()  # The "--" option terminator
    CHANNEL = auto()  # Channel identifier (stdout, stdin, channelId)
    INDEX = auto()  # List/string index expression


# Deprecated module-level alias for the combined VAR_READ + VAR_WRITE role.
# Prefer assembling the frozenset inline at the declaration site
# (``frozenset({ArgRole.VAR_READ, ArgRole.VAR_WRITE})``) — this name remains
# only as a convenience for ``dict with`` / ``dict update``-style decls and
# may be removed in a future release.
VAR_READ_WRITE: frozenset[ArgRole] = frozenset({ArgRole.VAR_READ, ArgRole.VAR_WRITE})


class BodyKind(Enum):
    """How a ``BODY`` argument relates to its enclosing block.

    ``INLINE`` — the body executes in the caller's scope as part of the
    current control flow (``catch``, ``while``, ``foreach``, ``dict for``,
    …).  Variable references inside it must be discovered by the SSA scan
    so reads/writes are visible to the enclosing function's analysis.

    ``STRUCTURAL`` — the body defines its own scope and is lowered/analysed
    as a separate procedure or handler (``proc``, ``when``, ``tcltest::test``).
    Variable references inside the body are not part of the enclosing
    block's data flow and must be excluded from local statement scans.
    """

    INLINE = auto()
    STRUCTURAL = auto()


@dataclass(frozen=True, slots=True)
class Arity:
    """Argument count range for a command or subcommand.

    Attributes:
        min: Minimum number of arguments (excluding command name).
        max: Maximum number of arguments.  Defaults to ``Arity.ANY``
             (``sys.maxsize``), meaning no upper bound.
    """

    ANY: ClassVar[int] = sys.maxsize

    min: int = 0
    max: int = ANY

    @property
    def is_unlimited(self) -> bool:
        """True when there is no upper bound on argument count."""
        return self.max == Arity.ANY

    def accepts(self, n: int) -> bool:
        """True when *n* arguments satisfy this arity constraint."""
        return self.min <= n <= self.max


@dataclass(frozen=True, slots=True)
class CommandSig:
    """Signature for a simple command.

    Attributes:
        arity: Argument count bounds for the command.
        arg_roles: Maps argument index (0-based, after command name) to a
                   ``frozenset`` of roles.  An argument can carry multiple
                   roles at once (e.g. ``dict with`` arg 0 is both
                   ``VAR_READ`` and ``VAR_WRITE``).  Unlisted args default to
                   VALUE.
    """

    arity: Arity = field(default_factory=Arity)
    arg_roles: dict[int, frozenset[ArgRole]] = field(default_factory=dict)
    arg_role_resolver: Callable[[list[str]], dict[int, frozenset[ArgRole]]] | None = field(
        default=None, hash=False, compare=False
    )
    leading_options: frozenset[str] = field(default_factory=frozenset)
    """Declared option flags (e.g. ``-nonewline``, ``-nocase``).

    When non-empty, the arity checker skips leading arguments that match
    a declared option before counting positional arguments.  This lets
    ``Arity(1, 2)`` correctly accept ``puts -nonewline channel string``
    (3 raw args, but only 2 positional) while rejecting ``puts a b c``
    (3 positional args, exceeds max 2).
    """


@dataclass(frozen=True, slots=True)
class SubcommandSig:
    """Signature for a command with subcommands (e.g., namespace, dict, string).

    Attributes:
        subcommands: Maps subcommand name to its CommandSig.
        allow_unknown: If ``True``, unknown subcommands are ignored instead of
            reported as diagnostics (used for generated dialect packs).
    """

    subcommands: dict[str, CommandSig] = field(default_factory=dict)
    allow_unknown: bool = False
