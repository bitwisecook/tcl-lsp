# Enriched from F5 iRules reference documentation.
"""ICAP::method -- Returns the ICAP request method."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ICAP__method.html"


@register
class IcapMethodCommand(CommandDef):
    name = "ICAP::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ICAP::method",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the ICAP request method.",
                synopsis=("ICAP::method",),
                snippet=(
                    "The ICAP::method command returns the ICAP request method.\n"
                    'This will either be "REQMOD" or "RESPMOD"'
                ),
                source=_SOURCE,
                examples=(
                    "when ICAP_REQUEST {\n"
                    '                if {[ICAP::method] == "REQMOD"} {\n'
                    "                    ICAP::header add X-Request $seqno\n"
                    "                }\n"
                    "            }"
                ),
                return_value="Returns the ICAP request method.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ICAP::method",
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
