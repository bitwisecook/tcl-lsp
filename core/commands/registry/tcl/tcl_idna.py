"""tcl::idna -- Internationalised Domain Name (IDNA/Punycode) helpers."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class TclIdnaCommand(CommandDef):
    name = "tcl::idna"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::idna",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Internationalised Domain Name (IDNA/Punycode) helpers.",
                synopsis=("tcl::idna subcommand ?arg ...?",),
                source="Tcl man page idna.n",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="tcl::idna subcommand ?arg ...?"),),
            validation=ValidationSpec(arity=Arity(1)),
            return_type=TclType.STRING,
        )
