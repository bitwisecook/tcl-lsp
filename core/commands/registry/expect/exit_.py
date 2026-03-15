"""exit -- Exit expect (with onexit/noexit support)."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exit(1)"


@register
class ExitCommand(CommandDef):
    name = "exit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exit",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Exit Expect, optionally running an onexit handler.",
                synopsis=(
                    "exit ?-onexit command? ?status?",
                    "exit ?-noexit? ?status?",
                ),
                snippet=(
                    "With ``-onexit``, registers a handler to run at exit. "
                    "With ``-noexit``, prepares for exit but does not actually "
                    "exit (useful for cleaning up in libraries)."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exit ?-onexit command | -noexit? ?status?",
                    options=(
                        OptionSpec(
                            name="-onexit",
                            takes_value=True,
                            value_hint="command",
                            detail="Register a handler to run at exit.",
                        ),
                        OptionSpec(name="-noexit", detail="Prepare for exit without exiting."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_roles={0: ArgRole.BODY},
        )
