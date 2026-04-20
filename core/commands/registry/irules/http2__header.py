# Enriched from F5 iRules reference documentation.
"""HTTP2::header -- Queries or modifies HTTP/2 pseudo-headers."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__header.html"


@register
class Http2HeaderCommand(CommandDef):
    name = "HTTP2::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries or modifies HTTP/2 pseudo-headers.",
                synopsis=(
                    "HTTP2::header <name>",
                    "HTTP2::header replace <name> ?<string>?",
                    "HTTP2::header remove <name>",
                ),
                snippet=(
                    "Queries or modifies HTTP/2 pseudo-headers.\n"
                    "The HTTP/2 pseudo-header names are lowercase and start with a ':' character.\n"
                    "Introduced in BIG-IP 16.1.0."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  if { [HTTP2::header :authority] starts_with "uat" }  {\n'
                    "    pool uat_pool\n"
                    "  } else {\n"
                    "    pool main_pool\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP2::header <name>",
                    arity=Arity(1, 1),
                    pure=True,
                ),
            ),
            subcommands={
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(1, 2),
                    detail="Replace pseudo-header value.",
                    synopsis="HTTP2::header replace <name> ?<string>?",
                    mutator=True,
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1, 1),
                    detail="Remove a pseudo-header.",
                    synopsis="HTTP2::header remove <name>",
                    mutator=True,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP"}),
                also_in=frozenset({"MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
