# Enriched from F5 iRules reference documentation.
"""TCP::naglestate -- Returns current state of Nagle algorithm."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__naglestate.html"


@register
class TcpNaglestateCommand(CommandDef):
    name = "TCP::naglestate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::naglestate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns current state of Nagle algorithm.",
                synopsis=("TCP::naglestate",),
                snippet='If the Nagle mode is "enabled" or "disabled", it returns that mode. If "auto", it returns the current selection of the autotuning.',
                source=_SOURCE,
                examples=(
                    "# Get the TCP Nagle state of the TCP flow.\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "TCP Nagle state: [TCP::naglestate]"\n'
                    "}"
                ),
                return_value='The string "disabled" or "enabled"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::naglestate",
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
