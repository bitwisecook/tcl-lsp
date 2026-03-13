# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::enable -- Enables the plugin in the flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__enable.html"


@register
class EcaEnableCommand(CommandDef):
    name = "ECA::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables the plugin in the flow.",
                synopsis=("ECA::enable",),
                snippet="The ECA::enable command enables the plugin in the flow.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
