"""HTTP::respond -- Send an immediate HTTP response from an iRule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__respond.html"


def _av(value: str, detail: str, synopsis: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=value,
        detail=detail,
        hover=HoverSnippet(
            summary=detail.rstrip(".") + ".",
            synopsis=(synopsis,),
            source=_SOURCE,
        ),
    )


def _status(code: str, text: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=code,
        detail=f"{code} {text}",
    )


def _header(name: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(value=name, detail=f"HTTP response header: {name}")


def _hdr_val(value: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(value=value, detail=value)


# HTTP status codes (RFC 7231 + common extensions)
_HTTP_STATUS_CODES = (
    _status("100", "Continue"),
    _status("101", "Switching Protocols"),
    _status("102", "Processing"),
    _status("103", "Early Hints"),
    _status("200", "OK"),
    _status("201", "Created"),
    _status("202", "Accepted"),
    _status("203", "Non-Authoritative Information"),
    _status("204", "No Content"),
    _status("205", "Reset Content"),
    _status("206", "Partial Content"),
    _status("207", "Multi-Status"),
    _status("208", "Already Reported"),
    _status("226", "IM Used"),
    _status("300", "Multiple Choices"),
    _status("301", "Moved Permanently"),
    _status("302", "Found"),
    _status("303", "See Other"),
    _status("304", "Not Modified"),
    _status("305", "Use Proxy"),
    _status("307", "Temporary Redirect"),
    _status("308", "Permanent Redirect"),
    _status("400", "Bad Request"),
    _status("401", "Unauthorized"),
    _status("402", "Payment Required"),
    _status("403", "Forbidden"),
    _status("404", "Not Found"),
    _status("405", "Method Not Allowed"),
    _status("406", "Not Acceptable"),
    _status("407", "Proxy Authentication Required"),
    _status("408", "Request Timeout"),
    _status("409", "Conflict"),
    _status("410", "Gone"),
    _status("411", "Length Required"),
    _status("412", "Precondition Failed"),
    _status("413", "Payload Too Large"),
    _status("414", "URI Too Long"),
    _status("415", "Unsupported Media Type"),
    _status("416", "Range Not Satisfiable"),
    _status("417", "Expectation Failed"),
    _status("418", "I'm a teapot"),
    _status("421", "Misdirected Request"),
    _status("422", "Unprocessable Entity"),
    _status("423", "Locked"),
    _status("424", "Failed Dependency"),
    _status("425", "Too Early"),
    _status("426", "Upgrade Required"),
    _status("428", "Precondition Required"),
    _status("429", "Too Many Requests"),
    _status("431", "Request Header Fields Too Large"),
    _status("451", "Unavailable For Legal Reasons"),
    _status("500", "Internal Server Error"),
    _status("501", "Not Implemented"),
    _status("502", "Bad Gateway"),
    _status("503", "Service Unavailable"),
    _status("504", "Gateway Timeout"),
    _status("505", "HTTP Version Not Supported"),
    _status("506", "Variant Also Negotiates"),
    _status("507", "Insufficient Storage"),
    _status("508", "Loop Detected"),
    _status("510", "Not Extended"),
    _status("511", "Network Authentication Required"),
)

# Common HTTP response headers
_HTTP_RESPONSE_HEADERS = (
    _header("Accept-Patch"),
    _header("Accept-Ranges"),
    _header("Access-Control-Allow-Credentials"),
    _header("Access-Control-Allow-Headers"),
    _header("Access-Control-Allow-Methods"),
    _header("Access-Control-Allow-Origin"),
    _header("Access-Control-Expose-Headers"),
    _header("Access-Control-Max-Age"),
    _header("Age"),
    _header("Allow"),
    _header("Alt-Svc"),
    _header("Cache-Control"),
    _header("Connection"),
    _header("Content-Disposition"),
    _header("Content-Encoding"),
    _header("Content-Language"),
    _header("Content-Length"),
    _header("Content-Location"),
    _header("Content-Range"),
    _header("Content-Security-Policy"),
    _header("Content-Type"),
    _header("Date"),
    _header("Delta-Base"),
    _header("ETag"),
    _header("Expires"),
    _header("IM"),
    _header("Last-Modified"),
    _header("Link"),
    _header("Location"),
    _header("Pragma"),
    _header("Proxy-Authenticate"),
    _header("Public-Key-Pins"),
    _header("Refresh"),
    _header("Retry-After"),
    _header("Server"),
    _header("Set-Cookie"),
    _header("Strict-Transport-Security"),
    _header("Trailer"),
    _header("Transfer-Encoding"),
    _header("Tk"),
    _header("Upgrade"),
    _header("Vary"),
    _header("Via"),
    _header("Warning"),
    _header("WWW-Authenticate"),
    _header("X-Powered-By"),
    _header("X-Request-ID"),
    _header("X-UA-Compatible"),
    _header("X-XSS-Protection"),
)

# Per-header value completions for common headers
_CACHE_CONTROL_VALUES = (
    _hdr_val("max-age="),
    _hdr_val("must-revalidate"),
    _hdr_val("no-cache"),
    _hdr_val("no-store"),
    _hdr_val("public"),
    _hdr_val("private"),
)

_CONTENT_TYPE_VALUES = (
    _hdr_val("application/javascript"),
    _hdr_val("application/json"),
    _hdr_val("application/jwt"),
    _hdr_val("application/octet-stream"),
    _hdr_val("image/gif"),
    _hdr_val("image/heif"),
    _hdr_val("image/jpeg"),
    _hdr_val("image/png"),
    _hdr_val("image/webp"),
    _hdr_val("image/x-icon"),
    _hdr_val("multipart/byteranges"),
    _hdr_val("multipart/form-data"),
    _hdr_val("text/css"),
    _hdr_val("text/html"),
    _hdr_val("text/plain"),
)

_TRANSFER_ENCODING_VALUES = (
    _hdr_val("gzip"),
    _hdr_val("chunked"),
    _hdr_val("deflate"),
)

_VARY_VALUES = (
    _hdr_val("User-Agent"),
    _hdr_val("Accept-Encoding"),
)


@register
class HttpRespondCommand(CommandDef):
    name = "HTTP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Send an immediate HTTP response from an iRule.",
                synopsis=("HTTP::respond <status> ?option value ...?",),
                snippet=(
                    "Common options include `content`, `noserver`, `reset`, and `version`.\n\n"
                    "The response is sent when the current event completes. "
                    "You cannot alter it in later HTTP events or after another response "
                    "has already been sent.\n"
                    "\n"
                    "**Security**: When the response body contains user-supplied data\n"
                    "(HTTP headers, URI, payload), HTML-encode it to prevent XSS.\n"
                    "For blocking/maintenance pages, include `Connection close` and\n"
                    "`Cache-Control no-store` headers:\n"
                    "```tcl\n"
                    "HTTP::respond 403 content $html Connection close "
                    "Cache-Control no-store\n"
                    "```"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::respond <status> ?option value ...?",
                    arg_values={
                        0: _HTTP_STATUS_CODES,
                        1: (
                            _av(
                                "content", "Inline response body.", 'HTTP::respond 200 content "ok"'
                            ),
                            _av(
                                "noserver", "Suppress Server header.", "HTTP::respond 200 noserver"
                            ),
                            _av(
                                "reset", "Reset server-side connection.", "HTTP::respond 503 reset"
                            ),
                            _av(
                                "version", "Response HTTP version.", "HTTP::respond 200 version 1.1"
                            ),
                        ),
                    },
                    subcommand_arg_values={
                        ("content", 1): _HTTP_RESPONSE_HEADERS,
                        ("Cache-Control", 0): _CACHE_CONTROL_VALUES,
                        ("Content-Type", 0): _CONTENT_TYPE_VALUES,
                        ("Transfer-Encoding", 0): _TRANSFER_ENCODING_VALUES,
                        ("Vary", 0): _VARY_VALUES,
                    },
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            taint_output_sink="IRULE3001",
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
