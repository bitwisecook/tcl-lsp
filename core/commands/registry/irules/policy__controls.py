# Enriched from F5 iRules reference documentation.
"""POLICY::controls -- Returns details about the policy controls for the virtual server the iRule is enabled on."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/policy__controls.html"


_av = make_av(_SOURCE)


@register
class PolicyControlsCommand(CommandDef):
    name = "POLICY::controls"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POLICY::controls",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns details about the policy controls for the virtual server the iRule is enabled on.",
                synopsis=("POLICY::controls ('acceleration' |",),
                snippet=(
                    "Returns details about the policy controls for the policies associated\n"
                    "with the virtual server that the iRule is enabled on. Policy controls\n"
                    "are typically virtual server profiles or features which can be enabled,\n"
                    "disabled or modified per iRule execution via policies.\n"
                    "\n"
                    "As of v11.4 the following controls are available:\n"
                    " acceleration        - Application Acceleration Manager (AAM)\n"
                    " asm                 - Application Security Manager\n"
                    " avr                 - Application Visibility and Reporting\n"
                    " caching             - HTTP caching"
                ),
                source=_SOURCE,
                examples=(
                    "# Log the policy controls for this virtual server\n"
                    "when HTTP_REQUEST {\n"
                    "\n"
                    "        # Log the policy controls enabled on this virtual server\n"
                    '        log local0. "\\[POLICY::controls\\]: [POLICY::controls]"\n'
                    "\n"
                    "        # Loop through each possible control type and log whether it is enabledor not (1 for enabled, 0 for not enabled)\n"
                    "        foreach control {acceleration asm avr caching classification compression forwarding l7dos bot-defense request-adaptation response-adaptation server-ssl} {"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POLICY::controls ('acceleration' |",
                    arg_values={
                        0: (
                            _av(
                                "acceleration",
                                "POLICY::controls acceleration",
                                "POLICY::controls ('acceleration' |",
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
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
