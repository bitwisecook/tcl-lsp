# Enriched from F5 iRules reference documentation.
"""MR::release -- Releases the data collected via MR::collect iRule command."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__release.html"


@register
class MrReleaseCommand(CommandDef):
    name = "MR::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the data collected via MR::collect iRule command.",
                synopsis=("MR::release",),
                snippet=(
                    "Releases the payload data collected via MR::collect iRule command for further processing.\n"
                    "\n"
                    "This command is valid only when MR::collect has been called."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
