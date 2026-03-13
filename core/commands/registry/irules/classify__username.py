# Enriched from F5 iRules reference documentation.
"""CLASSIFY::username -- Assigns username to the flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFY__username.html"


@register
class ClassifyUsernameCommand(CommandDef):
    name = "CLASSIFY::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFY::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Assigns username to the flow.",
                synopsis=("CLASSIFY::username USERNAME (CONTEXT)?",),
                snippet=(
                    "This command assigns username to the flow. It could be used for\n"
                    "reporting / statistics / etc."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    CLASSIFY::username superuser\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFY::username USERNAME (CONTEXT)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
