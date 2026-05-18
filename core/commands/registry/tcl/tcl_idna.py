"""tcl::idna -- Internationalised Domain Name (Punycode) helpers (Tcl 9.0+).

Bundled package providing punycode/IDNA encode and decode. Loaded via
``package require tcl::idna 1.0``. Two name forms are registered for the
namespace-relative and fully-qualified spellings.
"""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page idna.n"

_HOVER = HoverSnippet(
    summary="Internationalised Domain Name (IDNA/Punycode) helpers.",
    synopsis=("tcl::idna subcommand ?arg ...?",),
    snippet=(
        "Sub-commands: `decode`, `encode`, `puny decode`, `puny encode`, "
        "`version`. Implements RFC 3492 punycode used for IDNA. Loaded "
        "via `package require tcl::idna 1.0`."
    ),
    source=_SOURCE,
)

_SUBCOMMANDS = (
    ArgumentValueSpec(value="decode", detail="Decode punycode in a hostname."),
    ArgumentValueSpec(value="encode", detail="Encode a hostname with punycode."),
    ArgumentValueSpec(value="puny", detail="Direct access to the punycode encoder/decoder."),
    ArgumentValueSpec(value="version", detail="Return the tcl::idna package version."),
)

_PUNY_SUBCOMMANDS = (
    ArgumentValueSpec(value="decode", detail="Decode a punycode string."),
    ArgumentValueSpec(value="encode", detail="Encode a string as punycode."),
)


def _ensemble_subcommands() -> dict[str, SubCommand]:
    return {
        "decode": SubCommand(
            name="decode",
            arity=Arity(1, 1),
            detail="Decode punycode in a hostname for display.",
            synopsis="tcl::idna decode hostname",
            return_type=TclType.STRING,
            pure=True,
        ),
        "encode": SubCommand(
            name="encode",
            arity=Arity(1, 1),
            detail="Encode a hostname with punycode where needed.",
            synopsis="tcl::idna encode hostname",
            return_type=TclType.STRING,
            pure=True,
        ),
        "puny": SubCommand(
            name="puny",
            arity=Arity(2, 3),
            detail="Direct access to the punycode encoder/decoder.",
            synopsis="tcl::idna puny decode|encode string ?case?",
            return_type=TclType.STRING,
            pure=True,
            arg_values={0: _PUNY_SUBCOMMANDS},
        ),
        "version": SubCommand(
            name="version",
            arity=Arity(0, 0),
            detail="Return the tcl::idna package version.",
            synopsis="tcl::idna version",
            return_type=TclType.STRING,
            pure=True,
        ),
    }


def _make_spec(name: str) -> CommandSpec:
    return CommandSpec(
        name=name,
        dialects=frozenset({"tcl9.0"}),
        required_package="tcl::idna",
        hover=_HOVER,
        forms=(
            FormSpec(
                kind=FormKind.DEFAULT,
                synopsis="tcl::idna subcommand ?arg ...?",
                arg_values={0: _SUBCOMMANDS},
            ),
        ),
        validation=ValidationSpec(arity=Arity(1)),
        subcommands=_ensemble_subcommands(),
    )


@register
class TclIdnaCommand(CommandDef):
    name = "tcl::idna"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _make_spec(cls.name)


@register
class TclIdnaQualifiedCommand(CommandDef):
    name = "::tcl::idna"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _make_spec(cls.name)
