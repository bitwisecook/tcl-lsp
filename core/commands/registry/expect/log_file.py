"""log_file -- Control logging to a file."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect log_file(1)"


@register
class LogFileCommand(CommandDef):
    name = "log_file"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="log_file",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control logging of session output to a file.",
                synopsis=(
                    "log_file ?-option ...? ?file?",
                    "log_file -info",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="log_file ?-option ...? ?file?",
                    options=(
                        OptionSpec(name="-a", detail="Append to existing log file."),
                        OptionSpec(name="-noappend", detail="Overwrite existing log file."),
                        OptionSpec(
                            name="-open",
                            takes_value=True,
                            value_hint="fileId",
                            detail="Log to an already-open Tcl file id.",
                        ),
                        OptionSpec(name="-leaveopen", detail="Leave the file open on close."),
                        OptionSpec(name="-info", detail="Return current log file settings."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
