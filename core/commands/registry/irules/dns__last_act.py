# Enriched from F5 iRules reference documentation.
"""DNS::last_act -- Sets the action to perform if no DNS service handles this packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__last_act.html"


_av = make_av(_SOURCE)


@register
class DnsLastActCommand(CommandDef):
    name = "DNS::last_act"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::last_act",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the action to perform if no DNS service handles this packet.",
                synopsis=("DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",),
                snippet=(
                    "This iRules command sets the action to perform if no DNS service\n"
                    "handles this packet\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "equests that are not handled by a local dns service\n"
                    "            when DNS_REQUEST {\n"
                    "                DNS::last_act drop\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",
                    arg_values={
                        0: (
                            _av(
                                "allow",
                                "DNS::last_act allow",
                                "DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",
                            ),
                            _av(
                                "drop",
                                "DNS::last_act drop",
                                "DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",
                            ),
                            _av(
                                "reject",
                                "DNS::last_act reject",
                                "DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",
                            ),
                            _av(
                                "hint",
                                "DNS::last_act hint",
                                "DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",
                            ),
                            _av(
                                "noerror",
                                "DNS::last_act noerror",
                                "DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
