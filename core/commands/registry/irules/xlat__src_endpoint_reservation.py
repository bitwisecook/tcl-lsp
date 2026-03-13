# Enriched from F5 iRules reference documentation.
"""XLAT::src_endpoint_reservation -- XLAT:src_endpoint_reservation"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__src_endpoint_reservation.html"


@register
class XlatSrcEndpointReservationCommand(CommandDef):
    name = "XLAT::src_endpoint_reservation"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::src_endpoint_reservation",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="XLAT:src_endpoint_reservation",
                synopsis=(
                    "XLAT::src_endpoint_reservation create",
                    "XLAT::src_endpoint_reservation update_lifetime TRANS_ADDR TRANS_PORT LSN_POOL XLAT_PROTO XLAT_LIFETIME",
                ),
                snippet=(
                    "Create, update, or get reserved entry values.\n"
                    "\n"
                    "Syntax:\n"
                    "XLAT::src_endpoint_reservation create [-no-persist] [-dslite  <local> <remote>] [-pool <source translation object/pool name>] [-translation-loose|-translation-strict <ip> <port>] <client ip> <client port> <protocol> <lifetime>;\n"
                    "\n"
                    'Creates a reservation in the reservation table which can be viewed using the command "lsndb list endpoint-reservation" for the lifetime specified by the user. The command has the following characteristics:\n'
                    "    1) The returned endpoint cannot be reserved for another client IP:port as long as it is active."
                ),
                source=_SOURCE,
                return_value="create returns the translation endpoint used for the reservation.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::src_endpoint_reservation create",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            excluded_events=("RULE_INIT",),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
