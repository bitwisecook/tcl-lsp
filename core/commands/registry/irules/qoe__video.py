# Generated from F5 iRules reference documentation -- do not edit manually.
"""QOE::video -- Deprecated: Returns a set of video QOE attributes from the current video connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/QOE__video.html"


@register
class QoeVideoCommand(CommandDef):
    name = "QOE::video"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="QOE::video",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Returns a set of video QOE attributes from the current video connection.",
                synopsis=("QOE::video",),
                snippet="This command returns a set of video QOE attributes from the current video connection.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="QOE::video",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"QOE"}), also_in=frozenset({"CLIENT_CLOSED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
