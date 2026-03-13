# Enriched from F5 iRules reference documentation.
"""SSL::session -- Drops a session from the SSL session cache."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__session.html"


_av = make_av(_SOURCE)


@register
class SslSessionCommand(CommandDef):
    name = "SSL::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Drops a session from the SSL session cache.",
                synopsis=("SSL::session invalidate ( drop | nodrop )?",),
                snippet='Invalidates the current session. If no parameter is specified, or the "drop" parameter is specified, this commands drops the current SSL session ID from the session cache to prevent reuse of the session. If "nodrop" is specified, the current session will be invalidated but the session will not be dropped from the session cache.',
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::uri] contains "/maint_mode" } {\n'
                    "        ## send content and die\n"
                    "        HTTP::respond 200 content $::error_html Connection Close\n"
                    "        event HTTP_REQUEST disable\n"
                    "        SSL::session invalidate\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::session invalidate Invalidates the current session. Specifically, this command drops the current SSL session ID from the session cache to prevent reuse of the session.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::session invalidate ( drop | nodrop )?",
                    arg_values={
                        0: (
                            _av(
                                "drop",
                                "SSL::session drop",
                                "SSL::session invalidate ( drop | nodrop )?",
                            ),
                            _av(
                                "nodrop",
                                "SSL::session nodrop",
                                "SSL::session invalidate ( drop | nodrop )?",
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
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
