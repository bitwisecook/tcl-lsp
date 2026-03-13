"""Type hints for command arguments and return values.

This module is deliberately free of registry imports so command
definition files can safely import from it without introducing
circular dependencies (same pattern as ``signatures.py``).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from ...compiler.types import TclType


@dataclass(frozen=True, slots=True)
class ArgTypeHint:
    """What intrep a command expects for an argument position.

    Attributes:
        expected: The Tcl type the argument should already have.
            ``None`` means no constraint.
        shimmers: ``True`` if the command forces conversion to this type,
            destroying the previous intrep.
    """

    expected: TclType | None = None
    shimmers: bool = False


@dataclass(frozen=True, slots=True)
class CommandTypeHint:
    """Type metadata for a simple command.

    Attributes:
        return_type: The Tcl type the command produces.
            ``None`` means unknown / variable.
        arg_types: Maps argument index (0-based after command name)
            to the expected type for that argument.
    """

    return_type: TclType | None = None
    arg_types: dict[int, ArgTypeHint] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class SubcommandTypeHint:
    """Type metadata for a subcommand-based command.

    Attributes:
        subcommands: Maps subcommand name to its type hints.
    """

    subcommands: dict[str, CommandTypeHint] = field(default_factory=dict)
