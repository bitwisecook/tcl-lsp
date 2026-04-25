# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::cs_attribute -- Queries for or sets attributes for the client-side challenge."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_attribute.html"


_av = make_av(_SOURCE)


@register
class BotdefenseCsAttributeCommand(CommandDef):
    name = "BOTDEFENSE::cs_attribute"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::cs_attribute",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or sets attributes for the client-side challenge.",
                synopsis=("BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",),
                snippet="Queries for or sets attributes for the client-side challenge. These attributes are only effective if a client-side action is taken on the current request.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Make sure that the data for the device_id is always collected when taking a client-side action.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    "    BOTDEFENSE::cs_attribute device_id enable\n"
                    "}"
                ),
                return_value="* When called with an argument the command overrides the decision of Bot Defense whether to collect device id. * When called without an argument, the command returns whether Bot Defense attempts to collect the device id during the request (initiate response).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",
                    arg_values={
                        0: (
                            _av(
                                "device_id",
                                "BOTDEFENSE::cs_attribute device_id",
                                "BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
