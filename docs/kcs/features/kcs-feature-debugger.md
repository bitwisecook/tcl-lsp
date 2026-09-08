# KCS: feature — Tcl Debugger

> **Audience:** User
> **Type:** Functionality

## Summary

Interactive CLI debugger for Tcl scripts with single-stepping, breakpoints,
variable inspection, and call stack visualisation.

## Applies to

tcl-lsp CLI

## How to use

Build the binary with `cargo build -p tcl-debugger`; it is not part of the
published toolchain.

```sh
# Debug a script interactively (reads commands from stdin)
tcl-debug script.tcl

# Speak the Debug Adapter Protocol over stdio for a DAP-capable front-end
tcl-debug --dap
```

The script is loaded and stopped at its first line; there is no separate start
command.

### Debugger commands

| Command | Short | Description |
|---------|-------|-------------|
| `step` | `s` | Step into (one statement) |
| `next` | `n` | Step over (skip proc calls) |
| `finish` | `f` | Step out (run until proc returns) |
| `continue` | `c` | Continue to next breakpoint or end |
| `break <line>` | `b` | Set a breakpoint |
| `vars` | `locals` | Show variables in the current frame |
| `print <var>` | `p` | Print a variable value |
| `stack` | `bt` | Show the call stack |
| `list` | `l` | Show the current stop location |
| `quit` | `q` | Exit debugger |

## Example

Debugging a three-line accumulator, breaking inside the loop:

```
$ tcl-debug sum.tcl
stopped at line 1 [Entry]: set total 0
b 3
breakpoints: [3]
c
stopped at line 3 [Breakpoint]: incr total $n
p total
total = 0
bt
  #0 global (line 3)
q
```

## Operational context

The debugger runs the project's native bytecode VM with a debug hook in the
execution loop that fires at source-line boundaries, so it costs nothing when
no debugger is attached. The same backend drives both the interactive shell
and the Debug Adapter Protocol server, so an editor front-end you configure
against `tcl-debug --dap` sees identical behaviour.

## Failure modes

- A script using a command the VM does not implement fails.
- No editor ships a debug-adapter configuration; point your editor's DAP
  client at `tcl-debug --dap` yourself.
