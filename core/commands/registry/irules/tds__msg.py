# Generated from F5 iRules reference documentation -- do not edit manually.
"""TDS::msg -- Returns TDS message data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TDS__msg.html"


@register
class TdsMsgCommand(CommandDef):
    name = "TDS::msg"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TDS::msg",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns TDS message data.",
                synopsis=("TDS::msg",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TDS::msg",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"TDS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
