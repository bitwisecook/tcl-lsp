"""Dialect-specific command specs for Expect (interactive process automation).

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..models import CommandSpec

# Import command modules to trigger @register decorators
from . import (
    close_,  # noqa: F401
    debug_,  # noqa: F401
    disconnect,  # noqa: F401
    exit_,  # noqa: F401
    exp_continue,  # noqa: F401
    exp_internal,  # noqa: F401
    exp_pid,  # noqa: F401
    exp_version,  # noqa: F401
    expect_,  # noqa: F401
    expect_after,  # noqa: F401
    expect_background,  # noqa: F401
    expect_before,  # noqa: F401
    expect_tty,  # noqa: F401
    expect_user,  # noqa: F401
    fork_,  # noqa: F401
    interact,  # noqa: F401
    log_file,  # noqa: F401
    log_user,  # noqa: F401
    match_max,  # noqa: F401
    overlay,  # noqa: F401
    parity_,  # noqa: F401
    remove_nulls,  # noqa: F401
    send_,  # noqa: F401
    send_error,  # noqa: F401
    send_log,  # noqa: F401
    send_tty,  # noqa: F401
    send_user,  # noqa: F401
    sleep_,  # noqa: F401
    spawn_,  # noqa: F401
    strace,  # noqa: F401
    stty,  # noqa: F401
    system,  # noqa: F401
    timestamp_,  # noqa: F401
    trap_,  # noqa: F401
    wait_,  # noqa: F401
)
from ._base import _REGISTRY


def expect_command_specs() -> tuple[CommandSpec, ...]:
    """Return Expect-specific command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)
