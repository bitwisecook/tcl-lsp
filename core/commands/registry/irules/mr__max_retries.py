# Enriched from F5 iRules reference documentation.
"""MR::max_retries -- Returns the number of retries allows for this router instance."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__max_retries.html"


@register
class MrMaxRetriesCommand(CommandDef):
    name = "MR::max_retries"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::max_retries",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number of retries allows for this router instance.",
                synopsis=("MR::max_retries",),
                snippet="returns the number of retries allowed",
                source=_SOURCE,
                examples=(
                    "when MR_FAILED {\n"
                    "    if {[MR::message retry_count] < [MR::max_retries]} {\n"
                    "        MR::message nexthop none\n"
                    "        MR::retry\n"
                    "    }\n"
                    "}"
                ),
                return_value="returns the number of retries allowed",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::max_retries",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
