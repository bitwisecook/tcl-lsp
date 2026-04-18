"""Allow-list of ``info`` subcommands for the var-escape analysis.

Most ``info`` subcommands reflect on the interpreter's global state
(command table, version info, script path). A small set reads the
current frame by name — those force the whole proc to the
pessimistic fallback. ``info exists <literal>`` is treated specially:
safe if the target is a bare name (escapes only that name), pessimistic
if dynamic.
"""

from __future__ import annotations

# Subcommands that read frame-local state by name and thus force the
# containing proc to the pessimistic fallback.
FRAME_INSPECTING_SUBCOMMANDS: frozenset[str] = frozenset(
    {
        "level",
        "frame",
        "vars",
        "locals",
        "coroutine",  # Exposes the current coroutine frame
    }
)

# Subcommands that are purely interpreter-global — safe.
INTERPRETER_GLOBAL_SUBCOMMANDS: frozenset[str] = frozenset(
    {
        "body",
        "args",
        "default",
        "commands",
        "procs",
        "class",
        "functions",
        "patchlevel",
        "tclversion",
        "nameofexecutable",
        "script",
        "library",
        "hostname",
        "sharedlibextension",
        "cmdcount",
        "complete",
        "exists",  # handled separately; see classify_info_subcommand
    }
)


def is_safe_info_subcommand(subcmd: str) -> bool:
    """Return True if this ``info <subcmd>`` never needs frame access."""
    return subcmd in INTERPRETER_GLOBAL_SUBCOMMANDS


def is_frame_inspecting_info_subcommand(subcmd: str) -> bool:
    """Return True if ``info <subcmd>`` enumerates or reads frame state."""
    return subcmd in FRAME_INSPECTING_SUBCOMMANDS
