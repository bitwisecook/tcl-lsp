"""http -- Tcl stdlib HTTP client package (package require http)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_PKG = "http"
_SOURCE = "Tcl stdlib http package"


@register
class HttpGeturl(CommandDef):
    name = "http::geturl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::geturl",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Retrieve a URL — the primary command for the http package.",
                synopsis=("http::geturl url ?options?",),
                snippet=(
                    "Retrieves the resource at *url* and returns a token that "
                    "can be passed to the other ``http::`` commands.  Options "
                    "include ``-query``, ``-headers``, ``-handler``, ``-command``, "
                    "``-timeout``, ``-type``, ``-method``, ``-keepalive`` and more."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
            credential_options=frozenset({"-headers"}),
            taint_network_sink_args=(0,),
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
class HttpConfig(CommandDef):
    name = "http::config"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::config",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set http package configuration options.",
                synopsis=(
                    "http::config",
                    "http::config ?options?",
                ),
                snippet=(
                    "Without arguments returns a list of all configuration "
                    "option/value pairs.  With arguments sets one or more "
                    "options: ``-accept``, ``-proxyhost``, ``-proxyport``, "
                    "``-proxyfilter``, ``-urlencoding``, ``-useragent``."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class HttpReset(CommandDef):
    name = "http::reset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::reset",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Reset an HTTP transaction.",
                synopsis=("http::reset token ?why?",),
                snippet=(
                    "Resets the HTTP transaction identified by *token*.  "
                    "The optional *why* string (default ``reset``) is passed "
                    "to the registered ``-command`` callback."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class HttpWait(CommandDef):
    name = "http::wait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::wait",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Wait for an HTTP transaction to complete.",
                synopsis=("http::wait token",),
                snippet=(
                    "Blocks the caller until the HTTP transaction identified by *token* completes."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpCleanup(CommandDef):
    name = "http::cleanup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::cleanup",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Clean up the state associated with an HTTP connection.",
                synopsis=("http::cleanup token",),
                snippet=(
                    "Releases all resources associated with *token*.  "
                    "The token must not be used after this call."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpFormatQuery(CommandDef):
    name = "http::formatQuery"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::formatQuery",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Generate an x-url-encoded query string from key/value pairs.",
                synopsis=("http::formatQuery key value ?key value ...?",),
                snippet=(
                    "Takes alternating key/value arguments and returns a "
                    "properly URL-encoded query string suitable for use as "
                    "a ``-query`` value in ``http::geturl``."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2)),
            pure=True,
        )


@register
class HttpQuoteString(CommandDef):
    name = "http::quoteString"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::quoteString",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="URL-encode a single string.",
                synopsis=("http::quoteString string",),
                snippet=(
                    "Applies URL encoding (percent-encoding) to *string* and returns the result."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class HttpRegister(CommandDef):
    name = "http::register"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::register",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Register a protocol handler (e.g. https) with the http package.",
                synopsis=("http::register proto defaultport command",),
                snippet=(
                    "Registers a handler for *proto* (e.g. ``https``).  "
                    "When ``http::geturl`` encounters this protocol, it "
                    "opens a socket via *command* on *defaultport*."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(3, 3)),
        )


@register
class HttpUnregister(CommandDef):
    name = "http::unregister"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::unregister",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Unregister a protocol handler from the http package.",
                synopsis=("http::unregister proto",),
                snippet="Removes the handler previously registered for *proto*.",
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


# Response accessor commands


@register
class HttpStatus(CommandDef):
    name = "http::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::status",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the status of an HTTP transaction (ok, reset, eof, error, timeout).",
                synopsis=("http::status token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpSize(CommandDef):
    name = "http::size"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::size",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the number of bytes received so far.",
                synopsis=("http::size token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpCode(CommandDef):
    name = "http::code"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::code",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the HTTP status line (e.g. ``HTTP/1.1 200 OK``).",
                synopsis=("http::code token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpNcode(CommandDef):
    name = "http::ncode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::ncode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the numeric HTTP status code (e.g. 200, 404).",
                synopsis=("http::ncode token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpMeta(CommandDef):
    name = "http::meta"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::meta",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the HTTP response headers as a dict-like list.",
                synopsis=("http::meta token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpData(CommandDef):
    name = "http::data"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::data",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the body of the HTTP response.",
                synopsis=("http::data token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class HttpError(CommandDef):
    name = "http::error"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::error",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the error message if the HTTP transaction failed.",
                synopsis=("http::error token",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
