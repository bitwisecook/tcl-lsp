# Enriched from F5 iRules reference documentation.
"""TCP::push_flag -- This command can be used to set/get the PUSH flag mode of a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__push_flag.html"


_av = make_av(_SOURCE)


@register
class TcpPushFlagCommand(CommandDef):
    name = "TCP::push_flag"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::push_flag",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the PUSH flag mode of a TCP connection.",
                synopsis=("TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",),
                snippet=(
                    "TCP::push_flag returns the PUSH flag mode of a TCP connection.\n"
                    "TCP::push_flag mode sets the PUSH flag mode to specified mode."
                ),
                source=_SOURCE,
                examples=(
                    "# get/set the PUSH flag mode of the TCP flow.\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "TCP set PUSH flag mode: [TCP::push_flag auto]"\n'
                    '    log local0. "TCP get PUSH flag more: [TCP::push_flag]"\n'
                    "}"
                ),
                return_value="TCP::push_flag returns the PUSH flag mode.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
                    arg_values={
                        0: (
                            _av(
                                "default",
                                "TCP::push_flag default",
                                "TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
                            ),
                            _av(
                                "none",
                                "TCP::push_flag none",
                                "TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
                            ),
                            _av(
                                "one",
                                "TCP::push_flag one",
                                "TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
                            ),
                            _av(
                                "auto",
                                "TCP::push_flag auto",
                                "TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
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
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
