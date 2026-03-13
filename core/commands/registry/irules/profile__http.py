# Enriched from F5 iRules reference documentation.
"""PROFILE::http -- Returns the value of an HTTP profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__http.html"


@register
class ProfileHttpCommand(CommandDef):
    name = "PROFILE::http"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::http",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of an HTTP profile setting.",
                synopsis=("PROFILE::http ATTR",),
                snippet="Returns the current value of the specified setting in the assigned HTTP profile.",
                source=_SOURCE,
                examples=(
                    "# For examples of the command output, add a simple logging iRule to a VIP:\n"
                    "when HTTP_REQUEST {\n"
                    '   log local0. "\\[PROFILE::http name\\]: [PROFILE::http name]"\n'
                    "}"
                ),
                return_value="Returns the current value of the specified setting in the assigned HTTP profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::http ATTR",
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
