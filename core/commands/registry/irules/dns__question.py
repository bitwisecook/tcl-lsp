# Enriched from F5 iRules reference documentation.
"""DNS::question -- Gets (v11.0+) or sets (v11.1+) the question field value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__question.html"


_av = make_av(_SOURCE)


@register
class DnsQuestionCommand(CommandDef):
    name = "DNS::question"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::question",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets (v11.0+) or sets (v11.1+) the question field value.",
                synopsis=(
                    "DNS::question ('name' | 'type') (VALUE)?",
                    "DNS::question 'class' (DNS_CLASS)?",
                ),
                snippet=(
                    "This iRules command gets (v11.0+) or sets (v11.1+) the question field\n"
                    "value.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_REQUEST {\n"
                    '    log local0. "my question name: [DNS::question name]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::question ('name' | 'type') (VALUE)?",
                    arg_values={
                        0: (
                            _av(
                                "name",
                                "DNS::question name",
                                "DNS::question ('name' | 'type') (VALUE)?",
                            ),
                            _av(
                                "type",
                                "DNS::question type",
                                "DNS::question ('name' | 'type') (VALUE)?",
                            ),
                            _av(
                                "class", "DNS::question class", "DNS::question 'class' (DNS_CLASS)?"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
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
