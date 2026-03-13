# Enriched from F5 iRules reference documentation.
"""clientside -- Causes the specified iRule commands to be evaluated under the client-side context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/clientside.html"


@register
class ClientsideCommand(CommandDef):
    name = "clientside"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="clientside",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the specified iRule commands to be evaluated under the client-side context.",
                synopsis=("clientside (NESTING_SCRIPT)?",),
                snippet="Causes the specified iRule commands to be evaluated under the client-side context. This command has no effect if the iRule is already being evaluated under the client-side context. If there is no argument, the command returns 1 if the current event is in the clientside context or 0 if not.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "   # Check if the client IP address is 10.1.1.80\n"
                    "   # [clientside {IP::remote_addr}] is equivalent to [IP::client_addr]\n"
                    "   if { [IP::addr [clientside {IP::remote_addr}] equals 10.1.1.80] } {\n"
                    "      # Do something like drop the packets in this example\n"
                    "      discard\n"
                    "   }\n"
                    "}"
                ),
                return_value="clientside Returns 1 if the current event is in the clientside context or 0 if not.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="clientside (NESTING_SCRIPT)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
