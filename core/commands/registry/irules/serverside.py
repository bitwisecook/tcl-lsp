# Enriched from F5 iRules reference documentation.
"""serverside -- Causes the specified iRule command to be evaluated under the server-side context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/serverside.html"


@register
class ServersideCommand(CommandDef):
    name = "serverside"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="serverside",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the specified iRule command to be evaluated under the server-side context.",
                synopsis=("serverside (NESTING_SCRIPT)?",),
                snippet="Causes the specified iRule command or commands to be evaluated under the server-side context. This command has no effect if the iRule is already being evaluated under the server-side context. If there is no argument, the command returns 1 if the current event is in the serverside context or 0 if not.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "\n"
                    "   # Check if the server (pool member) IP address is 10.1.1.80\n"
                    "   # [serverside {IP::remote_addr}] is equivalent to [IP::server_addr]\n"
                    "   if { [IP::addr [serverside {IP::remote_addr}] equals 10.1.1.80] } {\n"
                    "\n"
                    "      # Do something like drop the packets in this example\n"
                    "      discard\n"
                    "   }\n"
                    "}"
                ),
                return_value="serverside Returns 1 if the current event is in the serverside context or 0 if not.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="serverside (NESTING_SCRIPT)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
