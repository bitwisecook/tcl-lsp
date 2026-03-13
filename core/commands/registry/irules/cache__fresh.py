# Generated from F5 iRules reference documentation -- do not edit manually.
"""CACHE::fresh -- Returns state of freshness flag for request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__fresh.html"


@register
class CacheFreshCommand(CommandDef):
    name = "CACHE::fresh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::fresh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns state of freshness flag for request.",
                synopsis=("CACHE::fresh",),
                snippet="Returns 1 for cached document fresh, 0 otherwise.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::fresh",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CACHE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
