"""safe -- Tcl stdlib Safe Base package (auto-loaded, no package require needed)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl stdlib Safe Base"

# Safe interpreters are part of the Tcl core library but loaded via
# ``package require safe`` or auto-loaded when any safe:: command is invoked.
_PKG = "safe"


@register
class SafeInterpCreate(CommandDef):
    name = "safe::interpCreate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpCreate",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Create a safe child interpreter with restricted capabilities.",
                synopsis=("safe::interpCreate ?child? ?options...?",),
                snippet=(
                    "Creates a safe interpreter.  Options include "
                    "``-accessPath``, ``-statics``, ``-noStatics``, "
                    "``-nested``, ``-noNested``, ``-deleteHook``."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class SafeInterpInit(CommandDef):
    name = "safe::interpInit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpInit",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Initialise an existing interpreter as a safe interpreter.",
                synopsis=("safe::interpInit child ?options...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class SafeInterpConfigure(CommandDef):
    name = "safe::interpConfigure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpConfigure",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the configuration of a safe interpreter.",
                synopsis=("safe::interpConfigure child ?options...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class SafeInterpDelete(CommandDef):
    name = "safe::interpDelete"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpDelete",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Delete a safe interpreter and release its resources.",
                synopsis=("safe::interpDelete child",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class SafeInterpAddToAccessPath(CommandDef):
    name = "safe::interpAddToAccessPath"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpAddToAccessPath",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Add a directory to a safe interpreter's access path.",
                synopsis=("safe::interpAddToAccessPath child directory",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class SafeInterpFindInAccessPath(CommandDef):
    name = "safe::interpFindInAccessPath"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpFindInAccessPath",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the token for a directory in a safe interpreter's access path.",
                synopsis=("safe::interpFindInAccessPath child directory",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class SafeSetLogCmd(CommandDef):
    name = "safe::setLogCmd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::setLogCmd",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set or query the logging command for Safe Base messages.",
                synopsis=("safe::setLogCmd ?cmd arg...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class SafeSetSyncMode(CommandDef):
    name = "safe::setSyncMode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::setSyncMode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set or query the synchronous-source mode for a safe interpreter.",
                synopsis=("safe::setSyncMode ?child? ?boolean?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 2)),
        )
