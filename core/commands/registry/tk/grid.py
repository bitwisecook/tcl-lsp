"""grid -- Geometry manager that arranges widgets in a grid."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
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

_SOURCE = "Tk man page grid.n"
_av = make_av(_SOURCE)


@register
class GridCommand(CommandDef):
    name = "grid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="grid",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Geometry manager that arranges widgets in a grid.",
                synopsis=(
                    "grid slave ?slave ...? ?option value ...?",
                    "grid configure slave ?slave ...? ?option value ...?",
                    "grid columnconfigure master index ?-option value ...?",
                    "grid rowconfigure master index ?-option value ...?",
                    "grid bbox master ?column row? ?column2 row2?",
                    "grid forget slave ?slave ...?",
                    "grid info slave",
                    "grid location master x y",
                    "grid propagate master ?boolean?",
                    "grid size master",
                    "grid slaves master ?-option value?",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="grid option arg ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-row",
                            takes_value=True,
                            value_hint="n",
                            detail="Insert the slave so that it occupies the nth row in the grid.",
                        ),
                        OptionSpec(
                            name="-column",
                            takes_value=True,
                            value_hint="n",
                            detail="Insert the slave so that it occupies the nth column in the grid.",
                        ),
                        OptionSpec(
                            name="-rowspan",
                            takes_value=True,
                            value_hint="n",
                            detail="Insert the slave so that it occupies n rows in the grid.",
                        ),
                        OptionSpec(
                            name="-columnspan",
                            takes_value=True,
                            value_hint="n",
                            detail="Insert the slave so that it occupies n columns in the grid.",
                        ),
                        OptionSpec(
                            name="-sticky",
                            takes_value=True,
                            value_hint="nsew",
                            detail="Specifies which edges of the cell the slave sticks to (combination of n, s, e, w).",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies external horizontal padding for the slave.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies external vertical padding for the slave.",
                        ),
                        OptionSpec(
                            name="-ipadx",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies internal horizontal padding for the slave.",
                        ),
                        OptionSpec(
                            name="-ipady",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies internal vertical padding for the slave.",
                        ),
                        OptionSpec(
                            name="-in",
                            takes_value=True,
                            value_hint="master",
                            detail="Insert the slave into the specified master window.",
                        ),
                        OptionSpec(
                            name="-weight",
                            takes_value=True,
                            value_hint="int",
                            detail="Relative weight for apportioning extra space (columnconfigure/rowconfigure).",
                        ),
                        OptionSpec(
                            name="-minsize",
                            takes_value=True,
                            value_hint="amount",
                            detail="Minimum size of the column or row (columnconfigure/rowconfigure).",
                        ),
                        OptionSpec(
                            name="-pad",
                            takes_value=True,
                            value_hint="amount",
                            detail="Extra padding for the largest slave (columnconfigure/rowconfigure).",
                        ),
                        OptionSpec(
                            name="-uniform",
                            takes_value=True,
                            value_hint="group",
                            detail="Group columns/rows for uniform sizing (columnconfigure/rowconfigure).",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "anchor",
                                "Set the anchor point for the grid within the master window.",
                                "grid anchor master ?anchor?",
                            ),
                            _av(
                                "bbox",
                                "Return the bounding box of a cell or group of cells.",
                                "grid bbox master ?column row? ?column2 row2?",
                            ),
                            _av(
                                "columnconfigure",
                                "Query or set column properties of the grid.",
                                "grid columnconfigure master index ?-option value ...?",
                            ),
                            _av(
                                "configure",
                                "Set or query the grid options for one or more slaves.",
                                "grid configure slave ?slave ...? ?option value ...?",
                            ),
                            _av(
                                "forget",
                                "Remove each slave from the grid for its master.",
                                "grid forget slave ?slave ...?",
                            ),
                            _av(
                                "info",
                                "Return a list of the current grid configuration for the slave.",
                                "grid info slave",
                            ),
                            _av(
                                "location",
                                "Return the column and row containing the screen point x, y.",
                                "grid location master x y",
                            ),
                            _av(
                                "propagate",
                                "Control whether the master computes its geometry from slaves.",
                                "grid propagate master ?boolean?",
                            ),
                            _av(
                                "remove",
                                "Remove each slave from the grid, but remember its configuration.",
                                "grid remove slave ?slave ...?",
                            ),
                            _av(
                                "rowconfigure",
                                "Query or set row properties of the grid.",
                                "grid rowconfigure master index ?-option value ...?",
                            ),
                            _av(
                                "size",
                                "Return the size of the grid as a list of two elements (columns, rows).",
                                "grid size master",
                            ),
                            _av(
                                "slaves",
                                "Return a list of all slaves in the grid for the master.",
                                "grid slaves master ?-option value?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "anchor": SubCommand(
                    name="anchor",
                    arity=Arity(1, 2),
                    detail="Set the anchor point for the grid within the master window.",
                    synopsis="grid anchor master ?anchor?",
                ),
                "bbox": SubCommand(
                    name="bbox",
                    arity=Arity(1, 5),
                    detail="Return the bounding box of a cell or group of cells.",
                    synopsis="grid bbox master ?column row? ?column2 row2?",
                ),
                "columnconfigure": SubCommand(
                    name="columnconfigure",
                    arity=Arity(2),
                    detail="Query or set column properties of the grid.",
                    synopsis="grid columnconfigure master index ?-option value ...?",
                ),
                "configure": SubCommand(
                    name="configure",
                    arity=Arity(1),
                    detail="Set or query the grid options for one or more slaves.",
                    synopsis="grid configure slave ?slave ...? ?option value ...?",
                ),
                "forget": SubCommand(
                    name="forget",
                    arity=Arity(1),
                    detail="Remove each slave from the grid for its master.",
                    synopsis="grid forget slave ?slave ...?",
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(1, 1),
                    detail="Return a list of the current grid configuration for the slave.",
                    synopsis="grid info slave",
                ),
                "location": SubCommand(
                    name="location",
                    arity=Arity(3, 3),
                    detail="Return the column and row containing the screen point x, y.",
                    synopsis="grid location master x y",
                ),
                "propagate": SubCommand(
                    name="propagate",
                    arity=Arity(1, 2),
                    detail="Control whether the master computes its geometry from slaves.",
                    synopsis="grid propagate master ?boolean?",
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1),
                    detail="Remove each slave from the grid, but remember its configuration.",
                    synopsis="grid remove slave ?slave ...?",
                ),
                "rowconfigure": SubCommand(
                    name="rowconfigure",
                    arity=Arity(2),
                    detail="Query or set row properties of the grid.",
                    synopsis="grid rowconfigure master index ?-option value ...?",
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Return the size of the grid as a list of two elements (columns, rows).",
                    synopsis="grid size master",
                ),
                "slaves": SubCommand(
                    name="slaves",
                    arity=Arity(1),
                    detail="Return a list of all slaves in the grid for the master.",
                    synopsis="grid slaves master ?-option value?",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
