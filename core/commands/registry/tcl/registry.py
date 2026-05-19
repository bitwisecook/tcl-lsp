"""registry -- Windows registry access (Windows-only Tcl command)."""

from __future__ import annotations

from compiler.types import TclType

from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page registry.n"


@register
class RegistryCommand(CommandDef):
    name = "registry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="registry",
            dialects=frozenset({"tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Windows registry manipulation (Windows-only).",
                synopsis=(
                    "registry broadcast keyName ?-timeout ms?",
                    "registry delete keyName ?valueName?",
                    "registry get keyName valueName",
                    "registry keys keyName ?pattern?",
                    "registry set keyName ?valueName data ?type??",
                    "registry type keyName valueName",
                    "registry values keyName ?pattern?",
                ),
                snippet=(
                    "Not available in the WASM sandbox (Windows-specific) "
                    "— traps with ``unsupported command: registry``."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="registry subcommand keyName ?args ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
        )
