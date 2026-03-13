# Enriched from F5 iRules reference documentation.
"""POLICY::targets -- Returns or sets properties of the policy rule targets for the policies associated with the virtual server that the iRule is enabled on."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/policy__targets.html"


_av = make_av(_SOURCE)


@register
class PolicyTargetsCommand(CommandDef):
    name = "POLICY::targets"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POLICY::targets",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets properties of the policy rule targets for the policies associated with the virtual server that the iRule is enabled on.",
                synopsis=("POLICY::targets ('ltm-policy' |",),
                snippet=(
                    "Returns or sets properties of the policy rule targets for the policies\n"
                    "associated with the virtual server that the iRule is enabled on. A\n"
                    "policy rule target can be considered an action that the policy uses if\n"
                    "the rule conditions are met.\n"
                    "\n"
                    "As of v11.4 the following policy targets are available:\n"
                    " wam              - Application Acceleration Manager (AAM)\n"
                    " asm              - Application Security Manager\n"
                    " log              - Log\n"
                    " http-cookie      - HTTP cookie\n"
                    " http-header      - HTTP header\n"
                    " http-host        - HTTP host header\n"
                    " http-referer     - HTTP referer header"
                ),
                source=_SOURCE,
                examples=(
                    "# Log the policy targets for this virtual server\n"
                    "when HTTP_REQUEST {\n"
                    "\n"
                    "        # Log the policy targets enabled on this virtual server\n"
                    '        log local0. "\\[POLICY::targets\\]: [POLICY::targets]"\n'
                    "\n"
                    "        # Loop through each possible target type and log whether it is enabled or not (1 for enabled, 0 for not enabled)\n"
                    "        foreach target {asm wam log http-cookie http-header http-host http-referer http-set-cookie http-uri log tcl tcp-nagle} {"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POLICY::targets ('ltm-policy' |",
                    arg_values={
                        0: (
                            _av(
                                "ltm-policy",
                                "POLICY::targets ltm-policy",
                                "POLICY::targets ('ltm-policy' |",
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
