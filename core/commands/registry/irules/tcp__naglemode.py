# Enriched from F5 iRules reference documentation.
"""TCP::naglemode -- Returns setting of Nagle mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__naglemode.html"


@register
class TcpNaglemodeCommand(CommandDef):
    name = "TCP::naglemode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::naglemode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns setting of Nagle mode.",
                synopsis=("TCP::naglemode",),
                snippet="Returns the Nagle mode of a TCP flow.",
                source=_SOURCE,
                examples=(
                    "# Get the TCP Nagle mode of the TCP flow.\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "TCP Nagle mode: [TCP::naglemode]"\n'
                    "}"
                ),
                return_value="- enabled - disabled - auto",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::naglemode",
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
