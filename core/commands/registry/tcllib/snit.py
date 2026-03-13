"""snit -- Snit's Not Incr Tcl, OO framework (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib snit package"
_PACKAGE = "snit"


@register
class SnitTypeCommand(CommandDef):
    name = "snit::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a new snit type (class).",
                synopsis=("snit::type name definition",),
                snippet=(
                    "Defines a new object type. The definition body "
                    "contains option, variable, constructor, destructor, "
                    "method, and typemethod declarations."
                ),
                source=_SOURCE,
                examples=(
                    "snit::type Dog {\n"
                    "    option -name Fido\n"
                    '    method bark {} { return "Woof!" }\n'
                    "}"
                ),
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="snit::type name definition"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            creates_dynamic_barrier=True,
            never_inline_body=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class SnitWidgetCommand(CommandDef):
    name = "snit::widget"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a new snit megawidget type.",
                synopsis=("snit::widget name definition",),
                snippet=(
                    "Like snit::type but creates a Tk megawidget. "
                    "Automatically delegates to a hull widget."
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="snit::widget name definition"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            creates_dynamic_barrier=True,
            never_inline_body=True,
        )


@register
class SnitWidgetadaptorCommand(CommandDef):
    name = "snit::widgetadaptor"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a snit widget adaptor that wraps an existing widget.",
                synopsis=("snit::widgetadaptor name definition",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snit::widgetadaptor name definition",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            creates_dynamic_barrier=True,
            never_inline_body=True,
        )


@register
class SnitTypemethodCommand(CommandDef):
    name = "snit::typemethod"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a type method outside a type definition body.",
                synopsis=("snit::typemethod type name arglist body",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snit::typemethod type name arglist body",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
        )


@register
class SnitMethodCommand(CommandDef):
    name = "snit::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define an instance method outside a type definition body.",
                synopsis=("snit::method type name arglist body",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snit::method type name arglist body",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
        )
