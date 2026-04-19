# Enriched from F5 iRules reference documentation.
"""URI::protocol -- Returns the protocol of the given URI."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__protocol.html"


@register
class UriProtocolCommand(CommandDef):
    name = "URI::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the protocol of the given URI.",
                synopsis=("URI::protocol URI_STRING",),
                snippet="Returns the protocol of the given URI.",
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "        # Loop through some test URLs and URIs and log the URI::protocol value\n"
                    "        foreach uri [list \\\n"
                    "                http://test.com \\\n"
                    "                https://test.com \\\n"
                    "                ftp://test.com \\\n"
                    "                sip://test.com \\\n"
                    "                myproto://test.com \\\n"
                    "                /test.com \\\n"
                    "                /uri?url=http://test.example.com/uri \\\n"
                    "        ] {\n"
                    '                log local0. "\\[URI::protocol $uri\\]: [URI::protocol $uri]"\n'
                    "        }\n"
                    "}"
                ),
                return_value="Returns the protocol of the given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::protocol URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
