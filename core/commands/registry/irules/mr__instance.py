# Enriched from F5 iRules reference documentation.
"""MR::instance -- Returns the name of the current mr_router instance."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__instance.html"


@register
class MrInstanceCommand(CommandDef):
    name = "MR::instance"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::instance",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name of the current mr_router instance.",
                synopsis=("MR::instance",),
                snippet="returns the name of the current mr_router instance",
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    '    log local0. "[MR::protocol] router instance [MR::instance]"\n'
                    "}"
                ),
                return_value="returns the name of the current mr_router instance",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::instance",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
