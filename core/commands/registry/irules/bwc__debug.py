# Enriched from F5 iRules reference documentation.
"""BWC::debug -- This command is used for troubleshooting a bwc policy instance."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__debug.html"


_av = make_av(_SOURCE)


@register
class BwcDebugCommand(CommandDef):
    name = "BWC::debug"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::debug",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used for troubleshooting a bwc policy instance.",
                synopsis=(
                    "BWC::debug ('start')",
                    "BWC::debug ('stop')",
                ),
                snippet="This command enables debug logs on per policy instance. However the bwc sys db variables for bwc trace need to be enabled and appropriate levels needs to be set as required.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mycookie [IP::remote_addr]:[TCP::remote_port]\n"
                    "    BWC::policy attach test_pol $mycookie\n"
                    '    log  local0. "BWC::policy attach  $mycookie"\n'
                    "    BWC::debug start session\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::debug ('start')",
                    arg_values={
                        0: (
                            _av("start", "BWC::debug start", "BWC::debug ('start')"),
                            _av("stop", "BWC::debug stop", "BWC::debug ('stop')"),
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
