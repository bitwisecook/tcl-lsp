# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""ttk::notebook -- Themed tabbed notebook widget."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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
