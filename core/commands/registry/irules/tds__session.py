# Generated from F5 iRules reference documentation -- do not edit manually.
"""TDS::session -- Returns TDS session data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TDS__session.html"


@register
class TdsSessionCommand(CommandDef):
    name = "TDS::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TDS::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns TDS session data.",
                synopsis=("TDS::session",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TDS::session",
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
