# Enriched from F5 iRules reference documentation.
"""JSON::create -- Creates a new, empty JSON cache instance."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__create.html"


@register
class JsonCreateCommand(CommandDef):
    name = "JSON::create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::create",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new, empty JSON cache instance.",
                synopsis=("JSON::create",),
                snippet="Creates a new, empty JSON cache instance. It can then be filled with any JSON content and rendered. It will be deleted when no longer referenced by a Tcl variable.",
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set cache [JSON::create]\n"
                    "    set rootval [JSON::root $cache]\n"
                    "    JSON::set $rootval string HelloWorld\n"
                    "    set rendered [JSON::render $cache]\n"
                    "}"
                ),
                return_value="Returns the new JSON cache instance.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::create",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
