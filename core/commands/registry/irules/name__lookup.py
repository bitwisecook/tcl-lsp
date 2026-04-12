# Generated from F5 iRules reference documentation -- do not edit manually.
"""NAME::lookup -- Deprecated: Performs DNS query for A or PTR record corresponding to a hostname or IP address."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .resolv__lookup import ResolvLookupCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/NAME__lookup.html"


@register
class NameLookupCommand(CommandDef):
    name = "NAME::lookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NAME::lookup",
            deprecated_replacement=ResolvLookupCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Performs DNS query for A or PTR record corresponding to a hostname or IP address.",
                synopsis=("NAME::lookup",),
                snippet="Performs a DNS query, typically returning the A record for the indicated hostname, or the PTR record for the indicated IP address.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NAME::lookup",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
