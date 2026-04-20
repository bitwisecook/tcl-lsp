# Enriched from F5 iRules reference documentation.
"""MR::retry -- Send the current message to the router for routing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__retry.html"


@register
class MrRetryCommand(CommandDef):
    name = "MR::retry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::retry",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Send the current message to the router for routing.",
                synopsis=("MR::retry",),
                snippet="The MR::retry command instructs the Message Routing Framework submit the current message to the router to retry routing. This will clear the message's route status. The script may need to clear the message's existing nexthop and route fields if a new routetable lookup is desired. If a persistence record exists for this message, it may also need to be reset.",
                source=_SOURCE,
                examples=(
                    "when MR_FAILED {\n"
                    "    if {[MR::retry_count] < 2} {\n"
                    "        MR::message nexthop none\n"
                    '        MR::message route config tc1 host "10.1.1.1:1234"\n'
                    "        MR::retry\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::retry",
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
