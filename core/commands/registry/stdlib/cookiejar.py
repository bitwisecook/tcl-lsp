"""cookiejar -- Tcl stdlib HTTP cookie jar (package require cookiejar)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_PKG = "cookiejar"
_SOURCE = "Tcl stdlib cookiejar package"


@register
class HttpCookiejar(CommandDef):
    name = "http::cookiejar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::cookiejar",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Create or configure an HTTP cookie jar (TclOO class).",
                synopsis=(
                    "http::cookiejar create name ?filename?",
                    "http::cookiejar new ?filename?",
                ),
                snippet=(
                    "A TclOO class implementing RFC 6265 cookie management.  "
                    "Instances track cookies received in HTTP responses and "
                    "automatically attach them to subsequent requests."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class HttpCookiejarIdnaEncode(CommandDef):
    name = "http::cookiejar::IDNAencode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::cookiejar::IDNAencode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Encode a hostname to IDNA (Internationalised Domain Names) format.",
                synopsis=("http::cookiejar::IDNAencode hostname",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class HttpCookiejarIdnaDecode(CommandDef):
    name = "http::cookiejar::IDNAdecode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::cookiejar::IDNAdecode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Decode a hostname from IDNA format to Unicode.",
                synopsis=("http::cookiejar::IDNAdecode hostname",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )
