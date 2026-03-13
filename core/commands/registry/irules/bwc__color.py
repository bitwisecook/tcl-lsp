# Enriched from F5 iRules reference documentation.
"""BWC::color -- This command is used to classify a traffic flow to a particular color (application category)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__color.html"


_av = make_av(_SOURCE)


@register
class BwcColorCommand(CommandDef):
    name = "BWC::color"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::color",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to classify a traffic flow to a particular color (application category).",
                synopsis=("BWC::color ('set' | 'unset') POLICY_NAME APPLICATION_NAME",),
                snippet="After a flow has been assigned a policy, at some later time when the traffic is classified the user can assign an application to this flow. This uses the bwc config to create a bwc policy with the categories keyword.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mycookie [IP::remote_addr]:[TCP::remote_port]\n"
                    "    BWC::policy attach gold_user $mycookie\n"
                    "    BWC::color set gold_user p2p\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::color ('set' | 'unset') POLICY_NAME APPLICATION_NAME",
                    arg_values={
                        0: (
                            _av(
                                "set",
                                "BWC::color set",
                                "BWC::color ('set' | 'unset') POLICY_NAME APPLICATION_NAME",
                            ),
                            _av(
                                "unset",
                                "BWC::color unset",
                                "BWC::color ('set' | 'unset') POLICY_NAME APPLICATION_NAME",
                            ),
                        )
                    },
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
