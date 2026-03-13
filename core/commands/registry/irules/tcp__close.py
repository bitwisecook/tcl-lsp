# Enriched from F5 iRules reference documentation.
"""TCP::close -- Closes the TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__close.html"


@register
class TcpCloseCommand(CommandDef):
    name = "TCP::close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::close",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Closes the TCP connection.",
                synopsis=("TCP::close",),
                snippet="Sends the FIN byte to gracefully close the connection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    set my_loc "http://www.i-want-a-bigip-for-christmas.com"\n'
                    '    TCP::respond "HTTP/1.1 302 Found\\r\\nLocation: $my_loc\\r\\nConnection: close\\r\\nContent-Length: 0\\r\\n\\r\\n"\n'
                    "    TCP::close\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::close",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
