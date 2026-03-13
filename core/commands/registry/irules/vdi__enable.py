# Generated from F5 iRules reference documentation -- do not edit manually.
"""VDI::enable -- Enable VDI plugin."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/VDI__enable.html"


@register
class VdiEnableCommand(CommandDef):
    name = "VDI::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="VDI::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable VDI plugin.",
                synopsis=("VDI::enable",),
                snippet="The VDI::enable command enables VDI plugin in the flow.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="VDI::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP"}), also_in=frozenset({"CLIENT_ACCEPTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
