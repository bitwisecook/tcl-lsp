# Generated from F5 iRules reference documentation -- do not edit manually.
"""check -- Set the iRule validation level."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/check.html"


@register
class CheckCommand(CommandDef):
    name = "check"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="check",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the iRule validation level.",
                synopsis=("check",),
                snippet="Set the iRule validation level: - none: disable the validation.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="check",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
