"""opt -- Tcl stdlib option-processing utilities (package require opt)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_PKG = "opt"
_SOURCE = "Tcl stdlib opt package"


@register
class TclOptProc(CommandDef):
    name = "tcl::OptProc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptProc",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Define a proc with automatic option parsing.",
                synopsis=("tcl::OptProc name optlist body",),
                snippet=(
                    "Defines a procedure *name* whose arguments are parsed "
                    "according to *optlist*, a list of option descriptions.  "
                    "Inside *body*, option values are available as local "
                    "variables."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(3, 3)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class TclOptParse(CommandDef):
    name = "tcl::OptParse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptParse",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Parse a list of arguments according to an option description.",
                synopsis=("tcl::OptParse optlist arglist",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class TclOptProcArgGiven(CommandDef):
    name = "tcl::OptProcArgGiven"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptProcArgGiven",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return 1 if the named option was explicitly given, 0 otherwise.",
                synopsis=("tcl::OptProcArgGiven name",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class TclOptKeyRegister(CommandDef):
    name = "tcl::OptKeyRegister"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptKeyRegister",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Register an option description list under a key for later use.",
                synopsis=("tcl::OptKeyRegister optlist ?key?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TclOptKeyDelete(CommandDef):
    name = "tcl::OptKeyDelete"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptKeyDelete",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Delete a previously registered option description.",
                synopsis=("tcl::OptKeyDelete key",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class TclOptKeyError(CommandDef):
    name = "tcl::OptKeyError"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptKeyError",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Generate an error message for a registered option description.",
                synopsis=("tcl::OptKeyError key ?prefix?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TclOptKeyParse(CommandDef):
    name = "tcl::OptKeyParse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::OptKeyParse",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Parse arguments using a previously registered option description.",
                synopsis=("tcl::OptKeyParse key arglist",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )
