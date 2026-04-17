# Enriched from F5 iRules reference documentation.
"""STATS::setmin -- Ensures that the value of a Statistics profile setting is at the most value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STATS__setmin.html"


@register
class StatsSetminCommand(CommandDef):
    name = "STATS::setmin"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STATS::setmin",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Ensures that the value of a Statistics profile setting is at the most value.",
                synopsis=("STATS::setmin PROFILE_NAME FIELD_NAME (VALUE)?",),
                snippet=(
                    "Ensures that the value of the specified Statistics profile setting\n"
                    "(field) is at the most value."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STATS::setmin PROFILE_NAME FIELD_NAME (VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
