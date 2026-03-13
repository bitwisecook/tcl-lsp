# Enriched from F5 iRules reference documentation.
"""ADAPT::context_delete_all -- Deletes all dynamic contexts."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_delete_all.html"


@register
class AdaptContextDeleteAllCommand(CommandDef):
    name = "ADAPT::context_delete_all"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_delete_all",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deletes all dynamic contexts.",
                synopsis=("ADAPT::context_delete_all",),
                snippet=(
                    "Deletes all dynamic contexts on both sides of the virtual\n"
                    "server, making the static context the current context. This\n"
                    "is done automatically when the last of a connection flow and\n"
                    "its peer is torn down, so normally need not be called.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_delete_all"
                ),
                source=_SOURCE,
                examples=(
                    "# Conditionally revert to static contexts after request processed\n"
                    "# (contrived example, probably not useful).\n"
                    "when HTTP_PROXY_REQUEST {\n"
                    "    if {$revert_to_profile} {\n"
                    "        ADAPT::context_delete_all\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_delete_all",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
