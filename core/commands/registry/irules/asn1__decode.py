# Enriched from F5 iRules reference documentation.
"""ASN1::decode -- Decodes ASN.1 records."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASN1__decode.html"


@register
class Asn1DecodeCommand(CommandDef):
    name = "ASN1::decode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASN1::decode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Decodes ASN.1 records.",
                synopsis=("ASN1::decode ELEMENT FORMAT (VAR_NAME)*",),
                snippet=(
                    "This command is used to decode ASN.1 records. element specifies the\n"
                    "data to decode. It can be either a byte array (like is returned from\n"
                    "TCP::payload), or an element object returned by one of the other\n"
                    "commands. The bytes are decoded according to formatString, and the\n"
                    "results are stored in variables whose names are provided as command\n"
                    "arguments. If a variable with a name specified doesn't exist, it will\n"
                    "be created in the current scope. If the variable does exist, it's value\n"
                    "will be overwritten. The command returns the number of elements\n"
                    "decoded."
                ),
                source=_SOURCE,
                examples=(
                    'ASN1::decode $ele "?a?aa?b" ruleId type matchValue dnAttrs\n'
                    "if {![info exists ruleId] && ![info exists type]} {\n"
                    '  log local0. "ERR: extensibleMatch must contain either a matchingRule or type component"\n'
                    "}\n"
                    "# Handle default value for dnAttributes component\n"
                    "if {![info exists dnAttrs]} {\n"
                    "  set dnAttrs 0\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASN1::decode ELEMENT FORMAT (VAR_NAME)*",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
