# Enriched from F5 iRules reference documentation.
"""POP3::activation_mode -- Set the activation mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/POP3__activation_mode.html"


_av = make_av(_SOURCE)


@register
class Pop3ActivationModeCommand(CommandDef):
    name = "POP3::activation_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POP3::activation_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the activation mode.",
                synopsis=("POP3::activation_mode (none | allow | require)?",),
                snippet="Sets the activation mode to none (it will never activate), allow (if the POP3 client sends STARTTLS, we will activate TLS), or require (all commands will be rejected until STARTTLS is received).",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { !([IP::addr [IP::client_addr] ne 10.0.0.0/8) } {\n"
                    "                    POP3::activation_mode require\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POP3::activation_mode (none | allow | require)?",
                    arg_values={
                        0: (
                            _av(
                                "none",
                                "POP3::activation_mode none",
                                "POP3::activation_mode (none | allow | require)?",
                            ),
                            _av(
                                "allow",
                                "POP3::activation_mode allow",
                                "POP3::activation_mode (none | allow | require)?",
                            ),
                            _av(
                                "require",
                                "POP3::activation_mode require",
                                "POP3::activation_mode (none | allow | require)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
