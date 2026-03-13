# Enriched from F5 iRules reference documentation.
"""TCP::snd_ssthresh -- Returns the TCP slow start threshold (ssthresh)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__snd_ssthresh.html"


@register
class TcpSndSsthreshCommand(CommandDef):
    name = "TCP::snd_ssthresh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::snd_ssthresh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCP slow start threshold (ssthresh).",
                synopsis=("TCP::snd_ssthresh",),
                snippet=(
                    "The slow start threshold (ssthresh) is the point at which the\n"
                    "congestion window (cwnd) grows less aggressively. When the cwnd is\n"
                    "less than ssthresh, it roughly doubles for every cwnd worth of\n"
                    "acknowledged data. When cwnd is greater than ssthresh, it increases\n"
                    "by approximately one MSS for each cwnd worth of acknowledged data.\n"
                    "\n"
                    "ssthresh starts at 1,073,725,440 bytes unless there is a cmetrics\n"
                    "cache entry. When TCP detects packet loss it usually sets ssthresh\n"
                    "to a value between 1/2 cwnd and cwnd, depending on  connection\n"
                    "conditions and the congestion control algorithm."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # Get BIGIP's last slow-start threshold.\n"
                    '    log local0. "BIGIP\'s ssthresh: [TCP::snd_ssthresh]"\n'
                    "}"
                ),
                return_value="The connection slow start threshold in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::snd_ssthresh",
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
