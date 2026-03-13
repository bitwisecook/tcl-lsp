# Enriched from F5 iRules reference documentation.
"""XML::enable -- Changes the XML plugin from passthrough to full patching mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XML__enable.html"


@register
class XmlEnableCommand(CommandDef):
    name = "XML::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the XML plugin from passthrough to full patching mode.",
                synopsis=("XML::enable",),
                snippet="Changes the XML plugin from passthrough to full patching mode.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XML::enable",
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
