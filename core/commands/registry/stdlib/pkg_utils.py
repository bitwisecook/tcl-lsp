"""Tcl package management utilities (auto-loaded, no package require needed)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl stdlib package utilities"


@register
class PkgMkIndex(CommandDef):
    name = "pkg_mkIndex"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pkg_mkIndex",
            hover=HoverSnippet(
                summary="Build a ``pkgIndex.tcl`` file for one or more packages.",
                synopsis=(
                    "pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern ...?",
                ),
                snippet=(
                    "Scans *dir* for Tcl source and binary files matching "
                    "*pattern* (default ``*.tcl *.{so,dll}``) and builds a "
                    "``pkgIndex.tcl`` that enables ``package require`` to "
                    "find them."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class PkgCreate(CommandDef):
    name = "pkg::create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pkg::create",
            hover=HoverSnippet(
                summary="Generate a ``package ifneeded`` script for a package.",
                synopsis=(
                    "pkg::create -name packageName -version packageVersion ?-load filespec? ?-source filespec?",
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(4)),
        )
