"""tm -- Tcl module system commands (auto-loaded, no package require needed)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl stdlib tm module system"


@register
class TclTmPath(CommandDef):
    name = "tcl::tm::path"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::tm::path",
            hover=HoverSnippet(
                summary="Manage the list of paths searched for Tcl modules.",
                synopsis=(
                    "tcl::tm::path add ?path ...?",
                    "tcl::tm::path remove ?path ...?",
                    "tcl::tm::path list",
                ),
                snippet=(
                    "The ``add`` subcommand prepends paths to the module search "
                    "list, ``remove`` deletes them, and ``list`` returns the "
                    "current list."
                ),
                source=_SOURCE,
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(0),
                ),
                "list": SubCommand(
                    name="list",
                    arity=Arity(0, 0),
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(0),
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class TclTmRoots(CommandDef):
    name = "tcl::tm::roots"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::tm::roots",
            hover=HoverSnippet(
                summary="Set the root paths for Tcl module discovery.",
                synopsis=("tcl::tm::roots pathList",),
                snippet=(
                    "Given a list of root paths, computes version-specific "
                    "sub-directories and adds them to the module search list."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
