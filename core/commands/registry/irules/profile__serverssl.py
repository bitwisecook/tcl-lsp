# Enriched from F5 iRules reference documentation.
"""PROFILE::serverssl -- Returns the value of a Server SSL profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__serverssl.html"


@register
class ProfileServersslCommand(CommandDef):
    name = "PROFILE::serverssl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::serverssl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of a Server SSL profile setting.",
                synopsis=("PROFILE::serverssl ATTR",),
                snippet="Returns the current value of the specified setting in the assigned Server SSL profile.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if {[PROFILE::exists serverssl] == 1}{\n"
                    '        log local0. "server SSL profile enabled: [PROFILE::serverssl name]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns the current value of the specified setting in the assigned Server SSL profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::serverssl ATTR",
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
