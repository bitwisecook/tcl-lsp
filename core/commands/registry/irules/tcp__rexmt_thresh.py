# Enriched from F5 iRules reference documentation.
"""TCP::rexmt_thresh -- This command can be used to set/get the retransmission threshold of a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rexmt_thresh.html"


@register
class TcpRexmtThreshCommand(CommandDef):
    name = "TCP::rexmt_thresh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rexmt_thresh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the retransmission threshold of a TCP connection.",
                synopsis=("TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?",),
                snippet=(
                    "TCP::rexmt_thresh returns the retransmission threshold of a TCP connection.\n"
                    "TCP::rexmt_thresh TCP_REXMT_THRESH_VALUE sets the retransmission threshold to specified value."
                ),
                source=_SOURCE,
                examples=(
                    "t/set the retransmission threshold of the TCP flow.\n"
                    "    when CLIENT_ACCEPTED {\n"
                    '        log local0. "TCP set rtx thresh: [TCP::rexmt_thresh 100]"\n'
                    '        log local0. "TCP get rtx thresh: [TCP::rexmt_thresh]"\n'
                    "    }"
                ),
                return_value="TCP::rexmt_thresh returns the retransmission threshold of a TCP connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?",
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
