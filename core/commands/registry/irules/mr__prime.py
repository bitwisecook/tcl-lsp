# Enriched from F5 iRules reference documentation.
"""MR::prime -- establishes an outgoing connection to the specified host or hosts using the specified transport"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__prime.html"


_av = make_av(_SOURCE)


@register
class MrPrimeCommand(CommandDef):
    name = "MR::prime"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::prime",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="establishes an outgoing connection to the specified host or hosts using the specified transport",
                synopsis=(
                    "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                ),
                snippet="The MR::prime command instructs the Message Routing Framework to establish an outgoing connection to a specified host or pool if one does not exist. The setting of the specified virtual or transport-config will be used to establish the connection. If a pool is provided, outgoing connections will be created to all active poolmembers of the specified pool.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::prime config /Common/my_tc pool /Common/default_pool\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "MR::prime virtual",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                            _av(
                                "config",
                                "MR::prime config",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                            _av(
                                "pool",
                                "MR::prime pool",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                            _av(
                                "host",
                                "MR::prime host",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
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
