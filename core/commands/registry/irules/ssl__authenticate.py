# Enriched from F5 iRules reference documentation.
"""SSL::authenticate -- Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__authenticate.html"


_av = make_av(_SOURCE)


@register
class SslAuthenticateCommand(CommandDef):
    name = "SSL::authenticate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::authenticate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.",
                synopsis=("SSL::authenticate (once | always | (depth DEPTH))",),
                snippet=(
                    "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.\n"
                    "\n"
                    'SSL::authenticate <"once" | "always">\n'
                    "    Valid in a client-side context only, this command overrides the client-side SSL connection’s current setting regarding authentication frequency.\n"
                    "\n"
                    "SSL::authenticate depth <number>\n"
                    "    When the system evaluates the command in a client-side context, the command overrides the client-side SSL connection’s current setting regarding maximum certificate chain traversal depth."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    set session_flag 0\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::authenticate (once | always | (depth DEPTH))",
                    arg_values={
                        0: (
                            _av(
                                "depth",
                                "SSL::authenticate depth",
                                "SSL::authenticate (once | always | (depth DEPTH))",
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
