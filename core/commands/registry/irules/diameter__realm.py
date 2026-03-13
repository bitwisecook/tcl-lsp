# Enriched from F5 iRules reference documentation.
"""DIAMETER::realm -- Gets or sets the value of the origin-realm or destination-realm AVP."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__realm.html"


_av = make_av(_SOURCE)


@register
class DiameterRealmCommand(CommandDef):
    name = "DIAMETER::realm"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::realm",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the value of the origin-realm or destination-realm AVP.",
                synopsis=("DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )",),
                snippet=(
                    "This iRule command gets or sets the value of the origin-realm (code 296) or\n"
                    "destination-realm (code 283) AVP in the current Diameter message."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message with origin realm [DIAMETER::realm origin]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )",
                    arg_values={
                        0: (
                            _av(
                                "origin",
                                "DIAMETER::realm origin",
                                "DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )",
                            ),
                            _av(
                                "dest",
                                "DIAMETER::realm dest",
                                "DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
