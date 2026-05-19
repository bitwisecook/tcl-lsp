"""cmdline -- Command-line argument parsing (tcllib)."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
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


@register
class CmdlineTypedGetoptCommand(CommandDef):
    name = "cmdline::typedGetopt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse a single typed command-line option.",
                synopsis=("cmdline::typedGetopt argvVar optstring optVar valVar",),
                source=_SOURCE,
                return_value="1 on success, 0 when done, -1 on error.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::typedGetopt argvVar optstring optVar valVar",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
            arg_roles={
                0: frozenset({ArgRole.VAR_WRITE}),
                2: frozenset({ArgRole.VAR_WRITE}),
                3: frozenset({ArgRole.VAR_WRITE}),
            },
        )


@register
class CmdlineTypedGetoptionsCommand(CommandDef):
    name = "cmdline::typedGetoptions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse all typed command-line options according to a specification.",
                synopsis=("cmdline::typedGetoptions argvVar optlist ?usage?",),
                source=_SOURCE,
                return_value="A dictionary of parsed option values.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::typedGetoptions argvVar optlist ?usage?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
        )


@register
class CmdlineTypedUsageCommand(CommandDef):
    name = "cmdline::typedUsage"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Generate a usage string from a typed option specification.",
                synopsis=("cmdline::typedUsage optlist ?usage?",),
                source=_SOURCE,
                return_value="A formatted usage string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::typedUsage optlist ?usage?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            pure=True,
        )


@register
class CmdlineGetKnownOptCommand(CommandDef):
    name = "cmdline::getKnownOpt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse a single known command-line option.",
                synopsis=("cmdline::getKnownOpt argvVar optstring optVar valVar",),
                source=_SOURCE,
                return_value="1 on success, 0 when done, -1 on error.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::getKnownOpt argvVar optstring optVar valVar",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
            arg_roles={
                0: frozenset({ArgRole.VAR_WRITE}),
                2: frozenset({ArgRole.VAR_WRITE}),
                3: frozenset({ArgRole.VAR_WRITE}),
            },
        )


@register
class CmdlineGetKnownOptionsCommand(CommandDef):
    name = "cmdline::getKnownOptions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse all known command-line options according to a specification.",
                synopsis=("cmdline::getKnownOptions argvVar optlist ?usage?",),
                source=_SOURCE,
                return_value="A dictionary of parsed option values.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::getKnownOptions argvVar optlist ?usage?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
        )


@register
class CmdlineGetfilesCommand(CommandDef):
    name = "cmdline::getfiles"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Expand file patterns into a list of matching files.",
                synopsis=("cmdline::getfiles patterns quiet",),
                source=_SOURCE,
                return_value="A list of matching file paths.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cmdline::getfiles patterns quiet",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class CmdlineGetArgv0Command(CommandDef):
    name = "cmdline::getArgv0"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the application name from the command line.",
                synopsis=("cmdline::getArgv0",),
                source=_SOURCE,
                return_value="The application name.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="cmdline::getArgv0"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
            pure=True,
        )
