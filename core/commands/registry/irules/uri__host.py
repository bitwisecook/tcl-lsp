# Enriched from F5 iRules reference documentation.
"""URI::host -- Returns the host portion of a given URI."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__host.html"


@register
class UriHostCommand(CommandDef):
    name = "URI::host"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::host",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the host portion of a given URI.",
                synopsis=("URI::host URI_STRING",),
                snippet="Returns the host portion of a given URI.",
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "        # Loop through some test URLs and URIs and log the URI::host value\n"
                    "        foreach uri [list \\\n"
                    "                http://example.com/file.ext \\\n"
                    "                http://example.com:80/file.ext \\\n"
                    "                https://example.com:443/file.ext \\\n"
                    "                ftp://example.com/file.ext \\\n"
                    "                sip://example.com/file.ext \\\n"
                    "                myproto://example.com/file.ext \\\n"
                    "                /example.com \\\n"
                    "                /uri?url=http://example.com/uri \\\n"
                    "        ] {"
                ),
                return_value="Returns the host portion of a given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::host URI_STRING",
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
