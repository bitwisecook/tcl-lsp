# Enriched from F5 iRules reference documentation.
"""DNS::tsig -- Manipulates the current DNS message and its TSIG resource record."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__tsig.html"


_av = make_av(_SOURCE)


@register
class DnsTsigCommand(CommandDef):
    name = "DNS::tsig"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::tsig",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Manipulates the current DNS message and its TSIG resource record.",
                synopsis=(
                    "DNS::tsig 'remove'",
                    "DNS::tsig 'exists'",
                ),
                snippet=(
                    "This command manipulates the current DNS message and its TSIG resource\n"
                    "record.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_REQUEST {\n"
                    "  if { [DNS::tsig exists] } {\n"
                    "    DNS::tsig remove\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::tsig 'remove'",
                    arg_values={
                        0: (
                            _av("remove", "DNS::tsig remove", "DNS::tsig 'remove'"),
                            _av("exists", "DNS::tsig exists", "DNS::tsig 'exists'"),
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
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
