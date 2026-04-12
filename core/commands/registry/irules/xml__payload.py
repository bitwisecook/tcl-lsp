# Enriched from F5 iRules reference documentation.
"""XML::payload -- Queries for or manipulates XML payload."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XML__payload.html"


@register
class XmlPayloadCommand(CommandDef):
    name = "XML::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or manipulates XML payload.",
                synopsis=(
                    "XML::payload (LENGTH | (OFFSET LENGTH))?",
                    "XML::payload length",
                    "XML::payload replace OFFSET LENGTH XML_PAYLOAD",
                ),
                snippet="Queries for or manipulates XML payload.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XML::payload (LENGTH | (OFFSET LENGTH))?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
