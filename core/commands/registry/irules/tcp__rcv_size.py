# Enriched from F5 iRules reference documentation.
"""TCP::rcv_size -- Returns the maximum allowed advertised window size by BIG-IP."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rcv_size.html"


@register
class TcpRcvSizeCommand(CommandDef):
    name = "TCP::rcv_size"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rcv_size",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the maximum allowed advertised window size by BIG-IP.",
                synopsis=("TCP::rcv_size",),
                snippet="TCP configuration limits the advertised received window to control the memory impact of any single connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # Get BIGIP's receive window size.\n"
                    '    log local0. "BIGIP\'s rcv wnd size: [TCP::rcv_size]"\n'
                    "}"
                ),
                return_value="The maximum receive window in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rcv_size",
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
