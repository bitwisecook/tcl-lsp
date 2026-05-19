# Enriched from F5 iRules reference documentation.
"""DIAMETER::avp -- Provides detailed access to diameter attribute-value pairs."""

# Introduced: BIG-IP v11+ (Diameter module) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__avp.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        connection_side=ConnectionSide.BOTH,
    ),
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
class DiameterAvpCommand(CommandDef):
    name = "DIAMETER::avp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::avp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides detailed access to diameter attribute-value pairs.",
                synopsis=(
                    "DIAMETER::avp <subcommand> ?args?",
                    "DIAMETER::avp code <avp_code> ?vendor_id? ?index?",
                    "DIAMETER::avp data <avp_code> ?vendor_id? ?index?",
                ),
                snippet=(
                    "This iRule command gives access to set and get attribute-value pairs.\n"
                    "Specifics for each command are below in the syntax section.\n"
                    "\n"
                    "The AVP upon which this command operates is specified in a flexible\n"
                    "manner.  An AVP name or code (usually) must be specified, and an\n"
                    "optional index may be also specified.  Many commands also accept a\n"
                    "vendor-id.  When an AVP name is specified, it is converted to a code.\n"
                    "Names are written as listed in RFC 3588, formatted as e.g.,\n"
                    '"HOST-IP-ADDRESS".  AVP codes are 32-bit (4-octet) integer values.'
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_EGRESS {\n"
                    "     # Sets the flags of the AVP Product Name to 0 (for Vendor Specific, Mandatory, Protected and Reserved)\n"
                    "     DIAMETER::avp flags set 269 0\n"
                    "     # Checks that the flags are properly set (was a bug in 11.3, solved in 11.4)\n"
                    '     log local0. "AVP : [DIAMETER::avp flags get 269] "\n'
                    "     # Removes the Supported-Vendor-Id from the request\n"
                    "     DIAMETER::avp delete 265\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::avp <subcommand> ?args?",
                    arg_values={
                        0: (
                            _av(
                                "code",
                                "Get/set AVP code.",
                                "DIAMETER::avp code <avp_code> ?vendor_id? ?index?",
                            ),
                            _av(
                                "count",
                                "Count AVPs matching code.",
                                "DIAMETER::avp count <avp_code> ?vendor_id?",
                            ),
                            _av(
                                "length",
                                "Get AVP length.",
                                "DIAMETER::avp length <avp_code> ?vendor_id? ?index?",
                            ),
                            _av(
                                "create",
                                "Create a new AVP.",
                                "DIAMETER::avp create <avp_code> <data> ?vendor_id?",
                            ),
                            _av(
                                "data",
                                "Get/set AVP data.",
                                "DIAMETER::avp data <avp_code> ?vendor_id? ?index?",
                            ),
                            _av(
                                "delete",
                                "Delete an AVP.",
                                "DIAMETER::avp delete <avp_code> ?vendor_id? ?index?",
                            ),
                            _av(
                                "replace",
                                "Replace AVP data.",
                                "DIAMETER::avp replace <avp_code> <data> ?vendor_id? ?index?",
                            ),
                            _av(
                                "insert",
                                "Insert a new AVP.",
                                "DIAMETER::avp insert <position> <avp_code> <data> ?vendor_id?",
                            ),
                            _av(
                                "append",
                                "Append an AVP.",
                                "DIAMETER::avp append <avp_code> <data> ?vendor_id?",
                            ),
                            _av(
                                "flags",
                                "Get/set AVP flags.",
                                "DIAMETER::avp flags <get|set> <avp_code> ?value? ?vendor_id? ?index?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "code": SubCommand(
                    name="code",
                    arity=Arity(1, 3),
                    detail="Get/set AVP code.",
                    synopsis="DIAMETER::avp code <avp_code> ?vendor_id? ?index?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "count": SubCommand(
                    name="count",
                    arity=Arity(1, 2),
                    detail="Count AVPs matching code.",
                    synopsis="DIAMETER::avp count <avp_code> ?vendor_id?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "length": SubCommand(
                    name="length",
                    arity=Arity(1, 3),
                    detail="Get AVP length.",
                    synopsis="DIAMETER::avp length <avp_code> ?vendor_id? ?index?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "create": SubCommand(
                    name="create",
                    arity=Arity(2, 3),
                    detail="Create a new AVP.",
                    synopsis="DIAMETER::avp create <avp_code> <data> ?vendor_id?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "data": SubCommand(
                    name="data",
                    arity=Arity(1, 3),
                    detail="Get/set AVP data.",
                    synopsis="DIAMETER::avp data <avp_code> ?vendor_id? ?index?",
                    pure=True,
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1, 3),
                    detail="Delete an AVP.",
                    synopsis="DIAMETER::avp delete <avp_code> ?vendor_id? ?index?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(2, 4),
                    detail="Replace AVP data.",
                    synopsis="DIAMETER::avp replace <avp_code> <data> ?vendor_id? ?index?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(3, 4),
                    detail="Insert a new AVP at position.",
                    synopsis="DIAMETER::avp insert <position> <avp_code> <data> ?vendor_id?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "append": SubCommand(
                    name="append",
                    arity=Arity(2, 3),
                    detail="Append an AVP.",
                    synopsis="DIAMETER::avp append <avp_code> <data> ?vendor_id?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "flags": SubCommand(
                    name="flags",
                    arity=Arity(2, 5),
                    detail="Get/set AVP flags.",
                    synopsis="DIAMETER::avp flags <get|set> <avp_code> ?value? ?vendor_id? ?index?",
                    pure=True,
                    mutator=True,
                    arg_values={
                        0: (
                            _av(
                                "get",
                                "Get AVP flags.",
                                "DIAMETER::avp flags get <avp_code> ?vendor_id? ?index?",
                            ),
                            _av(
                                "set",
                                "Set AVP flags.",
                                "DIAMETER::avp flags set <avp_code> <value> ?vendor_id? ?index?",
                            ),
                        )
                    },
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
