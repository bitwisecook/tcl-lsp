"""fileutil -- File utility procedures (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
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
                    writes=True,
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
                synopsis=("fileutil::tempdir",),
                source=_SOURCE,
                return_value="The system temporary directory path.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="fileutil::tempdir"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )
