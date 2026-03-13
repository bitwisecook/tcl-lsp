"""HTTP::header -- Inspect or mutate HTTP headers in an iRule event."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, _SENSITIVE_HTTP_HEADERS, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__header.html"


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


@register
class HttpHeaderCommand(CommandDef):
    name = "HTTP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::header",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Inspect or mutate HTTP headers in an iRule event.",
                synopsis=("HTTP::header <subcommand> ?arg ...?",),
                snippet="Use subcommands like `value`, `insert`, `replace`, and `remove`.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::header <subcommand> ?arg ...?",
                    arg_values={
                        0: (
                            _av("at", "Get header name by index.", "HTTP::header at <index>"),
                            _av("count", "Count headers.", "HTTP::header count ?<name>?"),
                            _av("exists", "Check if header exists.", "HTTP::header exists <name>"),
                            _av(
                                "insert",
                                "Insert header/value pair.",
                                "HTTP::header insert <name> <value>",
                            ),
                            _av(
                                "insert_modssl_fields",
                                "Insert mod_ssl-compatible headers.",
                                "HTTP::header insert_modssl_fields",
                            ),
                            _av(
                                "is_keepalive",
                                "Check if connection is keep-alive.",
                                "HTTP::header is_keepalive",
                            ),
                            _av(
                                "is_redirect",
                                "Check if response is a redirect.",
                                "HTTP::header is_redirect",
                            ),
                            _av(
                                "lws",
                                "Enable/disable linear whitespace folding.",
                                "HTTP::header lws ?enable|disable?",
                            ),
                            _av("names", "List all header names.", "HTTP::header names"),
                            _av("remove", "Remove named header.", "HTTP::header remove <name>"),
                            _av(
                                "replace",
                                "Replace header value.",
                                "HTTP::header replace <name> <value>",
                            ),
                            _av(
                                "sanitize",
                                "Remove headers not in the allow list.",
                                "HTTP::header sanitize <name-list>",
                            ),
                            _av("value", "Get first header value.", "HTTP::header value <name>"),
                            _av(
                                "values", "Get all values for header.", "HTTP::header values <name>"
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "at": SubCommand(
                    name="at",
                    arity=Arity(1, 1),
                    detail="Get header name by index.",
                    synopsis="HTTP::header at <index>",
                ),
                "count": SubCommand(
                    name="count",
                    arity=Arity(0, 1),
                    detail="Count headers.",
                    synopsis="HTTP::header count ?<name>?",
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Check if header exists.",
                    synopsis="HTTP::header exists <name>",
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(2),
                    detail="Insert header/value pair.",
                    synopsis="HTTP::header insert <name> <value>",
                    credential_arg=2,
                    sensitive_headers=_SENSITIVE_HTTP_HEADERS,
                    mutator=True,
                ),
                "insert_modssl_fields": SubCommand(
                    name="insert_modssl_fields",
                    arity=Arity(0, 0),
                    detail="Insert mod_ssl-compatible headers.",
                    synopsis="HTTP::header insert_modssl_fields",
                    mutator=True,
                ),
                "is_keepalive": SubCommand(
                    name="is_keepalive",
                    arity=Arity(0, 0),
                    detail="Check if connection is keep-alive.",
                    synopsis="HTTP::header is_keepalive",
                ),
                "is_redirect": SubCommand(
                    name="is_redirect",
                    arity=Arity(0, 0),
                    detail="Check if response is a redirect.",
                    synopsis="HTTP::header is_redirect",
                ),
                "lws": SubCommand(
                    name="lws",
                    arity=Arity(0, 1),
                    detail="Enable/disable linear whitespace folding.",
                    synopsis="HTTP::header lws ?enable|disable?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::header lws",
                            arity=Arity(0, 0),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::header lws <enable|disable>",
                            arity=Arity(1, 1),
                            mutator=True,
                        ),
                    ),
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="List all header names.",
                    synopsis="HTTP::header names",
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1, 1),
                    detail="Remove named header.",
                    synopsis="HTTP::header remove <name>",
                    mutator=True,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(2),
                    detail="Replace header value.",
                    synopsis="HTTP::header replace <name> <value>",
                    credential_arg=2,
                    sensitive_headers=_SENSITIVE_HTTP_HEADERS,
                    mutator=True,
                ),
                "sanitize": SubCommand(
                    name="sanitize",
                    arity=Arity(1),
                    detail="Remove headers not in the allow list.",
                    synopsis="HTTP::header sanitize <name-list>",
                    mutator=True,
                ),
                "value": SubCommand(
                    name="value",
                    arity=Arity(1, 1),
                    detail="Get first header value.",
                    synopsis="HTTP::header value <name>",
                ),
                "values": SubCommand(
                    name="values",
                    arity=Arity(1, 1),
                    detail="Get all values for header.",
                    synopsis="HTTP::header values <name>",
                ),
            },
            validation=ValidationSpec(arity=Arity(1)),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            taint_output_sink="IRULE3002",
            taint_output_sink_subcommands=frozenset({"insert", "replace"}),
            cse_candidate=True,
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
