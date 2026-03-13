# Enriched from F5 iRules reference documentation.
"""TCP::rtt -- Returns the smoothed round-trip time estimate for a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rtt.html"


@register
class TcpRttCommand(CommandDef):
    name = "TCP::rtt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rtt",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the smoothed round-trip time estimate for a TCP connection.",
                synopsis=("TCP::rtt",),
                snippet="Returns the smoothed round-trip time estimate for a TCP connection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "clientside { set rtt [TCP::rtt] }\n"
                    "if {$rtt < 1600 } {\n"
                    '      log "NOcompress rtt=$rtt"\n'
                    "      COMPRESS::disable\n"
                    "   }\n"
                    "else {\n"
                    '      log "compress rtt=$rtt"\n'
                    "      COMPRESS::enable\n"
                    "      COMPRESS::gzip level 9\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rtt",
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
