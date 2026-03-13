# Enriched from F5 iRules reference documentation.
"""event -- Enables or disables evaluation of the specified iRule event or all iRule events on this connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/event.html"


_av = make_av(_SOURCE)


@register
class EventCommand(CommandDef):
    name = "event"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="event",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables or disables evaluation of the specified iRule event or all iRule events on this connection.",
                synopsis=(
                    "event info",
                    "event (enable | disable) ('all')?",
                    "event EVENTNAME (enable | disable)",
                ),
                snippet=(
                    "Enables or disables evaluation of the specified iRule event, or all\n"
                    "iRule events, on this connection. However, the iRule continues to run.\n"
                    "\n"
                    "**Pattern \u2014 after drop/reject**: Always follow `drop` or `reject`\n"
                    "with `event disable all` and `return` to prevent other iRules from\n"
                    "running on the now-invalid connection."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n  COMPRESS::method prefer gzip\n  event disable\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="event info",
                    arg_values={
                        0: (
                            _av("all", "event all", "event (enable | disable) ('all')?"),
                            _av("enable", "event enable", "event (enable | disable) ('all')?"),
                            _av("disable", "event disable", "event (enable | disable) ('all')?"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
