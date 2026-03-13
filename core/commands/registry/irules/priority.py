# Enriched from F5 iRules reference documentation.
"""priority -- Sets the order of execution for iRule events."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/priority.html"


@register
class PriorityCommand(CommandDef):
    name = "priority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="priority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the order of execution for iRule events.",
                synopsis=("priority EVENT_PRIORITY",),
                snippet=(
                    "The priority command is used as an attribute associated with any iRule\n"
                    "event. When the iRules are loaded into the internal iRules engine for a\n"
                    "given virtual server, they are stored in a table with the event name\n"
                    "and a priority (with a default of 500).\n"
                    "Lower numbered priority events are evaluated before higher numbered\n"
                    "priority events: When an event is triggered an event, the irules engine\n"
                    "passes control to each of the code blocks for that given event in the\n"
                    "order of lowest to highest priority."
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_ACCEPTED {\n       log "Client [IP::remote_addr] connected"\n    }'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="priority EVENT_PRIORITY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
