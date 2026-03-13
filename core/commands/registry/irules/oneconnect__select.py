# Enriched from F5 iRules reference documentation.
"""ONECONNECT::select -- Instruct the proxy to use persistence data as a OneConnect keying label when connecting to a server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ONECONNECT__select.html"


_av = make_av(_SOURCE)


@register
class OneconnectSelectCommand(CommandDef):
    name = "ONECONNECT::select"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ONECONNECT::select",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Instruct the proxy to use persistence data as a OneConnect keying label when connecting to a server.",
                synopsis=("ONECONNECT::select (persist | none)",),
                snippet=(
                    "The 'select persist' command instructs the proxy to use persistence data as the\n"
                    "OneConnect keying label when connecting to the server. NTLM connection pooling\n"
                    "leverages these commands internally, and it is not necessary for the user to\n"
                    "use them directly.  Persistance data should be established via the 'persist'\n"
                    "command."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST_SEND {\n"
                    '     if { $keymatch == "/myuri"} {\n'
                    "     ONECONNECT::label update $keymatch\n"
                    "     }\n"
                    "   }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ONECONNECT::select (persist | none)",
                    arg_values={
                        0: (
                            _av(
                                "persist",
                                "ONECONNECT::select persist",
                                "ONECONNECT::select (persist | none)",
                            ),
                            _av(
                                "none",
                                "ONECONNECT::select none",
                                "ONECONNECT::select (persist | none)",
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
