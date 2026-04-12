"""Tcl stdlib auto-loaded utility commands (no package require needed)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl stdlib auto-loaded utility"

# NOTE: parray is already registered in core/commands/registry/tcl/parray.py
# so we do not re-register it here.


@register
class TclWordBreakAfter(CommandDef):
    name = "tcl_wordBreakAfter"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl_wordBreakAfter",
            hover=HoverSnippet(
                summary="Return the index of the first word boundary after *start* in *str*.",
                synopsis=("tcl_wordBreakAfter str start",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class TclWordBreakBefore(CommandDef):
    name = "tcl_wordBreakBefore"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl_wordBreakBefore",
            hover=HoverSnippet(
                summary="Return the index of the first word boundary before *start* in *str*.",
                synopsis=("tcl_wordBreakBefore str start",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
        )


@register
class TclEndOfWord(CommandDef):
    name = "tcl_endOfWord"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl_endOfWord",
            hover=HoverSnippet(
                summary="Return the index of the first end-of-word after *start* in *str*.",
                synopsis=("tcl_endOfWord str start",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
        )


@register
class TclStartOfNextWord(CommandDef):
    name = "tcl_startOfNextWord"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl_startOfNextWord",
            hover=HoverSnippet(
                summary="Return the index of the first start-of-word after *start* in *str*.",
                synopsis=("tcl_startOfNextWord str start",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
        )


@register
class TclStartOfPreviousWord(CommandDef):
    name = "tcl_startOfPreviousWord"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl_startOfPreviousWord",
            hover=HoverSnippet(
                summary="Return the index of the first start-of-word before *start* in *str*.",
                synopsis=("tcl_startOfPreviousWord str start",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
        )
