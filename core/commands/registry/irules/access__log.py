# Enriched from F5 iRules reference documentation.
"""ACCESS::log -- Logs a message using APM logging framework."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__log.html"


@register
class AccessLogCommand(CommandDef):
    name = "ACCESS::log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::log",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Logs a message using APM logging framework.",
                synopsis=("ACCESS::log (COMPONENT_LOGLEVEL)? MSG",),
                snippet=(
                    "ACCESS::log [component.][loglevel] <message>\n"
                    "\n"
                    "Logs the specified message using the optionally specified APM component name\n"
                    "and log level as specified in the log setting for the access profile that is\n"
                    "assigned to the virtual server.\n"
                    "The message is sent to the destination specified in the log setting.\n"
                    "If component is specified, it must be one of the supported values (see below)\n"
                    "and must end with a dot character. If not specified, the accesscontrol component\n"
                    "is assumed.\n"
                    "If log level is specified, it must be one of the supported values (see below).\n"
                    "If not specified, notice level is assumed."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    ACCESS::log debug "an Access Control debug log"\n'
                    '    ACCESS::log sso.error "an SSO error log"\n'
                    '    ACCESS::log eca. "an ECA notice log"\n'
                    '    ACCESS::log "an Access Control notice log"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::log (COMPONENT_LOGLEVEL)? MSG",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
