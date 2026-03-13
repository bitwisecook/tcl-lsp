"""history -- Tcl history command ensemble (auto-loaded, no package require)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl stdlib history command"

_av = make_av(_SOURCE)


@register
class HistoryCommand(CommandDef):
    name = "history"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="history",
            unsafe=True,
            # No required_package — history is auto-loaded with Tcl.
            hover=HoverSnippet(
                summary="Manipulate the history list of previously executed commands.",
                synopsis=(
                    "history",
                    "history add command ?exec?",
                    "history change newValue ?event?",
                    "history clear",
                    "history event ?event?",
                    "history info ?count?",
                    "history keep ?count?",
                    "history nextid",
                    "history redo ?event?",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="history subcommand ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "add",
                                "Add a command to the history list.",
                                "history add command ?exec?",
                            ),
                            _av(
                                "change",
                                "Replace a history event.",
                                "history change newValue ?event?",
                            ),
                            _av("clear", "Clear the history list.", "history clear"),
                            _av(
                                "event",
                                "Return a history event by number or pattern.",
                                "history event ?event?",
                            ),
                            _av("info", "Return a formatted history list.", "history info ?count?"),
                            _av(
                                "keep",
                                "Get or set the size of the history list.",
                                "history keep ?count?",
                            ),
                            _av("nextid", "Return the next event number.", "history nextid"),
                            _av("redo", "Re-evaluate a history event.", "history redo ?event?"),
                        ),
                    },
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(1, 2),
                    detail="Add a command to the history list.",
                    synopsis="history add command ?exec?",
                ),
                "change": SubCommand(
                    name="change",
                    arity=Arity(1, 2),
                    detail="Replace a history event.",
                    synopsis="history change newValue ?event?",
                ),
                "clear": SubCommand(
                    name="clear",
                    arity=Arity(0, 0),
                    detail="Clear the history list.",
                    synopsis="history clear",
                ),
                "event": SubCommand(
                    name="event",
                    arity=Arity(0, 1),
                    detail="Return a history event by number or pattern.",
                    synopsis="history event ?event?",
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(0, 1),
                    detail="Return a formatted history list.",
                    synopsis="history info ?count?",
                ),
                "keep": SubCommand(
                    name="keep",
                    arity=Arity(0, 1),
                    detail="Get or set the size of the history list.",
                    synopsis="history keep ?count?",
                ),
                "nextid": SubCommand(
                    name="nextid",
                    arity=Arity(0, 0),
                    detail="Return the next event number.",
                    synopsis="history nextid",
                ),
                "redo": SubCommand(
                    name="redo",
                    arity=Arity(0, 1),
                    detail="Re-evaluate a history event.",
                    synopsis="history redo ?event?",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(0),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
