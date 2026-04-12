# Enriched from F5 iRules reference documentation.
"""SIP::method -- Returns the type of SIP request method."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__method.html"


@register
class SipMethodCommand(CommandDef):
    name = "SIP::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::method",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the type of SIP request method.",
                synopsis=("SIP::method",),
                snippet=(
                    "Returns the type of SIP request method.\n"
                    "\n"
                    "SIP::method\n"
                    "\n"
                    "     * Returns the type of SIP request method."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    "  log local0. [SIP::uri]\n"
                    "  log local0. [SIP::header Via 0]\n"
                    '  if {[SIP::method] == "INVITE"} {\n'
                    '    SIP::respond 401 "no way" X-Header "xxx here"\n'
                    "  }\n"
                    "}"
                ),
                return_value="Returns the type of SIP request method",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::method",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
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
