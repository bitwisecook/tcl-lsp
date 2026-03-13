# Enriched from F5 iRules reference documentation.
"""ACL::eval -- Enforce ACLs in your connections."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACL__eval.html"


@register
class AclEvalCommand(CommandDef):
    name = "ACL::eval"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACL::eval",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enforce ACLs in your connections.",
                synopsis=("ACL::eval ('-l7')?",),
                snippet=(
                    "The ACL::eval command allows admin to enforce ACLs for a\n"
                    "given connection through APM network access tunnels.\n"
                    "\n"
                    " * Requires APM module and network access"
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    ACL::eval\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACL::eval ('-l7')?",
                    options=(OptionSpec(name="-l7", detail="Option -l7.", takes_value=True),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
