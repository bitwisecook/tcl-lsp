# Enriched from F5 iRules reference documentation.
"""ASN1::element -- Returns ASN1.1 record elements."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASN1__element.html"


_av = make_av(_SOURCE)


@register
class Asn1ElementCommand(CommandDef):
    name = "ASN1::element"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASN1::element",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns ASN1.1 record elements.",
                synopsis=(
                    "ASN1::element init ('BER' | 'DER')",
                    "ASN1::element next ELEMENT (NUM_ELEMENTS)?",
                    "ASN1::element byte_offset ELEMENT (OFFSET)?",
                    "ASN1::element tag ELEMENT",
                ),
                snippet=(
                    "This command returns ASN1.1 record elements.\n"
                    "\n"
                    "ASN1::element init encodingType\n"
                    "\n"
                    "     * Returns an element (Tcl_Obj) handle used by the remaining commands.\n"
                    "       encodingType specifies the encoding type that subsequent commands\n"
                    "       should use (BER|DER).\n"
                    "\n"
                    "ASN1::element next element ?numberOfElements?\n"
                    "\n"
                    "     * Returns the next element found after element. If numberOfElements\n"
                    "       is specified, the command will move ahead that many elements,\n"
                    "       otherwise, the default is 1.\n"
                    "\n"
                    "ASN1::element byte_offset element ?offset?\n"
                    "\n"
                    "     * Returns the byte offset within the payload."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASN1::element init ('BER' | 'DER')",
                    arg_values={
                        0: (
                            _av("BER", "ASN1::element BER", "ASN1::element init ('BER' | 'DER')"),
                            _av("DER", "ASN1::element DER", "ASN1::element init ('BER' | 'DER')"),
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
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
