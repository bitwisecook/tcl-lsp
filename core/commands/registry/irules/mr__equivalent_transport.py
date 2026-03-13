# Enriched from F5 iRules reference documentation.
"""MR::equivalent_transport -- Gets or sets the transport that is usable as an equivalent transport."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__equivalent_transport.html"


_av = make_av(_SOURCE)


@register
class MrEquivalentTransportCommand(CommandDef):
    name = "MR::equivalent_transport"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::equivalent_transport",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the transport that is usable as an equivalent transport.",
                synopsis=(
                    "MR::equivalent_transport",
                    "MR::equivalent_transport none",
                    "MR::equivalent_transport (('virtual' VIRTUAL_SERVER_OBJ) | ('config' TRANSPORT_CONFIG))",
                ),
                snippet=(
                    "Gets or sets the transport that is usable as an equivalent transport. The equivalent transport may be used as an alternate when selecting a subsequent connection to the device the current connections communicates with.\n"
                    "        \n"
                    "Gets the transport that is usable as an equivalent transport. The equivalent transport may be used as an alternate when selecting a subsequent connection to the device the current connections communicates with.\n"
                    "            \n"
                    "Resets the transport that is usable as an equivalent transport."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    MR::equivalent_transport config /Common/inbound_tc\n"
                    "}"
                ),
                return_value="Returns the current equivalent transport. This will contain the transport type and transport name. For example: 'config /Common/inbound_tc'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::equivalent_transport",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "MR::equivalent_transport virtual",
                                "MR::equivalent_transport (('virtual' VIRTUAL_SERVER_OBJ) | ('config' TRANSPORT_CONFIG))",
                            ),
                            _av(
                                "config",
                                "MR::equivalent_transport config",
                                "MR::equivalent_transport (('virtual' VIRTUAL_SERVER_OBJ) | ('config' TRANSPORT_CONFIG))",
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
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
