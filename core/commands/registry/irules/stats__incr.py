# Enriched from F5 iRules reference documentation.
"""STATS::incr -- Increments the value of a Statistics profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STATS__incr.html"


@register
class StatsIncrCommand(CommandDef):
    name = "STATS::incr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STATS::incr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Increments the value of a Statistics profile setting.",
                synopsis=("STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?",),
                snippet=(
                    "Increments the value of the specified setting (field), in the specified\n"
                    "Statistics profile, by the specified value. If you do not specify a\n"
                    "value, the system increments by 1. It is possible to set a negative\n"
                    "value in order to decrement the counter. Returns the current value of\n"
                    "the field which was incremented."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "\n"
                    "   # Increment the number of unanswered HTTP requests\n"
                    '   log local0. "Incremented the current count to: [STATS::incr my_stats_profile_name "current_count"]"\n'
                    "}"
                ),
                return_value="Returns the current value of the field which was incremented.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?",
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
