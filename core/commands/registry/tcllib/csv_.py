"""csv -- CSV parsing and generation (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib csv package"
_PACKAGE = "csv"


@register
class CsvSplitCommand(CommandDef):
    name = "csv::split"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Split a CSV-formatted line into a list of values.",
                synopsis=(
                    "csv::split line ?sepChar? ?quoteChar?",
                    "csv::split -alternate line ?sepChar? ?quoteChar?",
                ),
                source=_SOURCE,
                examples='set fields [csv::split $line ","]',
                return_value="A Tcl list of field values.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::split line ?sepChar? ?quoteChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 4)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class CsvJoinCommand(CommandDef):
    name = "csv::join"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Join a list of values into a CSV-formatted line.",
                synopsis=("csv::join values ?sepChar? ?quoteChar?",),
                source=_SOURCE,
                examples='set line [csv::join $fields ","]',
                return_value="A CSV-formatted string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::join values ?sepChar? ?quoteChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 4)),
        )


@register
class CsvRead2MatrixCommand(CommandDef):
    name = "csv::read2matrix"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Read CSV data from a channel into a matrix object.",
                synopsis=("csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?",),
                source=_SOURCE,
                return_value="The number of lines read.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 5)),
        )


@register
class CsvReportCommand(CommandDef):
    name = "csv::report"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Format a matrix as a human-readable report.",
                synopsis=("csv::report cmd matrix ?chan?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::report cmd matrix ?chan?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
        )
