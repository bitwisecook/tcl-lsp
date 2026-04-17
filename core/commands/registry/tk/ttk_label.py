"""ttk::label -- Themed label widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_label.n"


@register
class TtkLabelCommand(CommandDef):
    name = "ttk::label"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::label",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed label widget.",
                synopsis=("ttk::label pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::label pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            value_hint="string",
                            detail="Text to display in the label.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable whose value is used as the label text.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            value_hint="imageName",
                            detail="Image to display in the label.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            value_hint="compoundType",
                            detail="How to display image relative to text.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the label.",
                        ),
                        OptionSpec(
                            name="-anchor",
                            takes_value=True,
                            value_hint="anchorPos",
                            detail="How the text or image is positioned within the widget.",
                        ),
                        OptionSpec(
                            name="-justify",
                            takes_value=True,
                            value_hint="justification",
                            detail="How to justify multiple lines of text.",
                        ),
                        OptionSpec(
                            name="-wraplength",
                            takes_value=True,
                            value_hint="length",
                            detail="Maximum line length for word wrapping.",
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
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the widget content.",
                        ),
                        OptionSpec(
                            name="-underline",
                            takes_value=True,
                            value_hint="index",
                            detail="Index of the character to underline for mnemonic activation.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            value_hint="relief",
                            detail="Border relief style for the label.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            value_hint="font",
                            detail="Font to use for the label text.",
                        ),
                        OptionSpec(
                            name="-foreground",
                            takes_value=True,
                            value_hint="colour",
                            detail="Foreground colour for the label text.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            value_hint="colour",
                            detail="Background colour for the label.",
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
