# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::status -- Returns NTLM authentication result."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__status.html"


@register
class EcaStatusCommand(CommandDef):
    name = "ECA::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns NTLM authentication result.",
                synopsis=("ECA::status",),
                snippet="The ECA::status command returns NTLM authentication result such as NTLM_STATUS_OK, NTLM_STATUS_WRONG_PASSWORD, NTLM_STATUS_NO_SUCH_USER.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::status",
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
