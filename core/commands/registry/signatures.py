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
    VAR_NAME = auto()  # A variable name written by the command (set/unset/incr)
    VAR_READ = auto()  # A variable name read without modification (info exists, array get)
    PARAM_LIST = auto()  # Procedure parameter list
    NAME = auto()  # A symbolic name (proc name, namespace name)
    PATTERN = auto()  # Pattern or regex
    OPTION = auto()  # A switch/flag
    VALUE = auto()  # Generic value argument
    SUBCOMMAND = auto()  # The subcommand word (e.g. "length" in "string length")
    OPTION_TERMINATOR = auto()  # The "--" option terminator
    CHANNEL = auto()  # Channel identifier (stdout, stdin, channelId)
    INDEX = auto()  # List/string index expression


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
        arg_roles: Maps argument index (0-based, after command name) to role.
                   Unlisted args default to VALUE.
    """

    arity: Arity = field(default_factory=Arity)
    arg_roles: dict[int, ArgRole] = field(default_factory=dict)
    arg_role_resolver: Callable[[list[str]], dict[int, ArgRole]] | None = field(
        default=None, hash=False, compare=False
    )


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
