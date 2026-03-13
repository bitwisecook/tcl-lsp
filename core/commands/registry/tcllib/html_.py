"""html -- HTML generation utilities (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib html package"
_PACKAGE = "html"


@register
class HtmlHtmlEntitiesCommand(CommandDef):
    name = "html::html_entities"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Replace special characters with HTML entities.",
                synopsis=("html::html_entities string",),
                source=_SOURCE,
                examples='set safe [html::html_entities "<b>text</b>"]',
                return_value="The entity-encoded string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="html::html_entities string",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class HtmlTagstripCommand(CommandDef):
    name = "html::tagstrip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Remove all HTML tags from a string.",
                synopsis=("html::tagstrip string",),
                source=_SOURCE,
                return_value="The text with all HTML tags removed.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="html::tagstrip string",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )
