# Enriched from F5 iRules reference documentation.
"""PSM::HTTP::disable -- To disable PSM for HTTP traffic."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSM__HTTP__disable.html"


@register
class PsmHttpDisableCommand(CommandDef):
    name = "PSM::HTTP::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSM::HTTP::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="To disable PSM for HTTP traffic.",
                synopsis=("PSM::HTTP::disable",),
                snippet="To disable PSM for HTTP traffic",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::uri] starts_with "/bypass" } {\n'
                    "        PSM::HTTP::disable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSM::HTTP::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"HTTP"}), also_in=frozenset({"CLIENT_ACCEPTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
