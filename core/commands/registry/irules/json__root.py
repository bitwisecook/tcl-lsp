# Enriched from F5 iRules reference documentation.
"""JSON::root -- Gets the JSON element (aka."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__root.html"


@register
class JsonRootCommand(CommandDef):
    name = "JSON::root"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::root",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the JSON element (aka.",
                synopsis=("JSON::root (JSON_CACHE)?",),
                snippet="If a JSON cache handle is supplied, returns the root element of that JSON cache instance. This is useful when a JSON profile is not being used.",
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set rootval [JSON::root]\n"
                    "    JSON::set $rootval string HelloWorld\n"
                    "    set rendered [JSON::render $cache]\n"
                    "}"
                ),
                return_value="Returns the JSON element at the root of the JSON cache instance.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::root (JSON_CACHE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
