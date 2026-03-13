"""platform -- Tcl stdlib platform identification package (package require platform)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_PKG = "platform"
_SOURCE = "Tcl stdlib platform package"


@register
class PlatformIdentify(CommandDef):
    name = "platform::identify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::identify",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the platform identifier for the current machine.",
                synopsis=("platform::identify",),
                snippet=(
                    "Returns a string like ``linux-x86_64`` or ``macosx-arm`` "
                    "that specifically identifies the current platform, including "
                    "CPU details and libc version where relevant."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class PlatformGeneric(CommandDef):
    name = "platform::generic"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::generic",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the generic platform identifier (less specific than identify).",
                synopsis=("platform::generic",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
            pure=True,
        )


@register
class PlatformPatterns(CommandDef):
    name = "platform::patterns"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::patterns",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return a list of platform patterns that match the given identifier.",
                synopsis=("platform::patterns id",),
                snippet=(
                    "Given a platform *id* from ``platform::identify``, "
                    "returns a list of platform patterns in order of "
                    "decreasing specificity."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


# platform::shell sub-package


@register
class PlatformShellIdentify(CommandDef):
    name = "platform::shell::identify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::shell::identify",
            required_package="platform::shell",
            hover=HoverSnippet(
                summary="Return the platform identifier for a given Tcl shell.",
                synopsis=("platform::shell::identify shell",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class PlatformShellGeneric(CommandDef):
    name = "platform::shell::generic"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::shell::generic",
            required_package="platform::shell",
            hover=HoverSnippet(
                summary="Return the generic platform identifier for a given Tcl shell.",
                synopsis=("platform::shell::generic shell",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
