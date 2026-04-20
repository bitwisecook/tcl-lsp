# Enriched from F5 iRules reference documentation.
"""ACCESS::restrict_irule_events -- Enable or disable HTTP and higher layer iRule events for the internal APM access control URIs."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__restrict_irule_events.html"


_av = make_av(_SOURCE)


@register
class AccessRestrictIruleEventsCommand(CommandDef):
    name = "ACCESS::restrict_irule_events"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::restrict_irule_events",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable or disable HTTP and higher layer iRule events for the internal APM access control URIs.",
                synopsis=("ACCESS::restrict_irule_events (enable | disable)",),
                snippet=(
                    "During access policy execution, ACCESS creates requests to various URIs\n"
                    "related to various access policy processing. These includes /my.policy\n"
                    "and other pages (logon, message box etc.) shown to the end user. By\n"
                    "default from 11.0.0 onward, HTTP and higher layer iRule events are not\n"
                    "raised for the internal access control URIs. All events except\n"
                    "ACCESS_SESSION_STARTED, ACCESS_SESSION_CLOSED,\n"
                    "ACCESS_POLICY_AGENT_EVENT, ACCESS_POLICY_COMPLETED are blocked (not\n"
                    "raised) for internal access control URI.\n"
                    "This command allows admin to overwrite the default behavior."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    ACCESS::restrict_irule_events disable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::restrict_irule_events (enable | disable)",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "ACCESS::restrict_irule_events enable",
                                "ACCESS::restrict_irule_events (enable | disable)",
                            ),
                            _av(
                                "disable",
                                "ACCESS::restrict_irule_events disable",
                                "ACCESS::restrict_irule_events (enable | disable)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
