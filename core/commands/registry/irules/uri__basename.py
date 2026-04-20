# Enriched from F5 iRules reference documentation.
"""URI::basename -- Extracts the basename part of a given uri string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__basename.html"


@register
class UriBasenameCommand(CommandDef):
    name = "URI::basename"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::basename",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Extracts the basename part of a given uri string.",
                synopsis=("URI::basename URI_STRING",),
                snippet=(
                    "Extracts the basename part of a given uri string.\n"
                    "For the following URI:\n"
                    "/main/index.jsp?user=test&login=check\n"
                    "\n"
                    "The basename is:\n"
                    "\n"
                    "index.jsp"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  set base [URI::basename [HTTP::uri]]\n"
                    '  log local0. "Basename of uri [HTTP::uri] is $base"\n'
                    "}"
                ),
                return_value="Return the basename part of a given uri string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::basename URI_STRING",
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
