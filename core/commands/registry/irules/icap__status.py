# Enriched from F5 iRules reference documentation.
"""ICAP::status -- Returns the ICAP response status code."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ICAP__status.html"


@register
class IcapStatusCommand(CommandDef):
    name = "ICAP::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ICAP::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the ICAP response status code.",
                synopsis=("ICAP::status",),
                snippet=(
                    "The ICAP::status command gets the ICAP response status code. For\n"
                    "example, 100, 200, 204."
                ),
                source=_SOURCE,
                examples=(
                    "when ICAP_RESPONSE {\n"
                    "                if {[ICAP::status] == 204} {\n"
                    '                    log local0. "ICAP server responded 204"\n'
                    "                }\n"
                    "            }"
                ),
                return_value="Return the ICAP response status code.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ICAP::status",
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
