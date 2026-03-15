"""interact -- Give control to the user for interactive use."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect interact(1)"


def _interact_arg_roles(args: list[str]) -> dict[int, ArgRole]:
    """Resolve BODY arg roles for interact string/body pairs."""
    roles: dict[int, ArgRole] = {}
    i = 0
    while i < len(args):
        arg = args[i]
        if arg in (
            "-re",
            "-ex",
            "-echo",
            "-nobuffer",
            "-f",
            "-F",
            "-reset",
            "-iwrite",
            "-iread",
        ):
            i += 1
            continue
        if arg in ("-input", "-output", "-u", "-o", "-i"):
            i += 2  # option + value
            continue
        if arg == "--":
            i += 1
            continue
        # pattern followed by body
        if i + 1 < len(args):
            roles[i + 1] = ArgRole.BODY
            i += 2
        else:
            i += 1
    return roles


@register
class InteractCommand(CommandDef):
    name = "interact"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="interact",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Give control of the current process to the user for interactive use.",
                synopsis=(
                    "interact ?-opts? ?string body ...?",
                    "interact",
                ),
                snippet=(
                    "Connects the user's terminal to the spawned process. "
                    "With string/body pairs, intercepts matching input and "
                    "executes the body instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="interact ?-opts? ?string body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match as regular expression."),
                        OptionSpec(name="-ex", detail="Match as exact string."),
                        OptionSpec(
                            name="-input",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify input source.",
                        ),
                        OptionSpec(
                            name="-output",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify output destination.",
                        ),
                        OptionSpec(
                            name="-u",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Connect user to the specified process.",
                        ),
                        OptionSpec(name="-o", detail="Apply to output."),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify spawn id.",
                        ),
                        OptionSpec(name="-echo", detail="Echo characters."),
                        OptionSpec(name="-nobuffer", detail="Do not buffer input."),
                        OptionSpec(name="-f", detail="Force — do not flush."),
                        OptionSpec(name="-F", detail="Force — flush."),
                        OptionSpec(name="-reset", detail="Reset terminal modes."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_interact_arg_roles,
        )
