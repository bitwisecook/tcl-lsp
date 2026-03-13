# Generated from F5 iRules reference documentation -- do not edit manually.
"""QOE::enable -- Deprecated: Enables the video QOE filter and allows processing video on a connection basis."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/QOE__enable.html"


@register
class QoeEnableCommand(CommandDef):
    name = "QOE::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="QOE::enable",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Enables the video QOE filter and allows processing video on a connection basis.",
                synopsis=("QOE::enable",),
                snippet="This command enables the video QOE filter and allows processing video on a connection basis.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="QOE::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CLASSIFICATION", "FASTHTTP", "QOE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
