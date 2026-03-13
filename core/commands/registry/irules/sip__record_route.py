# Enriched from F5 iRules reference documentation.
"""SIP::record-route -- This command allows you get get information in the SIP record-route header."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__record_route.html"


_av = make_av(_SOURCE)


@register
class SipRecordRouteCommand(CommandDef):
    name = "SIP::record-route"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::record-route",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you get get information in the SIP record-route header.",
                synopsis=("SIP::record-route (INDEX | 'top')",),
                snippet=(
                    "This command allows you get get information in the SIP record-route header.\n"
                    "\n"
                    "Synax\n"
                    "\n"
                    "SIP::record-route <index>\n"
                    "\n"
                    '     * Get SIP header "route" at index\n'
                    "\n"
                    ' <index> is a numeric zero-based index or the keyword "top". The "top" keyword" will acess the first element as opposed to the first line of the record-route headers.'
                ),
                source=_SOURCE,
                examples=("when SIP_REQUEST {\n  log local0. [SIP::recordroute top]\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::record-route (INDEX | 'top')",
                    arg_values={
                        0: (
                            _av(
                                "top", "SIP::record-route top", "SIP::record-route (INDEX | 'top')"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
