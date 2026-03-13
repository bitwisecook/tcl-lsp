# Enriched from F5 iRules reference documentation.
"""DIAMETER::avp -- Provides detailed access to diameter attribute-value pairs."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__avp.html"


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
                synopsis=("DIAMETER::avp (",),
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
                    synopsis="DIAMETER::avp (",
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
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
