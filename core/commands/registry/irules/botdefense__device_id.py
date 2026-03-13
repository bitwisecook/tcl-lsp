# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::device_id -- Returns the Device ID of the client, as retrieved from the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__device_id.html"


@register
class BotdefenseDeviceIdCommand(CommandDef):
    name = "BOTDEFENSE::device_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::device_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the Device ID of the client, as retrieved from the request.",
                synopsis=("BOTDEFENSE::device_id",),
                snippet="Returns a number, representing the Device ID of the client, as retrieved from the request. If the Device ID is unknown, 0 is returned. By default, the Device ID is collected from the client, if it is enabled in the configuration. However, this can be overridden using the BOTDEFENSE::cs_attribute command.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the Device ID of the client, upon each request, if it is known.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    "    if {[BOTDEFENSE::device_id] != 0} {\n"
                    '        set log "botdefense device_id of client IP [IP::client_addr] is"\n'
                    '        append log " [BOTDEFENSE::device_id]"\n'
                    "        HSL::send $hsl $log\n"
                    "    }\n"
                    "}"
                ),
                return_value="The number representing the device ID of the client that sent the request, or 0 if there is no such value",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::device_id",
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
