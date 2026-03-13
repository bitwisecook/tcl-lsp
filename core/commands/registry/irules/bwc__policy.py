# Enriched from F5 iRules reference documentation.
"""BWC::policy -- The bwc irule allows a bwc policy to be attached or detached to a specific flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__policy.html"


_av = make_av(_SOURCE)


@register
class BwcPolicyCommand(CommandDef):
    name = "BWC::policy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::policy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="The bwc irule allows a bwc policy to be attached or detached to a specific flow.",
                synopsis=("BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?",),
                snippet=(
                    'A bwc policy must exist for the given policy name, the irule will return an error if the policy cannot be found. The policy name should be give without a path name: e.g. "gold_user" not "/Common/gold_user". The irule will internally try to determine the correct pathname through lookup_folder_path_obj().\n'
                    "\n"
                    "Once the irule has found the correct bwc policy name, it will know if the policy is static or dynamic. If the policy is dynamic a third arg session is required. The session is used as the bwc_cookie_t argument to the bwc public api bwc_dynamic_policy_instantiate()."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n            BWC::policy attach gold_class\n        }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?",
                    arg_values={
                        0: (
                            _av(
                                "attach",
                                "BWC::policy attach",
                                "BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?",
                            ),
                            _av(
                                "detach",
                                "BWC::policy detach",
                                "BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?",
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
