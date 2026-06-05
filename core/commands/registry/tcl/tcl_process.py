"""tcl::process -- Manage subprocesses started by exec / open |cmd."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class TclProcessCommand(CommandDef):
    name = "tcl::process"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::process",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Manage subprocesses started by exec / open |cmd.",
                synopsis=("tcl::process subcommand ?arg ...?",),
                source="Tcl man page process.n",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="tcl::process subcommand ?arg ...?"),),
            validation=ValidationSpec(arity=Arity(1)),
            return_type=TclType.STRING,
        )
