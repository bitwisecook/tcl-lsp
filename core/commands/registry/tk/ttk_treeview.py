"""ttk::treeview -- Themed hierarchical multicolumn data display widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_treeview.n"

_av = make_av(_SOURCE)

_SELECTMODE_VALUES = (
    _av("extended", "Multiple items may be selected."),
    _av("browse", "Only one item may be selected at a time."),
    _av("none", "Selection is disabled."),
)

_SHOW_VALUES = (
    _av("tree", "Display the tree column (column #0)."),
    _av("headings", "Display the heading row."),
    _av("tree headings", "Display both the tree column and the heading row."),
)

_SUBCOMMANDS = (
    _av("insert", "Create a new child item.", "pathName insert parent index ?-id id? ?options?"),
    _av("delete", "Delete items and all their descendants.", "pathName delete itemList"),
    _av("detach", "Unlink items from the tree without deleting them.", "pathName detach itemList"),
    _av("exists", "Return 1 if the specified item exists, 0 otherwise.", "pathName exists item"),
    _av("focus", "Get or set the focus item.", "pathName focus ?item?"),
    _av(
        "heading",
        "Query or modify the heading options for a column.",
        "pathName heading column ?-option? ?value ...?",
    ),
    _av("identify", "Identify the element at a given position.", "pathName identify component x y"),
    _av("index", "Return the integer index of an item.", "pathName index item"),
    _av(
        "item",
        "Query or modify the options for an item.",
        "pathName item item ?-option? ?value ...?",
    ),
    _av("move", "Move an item to a new position.", "pathName move item parent index"),
    _av("next", "Return the next sibling of an item.", "pathName next item"),
    _av("parent", "Return the parent of an item.", "pathName parent item"),
    _av("prev", "Return the previous sibling of an item.", "pathName prev item"),
    _av("see", "Ensure the specified item is visible.", "pathName see item"),
    _av("selection", "Query or modify the selection.", "pathName selection ?operation? ?itemList?"),
    _av("set", "Query or set column values for an item.", "pathName set item ?column? ?value?"),
    _av("tag", "Manage item tags.", "pathName tag subcommand ?arg ...?"),
    _av("xview", "Query or change the horizontal scroll position.", "pathName xview ?args?"),
    _av("yview", "Query or change the vertical scroll position.", "pathName yview ?args?"),
    _av(
        "children",
        "Return the list of children of an item.",
        "pathName children item ?newchildren?",
    ),
    _av(
        "column", "Query or modify column options.", "pathName column column ?-option? ?value ...?"
    ),
    _av("bbox", "Return the bounding box of an item.", "pathName bbox item ?column?"),
)


@register
class TtkTreeviewCommand(CommandDef):
    name = "ttk::treeview"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::treeview",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed hierarchical multicolumn data display widget.",
                synopsis=("ttk::treeview pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::treeview pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-columns",
                            takes_value=True,
                            value_hint="columnList",
                            detail="List of column identifiers.",
                        ),
                        OptionSpec(
                            name="-displaycolumns",
                            takes_value=True,
                            value_hint="columnList",
                            detail="List of columns to display, or #all.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="rows",
                            detail="Number of rows to display.",
                        ),
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the widget content.",
                        ),
                        OptionSpec(
                            name="-selectmode",
                            takes_value=True,
                            value_hint="mode",
                            detail="Selection mode (extended, browse, or none).",
                        ),
                        OptionSpec(
                            name="-show",
                            takes_value=True,
                            value_hint="components",
                            detail="Which parts of the treeview to display (tree, headings, or both).",
                        ),
                        OptionSpec(
                            name="-xscrollcommand",
                            takes_value=True,
                            value_hint="script",
                            detail="Command prefix for horizontal scroll communication.",
                        ),
                        OptionSpec(
                            name="-yscrollcommand",
                            takes_value=True,
                            value_hint="script",
                            detail="Command prefix for vertical scroll communication.",
                        ),
                        OptionSpec(
                            name="-style",
                            takes_value=True,
                            value_hint="style",
                            detail="Style to use for the widget.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            value_hint="className",
                            detail="Widget class name for option-database lookups.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            value_hint="cursor",
                            detail="Cursor to display when the pointer is over the widget.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            value_hint="focusSpec",
                            detail="Whether the widget accepts focus during keyboard traversal.",
                        ),
                    ),
                ),
            ),
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
