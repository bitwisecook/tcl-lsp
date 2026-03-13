"""encoding -- Convert between character encodings."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register

_SOURCE = "Tcl man page encoding.n"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av(
        "convertfrom",
        "Convert byte data to Unicode.",
        "encoding convertfrom ?-profile profile? ?encoding? data",
    ),
    _av(
        "convertto",
        "Convert Unicode to byte data.",
        "encoding convertto ?-profile profile? ?encoding? string",
    ),
    _av("dirs", "Return or set encoding search path.", "encoding dirs ?directoryList?"),
    _av("names", "Return list of available encodings.", "encoding names"),
    _av("profiles", "Return list of available profiles.", "encoding profiles"),
    _av("system", "Query or set system encoding.", "encoding system ?encoding?"),
    _av("user", "Query or set user encoding.", "encoding user ?encoding?"),
)

_PROFILES = (
    _av("strict", "Stop on conversion error. Unicode-conformant."),
    _av("tcl8", "Map invalid bytes to equivalent code points. Tcl 8 compatible."),
    _av("replace", "Replace invalid data with U+FFFD. Unicode-conformant."),
)


@register
class EncodingCommand(CommandDef):
    name = "encoding"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="encoding",
            hover=HoverSnippet(
                summary="Convert between character encodings.",
                synopsis=(
                    "encoding convertfrom ?-profile profile? ?encoding? data",
                    "encoding convertto ?-profile profile? ?encoding? string",
                    "encoding names",
                    "encoding system ?encoding?",
                ),
                snippet="Use `encoding names` to list available encodings.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="encoding subcommand ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-profile",
                            takes_value=True,
                            value_hint="profile",
                            detail="Encoding profile (strict, tcl8, replace).",
                        ),
                    ),
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "convertfrom": SubCommand(
                    name="convertfrom",
                    arity=Arity(1),
                    detail="Convert byte data to Unicode.",
                    synopsis="encoding convertfrom ?-profile profile? ?encoding? data",
                    return_type=TclType.STRING,
                    arg_values={0: _PROFILES},
                ),
                "convertto": SubCommand(
                    name="convertto",
                    arity=Arity(1),
                    detail="Convert Unicode to byte data.",
                    synopsis="encoding convertto ?-profile profile? ?encoding? string",
                    return_type=TclType.BYTEARRAY,
                    arg_values={0: _PROFILES},
                ),
                "dirs": SubCommand(
                    name="dirs",
                    arity=Arity(0, 1),
                    detail="Return or set encoding search path.",
                    synopsis="encoding dirs ?directoryList?",
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="Return list of available encodings.",
                    synopsis="encoding names",
                    return_type=TclType.LIST,
                ),
                "profiles": SubCommand(
                    name="profiles",
                    arity=Arity(0, 0),
                    detail="Return list of available profiles.",
                    synopsis="encoding profiles",
                ),
                "system": SubCommand(
                    name="system",
                    arity=Arity(0, 1),
                    detail="Query or set system encoding.",
                    synopsis="encoding system ?encoding?",
                    return_type=TclType.STRING,
                ),
                "user": SubCommand(
                    name="user",
                    arity=Arity(0, 1),
                    detail="Query or set user encoding.",
                    synopsis="encoding user ?encoding?",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={None: TaintColour.TAINTED},
            source_subcommands=frozenset({"convertfrom"}),
        )
