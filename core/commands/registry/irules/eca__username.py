# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::username -- Returns NTLM authenticating username."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__username.html"


@register
class EcaUsernameCommand(CommandDef):
    name = "ECA::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns NTLM authenticating username.",
                synopsis=("ECA::username",),
                snippet="The ECA::username command returns NTLM authenticating username.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::username",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
