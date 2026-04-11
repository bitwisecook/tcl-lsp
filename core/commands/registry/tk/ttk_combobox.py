"""ttk::combobox -- Themed combobox widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_combobox.n"

_av = make_av(_SOURCE)

_STATE_VALUES = (
    _av("normal", "The default state; the combobox is editable."),
    _av("readonly", "The combobox value is not directly editable."),
    _av("disabled", "The combobox is greyed out and unresponsive."),
)


@register
class TtkComboboxCommand(CommandDef):
    name = "ttk::combobox"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::combobox",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed combobox widget.",
                synopsis=("ttk::combobox pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::combobox pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable linked to the current combobox value.",
                        ),
                        OptionSpec(
                            name="-values",
                            takes_value=True,
                            value_hint="valueList",
                            detail="List of values to display in the drop-down list.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the combobox in characters.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="height",
                            detail="Maximum number of rows in the drop-down listbox.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            value_hint="stateSpec",
                            detail="Widget state (normal, readonly, or disabled).",
                        ),
                        OptionSpec(
                            name="-postcommand",
                            takes_value=True,
                            value_hint="script",
                            detail="Script to evaluate just before displaying the drop-down list.",
                        ),
                        OptionSpec(
                            name="-validate",
                            takes_value=True,
                            value_hint="validateMode",
                            detail="When to run validation (none, focus, focusin, focusout, key, all).",
                        ),
                        OptionSpec(
                            name="-validatecommand",
                            takes_value=True,
                            value_hint="script",
                            detail="Script to evaluate for input validation.",
                        ),
                        OptionSpec(
                            name="-invalidcommand",
                            takes_value=True,
                            value_hint="script",
                            detail="Script to evaluate when validation fails.",
                        ),
                        OptionSpec(
                            name="-xscrollcommand",
                            takes_value=True,
                            value_hint="script",
                            detail="Command prefix for horizontal scroll communication.",
                        ),
                        OptionSpec(
                            name="-exportselection",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Whether the selection is exported to the X selection.",
                        ),
                        OptionSpec(
                            name="-justify",
                            takes_value=True,
                            value_hint="justification",
                            detail="How to justify the text within the combobox.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            value_hint="font",
                            detail="Font to use for the combobox text.",
                        ),
                        OptionSpec(
                            name="-foreground",
                            takes_value=True,
                            value_hint="colour",
                            detail="Foreground colour for the combobox text.",
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
