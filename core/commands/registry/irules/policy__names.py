# Enriched from F5 iRules reference documentation.
"""POLICY::names -- Returns details about the policy names for the virtual server the iRule is enabled on."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/policy__names.html"


_av = make_av(_SOURCE)


@register
class PolicyNamesCommand(CommandDef):
    name = "POLICY::names"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POLICY::names",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns details about the policy names for the virtual server the iRule is enabled on.",
                synopsis=("POLICY::names (active | matched | unmatched)",),
                snippet=(
                    "iRule command which returns details about the policy names for the\n"
                    "virtual server the iRule is enabled on."
                ),
                source=_SOURCE,
                examples=(
                    "# Log the policy names for this virtual server\n"
                    "when HTTP_REQUEST {\n"
                    '        log local0. "Enabled on this VS: \\[POLICY::names active\\]: [POLICY::names active]"\n'
                    '        log local0. "Matched: \\[POLICY::names matched\\]: [POLICY::names matched]"\n'
                    '        log local0. "Not matched: \\[POLICY::names unmatched\\]: [POLICY::names unmatched]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POLICY::names (active | matched | unmatched)",
                    arg_values={
                        0: (
                            _av(
                                "active",
                                "POLICY::names active",
                                "POLICY::names (active | matched | unmatched)",
                            ),
                            _av(
                                "matched",
                                "POLICY::names matched",
                                "POLICY::names (active | matched | unmatched)",
                            ),
                            _av(
                                "unmatched",
                                "POLICY::names unmatched",
                                "POLICY::names (active | matched | unmatched)",
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
