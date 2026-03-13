# Enriched from F5 iRules reference documentation.
"""FIX::tag -- Defines/deletes the mapping between senderCompID and a tag map data group."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FIX__tag.html"


@register
class FixTagCommand(CommandDef):
    name = "FIX::tag"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FIX::tag",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Defines/deletes the mapping between senderCompID and a tag map data group.",
                synopsis=(
                    "FIX::tag map set SENDER DATA_GROUP",
                    "FIX::tag map delete",
                    "FIX::tag get TAG",
                ),
                snippet=(
                    "This command can either retrieve tag value or update the mapping\n"
                    "between senderCompID and a tag map data group. In latter case If a\n"
                    "mapping is already defined in the profile attributes for\n"
                    "sender-tag-map, it is overwritten by the iRule mapping."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "  # with the follow command, tag 10001 is replaced to 20001 for the messages sent by client_1\n"
                    "  # before sending to pool member and reverse-replaced(20001 to 10001) to client_1\n"
                    "  FIX::tag map set client_1 data_group_1\n"
                    "  FIX::tag map set client_2 data_group_1\n"
                    "  FIX::tag map set client_3 data_group_2\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FIX::tag map set SENDER DATA_GROUP",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FIX"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
