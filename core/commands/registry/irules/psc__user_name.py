# Enriched from F5 iRules reference documentation.
"""PSC::user_name -- Get or set user name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__user_name.html"


@register
class PscUserNameCommand(CommandDef):
    name = "PSC::user_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::user_name",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set user name.",
                synopsis=("PSC::user_name (USERNAME)?",),
                snippet="The PSC::user_name command gets the user name or sets the user name when the optional value is given.",
                source=_SOURCE,
                return_value="Return the user name when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::user_name (USERNAME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
