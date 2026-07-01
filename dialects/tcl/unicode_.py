"""unicode -- Unicode normalization (Tcl 9.1)."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page unicode.n"

_av = make_av(_SOURCE)


def _normalise_sub(name: str, form: str) -> SubCommand:
    """Build a ``unicode to<form>`` normalization subcommand."""
    return SubCommand(
        name=name,
        # ``?-profile PROFILE? STRING`` — 1 positional, plus an optional
        # value-bearing ``-profile`` flag, so 1 to 3 words total.
        arity=Arity(1, 3),
        detail=f"Return STRING converted to Unicode Normalization Form {form}.",
        synopsis=f"unicode {name} ?-profile PROFILE? STRING",
        options=(OptionSpec(name="-profile", takes_value=True, value_hint="PROFILE"),),
        pure=True,
        return_type=TclType.STRING,
    )


@register
class UnicodeCommand(CommandDef):
    name = "unicode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="unicode",
            not_proc_factory=True,
            dialects=frozenset({"tcl9.1"}),
            hover=HoverSnippet(
                summary="Unicode normalization (Tcl 9.1).",
                synopsis=("unicode subcommand ?-profile PROFILE? STRING",),
                snippet=(
                    "Converts a string to one of the Unicode normalization forms "
                    "(NFC, NFD, NFKC, NFKD). The optional `-profile` option selects "
                    "the encoding profile used for invalid input."
                ),
                source=_SOURCE,
                return_value="The normalized string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unicode subcommand ?-profile PROFILE? STRING",
                    arg_values={
                        0: (
                            _av(
                                "tonfc",
                                "Convert to Normalization Form C (NFC).",
                                "unicode tonfc ?-profile PROFILE? STRING",
                            ),
                            _av(
                                "tonfd",
                                "Convert to Normalization Form D (NFD).",
                                "unicode tonfd ?-profile PROFILE? STRING",
                            ),
                            _av(
                                "tonfkc",
                                "Convert to Normalization Form KC (NFKC).",
                                "unicode tonfkc ?-profile PROFILE? STRING",
                            ),
                            _av(
                                "tonfkd",
                                "Convert to Normalization Form KD (NFKD).",
                                "unicode tonfkd ?-profile PROFILE? STRING",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "tonfc": _normalise_sub("tonfc", "C (NFC)"),
                "tonfd": _normalise_sub("tonfd", "D (NFD)"),
                "tonfkc": _normalise_sub("tonfkc", "KC (NFKC)"),
                "tonfkd": _normalise_sub("tonfkd", "KD (NFKD)"),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
        )
