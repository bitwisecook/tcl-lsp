"""tcl::process -- Subprocess management ensemble (Tcl 9.0+, TIP 463).

The ``tcl::process`` command tracks subprocesses spawned via ``exec`` /
``open |cmd``.  Pure-host capability — never reachable in WASM/WASI builds
(no process model), so the registration exists for LSP recognition and the
runtime drops it on the trapping-stub list.

Two name forms are registered because Tcl callers may use the
namespace-relative spelling ``tcl::process`` (inside ``namespace eval
::tcl``) or the fully-qualified ``::tcl::process``.
"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page process.n"

_HOVER = HoverSnippet(
    summary="Manage subprocesses started by exec / open |cmd.",
    synopsis=("tcl::process subcommand ?arg ...?",),
    snippet=(
        "Sub-commands: `autopurge`, `list`, `purge`, `status`. "
        "Introduced in Tcl 9.0 by TIP 463. Process management is a "
        "host-only capability and is not available in WASM/WASI builds."
    ),
    source=_SOURCE,
)

_SUBCOMMANDS = (
    ArgumentValueSpec(
        value="autopurge", detail="Query or set automatic purge of terminated subprocesses."
    ),
    ArgumentValueSpec(value="list", detail="List PIDs of known subprocesses."),
    ArgumentValueSpec(value="purge", detail="Clean up terminated subprocesses."),
    ArgumentValueSpec(value="status", detail="Report exit status of subprocesses."),
)


def _ensemble_subcommands() -> dict[str, SubCommand]:
    return {
        "autopurge": SubCommand(
            name="autopurge",
            arity=Arity(0, 1),
            detail="Query (no args) or set (boolean) the autopurge flag.",
            synopsis="tcl::process autopurge ?flag?",
            return_type=TclType.BOOLEAN,
        ),
        "list": SubCommand(
            name="list",
            arity=Arity(0, 0),
            detail="Return list of subprocess PIDs.",
            synopsis="tcl::process list",
            return_type=TclType.LIST,
        ),
        "purge": SubCommand(
            name="purge",
            arity=Arity(0, 1),
            detail="Purge terminated subprocesses (optionally filtered by PID list).",
            synopsis="tcl::process purge ?pids?",
            return_type=TclType.STRING,
        ),
        "status": SubCommand(
            name="status",
            arity=Arity(0),
            detail="Return dict mapping PID -> status. `-wait` blocks until status available.",
            synopsis="tcl::process status ?switches? ?pids?",
            return_type=TclType.DICT,
            options=(
                OptionSpec(name="-wait", detail="Block until status is available."),
                OptionSpec(name="--", detail="Marks end of switches."),
            ),
        ),
    }


def _make_spec(name: str) -> CommandSpec:
    return CommandSpec(
        name=name,
        dialects=frozenset({"tcl9.0"}),
        hover=_HOVER,
        forms=(
            FormSpec(
                kind=FormKind.DEFAULT,
                synopsis="tcl::process subcommand ?arg ...?",
                arg_values={0: _SUBCOMMANDS},
            ),
        ),
        validation=ValidationSpec(arity=Arity(1)),
        subcommands=_ensemble_subcommands(),
        has_destructive_ops=True,
        side_effect_hints=(
            SideEffect(
                target=SideEffectTarget.UNKNOWN,
                reads=True,
                writes=True,
                connection_side=ConnectionSide.NONE,
            ),
        ),
    )


@register
class TclProcessCommand(CommandDef):
    name = "tcl::process"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _make_spec(cls.name)


@register
class TclProcessQualifiedCommand(CommandDef):
    name = "::tcl::process"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _make_spec(cls.name)
