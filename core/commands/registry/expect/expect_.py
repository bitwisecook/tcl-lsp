"""expect -- Wait for output matching a pattern."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect expect(1)"


def _expect_arg_roles(args: list[str]) -> dict[int, ArgRole]:
    """Resolve BODY arg roles for expect pattern/body pairs.

    The expect command uses: expect ?opts? pat1 body1 ?pat2 body2 ...?
    Options: -re, -ex, -gl, -nocase, -timeout N, -i spawn_id, -indices,
    -notransfer, -nobrace.  Patterns and bodies alternate after options.
    The special patterns ``timeout``, ``eof``, ``default``, ``full_buffer``
    and ``null`` are followed by a body.
    """
    roles: dict[int, ArgRole] = {}
    i = 0
    while i < len(args):
        arg = args[i]
        if arg in ("-re", "-ex", "-gl", "-nocase", "-indices", "-notransfer", "-nobrace"):
            i += 1
            continue
        if arg in ("-timeout", "-i"):
            i += 2  # option + value
            continue
        if arg == "--":
            i += 1
            continue
        # At this point, arg is a pattern; the next is the body.
        if i + 1 < len(args):
            roles[i + 1] = ArgRole.BODY
            i += 2
        else:
            i += 1
    return roles


@register
class ExpectCommand(CommandDef):
    name = "expect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Wait for output matching a pattern from a spawned process.",
                synopsis=(
                    "expect ?-opts? pattern body ?pattern body ...?",
                    "expect -re {regexp} { actions }",
                    "expect timeout { timeout_actions }",
                    "expect eof { eof_actions }",
                ),
                snippet=(
                    "Waits until one of the patterns matches the output of the "
                    "current spawned process, then executes the corresponding body. "
                    "Special patterns: ``timeout``, ``eof``, ``default``, "
                    "``full_buffer``, ``null``."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match pattern as a Tcl regular expression."),
                        OptionSpec(name="-ex", detail="Match pattern as an exact string."),
                        OptionSpec(name="-gl", detail="Match pattern as a glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                        OptionSpec(
                            name="-timeout",
                            takes_value=True,
                            value_hint="seconds",
                            detail="Override the timeout for this expect.",
                        ),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify the spawn id to expect from.",
                        ),
                        OptionSpec(name="-indices", detail="Store match indices in expect_out."),
                        OptionSpec(name="-notransfer", detail="Do not consume matched output."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
