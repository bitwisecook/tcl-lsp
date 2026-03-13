# Enriched from F5 iRules reference documentation.
"""ONECONNECT::reuse -- Controls server-side connection reuse."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ONECONNECT-reuse.html"


@register
class OneconnectReuseCommand(CommandDef):
    name = "ONECONNECT::reuse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ONECONNECT::reuse",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls server-side connection reuse.",
                synopsis=("ONECONNECT::reuse (BOOL_VALUE)?",),
                snippet=(
                    "This command controls whether server-side connections are picked from\n"
                    "the pool of idle connections, and whether idle server-side connections\n"
                    "are returned to the pool or closed when a client connection detaches or\n"
                    "closes. It will also display the current status of connection reuse, if\n"
                    "called without any options.\n"
                    "For information on how to control the detaching behavior, see\n"
                    "ONECONNECT::detach.\n"
                    "The semantics of this command depend on the context in which it is\n"
                    "being executed. Refer to Considering Context Part 1 and\n"
                    "Considering Context Part 2 for more information on contexts."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "if {[HTTP::method] equals GET } {\n"
                    "      ONECONNECT::reuse enable\n"
                    "   } else {\n"
                    "      ONECONNECT::reuse disable\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ONECONNECT::reuse (BOOL_VALUE)?",
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
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
