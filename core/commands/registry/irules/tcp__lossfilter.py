# Enriched from F5 iRules reference documentation.
"""TCP::lossfilter -- Sets the TCP Loss Ignore Parameters."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__lossfilter.html"


@register
class TcpLossfilterCommand(CommandDef):
    name = "TCP::lossfilter"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::lossfilter",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the TCP Loss Ignore Parameters.",
                synopsis=("TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST",),
                snippet=(
                    "Sets the maximum size burst loss (in packets) and maximum number of packets per million lost before triggering congestion response.\n"
                    "  * Burst range is valid from 0 to 32. Higher values decrease the\n"
                    "    chance of performing congestion control.\n"
                    "  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n"
                    "    million before congestion control kicks in."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side loss filter.\n"
                    "    # Ignore up to 150 losses per million packets and burst losses of up to 10 packets.\n"
                    "    clientside {\n"
                    "        TCP::lossfilter 150 10\n"
                    "    }\n"
                    "    # No loss filter on server-side.\n"
                    "    serverside {\n"
                    "        TCP::lossfilter 0 0\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
