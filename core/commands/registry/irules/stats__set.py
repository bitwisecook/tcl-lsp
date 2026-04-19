# Enriched from F5 iRules reference documentation.
"""STATS::set -- Sets the value of a Statistics profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STATS__set.html"


@register
class StatsSetCommand(CommandDef):
    name = "STATS::set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STATS::set",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the value of a Statistics profile setting.",
                synopsis=("STATS::set PROFILE_NAME FIELD_NAME (VALUE)?",),
                snippet=(
                    "Sets the value of the specified setting (field), in the specified\n"
                    "Statistics profile, to the specified value. If you do not specify a\n"
                    "value, the BIG-IP system sets the value to 0."
                ),
                source=_SOURCE,
                examples=(
                    "# Workaround for CR117956 (see version specific notes below for details)\n"
                    "when RULE_INIT {\n"
                    "    set ::stats_rst 1\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STATS::set PROFILE_NAME FIELD_NAME (VALUE)?",
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
