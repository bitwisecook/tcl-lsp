# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::disable -- Disables the plugin in the flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__disable.html"


@register
class EcaDisableCommand(CommandDef):
    name = "ECA::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables the plugin in the flow.",
                synopsis=("ECA::disable",),
                snippet="The ECA::disable command disables the plugin in the flow.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::disable",
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
