# Enriched from F5 iRules reference documentation.
"""AUTH::response_data -- Returns pairwise auth query results."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__response_data.html"


@register
class AuthResponseDataCommand(CommandDef):
    name = "AUTH::response_data"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::response_data",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns pairwise auth query results.",
                synopsis=("AUTH::response_data (AUTH_ID)?",),
                snippet=(
                    "AUTH::response_data returns the a set of name/value query results from\n"
                    "the most recent query. This command would normally be called from the\n"
                    "AUTH_RESULT event. The format of the data returned is suitable for\n"
                    "setting as the value of a TCL array.\n"
                    "AUTH::subscribe must first be called to register interest in query\n"
                    "results prior to calling AUTH::authenticate. As a convenience when\n"
                    "using the builtin system auth rules, these rules will call\n"
                    "AUTH::subscribe if the variable tmm_auth_subscription is set."
                ),
                source=_SOURCE,
                examples=('when CLIENT_ACCEPTED {\n        set tmm_auth_subscription "*"\n    }'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::response_data (AUTH_ID)?",
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
