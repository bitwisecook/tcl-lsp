# Enriched from F5 iRules reference documentation.
"""WAM::enable -- Enables Web Accelerator plugin processing on the connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WAM__enable.html"


@register
class WamEnableCommand(CommandDef):
    name = "WAM::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WAM::enable",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables Web Accelerator plugin processing on the connection.",
                synopsis=("WAM::enable",),
                snippet=(
                    "Enables the WAM plugin for the current TCP connection. WAM will remain\n"
                    "enabled on the current TCP connection until it is closed or\n"
                    "WAM::disable is called."
                ),
                source=_SOURCE,
                examples=(
                    "# Disable WAM for HTTP paths ending in .php\n"
                    "when HTTP_REQUEST {\n"
                    "  WAM::enable\n"
                    '  if { [HTTP::path] ends_with ".php" } {\n'
                    "    WAM::disable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WAM::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
