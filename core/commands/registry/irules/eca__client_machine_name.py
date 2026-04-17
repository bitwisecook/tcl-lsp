# Generated from F5 iRules reference documentation -- do not edit manually.
"""ECA::client_machine_name -- Returns NTLM authenticating user's machine name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ECA__client_machine_name.html"


@register
class EcaClientMachineNameCommand(CommandDef):
    name = "ECA::client_machine_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ECA::client_machine_name",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns NTLM authenticating user's machine name.",
                synopsis=("ECA::client_machine_name",),
                snippet="The ECA::client_machine_name command returns NTLM returns authenticating user's machine name",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ECA::client_machine_name",
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
