# Enriched from F5 iRules reference documentation.
"""UDP::remote_port -- Returns the remote UDP port/service number."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__remote_port.html"


_av = make_av(_SOURCE)


@register
class UdpRemotePortCommand(CommandDef):
    name = "UDP::remote_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::remote_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the remote UDP port/service number.",
                synopsis=("UDP::remote_port (clientside | serverside)?",),
                snippet="Returns the remote UDP port/service number.",
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '  SIP::header insert "Via" "[lindex [split [SIP::via 0] ";"] 0]received=[IP::client_addr]rport=[UDP::remote_port][lindex [split [SIP::via 0] ";"] 1]"\n'
                    '  SIP::header remove "Via" 1\n'
                    "}"
                ),
                return_value="Returns the remote UDP port/service number",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::remote_port (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "UDP::remote_port clientside",
                                "UDP::remote_port (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "UDP::remote_port serverside",
                                "UDP::remote_port (clientside | serverside)?",
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
