# Enriched from F5 iRules reference documentation.
"""REWRITE::payload -- Queries for or manipulates REWRITE payload."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REWRITE__payload.html"


@register
class RewritePayloadCommand(CommandDef):
    name = "REWRITE::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REWRITE::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or manipulates REWRITE payload.",
                synopsis=(
                    "REWRITE::payload (LENGTH | (OFFSET LENGTH))?",
                    "REWRITE::payload length",
                    "REWRITE::payload replace OFFSET LENGTH PAYLOAD",
                ),
                snippet=(
                    "Queries for or manipulates REWRITE payload (content) information. With\n"
                    "this command, you can retrieve content, query for content size, or\n"
                    "replace a certain amount of content."
                ),
                source=_SOURCE,
                examples=(
                    "when REWRITE_RESPONSE_DONE {\n"
                    "    # The rewrite_response_done event isn't absolutely necessary because browser will just ignore any html tags that it doesn't recongnize.\n"
                    "    # However, it will be cleaner if we remove it nevertheless\n"
                    "\n"
                    "    set data [REWRITE::payload]\n"
                    "    # Find the tags we inserted\n"
                    "    set start [string first {<apm_do_not_touch>} $data]\n"
                    "    set end [string last {</apm_do_not_touch>} $data]\n"
                    "    # Determines the amount of characters to remove"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REWRITE::payload (LENGTH | (OFFSET LENGTH))?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"REWRITE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
