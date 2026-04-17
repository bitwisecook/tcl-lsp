# Enriched from F5 iRules reference documentation.
"""SSL::unclean_shutdown -- Sets the value of the Unclean Shutdown setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__unclean_shutdown.html"


_av = make_av(_SOURCE)


@register
class SslUncleanShutdownCommand(CommandDef):
    name = "SSL::unclean_shutdown"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::unclean_shutdown",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the value of the Unclean Shutdown setting.",
                synopsis=("SSL::unclean_shutdown (enable | disable)",),
                snippet="Sets the value of the Unclean Shutdown setting. This command only affects the current connection, and only affects the current context (e.g., when run in a client-side context, it only affects the current client-side connection).",
                source=_SOURCE,
                examples=(
                    "# Note that for this iRule, unclean shutdown should be disabled in the clientssl profile\n"
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::header "User-Agent"] contains "MSIE" } {\n'
                    "        SSL::unclean_shutdown enable\n"
                    "    }\n"
                    "}"
                ),
                return_value='SSL::unclean_shutdown <"enable" | "disable"> Sets the value of the current client-side or server-side SSL connection’s Unclean Shutdown setting.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::unclean_shutdown (enable | disable)",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "SSL::unclean_shutdown enable",
                                "SSL::unclean_shutdown (enable | disable)",
                            ),
                            _av(
                                "disable",
                                "SSL::unclean_shutdown disable",
                                "SSL::unclean_shutdown (enable | disable)",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
