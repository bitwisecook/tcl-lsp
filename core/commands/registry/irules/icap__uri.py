# Enriched from F5 iRules reference documentation.
"""ICAP::uri -- Sets or returns the ICAP request URI."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ICAP__uri.html"


@register
class IcapUriCommand(CommandDef):
    name = "ICAP::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ICAP::uri",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the ICAP request URI.",
                synopsis=("ICAP::uri (URI_STRING)?",),
                snippet="The ICAP::uri command sets or returns the ICAP request URI.",
                source=_SOURCE,
                examples=(
                    "when ICAP_REQUEST {\n"
                    '                if {[ICAP::uri] contains "movie"} {\n'
                    "                    ICAP::uri http://icap.mydomain.org/video\n"
                    "                }\n"
                    "            }"
                ),
                return_value="Returns the ICAP request URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ICAP::uri (URI_STRING)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ICAP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
