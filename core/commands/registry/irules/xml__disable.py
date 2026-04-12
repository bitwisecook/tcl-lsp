# Enriched from F5 iRules reference documentation.
"""XML::disable -- Changes the XML plugin from full patching mode to passthrough."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XML__disable.html"


@register
class XmlDisableCommand(CommandDef):
    name = "XML::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the XML plugin from full patching mode to passthrough.",
                synopsis=("XML::disable",),
                snippet="Changes the XML plugin from full patching mode to passthrough.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XML::disable",
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
