"""uri -- URI parsing and generation (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib uri package"
_PACKAGE = "uri"


@register
class UriSplitCommand(CommandDef):
    name = "uri::split"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Split a URI into its component parts.",
                synopsis=("uri::split url ?defaultScheme?",),
                snippet=(
                    "Parses the URI and returns a key-value list with "
                    "scheme, host, port, path, query, fragment, etc."
                ),
                source=_SOURCE,
                examples="array set parts [uri::split $url]",
                return_value="A key-value list of URI components.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="uri::split url ?defaultScheme?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class UriJoinCommand(CommandDef):
    name = "uri::join"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Construct a URI from component parts.",
                synopsis=("uri::join ?key value ...?",),
                source=_SOURCE,
                examples=("set url [uri::join scheme https host example.com path /index.html]"),
                return_value="A URI string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="uri::join ?key value ...?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class UriResolveCommand(CommandDef):
    name = "uri::resolve"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Resolve a relative URI reference against a base URI.",
                synopsis=("uri::resolve base url",),
                source=_SOURCE,
                examples="set abs [uri::resolve $baseUrl $relativeUrl]",
                return_value="The resolved absolute URI.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="uri::resolve base url"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )
