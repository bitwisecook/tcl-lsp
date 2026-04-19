# Generated from F5 iRules reference documentation -- do not edit manually.
"""NAME::response -- Deprecated: Returns a list of records received in response to a DNS query."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .resolv__lookup import ResolvLookupCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/NAME__response.html"


@register
class NameResponseCommand(CommandDef):
    name = "NAME::response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NAME::response",
            deprecated_replacement=ResolvLookupCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Returns a list of records received in response to a DNS query.",
                synopsis=("NAME::response",),
                snippet="Returns a list of records received in response to a DNS query made with the NAME_ _lookup command.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NAME::response",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"NAME"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
