# Enriched from F5 iRules reference documentation.
"""ADAPT::result -- Sets or returns the adaptation result code."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__result.html"


_av = make_av(_SOURCE)


@register
class AdaptResultCommand(CommandDef):
    name = "ADAPT::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the adaptation result code.",
                synopsis=(
                    "ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?",
                ),
                snippet=(
                    "The ADAPT::result command sets or returns the adaptation result\n"
                    "code of the ADAPT filter on the current or specified side of the\n"
                    "virtual server connection for which the iRule is being executed.\n"
                    "\n"
                    "Possible result codes are:\n"
                    "    * unknown - The internal virtual server has not returned a\n"
                    "      result yet. It is not possible to change the result code\n"
                    "      to this value.\n"
                    "    * bypass - The internal virtual server does not need to modify\n"
                    "      the request or response."
                ),
                source=_SOURCE,
                examples=(
                    "when ADAPT_REQUEST_RESULT {\n"
                    '     if {[ADAPT::result] == "respond"} {\n'
                    "        # Force ADAPT to ignore any direct response from IVS\n"
                    "        # (contrived example, probably not useful as-is).\n"
                    "        ADAPT::result bypass\n"
                    "     }\n"
                    "}"
                ),
                return_value="Returns the current or modified result code.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?",
                    arg_values={
                        0: (
                            _av(
                                "bypass",
                                "ADAPT::result bypass",
                                "ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?",
                            ),
                            _av(
                                "close",
                                "ADAPT::result close",
                                "ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?",
                            ),
                            _av(
                                "abort",
                                "ADAPT::result abort",
                                "ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"REQUESTADAPT", "RESPONSEADAPT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
