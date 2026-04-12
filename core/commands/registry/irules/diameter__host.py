# Enriched from F5 iRules reference documentation.
"""DIAMETER::host -- Gets or sets the value of the origin-host or destination-host AVP."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__host.html"


_av = make_av(_SOURCE)


@register
class DiameterHostCommand(CommandDef):
    name = "DIAMETER::host"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::host",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the value of the origin-host or destination-host AVP.",
                synopsis=("DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )",),
                snippet=(
                    "This iRule command gets or sets the value of the origin-host (code\n"
                    "264) or destination-host (code 293) AVP in the current message."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message with origin host [DIAMETER::host origin]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )",
                    arg_values={
                        0: (
                            _av(
                                "origin",
                                "DIAMETER::host origin",
                                "DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )",
                            ),
                            _av(
                                "dest",
                                "DIAMETER::host dest",
                                "DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )",
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
