# Enriched from F5 iRules reference documentation.
"""POLICY::rules -- Returns the policy rules of the supplied policy that had actions executed."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/policy__rules.html"


_av = make_av(_SOURCE)


@register
class PolicyRulesCommand(CommandDef):
    name = "POLICY::rules"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POLICY::rules",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the policy rules of the supplied policy that had actions executed.",
                synopsis=("POLICY::rules ('matched')? POLICY_NAME",),
                snippet=(
                    "Returns the policy rules of the supplied policy that had actions\nexecuted."
                ),
                source=_SOURCE,
                examples=(
                    "# Log the policy targets for this virtual server\n"
                    "when HTTP_REQUEST {\n"
                    "\n"
                    '        log local0. "Looping through \\[POLICY::names matched\\]: [POLICY::names matched]"\n'
                    "        foreach policy [POLICY::names matched] {\n"
                    '                log local0. "\\[POLICY::rules matched $policy\\]: [POLICY::rules matched $policy]"\n'
                    "        }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POLICY::rules ('matched')? POLICY_NAME",
                    arg_values={
                        0: (
                            _av(
                                "matched",
                                "POLICY::rules matched",
                                "POLICY::rules ('matched')? POLICY_NAME",
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
