# Enriched from F5 iRules reference documentation.
"""urlcatblindquery -- Query the encrypted URL's hash for URL categorization."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .category__lookup import CategoryLookupCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/urlcatblindquery.html"


@register
class UrlcatblindqueryCommand(CommandDef):
    name = "urlcatblindquery"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="urlcatblindquery",
            deprecated_replacement=CategoryLookupCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query the encrypted URL's hash for URL categorization.",
                synopsis=("urlcatblindquery ENCRYPTED_URL_STRING",),
                snippet="Query the encrypted URL's hash for URL categorization",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="urlcatblindquery ENCRYPTED_URL_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DATA_GROUP,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
