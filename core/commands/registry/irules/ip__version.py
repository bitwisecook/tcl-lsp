# Enriched from F5 iRules reference documentation.
"""IP::version -- Returns the IP version of a connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__version.html"


@register
class IpVersionCommand(CommandDef):
    name = "IP::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::version",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the IP version of a connection.",
                synopsis=("IP::version",),
                snippet="Returns the IP version of a connection. When called in a clientside event, this command returns the IP version for the clientside connection. When called in a serverside event, this command returns the IP version for the serverside connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '   log local0. "Client [IP::client_addr], VS: [IP::local_addr],\\\n'
                    '      \\[IP::version\\]: [IP::version], \\[IP::protocol\\]: [IP::protocol]"\n'
                    "}"
                ),
                return_value="IP version of a connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::version",
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
