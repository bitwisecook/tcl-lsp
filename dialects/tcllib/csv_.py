"""csv -- CSV parsing and generation (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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


@register
class CsvJoinlistCommand(CommandDef):
    name = "csv::joinlist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Join a list of lists into CSV-formatted lines.",
                synopsis=("csv::joinlist values ?sepChar? ?quoteChar? ?quoteStyle?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::joinlist values ?sepChar? ?quoteChar? ?quoteStyle?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 4)),
            pure=True,
        )


@register
class CsvJoinmatrixCommand(CommandDef):
    name = "csv::joinmatrix"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Join a matrix object into CSV-formatted lines.",
                synopsis=("csv::joinmatrix matrix ?sepChar? ?quoteChar? ?quoteStyle?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::joinmatrix matrix ?sepChar? ?quoteChar? ?quoteStyle?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 4)),
        )


@register
class CsvIscompleteCommand(CommandDef):
    name = "csv::iscomplete"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Test whether a CSV record is complete or has unbalanced quotes.",
                synopsis=("csv::iscomplete data",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::iscomplete data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class CsvRead2queueCommand(CommandDef):
    name = "csv::read2queue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Read CSV data from a channel into a queue object.",
                synopsis=("csv::read2queue ?-alternate? chan q ?sepChar? ?quoteChar?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::read2queue ?-alternate? chan q ?sepChar? ?quoteChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 4)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class CsvSplit2matrixCommand(CommandDef):
    name = "csv::split2matrix"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Split CSV data and store it into a matrix object.",
                synopsis=("csv::split2matrix ?-alternate? m line ?sepChar? ?quoteChar?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::split2matrix ?-alternate? m line ?sepChar? ?quoteChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 5)),
        )


@register
class CsvSplit2queueCommand(CommandDef):
    name = "csv::split2queue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Split CSV data and store it into a queue object.",
                synopsis=("csv::split2queue ?-alternate? q line ?sepChar?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::split2queue ?-alternate? q line ?sepChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 4)),
        )


@register
class CsvWritematrixCommand(CommandDef):
    name = "csv::writematrix"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Write a matrix object to a channel in CSV format.",
                synopsis=("csv::writematrix m chan ?sepChar? ?quoteChar?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::writematrix m chan ?sepChar? ?quoteChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 4)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class CsvWritequeueCommand(CommandDef):
    name = "csv::writequeue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Write a queue object to a channel in CSV format.",
                synopsis=("csv::writequeue q chan ?sepChar? ?quoteChar?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="csv::writequeue q chan ?sepChar? ?quoteChar?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 4)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
