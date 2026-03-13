# Enriched from F5 iRules reference documentation.
"""DNSMSG::header -- Returns a field from the header of a dns_message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNSMSG-header.html"


_av = make_av(_SOURCE)


@register
class DnsmsgHeaderCommand(CommandDef):
    name = "DNSMSG::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNSMSG::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a field from the header of a dns_message.",
                synopsis=(
                    "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                ),
                snippet="Takes a dns_message structure and field name, and returns the specified field value from the header.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '        set result [RESOLVER::name_lookup "/Common/r1" www.abc.com a]\n'
                    "        set rcode [DNSMSG::header $result rcode]\n"
                    "}"
                ),
                return_value="Returns a field from the header.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                    arg_values={
                        0: (
                            _av(
                                "rcode",
                                "DNSMSG::header rcode",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "opcode",
                                "DNSMSG::header opcode",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "id",
                                "DNSMSG::header id",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "ra",
                                "DNSMSG::header ra",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "rd",
                                "DNSMSG::header rd",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "tc",
                                "DNSMSG::header tc",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "qr",
                                "DNSMSG::header qr",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "aa",
                                "DNSMSG::header aa",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "ad",
                                "DNSMSG::header ad",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                            _av(
                                "cd",
                                "DNSMSG::header cd",
                                "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
