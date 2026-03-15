# KCS: feature — Tcl Debugger

## Summary

Interactive CLI debugger for Tcl scripts with single-stepping, breakpoints,
variable inspection, and call stack visualisation.

## Surface

cli

## Availability

| Context | How |
|---------|-----|
| CLI     | `python -m debugger script.tcl` |

## How to use

```sh
# Debug a script (auto-detects best backend)
uv run python -m debugger script.tcl

# Force a specific backend
uv run python -m debugger --backend vm script.tcl
uv run python -m debugger --backend tclsh script.tcl
uv run python -m debugger --backend tkinter script.tcl

# Read from stdin
echo 'puts hello' | uv run python -m debugger -
```

### Debugger commands

| Command | Short | Description |
|---------|-------|-------------|
| `run` | | Start execution |
| `step` | `s` | Step into (one statement) |
| `next` | `n` | Step over (skip proc calls) |
| `finish` | `fin` | Step out (run until proc returns) |
| `continue` | `c` | Continue to next breakpoint or end |
| `break <line>` | `b` | Set a breakpoint |
| `delete <id>` | `d` | Delete a breakpoint |
| `vars [frame]` | | Show variables in scope |
| `print <var>` | `p` | Print a variable value |
| `stack` | | Show the call stack |
| `list [line]` | `l` | Show source context |
| `quit` | `q` | Exit debugger |

### Backends

The debugger supports three backends, selected automatically in priority
order (tclsh > tkinter > VM) or via `--backend`:

- **tclsh** — Uses `trace add execution source enterstep` for stepping.
  Best compatibility with standard Tcl.
- **tkinter** — Uses Python's `tkinter.Tcl()` with `createcommand` bridge.
  No subprocess needed.
- **VM** — Uses the project's bytecode VM with a debug hook in the
  execution loop.  Full variable and stack introspection.

## Operational context

The debugger consists of:

- A debug hook injected into the `BytecodeVM.execute()` loop that fires at
  source line boundaries (zero overhead when not attached).
- A `DebugController` that manages breakpoints, step modes, and blocks the
  VM thread when stopped.
- Backend-specific implementations that adapt tclsh, tkinter, and the VM
  to a common `DebugBackend` interface.
- Shared Tcl runtime discovery in `core/tcl_discovery.py` (also used by
  the iRule test framework).

## File-path anchors

- `debugger/` — main debugger package
- `debugger/controller.py` — breakpoints, stepping, state inspection
- `debugger/cli.py` — readline-based CLI frontend
- `debugger/backends/vm_backend.py` — VM backend
- `debugger/backends/tclsh_backend.py` — tclsh subprocess backend
- `debugger/backends/tkinter_backend.py` — tkinter in-process backend
- `debugger/tcl/debug_helper.tcl` — Tcl-side instrumentation
- `core/tcl_discovery.py` — shared tclsh/tkinter detection
- `vm/machine.py` — debug hook insertion point

## Failure modes

- VM backend: scripts using commands not implemented in the VM will fail.
- tclsh backend: `trace add execution source enterstep` requires Tcl 8.5+.
- tkinter backend: requires Python built with Tcl/Tk support.

## Test anchors

- `tests/test_debugger_hook.py` — VM hook unit tests
- `tests/test_debugger_controller.py` — controller and stepping tests
- `tests/test_debugger_backends.py` — backend factory tests
