# Enriched from F5 iRules reference documentation.
"""ACCESS::flowid -- Sets the flow id for SSL Orchestrator using APM logging framework."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__flowid.html"


@register
class AccessFlowidCommand(CommandDef):
    name = "ACCESS::flowid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::flowid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the flow id for SSL Orchestrator using APM logging framework.",
                synopsis=("ACCESS::flowid (FID)?",),
                snippet=(
                    "ACCESS::flowid [FID]\n"
                    "\n"
                    "Calculates the flow id from the IFC and 4-tuple information, if it doesn't\n"
                    "exist already, and stores it in the opaque storage for the connflow.\n"
                    "Requires APM to be provisioned.\n"
                    "\n"
                    "Command Syntax\n"
                    "\n"
                    "ACCESS::flowid\n"
                    "\n"
                    "    * Returns the flow id, if it exists, or calculates it, then stores it in\n"
                    "      the opaque data structure for the connflow.\n"
                    "\n"
                    "ACCESS::flowid <FID>\n"
                    "\n"
                    "    * Sets the flow id to FID"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    ACCESS::flowid "example"\n'
                    "    set ctx(FID) [ACCESS::flowid]\n"
                    "}"
                ),
                return_value="The flow id is returned",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::flowid (FID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
