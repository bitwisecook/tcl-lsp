# Enriched from F5 iRules reference documentation.
"""PEM::disable -- PEM iRule command to disable PEM feature on current flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PEM__disable.html"


@register
class PemDisableCommand(CommandDef):
    name = "PEM::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PEM::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="PEM iRule command to disable PEM feature on current flow.",
                synopsis=("PEM::disable",),
                snippet="Disable PEM for the current flow. Note that the config must already contain a Policy Enforcement Profile.",
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    PEM::disable;\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PEM::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
