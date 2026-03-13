# Enriched from F5 iRules reference documentation.
"""SIP::response -- Gets or rewrites the SIP response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__response.html"


_av = make_av(_SOURCE)


@register
class SipResponseCommand(CommandDef):
    name = "SIP::response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::response",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or rewrites the SIP response.",
                synopsis=(
                    "SIP::response (code | phrase)",
                    "SIP::response rewrite CODE (PHRASE)?",
                ),
                snippet=(
                    "These commands allow you to get or rewrite the SIP response code or\nphrase."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_RESPONSE {\n"
                    "  log local0. [SIP::via 0]\n"
                    "  SIP::header remove Via 0\n"
                    '  SIP::response rewrite 123 "no xxx"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::response (code | phrase)",
                    arg_values={
                        0: (
                            _av("code", "SIP::response code", "SIP::response (code | phrase)"),
                            _av("phrase", "SIP::response phrase", "SIP::response (code | phrase)"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
