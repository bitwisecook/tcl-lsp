# Enriched from F5 iRules reference documentation.
"""SCTP::server_port -- Returns the SCTP port/service number of the specified server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__server_port.html"


@register
class SctpServerPortCommand(CommandDef):
    name = "SCTP::server_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::server_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the SCTP port/service number of the specified server.",
                synopsis=("SCTP::server_port",),
                snippet=(
                    "Returns the SCTP port/service number of the specified server. This command is equivalent to the command serverside { SCTP::remote_port }.\n"
                    "\n"
                    "SCTP::server_port\n"
                    "    Returns the SCTP port/service number of the specified server."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    if { [SCTP::server_port] > 1000 } {\n"
                    "        pool slow_pool\n"
                    "     }\n"
                    "      else {\n"
                    "         pool fast_pool\n"
                    "       }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::server_port",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
