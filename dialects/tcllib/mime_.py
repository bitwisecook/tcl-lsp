"""mime -- MIME handling (tcllib)."""

from __future__ import annotations

from typing import Any

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "tcllib mime package"
_PACKAGE = "mime"


@register
class MimeInitialiseCommand(CommandDef):
    name = "mime::initialize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Create a MIME part from data, a file, or a channel.",
                synopsis=(
                    "mime::initialize ?-canonical type? ?-params dict? "
                    "?-encoding enc? ?-header dict? "
                    "?-string text | -file path | -chan channel?",
                ),
                source=_SOURCE,
                return_value="A MIME token.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::initialize ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class MimeGetpropertyCommand(CommandDef):
    name = "mime::getproperty"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return a property of a MIME part.",
                synopsis=("mime::getproperty token ?property?",),
                source=_SOURCE,
                return_value="The property value or a key-value list of all properties.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::getproperty token ?property?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class MimeGetbodyCommand(CommandDef):
    name = "mime::getbody"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the body of a MIME part.",
                synopsis=("mime::getbody token ?-decode? ?-command cmdprefix?",),
                source=_SOURCE,
                return_value="The body content.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::getbody token ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class MimeFinaliseCommand(CommandDef):
    name = "mime::finalize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Destroy a MIME part and free resources.",
                synopsis=("mime::finalize token ?-subordinates all?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::finalize token ?-subordinates all?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
        )


def _mm(name: str, summary: str, synopsis: str, arity: Arity, **kw: Any) -> type:
    """Register a mime command."""
    full_name = f"mime::{name}"

    @register
    class _Cmd(CommandDef):
        name = full_name

        @classmethod
        def spec(cls) -> CommandSpec:
            return CommandSpec(
                name=full_name,
                tcllib_package=_PACKAGE,
                hover=HoverSnippet(summary=summary, synopsis=(synopsis,), source=_SOURCE),
                forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=synopsis),),
                validation=ValidationSpec(arity=arity),
                **kw,
            )

    return _Cmd


_SE_FILE = (
    SideEffect(target=SideEffectTarget.FILE_IO, writes=True, connection_side=ConnectionSide.NONE),
)
_SE_VAR = (
    SideEffect(target=SideEffectTarget.VARIABLE, writes=True, connection_side=ConnectionSide.NONE),
)
_SE_INTERP = (
    SideEffect(
        target=SideEffectTarget.INTERP_STATE, writes=True, connection_side=ConnectionSide.NONE
    ),
)
_SE_INTERP_R = (
    SideEffect(
        target=SideEffectTarget.INTERP_STATE, reads=True, connection_side=ConnectionSide.NONE
    ),
)

_mm(
    "copymessage",
    "Copy MIME message to a channel.",
    "mime::copymessage token channel",
    Arity(2, 2),
    side_effect_hints=_SE_FILE,
)
_mm(
    "getheader",
    "Get header value(s) from a MIME token.",
    "mime::getheader token ?key?",
    Arity(1, 2),
    pure=True,
)
_mm(
    "setheader",
    "Set a header on a MIME token.",
    "mime::setheader token key value ?-mode mode?",
    Arity(3),
    side_effect_hints=_SE_VAR,
)
_mm(
    "parseaddress",
    "Parse an RFC 2822 address string.",
    "mime::parseaddress string",
    Arity(1, 1),
    pure=True,
)
_mm(
    "parsedatetime",
    "Parse an RFC 2822 date/time string.",
    "mime::parsedatetime datestring property",
    Arity(2, 2),
    pure=True,
)
_mm(
    "uniqueID",
    "Generate a globally unique identifier.",
    "mime::uniqueID",
    Arity(0, 0),
    side_effect_hints=_SE_INTERP,
)
_mm(
    "mapencoding",
    "Map a Tcl encoding name to a MIME charset.",
    "mime::mapencoding encoding",
    Arity(1, 1),
    pure=True,
)
_mm(
    "reversemapencoding",
    "Map a MIME charset to a Tcl encoding name.",
    "mime::reversemapencoding mimeType",
    Arity(1, 1),
    pure=True,
)
_mm(
    "word_encode",
    "Encode a string per RFC 2047.",
    "mime::word_encode charset method string ?options?",
    Arity(3),
    pure=True,
)
_mm(
    "word_decode",
    "Decode an RFC 2047 encoded word.",
    "mime::word_decode encoded",
    Arity(1, 1),
    pure=True,
)
_mm(
    "field_decode",
    "Decode an RFC 2047 header field.",
    "mime::field_decode field",
    Arity(1, 1),
    pure=True,
)
_mm("getsize", "Return the size of a MIME message.", "mime::getsize token", Arity(1, 1), pure=True)
_mm(
    "getContentType",
    "Return the content type of a MIME token.",
    "mime::getContentType token",
    Arity(1, 1),
    pure=True,
)
_mm(
    "getTransferEncoding",
    "Return the transfer encoding of a MIME token.",
    "mime::getTransferEncoding token",
    Arity(1, 1),
    pure=True,
)
_mm(
    "buildmessage",
    "Construct a MIME message from a token.",
    "mime::buildmessage token",
    Arity(1, 1),
    side_effect_hints=_SE_INTERP_R,
)
