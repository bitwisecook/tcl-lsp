"""ttk::notebook -- Themed tabbed notebook widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_notebook.n"

_av = make_av(_SOURCE)

_SUBCOMMANDS = (
    _av("add", "Add a pane to the notebook.", "pathName add window ?options?"),
    _av("forget", "Remove a pane from the notebook.", "pathName forget tabId"),
    _av("hide", "Hide a tab without removing it.", "pathName hide tabId"),
    _av("identify", "Identify the element at a given position.", "pathName identify component x y"),
    _av("index", "Return the numeric index of a tab.", "pathName index tabId"),
    _av("insert", "Insert a pane at a given position.", "pathName insert pos subwindow ?options?"),
    _av("select", "Select a tab or return the currently selected pane.", "pathName select ?tabId?"),
    _av("tab", "Query or modify tab options.", "pathName tab tabId ?-option? ?value ...?"),
    _av("tabs", "Return a list of all pane windows.", "pathName tabs"),
)


@register
class TtkNotebookCommand(CommandDef):
    name = "ttk::notebook"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::notebook",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed tabbed notebook widget.",
                synopsis=("ttk::notebook pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::notebook pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the notebook.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="height",
                            detail="Desired height of the notebook.",
                        ),
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the notebook content.",
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
