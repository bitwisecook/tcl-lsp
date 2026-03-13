# Enriched from F5 iRules reference documentation.
"""ONECONNECT::detach -- Detaches server-side OneConnect connections."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ONECONNECT__detach.html"


@register
class OneconnectDetachCommand(CommandDef):
    name = "ONECONNECT::detach"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ONECONNECT::detach",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Detaches server-side OneConnect connections.",
                synopsis=("ONECONNECT::detach BOOL_VALUE",),
                snippet=(
                    "Controls the behavior of a server-side connection when a OneConnect\n"
                    "profile is on the virtual server. The default behavior is that the\n"
                    "server-side connection detaches after each response is completed, and a\n"
                    "new load balancing decision and persistence look-up are performed for\n"
                    "every request.\n"
                    "Disabling detaching prevents this behavior.\n"
                    'Note: the use of the terms "request" and "response" imply the presence\n'
                    "of a supported layer 7 profile (e.g. the HTTP profile) on the virtual\n"
                    "server. An iRule can also detaching the server-side connection using\n"
                    "the LB::detach command."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    if { $headreq } {\n"
                    "        # Response to HEAD request. Detach after done.\n"
                    "        ONECONNECT::detach enable\n"
                    "        ONECONNECT::reuse enable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ONECONNECT::detach BOOL_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
