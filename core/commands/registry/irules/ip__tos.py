# Enriched from F5 iRules reference documentation.
"""IP::tos -- Returns (or sets) the ToS value encoded within a packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__tos.html"


_av = make_av(_SOURCE)


@register
class IpTosCommand(CommandDef):
    name = "IP::tos"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::tos",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns (or sets) the ToS value encoded within a packet.",
                synopsis=("IP::tos (clientside | serverside)? (IP_TOS)?",),
                snippet="Returns (or sets) the ToS value encoded within a packet. The Type of Service (ToS) standard is a means by which network equipment can identify and treat traffic differently based on an identifier. As traffic enters the site, the BIG-IP system can apply a rule that sends the traffic to different pools of servers based on the ToS level within a packet, or can set the ToS value on traffic matching specific patterns.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [IP::tos] == 64 } {\n"
                    "     pool telnet_pool\n"
                    "  } else {\n"
                    "     pool slow_pool\n"
                    " }\n"
                    "}"
                ),
                return_value="Returns the ToS value encoded within a packet",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::tos (clientside | serverside)? (IP_TOS)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "IP::tos clientside",
                                "IP::tos (clientside | serverside)? (IP_TOS)?",
                            ),
                            _av(
                                "serverside",
                                "IP::tos serverside",
                                "IP::tos (clientside | serverside)? (IP_TOS)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
