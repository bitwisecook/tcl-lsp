# Enriched from F5 iRules reference documentation.
"""UDP::local_port -- Returns the local UDP port/service number."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__local_port.html"


_av = make_av(_SOURCE)


@register
class UdpLocalPortCommand(CommandDef):
    name = "UDP::local_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::local_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the local UDP port/service number.",
                synopsis=("UDP::local_port (clientside | serverside)?",),
                snippet="Returns the local UDP port/service number.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [matchclass [UDP::local_port] equals $::ValidUDPPorts ] } {\n"
                    "    pool udp_pool\n"
                    "  } else {\n"
                    "     discard\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the local UDP port/service number",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::local_port (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "UDP::local_port clientside",
                                "UDP::local_port (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "UDP::local_port serverside",
                                "UDP::local_port (clientside | serverside)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="udp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
