# Enriched from F5 iRules reference documentation.
"""SIP::respond -- Terminates a SIP response and responds with one of your creation."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__respond.html"


@register
class SipRespondCommand(CommandDef):
    name = "SIP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Terminates a SIP response and responds with one of your creation.",
                synopsis=("SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?",),
                snippet=(
                    "This command allows you to terminate a SIP request and send a custom\n"
                    "formatted response directly from the iRule."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    "  log local0. [SIP::uri]\n"
                    "  log local0. [SIP::header Via 0]\n"
                    '  if {[SIP::method] == "INVITE"} {\n'
                    '    SIP::respond 401 "no way" X-Header "xxx here"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
