"""spawn -- Start a new process for interaction."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect spawn(1)"


@register
class SpawnCommand(CommandDef):
    name = "spawn"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="spawn",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Start a new process and prepare it for interaction.",
                synopsis=("spawn ?-option ...? program ?args ...?",),
                snippet=(
                    "Starts a new process and connects its stdin/stdout to "
                    "the Expect channel. Returns the spawn id in the variable "
                    "`spawn_id`."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="spawn ?-option ...? program ?args ...?",
                    options=(
                        OptionSpec(name="-noecho", detail="Suppress echoing of the command."),
                        OptionSpec(name="-console", detail="Redirect console output to spawn."),
                        OptionSpec(
                            name="-ignore",
                            takes_value=True,
                            value_hint="signal",
                            detail="Ignore the named signal in the spawned process.",
                        ),
                        OptionSpec(name="-leaveopen", detail="Leave the file descriptor open."),
                        OptionSpec(name="-pty", detail="Open a pty for the process."),
                        OptionSpec(name="-nottycopy", detail="Do not copy tty modes."),
                        OptionSpec(name="-nottyinit", detail="Do not initialise the tty."),
                        OptionSpec(
                            name="-open",
                            takes_value=True,
                            value_hint="fileId",
                            detail="Use an already-open file id.",
                        ),
                        OptionSpec(name="-trap", detail="Enable signal trapping."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
