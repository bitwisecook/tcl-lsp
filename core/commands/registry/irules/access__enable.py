# Enriched from F5 iRules reference documentation.
"""ACCESS::enable -- Enables the access control enforcement for a particular request URI."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__enable.html"


@register
class AccessEnableCommand(CommandDef):
    name = "ACCESS::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables the access control enforcement for a particular request URI.",
                synopsis=("ACCESS::enable",),
                snippet=(
                    "This command enables the access control enforcement for a particular\n"
                    "request URI.\n"
                    "\n"
                    "ACCESS::enable\n"
                    "\n"
                    "     * Enables the access control enforcement for a particular request\n"
                    "       URI.\n"
                    "\n"
                    " * Requires APM module"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "\n"
                    "       # Check the requested HTTP path\n"
                    "       switch -glob [string tolower [HTTP::path]] {\n"
                    '              "/apm_uri1*" -\n'
                    '              "/apm_uri2*" -\n'
                    '              "/apm_uri3*" {\n'
                    "                     # Enable APM for these paths\n"
                    "                     ACCESS::enable\n"
                    "              }\n"
                    "              default {\n"
                    "                     # Disable APM for all other paths\n"
                    "                     ACCESS::disable\n"
                    "              }\n"
                    "       }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
