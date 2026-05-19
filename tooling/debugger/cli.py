"""Readline-based CLI debugger frontend."""

from __future__ import annotations

import os
import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .backends.base import DebugBackend
    from .controller import DebugController

_HISTORY_FILE = os.path.expanduser("~/.tcldbg_history")
_MAX_HISTORY = 1000

# ANSI colour helpers
_BOLD = "\033[1m"
_DIM = "\033[2m"
_GREEN = "\033[32m"
_YELLOW = "\033[33m"
_CYAN = "\033[36m"
_RED = "\033[31m"
_RESET = "\033[0m"


def _colour(text: str, colour: str) -> str:
    if not sys.stdout.isatty():
        return text
    return f"{colour}{text}{_RESET}"


class CliDebugger:
    """Interactive command-line debugger.

    Supports the VM backend with full stepping/inspection, and the
    tclsh/tkinter backends with basic stepping.
    """

    def __init__(self, backend: DebugBackend) -> None:
        self._backend = backend
        self._breakpoints: dict[int, int] = {}  # bp_id -> line
        self._next_bp_id = 1
        self._source_lines: list[str] = []
        self._script_path: str = ""
        self._running = False
        self._finished = False

        # Wire up callbacks
        backend.on_output(self._handle_output)
        backend.on_finished(self._handle_finished)

    def _handle_output(self, text: str) -> None:
        sys.stdout.write(text)
        sys.stdout.flush()

    def _handle_finished(self) -> None:
        self._finished = True
        print(_colour("Script finished.", _DIM))

    def _get_controller(self) -> DebugController | None:
        """Get the DebugController if using the VM backend."""
        from .backends.vm_backend import VmBackend

        if isinstance(self._backend, VmBackend):
            return self._backend.controller
        return None

    def _wait_for_stop(self, timeout: float = 30.0) -> bool:
        """Wait for the backend to stop."""
        ctrl = self._get_controller()
        if ctrl is not None:
            return ctrl.wait_for_stop(timeout=timeout)

        # For tclsh/tkinter backends, check for wait_for_stop method
        wait_fn = getattr(self._backend, "wait_for_stop", None)
        if wait_fn is not None:
            return wait_fn(timeout=timeout)
        return False

    def run(self, script_path: str, source: str | None = None) -> None:
        """Start the debugger session."""
        self._script_path = script_path

        # Load source for display
        if source is not None:
            self._source_lines = source.splitlines()
        else:
            try:
                with open(script_path, encoding="utf-8") as f:
                    self._source_lines = f.read().splitlines()
            except OSError:
                self._source_lines = []

        self._backend.launch(script_path, source=source)

        # Set up readline
        self._setup_readline()

        print(_colour(f"Tcl Debugger — {script_path}", _BOLD))

        backend_name = type(self._backend).__name__
        print(_colour(f"Backend: {backend_name}", _DIM))
        print(f"Type {_colour('help', _CYAN)} for available commands.\n")

        # Main command loop
        self._command_loop()

    def _setup_readline(self) -> None:
        try:
            import readline

            try:
                readline.read_history_file(_HISTORY_FILE)
            except FileNotFoundError:
                pass
            readline.set_history_length(_MAX_HISTORY)

            commands = [
                "break",
                "delete",
                "run",
                "step",
                "next",
                "finish",
                "continue",
                "vars",
                "print",
                "stack",
                "list",
                "help",
                "quit",
                "s",
                "n",
                "c",
                "q",
                "bp",
            ]

            def completer(text: str, state: int) -> str | None:
                options = [c for c in commands if c.startswith(text)]
                return options[state] if state < len(options) else None

            readline.set_completer(completer)
            readline.parse_and_bind("tab: complete")
        except ImportError:
            pass

    def _save_history(self) -> None:
        try:
            import readline

            readline.write_history_file(_HISTORY_FILE)
        except (ImportError, OSError):
            pass

    def _command_loop(self) -> None:
        """Main debugger command loop."""
        while True:
            try:
                line = input(_colour("(tcldbg) ", _GREEN))
            except (EOFError, KeyboardInterrupt):
                print()
                self._do_quit()
                return

            line = line.strip()
            if not line:
                continue

            parts = line.split(None, 1)
            cmd = parts[0].lower()
            args = parts[1] if len(parts) > 1 else ""

            try:
                if cmd in ("help", "h", "?"):
                    self._do_help()
                elif cmd in ("break", "b", "bp"):
                    self._do_break(args)
                elif cmd in ("delete", "del", "d"):
                    self._do_delete(args)
                elif cmd == "run":
                    self._do_run()
                elif cmd in ("step", "s"):
                    self._do_step()
                elif cmd in ("next", "n"):
                    self._do_next()
                elif cmd in ("finish", "fin"):
                    self._do_finish()
                elif cmd in ("continue", "c"):
                    self._do_continue()
                elif cmd == "vars":
                    self._do_vars(args)
                elif cmd in ("print", "p"):
                    self._do_print(args)
                elif cmd == "stack":
                    self._do_stack()
                elif cmd in ("list", "l"):
                    self._do_list(args)
                elif cmd in ("quit", "q", "exit"):
                    self._do_quit()
                    return
                else:
                    print(f"Unknown command: {cmd}. Type 'help' for help.")
            except Exception as exc:
                print(_colour(f"Error: {exc}", _RED))

    # -- Commands -------------------------------------------------------------

    def _do_help(self) -> None:
        print(f"""
{_colour("Debugger Commands:", _BOLD)}
  {_colour("run", _CYAN)}              Start/restart execution
  {_colour("step", _CYAN)}, s           Step into (execute one statement)
  {_colour("next", _CYAN)}, n           Step over (skip procedure calls)
  {_colour("finish", _CYAN)}, fin       Step out (run until proc returns)
  {_colour("continue", _CYAN)}, c       Continue until breakpoint or end
  {_colour("break", _CYAN)} <line>, b   Set a breakpoint at line
  {_colour("delete", _CYAN)} <id>, d    Delete breakpoint by ID
  {_colour("vars", _CYAN)} [frame]      Show variables (optionally in frame N)
  {_colour("print", _CYAN)} <var>, p    Print a variable's value
  {_colour("stack", _CYAN)}             Show the call stack
  {_colour("list", _CYAN)} [line], l    Show source around current/given line
  {_colour("quit", _CYAN)}, q           Exit the debugger
""")

    def _do_break(self, args: str) -> None:
        if not args:
            # List breakpoints
            if not self._breakpoints:
                print("No breakpoints set.")
                return
            for bp_id, line in sorted(self._breakpoints.items()):
                src = ""
                if 0 < line <= len(self._source_lines):
                    src = f"  {_colour(self._source_lines[line - 1].strip(), _DIM)}"
                print(f"  #{bp_id}: line {line}{src}")
            return

        try:
            line = int(args)
        except ValueError:
            print(f"Invalid line number: {args}")
            return

        bp_id = self._next_bp_id
        self._next_bp_id += 1
        self._breakpoints[bp_id] = line

        # Update the backend
        all_lines = set(self._breakpoints.values())
        self._backend.set_breakpoints(all_lines)

        # Also update controller breakpoints if using VM backend
        ctrl = self._get_controller()
        if ctrl is not None:
            ctrl.set_breakpoints(all_lines)

        src = ""
        if 0 < line <= len(self._source_lines):
            src = f"  {_colour(self._source_lines[line - 1].strip(), _DIM)}"
        print(f"Breakpoint #{bp_id} at line {line}{src}")

    def _do_delete(self, args: str) -> None:
        if not args:
            print("Usage: delete <breakpoint-id>")
            return
        try:
            bp_id = int(args)
        except ValueError:
            print(f"Invalid breakpoint ID: {args}")
            return
        if bp_id not in self._breakpoints:
            print(f"No breakpoint #{bp_id}")
            return
        del self._breakpoints[bp_id]
        all_lines = set(self._breakpoints.values())
        self._backend.set_breakpoints(all_lines)
        ctrl = self._get_controller()
        if ctrl is not None:
            ctrl.set_breakpoints(all_lines)
        print(f"Deleted breakpoint #{bp_id}")

    def _do_run(self) -> None:
        if self._finished:
            print("Script already finished. Restart not yet supported.")
            return
        self._running = True
        self._backend.step_in()  # start and stop at first line
        if self._wait_for_stop():
            self._show_current_position()

    def _do_step(self) -> None:
        if not self._running:
            self._do_run()
            return
        if self._finished:
            print("Script already finished.")
            return
        self._backend.step_in()
        if self._wait_for_stop():
            self._show_current_position()

    def _do_next(self) -> None:
        if not self._running:
            self._do_run()
            return
        if self._finished:
            print("Script already finished.")
            return
        self._backend.step_over()
        if self._wait_for_stop():
            self._show_current_position()

    def _do_finish(self) -> None:
        if not self._running:
            print("Not running. Use 'run' to start.")
            return
        if self._finished:
            print("Script already finished.")
            return
        self._backend.step_out()
        if self._wait_for_stop():
            self._show_current_position()

    def _do_continue(self) -> None:
        if not self._running:
            self._do_run()
            return
        if self._finished:
            print("Script already finished.")
            return
        self._backend.continue_execution()
        if self._wait_for_stop():
            self._show_current_position()

    def _do_vars(self, args: str) -> None:
        if not self._running:
            print("Not running. Use 'run' to start.")
            return
        frame_id = 0
        if args:
            try:
                frame_id = int(args)
            except ValueError:
                print(f"Invalid frame ID: {args}")
                return

        variables = self._backend.get_variables(frame_id)
        if not variables:
            print("  (no variables)")
            return

        for var in variables:
            if var.type == "alias":
                target = var.alias_target or "?"
                print(
                    f"  {_colour(var.name, _CYAN)} -> {target} = {_colour(repr(var.value), _YELLOW)}"
                )
            elif var.type == "array":
                print(f"  {_colour(var.name, _CYAN)} = {_colour(var.value, _DIM)}")
                if var.children:
                    for child in var.children:
                        print(
                            f"    {_colour(child.name, _CYAN)} = {_colour(repr(child.value), _YELLOW)}"
                        )
            else:
                print(f"  {_colour(var.name, _CYAN)} = {_colour(repr(var.value), _YELLOW)}")

    def _do_print(self, args: str) -> None:
        if not args:
            print("Usage: print <variable-name>")
            return
        if not self._running:
            print("Not running. Use 'run' to start.")
            return

        # Try to find the variable in the current scope
        variables = self._backend.get_variables(0)
        for var in variables:
            if var.name == args:
                if var.type == "array" and var.children:
                    for child in var.children:
                        print(f"  {child.name} = {repr(child.value)}")
                else:
                    print(repr(var.value))
                return

        # Fall back to evaluate
        result = self._backend.evaluate(f"set {args}")
        print(result)

    def _do_stack(self) -> None:
        if not self._running:
            print("Not running. Use 'run' to start.")
            return

        frames = self._backend.get_stack_trace()
        for frame in frames:
            marker = _colour("->", _GREEN) if frame.id == 0 else "  "
            ns = f" ({frame.namespace})" if frame.namespace != "::" else ""
            line_info = f" line {frame.line}" if frame.line > 0 else ""
            print(f"  {marker} #{frame.id}: {_colour(frame.name, _CYAN)}{ns}{line_info}")

    def _do_list(self, args: str) -> None:
        if not self._source_lines:
            print("No source available.")
            return

        # Determine centre line
        if args:
            try:
                centre = int(args)
            except ValueError:
                print(f"Invalid line number: {args}")
                return
        else:
            ctrl = self._get_controller()
            if ctrl is not None:
                centre = ctrl.current_line
            else:
                centre = getattr(self._backend, "_current_line", 1)

        # Show 5 lines above and below
        start = max(1, centre - 5)
        end = min(len(self._source_lines), centre + 5)

        for i in range(start, end + 1):
            line_text = self._source_lines[i - 1]
            is_current = i == centre and self._running
            has_bp = i in self._breakpoints.values()

            marker = " "
            if is_current:
                marker = _colour("->", _GREEN)
            elif has_bp:
                marker = _colour("*", _RED)

            line_num = _colour(f"{i:4d}", _DIM)
            if is_current:
                print(f"  {marker} {line_num}  {_colour(line_text, _BOLD)}")
            else:
                print(f"  {marker} {line_num}  {line_text}")

    def _do_quit(self) -> None:
        self._save_history()
        self._backend.terminate()
        print("Bye.")

    # -- Display helpers ------------------------------------------------------

    def _show_current_position(self) -> None:
        ctrl = self._get_controller()
        if ctrl is not None:
            line = ctrl.current_line
            cmd = ctrl.current_command
            reason = "breakpoint" if line in self._breakpoints.values() else "step"
        else:
            line = getattr(self._backend, "_current_line", 0)
            cmd = getattr(self._backend, "_current_cmd", "")
            reason = "step"

        if line <= 0:
            return

        src = ""
        if 0 < line <= len(self._source_lines):
            src = self._source_lines[line - 1].strip()
        elif cmd:
            src = cmd

        if reason == "breakpoint":
            print(f"Hit {_colour('breakpoint', _RED)} at line {line}: {_colour(src, _BOLD)}")
        else:
            print(f"Line {line}: {_colour(src, _BOLD)}")
