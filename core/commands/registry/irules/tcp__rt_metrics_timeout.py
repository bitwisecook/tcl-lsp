# Enriched from F5 iRules reference documentation.
"""TCP::rt_metrics_timeout -- Sets cmetrics cache entry lifetime (timeout)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rt_metrics_timeout.html"


@register
class TcpRtMetricsTimeoutCommand(CommandDef):
    name = "TCP::rt_metrics_timeout"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rt_metrics_timeout",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets cmetrics cache entry lifetime (timeout).",
                synopsis=("TCP::rt_metrics_timeout TIMEOUT",),
                snippet="If the TCP profile enables cmetrics-cache, then the entries there remain for a number of seconds equivalent to cmetrics-cache-timeout. This iRule supercedes that setting.",
                source=_SOURCE,
                examples=("when CLIENT_CLOSED {\n    TCP::rt_metrics_timeout 300\n}"),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rt_metrics_timeout TIMEOUT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
