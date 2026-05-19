"""uri -- URI parsing and generation (tcllib)."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity, BodyKind
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


@register
class UriCanonicalizeCommand(CommandDef):
    name = "uri::canonicalize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Canonicalize a URI.",
                synopsis=("uri::canonicalize uri",),
                source=_SOURCE,
                return_value="The canonicalized URI string.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="uri::canonicalize uri"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class UriIsrelativeCommand(CommandDef):
    name = "uri::isrelative"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Test whether a URI is relative.",
                synopsis=("uri::isrelative uri",),
                source=_SOURCE,
                return_value="1 if the URI is relative, 0 otherwise.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="uri::isrelative uri"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            return_type=TclType.BOOLEAN,
        )


@register
class UriGeturlCommand(CommandDef):
    name = "uri::geturl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Fetch the contents of a URI.",
                synopsis=("uri::geturl url ?options...?",),
                source=_SOURCE,
                return_value="The fetched content.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="uri::geturl url ?options...?"),),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class UriRegisterCommand(CommandDef):
    name = "uri::register"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Register a new URI scheme handler.",
                synopsis=("uri::register schemeList script",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="uri::register schemeList script"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            arg_roles={1: frozenset({ArgRole.BODY})},
            # The registered script becomes a scheme handler that runs
            # later inside ``uri::split``'s dispatch — it does not share
            # the caller's scope.  STRUCTURAL keeps the body out of the
            # enclosing block's data flow.
            body_kind=BodyKind.STRUCTURAL,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class UriSetQuirkOptionCommand(CommandDef):
    name = "uri::setQuirkOption"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Set a quirk option for URI processing.",
                synopsis=("uri::setQuirkOption option ?value...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(kind=FormKind.DEFAULT, synopsis="uri::setQuirkOption option ?value...?"),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
