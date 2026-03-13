# Enriched from F5 iRules reference documentation.
"""TCP::bandwidth -- Returns the estimated bandwidth of the connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__bandwidth.html"


@register
class TcpBandwidthCommand(CommandDef):
    name = "TCP::bandwidth"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::bandwidth",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the estimated bandwidth of the connection.",
                synopsis=("TCP::bandwidth",),
                snippet=(
                    'Returns the estimated bandwidth measured as "congestion window" / "the measured round trip time".\n'
                    "The values returned are only estimates, and can vary even during the connection.\n"
                    "\n"
                    "Note: Starting with BIG-IP v9.4.2, client bandwidth calculations are unavailable, always returning 0. Starting with BIG-IP v12.0 nonzero values are returned."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    'if {[HTTP::uri] starts_with "/xxx.css"}{\n'
                    "      set bandwidth [TCP::bandwidth]\n"
                    "      if {$bandwidth < XXX} {\n"
                    '         HTTP::uri "/boring-xxx.css"\n'
                    "      }\n"
                    "   }\n"
                    "}"
                ),
                return_value="The estimated bandwidth in kilobits per second.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::bandwidth",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
