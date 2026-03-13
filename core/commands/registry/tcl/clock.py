"""clock -- Obtain and manipulate dates and times."""

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
from ._base import register

_SOURCE = "Tcl man page clock.n"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


def _opt(
    name: str, detail: str = "", *, takes_value: bool = False, value_hint: str = ""
) -> OptionSpec:
    return OptionSpec(name=name, detail=detail, takes_value=takes_value, value_hint=value_hint)


_SUBCOMMANDS = (
    _av("add", "Add a duration to a clock value.", "clock add timeVal count unit ?count unit ...?"),
    _av("clicks", "Return high-resolution click counter.", "clock clicks ?-option?"),
    _av(
        "format",
        "Format a clock value as a date/time string.",
        "clock format timeVal ?-option value ...?",
    ),
    _av("microseconds", "Return current time in microseconds.", "clock microseconds"),
    _av("milliseconds", "Return current time in milliseconds.", "clock milliseconds"),
    _av(
        "scan",
        "Parse a date/time string to a clock value.",
        "clock scan inputString ?-option value ...?",
    ),
    _av("seconds", "Return current time in seconds.", "clock seconds"),
)

_TIME_UNITS = (
    _av("seconds", "Seconds."),
    _av("minutes", "Minutes (60 seconds)."),
    _av("hours", "Hours (3600 seconds)."),
    _av("days", "Days (86400 seconds)."),
    _av("weekdays", "Weekdays (skipping Saturday and Sunday)."),
    _av("weeks", "Weeks (7 days)."),
    _av("months", "Calendar months."),
    _av("years", "Calendar years."),
)

_FORMAT_SCAN_OPTIONS = (
    _opt("-base", "Base time for relative scanning.", takes_value=True, value_hint="timeVal"),
    _opt("-format", "strftime-style format string.", takes_value=True, value_hint="format"),
    _opt("-gmt", "Use GMT instead of local time.", takes_value=True, value_hint="boolean"),
    _opt("-locale", "Locale for month/day names.", takes_value=True, value_hint="locale"),
    _opt("-timezone", "Time zone for conversion.", takes_value=True, value_hint="zone"),
    _opt("-validate", "Validate date fields strictly.", takes_value=True, value_hint="boolean"),
)


@register
class ClockCommand(CommandDef):
    name = "clock"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="clock",
            hover=HoverSnippet(
                summary="Obtain and manipulate dates and times.",
                synopsis=(
                    "clock add timeVal count unit ?...?",
                    "clock format timeVal ?-option value ...?",
                    "clock scan inputString ?-option value ...?",
                    "clock seconds",
                ),
                snippet="Use `clock seconds` for epoch time, `clock format` to display, `clock scan` to parse.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="clock subcommand ?arg ...?",
                    options=_FORMAT_SCAN_OPTIONS,
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(2),
                    detail="Add a duration to a clock value.",
                    synopsis="clock add timeVal count unit ?count unit ...?",
                    pure=True,
                    return_type=TclType.INT,
                    arg_values={1: _TIME_UNITS, 3: _TIME_UNITS, 5: _TIME_UNITS, 7: _TIME_UNITS},
                ),
                "clicks": SubCommand(
                    name="clicks",
                    arity=Arity(0, 1),
                    detail="Return high-resolution click counter.",
                    synopsis="clock clicks ?-option?",
                    return_type=TclType.INT,
                ),
                "format": SubCommand(
                    name="format",
                    arity=Arity(1),
                    detail="Format a clock value as a date/time string.",
                    synopsis="clock format timeVal ?-option value ...?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "microseconds": SubCommand(
                    name="microseconds",
                    arity=Arity(0, 0),
                    detail="Return current time in microseconds.",
                    synopsis="clock microseconds",
                    return_type=TclType.INT,
                ),
                "milliseconds": SubCommand(
                    name="milliseconds",
                    arity=Arity(0, 0),
                    detail="Return current time in milliseconds.",
                    synopsis="clock milliseconds",
                    return_type=TclType.INT,
                ),
                "scan": SubCommand(
                    name="scan",
                    arity=Arity(1),
                    detail="Parse a date/time string to a clock value.",
                    synopsis="clock scan inputString ?-option value ...?",
                    pure=True,
                    return_type=TclType.INT,
                ),
                "seconds": SubCommand(
                    name="seconds",
                    arity=Arity(0, 0),
                    detail="Return current time in seconds.",
                    synopsis="clock seconds",
                    return_type=TclType.INT,
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
