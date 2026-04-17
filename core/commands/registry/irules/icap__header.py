# Enriched from F5 iRules reference documentation.
"""ICAP::header -- Sets or returns ICAP attributes in the ICAP header."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ICAP__header.html"


_av = make_av(_SOURCE)


@register
class IcapHeaderCommand(CommandDef):
    name = "ICAP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ICAP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns ICAP attributes in the ICAP header.",
                synopsis=(
                    "ICAP::header 'names'",
                    "ICAP::header 'at' HEADER_INDEX",
                    "ICAP::header 'count' (HEADER_NAME)?",
                    "ICAP::header 'exists' HEADER_NAME",
                ),
                snippet="The ICAP::header command sets or returns attributes in the ICAP header.",
                source=_SOURCE,
                examples=(
                    "when ICAP_RESPONSE {\n"
                    "                ICAP::header remove X-ICAP-my-custom-header\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ICAP::header 'names'",
                    arg_values={
                        0: (
                            _av("names", "ICAP::header names", "ICAP::header 'names'"),
                            _av("at", "ICAP::header at", "ICAP::header 'at' HEADER_INDEX"),
                            _av(
                                "count", "ICAP::header count", "ICAP::header 'count' (HEADER_NAME)?"
                            ),
                            _av(
                                "exists", "ICAP::header exists", "ICAP::header 'exists' HEADER_NAME"
                            ),
                        )
                    },
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
