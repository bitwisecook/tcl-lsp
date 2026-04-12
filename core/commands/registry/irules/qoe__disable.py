# Generated from F5 iRules reference documentation -- do not edit manually.
"""QOE::disable -- Deprecated: Disables the video QOE filter from processing any video or non-video traffic on a connection basis."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/QOE__disable.html"


@register
class QoeDisableCommand(CommandDef):
    name = "QOE::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="QOE::disable",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Disables the video QOE filter from processing any video or non-video traffic on a connection basis.",
                synopsis=("QOE::disable",),
                snippet="This command disables the video QOE filter from processing any video or non-video traffic on a connection basis.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="QOE::disable",
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
