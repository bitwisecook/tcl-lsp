# Enriched from F5 iRules reference documentation.
"""TAP::action -- Returns or updates security token action."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TAP__action.html"


_av = make_av(_SOURCE)


@register
class TapActionCommand(CommandDef):
    name = "TAP::action"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TAP::action",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or updates security token action.",
                synopsis=(
                    "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                ),
                snippet="Returns action supplied by TAP service. If supplied new action to set function returns previous action.",
                source=_SOURCE,
                examples=(
                    "when TAP_REQUEST {\n"
                    '    if {    ([TAP::action] eq "block") } {\n'
                    "        drop\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns one of the following actions: allow, alarm, basicPolicy, strictPolicy, jsInection, captcha, block, tcpReset, deception.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                    arg_values={
                        0: (
                            _av(
                                "allow",
                                "TAP::action allow",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "alarm",
                                "TAP::action alarm",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "basicPolicy",
                                "TAP::action basicPolicy",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "strictPolicy",
                                "TAP::action strictPolicy",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "jsInjection",
                                "TAP::action jsInjection",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "captcha",
                                "TAP::action captcha",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "block",
                                "TAP::action block",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "tcpReset",
                                "TAP::action tcpReset",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "deception",
                                "TAP::action deception",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                            _av(
                                "conviction",
                                "TAP::action conviction",
                                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"TAP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
