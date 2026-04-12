"""ttk::style -- Manipulate ttk styles and themes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_style.n"

_av = make_av(_SOURCE)

_SUBCOMMANDS = (
    _av(
        "configure",
        "Set or query style options.",
        "ttk::style configure style ?-option ?value ...??",
    ),
    _av(
        "map",
        "Set dynamic (state-dependent) style options.",
        "ttk::style map style ?-option {statespec value ...} ...?",
    ),
    _av(
        "lookup",
        "Look up a style option value.",
        "ttk::style lookup style -option ?state? ?default?",
    ),
    _av("layout", "Define or query the layout of a style.", "ttk::style layout style ?layoutSpec?"),
    _av("element", "Manage style elements.", "ttk::style element subcommand ?args?"),
    _av("theme", "Manage and query themes.", "ttk::style theme subcommand ?args?"),
)

_ELEMENT_SUBCOMMANDS = (
    _av("create", "Create a new element.", "ttk::style element create name type ?args?"),
    _av("names", "Return a list of all registered element names.", "ttk::style element names"),
    _av(
        "options",
        "Return the list of options for an element.",
        "ttk::style element options element",
    ),
)

_THEME_SUBCOMMANDS = (
    _av("create", "Create a new theme.", "ttk::style theme create name ?options?"),
    _av("names", "Return a list of available theme names.", "ttk::style theme names"),
    _av(
        "settings",
        "Evaluate a script in the context of a theme.",
        "ttk::style theme settings name script",
    ),
    _av("use", "Set the current theme.", "ttk::style theme use ?themeName?"),
)


@register
class TtkStyleCommand(CommandDef):
    name = "ttk::style"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::style",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Manipulate ttk styles and themes.",
                synopsis=("ttk::style subcommand ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::style subcommand ?arg ...?",
                    arg_values={
                        0: _SUBCOMMANDS,
                    },
                ),
            ),
            subcommands={
                "configure": SubCommand(
                    name="configure",
                    arity=Arity(1),
                    detail="Set or query style options.",
                    synopsis="ttk::style configure style ?-option ?value ...??",
                ),
                "element": SubCommand(
                    name="element",
                    arity=Arity(1),
                    detail="Manage style elements.",
                    synopsis="ttk::style element subcommand ?args?",
                    arg_values={0: _ELEMENT_SUBCOMMANDS},
                ),
                "layout": SubCommand(
                    name="layout",
                    arity=Arity(1, 2),
                    detail="Define or query the layout of a style.",
                    synopsis="ttk::style layout style ?layoutSpec?",
                ),
                "lookup": SubCommand(
                    name="lookup",
                    arity=Arity(2, 4),
                    detail="Look up a style option value.",
                    synopsis="ttk::style lookup style -option ?state? ?default?",
                ),
                "map": SubCommand(
                    name="map",
                    arity=Arity(1),
                    detail="Set dynamic (state-dependent) style options.",
                    synopsis="ttk::style map style ?-option {statespec value ...} ...?",
                ),
                "theme": SubCommand(
                    name="theme",
                    arity=Arity(1),
                    detail="Manage and query themes.",
                    synopsis="ttk::style theme subcommand ?args?",
                    arg_values={0: _THEME_SUBCOMMANDS},
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
