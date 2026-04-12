# Generated from F5 iRules reference documentation -- do not edit manually.
"""tcpdump -- F5 iRules command `tcpdump`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/tcpdump.html"


@register
class TcpdumpCommand(CommandDef):
    name = "tcpdump"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcpdump",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `tcpdump`.",
                synopsis=("tcpdump",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tcpdump",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
