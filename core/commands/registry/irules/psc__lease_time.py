# Enriched from F5 iRules reference documentation.
"""PSC::lease_time -- Get the session lease time."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__lease-time.html"


@register
class PscLeaseTimeCommand(CommandDef):
    name = "PSC::lease_time"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::lease_time",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get the session lease time.",
                synopsis=("PSC::lease_time IP_ADDR (LEASE_TIME)?",),
                snippet="The PSC::lease_time command gets the lease time of the session.",
                source=_SOURCE,
                return_value="Return the session lease time.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::lease_time IP_ADDR (LEASE_TIME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
