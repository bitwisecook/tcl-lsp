"""Allow-list of ``info`` subcommands for the var-escape analysis.

Audited against Tcl 9.0's ``info`` dispatch table (``tclCmdIL.c``).
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
        "level",  # Returns caller frame args or a depth int.
        "frame",  # Returns a frame descriptor dict (file, line, cmd).
        "vars",  # Enumerates vars visible in the current frame.
        "locals",  # Enumerates locals of the current frame.
        "coroutine",  # Exposes the current coroutine frame.
        "errorstack",  # Exposes the error callstack — caller-frame data.
    }
)

# Subcommands that are purely interpreter-global — safe.
INTERPRETER_GLOBAL_SUBCOMMANDS: frozenset[str] = frozenset(
    {
        # Proc / method introspection (reads the global proc table, not
        # the current frame).
        "body",
        "args",
        "default",
        "commands",
        "procs",
        "class",
        "functions",
        "cmdtype",
        # Version / build / environment.
        "patchlevel",
        "tclversion",
        "nameofexecutable",
        "sharedlibextension",
        "library",
        "hostname",
        "script",
        # Runtime bookkeeping — no frame access.
        "cmdcount",
        "complete",
        "cancel",
        "loaded",
        # TclOO object introspection — reads object metadata.
        "object",
        # Tcl 9 additions: constants and globals expose data by name
        # but not via the current proc frame.
        "constant",
        "constants",
        "globals",
        # Handled separately; see classify_info_subcommand.
        "exists",
    }
)


def is_safe_info_subcommand(subcmd: str) -> bool:
    """Return True if this ``info <subcmd>`` never needs frame access."""
    return subcmd in INTERPRETER_GLOBAL_SUBCOMMANDS


def is_frame_inspecting_info_subcommand(subcmd: str) -> bool:
    """Return True if ``info <subcmd>`` enumerates or reads frame state."""
    return subcmd in FRAME_INSPECTING_SUBCOMMANDS
