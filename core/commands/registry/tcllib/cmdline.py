"""cmdline -- Command-line argument parsing (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib cmdline package"
_PACKAGE = "cmdline"


@register
class CmdlineGetoptCommand(CommandDef):
    name = "cmdline::getopt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse a single command-line option.",
                synopsis=("cmdline::getopt argvVar optstring optVar valVar",),
                snippet=(
                    "Processes a single option from the argument list. "
                    "Returns 1 if an option was found, 0 if no more "
                    "options, or -1 on error."
                ),
                source=_SOURCE,
                return_value="1 on success, 0 when done, -1 on error.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::getopt argvVar optstring optVar valVar",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class CmdlineGetoptionsCommand(CommandDef):
    name = "cmdline::getoptions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse all command-line options according to a specification.",
                synopsis=("cmdline::getoptions argvVar optlist ?usage?",),
                snippet=(
                    "Parses the argument list against the option "
                    "specification and returns a dictionary of option values."
                ),
                source=_SOURCE,
                examples=(
                    "set options [cmdline::getoptions argv {\n"
                    '    {verbose "Turn on verbose output"}\n'
                    '    {output.arg "" "Output file"}\n'
                    "}]"
                ),
                return_value="A dictionary of parsed option values.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::getoptions argvVar optlist ?usage?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
        )


@register
class CmdlineUsageCommand(CommandDef):
    name = "cmdline::usage"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Generate a usage string from an option specification.",
                synopsis=("cmdline::usage optlist ?usage?",),
                source=_SOURCE,
                return_value="A formatted usage string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::usage optlist ?usage?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            pure=True,
        )
