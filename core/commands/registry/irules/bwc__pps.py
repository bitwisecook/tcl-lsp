# Enriched from F5 iRules reference documentation.
"""BWC::pps -- This command is used to modify pps value for the policy."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__pps.html"


@register
class BwcPpsCommand(CommandDef):
    name = "BWC::pps"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::pps",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to modify pps value for the policy.",
                synopsis=("BWC::pps BW_VALUE",),
                snippet="After a policy is created, irule can modify the pps for the session. The value is specified in packets per second.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mycookie [IP::remote_addr]:[TCP::remote_port]\n"
                    "    BWC::policy attach gold_user $mycookie\n"
                    "    BWC::pps 77\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::pps BW_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
