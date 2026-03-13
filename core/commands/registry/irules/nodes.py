# Enriched from F5 iRules reference documentation.
"""nodes -- Lists all nodes within a given pool."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/nodes.html"


@register
class NodesCommand(CommandDef):
    name = "nodes"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="nodes",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Lists all nodes within a given pool.",
                synopsis=("nodes (-list)? POOL_OBJ",),
                snippet=(
                    "This command behaves like active_nodes but lists all nodes in a pool,\n"
                    "not just nodes that are currently active."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "        set in_path [HTTP::path]\n"
                    '        log local0. "debug request: path $in_path"\n'
                    "        switch -glob $in_path {\n"
                    '                "/pool*" {\n'
                    '                        set pool [string map {"/pool" ""} $in_path]\n'
                    '                        HTTP::respond 200 content "[active_members $pool]:[nodes $pool]"\n'
                    "                }\n"
                    "        }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="nodes (-list)? POOL_OBJ",
                    options=(OptionSpec(name="-list", detail="Option -list.", takes_value=False),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
