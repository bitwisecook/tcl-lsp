# KCS: feature — Tcl Debugger

> **Audience:** User
> **Type:** Functionality

## Summary

Interactive CLI debugger for Tcl scripts with single-stepping, breakpoints,
variable inspection, and call stack visualisation.

## Applies to

tcl-lsp CLI, all-editors

## Availability

| Context | How |
|---------|-----|
| CLI     | `tcl-debug script.tcl` (interactive) |
| Editors | `tcl-debug --dap` — a Debug Adapter Protocol server over stdio |

## How to use

```sh
# Debug a script interactively (reads commands from stdin)
tcl-debug script.tcl

# Speak the Debug Adapter Protocol over stdio for an editor front-end
tcl-debug --dap
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

### Backend

The debugger runs the project's native bytecode VM (`tcl-vm`) with a debug
hook in the execution loop, giving full variable and stack introspection.

## Operational context

The debugger consists of:

- A debug hook in the VM execution loop that fires at source line boundaries
  (minimal overhead when no debugger is attached).
- A backend that manages breakpoints, step modes, and blocks the VM thread
  when stopped, exposed over a common `DebugBackend` interface.
- A DAP server (`tcl-debug --dap`) that maps the same backend to the Debug
  Adapter Protocol for editor front-ends.

## Failure modes

- Scripts using commands not yet implemented in the VM will fail.

## Test anchors

- `rust/tcl-debugger/` crate tests — backend, stepping, and DAP tests
