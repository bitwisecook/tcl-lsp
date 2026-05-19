"""fileutil -- File utility procedures (tcllib)."""

from __future__ import annotations

from typing import Any

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "tcllib fileutil package"
_PACKAGE = "fileutil"


@register
class FileutilCatCommand(CommandDef):
    name = "fileutil::cat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the contents of one or more files.",
                synopsis=("fileutil::cat ?-encoding enc? ?--? file ...",),
                snippet=(
                    "Reads the contents of the named files and returns "
                    "them as a single concatenated string."
                ),
                source=_SOURCE,
                examples="set contents [fileutil::cat myfile.txt]",
                return_value="The concatenated file contents.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fileutil::cat ?-encoding enc? ?--? file ...",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class FileutilWriteFileCommand(CommandDef):
    name = "fileutil::writeFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Write data to a file, replacing any existing content.",
                synopsis=("fileutil::writeFile ?-encoding enc? ?--? file data",),
                source=_SOURCE,
                examples='fileutil::writeFile output.txt "hello world"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fileutil::writeFile ?-encoding enc? ?--? file data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 4)),
        )


@register
class FileutilTempfileCommand(CommandDef):
    name = "fileutil::tempfile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Create a temporary file and return its path.",
                synopsis=("fileutil::tempfile ?prefix?",),
                source=_SOURCE,
                return_value="The path to the new temporary file.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="fileutil::tempfile ?prefix?"),),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


@register
class FileutilTempdirCommand(CommandDef):
    name = "fileutil::tempdir"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the path of the system temporary directory.",
                synopsis=("fileutil::tempdir ?path?",),
                source=_SOURCE,
                return_value="The system temporary directory path.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="fileutil::tempdir ?path?"),),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


def _fileutil_se(
    *,
    reads: bool = False,
    writes: bool = False,
) -> tuple[SideEffect, ...]:
    return (
        SideEffect(
            target=SideEffectTarget.FILE_IO,
            reads=reads,
            writes=writes,
            connection_side=ConnectionSide.NONE,
        ),
    )


def _update_in_place_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Last argument is the body script."""
    if not args:
        return {}
    return {len(args) - 1: frozenset({ArgRole.BODY})}


def _fu(name: str, summary: str, synopsis: str, arity: Arity, **kw: Any) -> type:
    """Helper to generate simple fileutil command defs."""
    full_name = f"fileutil::{name}"

    @register
    class _Cmd(CommandDef):
        name = full_name

        @classmethod
        def spec(cls) -> CommandSpec:
            return CommandSpec(
                name=full_name,
                tcllib_package=_PACKAGE,
                hover=HoverSnippet(summary=summary, synopsis=(synopsis,), source=_SOURCE),
                forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=synopsis),),
                validation=ValidationSpec(arity=arity),
                **kw,
            )

    return _Cmd


_fu(
    "foreachLine",
    "Iterate over each line of a file.",
    "fileutil::foreachLine var filename cmd",
    Arity(3, 3),
    arg_roles={0: frozenset({ArgRole.VAR_WRITE}), 2: frozenset({ArgRole.BODY})},
    side_effect_hints=_fileutil_se(reads=True),
)
_fu(
    "grep",
    "Search for a pattern in files.",
    "fileutil::grep pattern ?files?",
    Arity(1, 2),
    side_effect_hints=_fileutil_se(reads=True),
)
_fu(
    "find",
    "Recursively find files matching a filter.",
    "fileutil::find ?basedir? ?filtercmd?",
    Arity(0, 2),
    side_effect_hints=_fileutil_se(reads=True),
)
_fu(
    "findByPattern",
    "Find files matching glob or regexp patterns.",
    "fileutil::findByPattern basedir ?-glob|-regexp? ?--? patterns",
    Arity(1),
    side_effect_hints=_fileutil_se(reads=True),
)
_fu(
    "touch",
    "Update file access/modification times.",
    "fileutil::touch ?options? filename ...",
    Arity(0),
    side_effect_hints=_fileutil_se(writes=True),
)
_fu(
    "test",
    "Test file properties.",
    "fileutil::test path codes ?msgvar? ?label?",
    Arity(2, 4),
    arg_roles={2: frozenset({ArgRole.VAR_WRITE})},
)
_fu(
    "updateInPlace",
    "Update a file in place using a command.",
    "fileutil::updateInPlace ?options? fileName cmdOrBody",
    Arity(2),
    arg_role_resolver=_update_in_place_arg_roles,
    body_arg_implicit_args=1,
    side_effect_hints=_fileutil_se(reads=True, writes=True),
)
_fu(
    "install",
    "Copy a file and optionally set permissions.",
    "fileutil::install ?-m mode? source destination",
    Arity(2),
    side_effect_hints=_fileutil_se(writes=True),
)
_fu(
    "fileType",
    "Determine the type of a file.",
    "fileutil::fileType filename",
    Arity(1, 1),
    side_effect_hints=_fileutil_se(reads=True),
)
_fu(
    "appendToFile",
    "Append data to a file.",
    "fileutil::appendToFile ?options? file data",
    Arity(2),
    side_effect_hints=_fileutil_se(writes=True),
)
_fu(
    "insertIntoFile",
    "Insert data into a file at an offset.",
    "fileutil::insertIntoFile ?options? file at data",
    Arity(3),
    side_effect_hints=_fileutil_se(reads=True, writes=True),
)
_fu(
    "removeFromFile",
    "Remove data from a file.",
    "fileutil::removeFromFile ?options? file at n",
    Arity(3),
    side_effect_hints=_fileutil_se(reads=True, writes=True),
)
_fu(
    "replaceInFile",
    "Replace data in a file.",
    "fileutil::replaceInFile ?options? file at n data",
    Arity(4),
    side_effect_hints=_fileutil_se(reads=True, writes=True),
)
_fu(
    "stripPwd",
    "Remove the current directory prefix from a path.",
    "fileutil::stripPwd path",
    Arity(1, 1),
    pure=True,
)
_fu(
    "stripN", "Remove N leading path components.", "fileutil::stripN path n", Arity(2, 2), pure=True
)
_fu(
    "jail",
    "Confine a path under a base directory.",
    "fileutil::jail base path",
    Arity(2, 2),
    pure=True,
)
_fu(
    "lexnormalize",
    "Lexically normalise a path.",
    "fileutil::lexnormalize path",
    Arity(1, 1),
    pure=True,
)
_fu("relative", "Compute a relative path.", "fileutil::relative base dst", Arity(2, 2), pure=True)
_fu(
    "relativeUrl",
    "Compute a relative URL path.",
    "fileutil::relativeUrl base dst",
    Arity(2, 2),
    pure=True,
)
_fu(
    "fullnormalize",
    "Fully normalise a path (resolves symlinks).",
    "fileutil::fullnormalize path",
    Arity(1, 1),
    side_effect_hints=_fileutil_se(reads=True),
)
_fu(
    "maketempdir",
    "Create a temporary directory.",
    "fileutil::maketempdir ?options?",
    Arity(0),
    side_effect_hints=_fileutil_se(writes=True),
)
_fu(
    "tempdirReset",
    "Reset the cached temporary directory path.",
    "fileutil::tempdirReset",
    Arity(0, 0),
    side_effect_hints=(
        SideEffect(
            target=SideEffectTarget.VARIABLE, writes=True, connection_side=ConnectionSide.NONE
        ),
    ),
)
