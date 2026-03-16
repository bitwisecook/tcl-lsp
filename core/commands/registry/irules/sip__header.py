# Enriched from F5 iRules reference documentation.
"""SIP::header -- Gets or sets SIP header information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__header.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)


@register
class SipHeaderCommand(CommandDef):
    name = "SIP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets SIP header information.",
                synopsis=(
                    "SIP::header SIP_HEADER_NAME (INDEX)?",
                    "SIP::header ('value' | 'remove') HEADER_NAME (INDEX)?",
                    "SIP::header ('insert') HEADER_NAME HEADER_VALUE (INDEX)?",
                    "SIP::header 'names'",
                ),
                snippet=(
                    "This set of commands allows you to get or set information in the SIP\n"
                    "header.\n"
                    "\n"
                    "Note: These commands still work on MBLB (Message Based Load\n"
                    "Balancing) SIP post 11.6+, but there are new commands that only\n"
                    "run on MRF (Message Routing Framework) SIP and were introduced\n"
                    "in 11.6."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST_SEND {\n"
                    "  log local0. [SIP::method]\n"
                    '  SIP::header insert Via [format "SIP/2.0/TCP %s:%s" [IP::local_addr] [TCP::local_port]]\n'
                    '  SIP::header insert Y-Header "it is yyy"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::header <subcommand|name> ?args?",
                    arg_values={
                        0: (
                            _av("value", "Get header value.", "SIP::header value <name> ?index?"),
                            _av("remove", "Remove a header.", "SIP::header remove <name> ?index?"),
                            _av(
                                "insert",
                                "Insert a header.",
                                "SIP::header insert <name> <value> ?index?",
                            ),
                            _av("names", "List all header names.", "SIP::header names"),
                            _av("at", "Get header at index.", "SIP::header at <index>"),
                            _av("exists", "Check if header exists.", "SIP::header exists <name>"),
                            _av(
                                "replace",
                                "Replace header value.",
                                "SIP::header replace <name> <value> ?index?",
                            ),
                            _av("count", "Count headers with name.", "SIP::header count <name>"),
                            _av(
                                "values", "Get all values for header.", "SIP::header values <name>"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"SIP"}),
                also_in=frozenset({"MR_EGRESS", "MR_FAILED", "MR_INGRESS"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "value": SubCommand(
                    name="value",
                    arity=Arity(1, 2),
                    detail="Get header value by name and optional index.",
                    synopsis="SIP::header value <name> ?index?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1, 2),
                    detail="Remove a header by name and optional index.",
                    synopsis="SIP::header remove <name> ?index?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(2, 3),
                    detail="Insert a new header.",
                    synopsis="SIP::header insert <name> <value> ?index?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="List all header names.",
                    synopsis="SIP::header names",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "at": SubCommand(
                    name="at",
                    arity=Arity(1, 1),
                    detail="Get header at index.",
                    synopsis="SIP::header at <index>",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Check if header exists.",
                    synopsis="SIP::header exists <name>",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(2, 3),
                    detail="Replace header value.",
                    synopsis="SIP::header replace <name> <value> ?index?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "count": SubCommand(
                    name="count",
                    arity=Arity(1, 1),
                    detail="Count headers with name.",
                    synopsis="SIP::header count <name>",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "values": SubCommand(
                    name="values",
                    arity=Arity(1, 1),
                    detail="Get all values for header.",
                    synopsis="SIP::header values <name>",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
