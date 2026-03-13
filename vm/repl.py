"""Curses-based REPL with readline fallback for the Tcl VM."""

from __future__ import annotations

import os
import sys

from .interp import TclInterp
from .types import TclBreak, TclContinue, TclError, TclReturn

_HISTORY_FILE = os.path.expanduser("~/.tclvm_history")
_MAX_HISTORY = 1000


# Readline fallback REPL


def _readline_repl(interp: TclInterp) -> None:
    """Simple readline-based REPL (fallback when curses unavailable)."""
    try:
        import readline

        try:
            readline.read_history_file(_HISTORY_FILE)
        except FileNotFoundError:
            pass
        readline.set_history_length(_MAX_HISTORY)

        # Tab completion
        def completer(text: str, state: int) -> str | None:
            options = [n for n in interp.command_names() if n.startswith(text)]
            options.extend(n for n in interp.procedures if n.startswith(text))
            return options[state] if state < len(options) else None

        readline.set_completer(completer)
        readline.parse_and_bind("tab: complete")
    except ImportError:
        pass

    interp.global_frame.set_var("tcl_interactive", "1")
    buf = ""

    while True:
        prompt = "% " if not buf else "> "
        try:
            line = input(prompt)
        except EOFError:
            print()
            break
        except KeyboardInterrupt:
            print()
            buf = ""
            continue

        buf = (buf + "\n" + line) if buf else line

        if not interp.is_complete(buf):
            continue

        source = buf
        buf = ""

        if not source.strip():
            continue

        try:
            result = interp.eval(source)
            if result.value:
                print(result.value)
        except TclError as e:
            print(f"error: {e.message}", file=sys.stderr)
        except TclReturn as ret:
            if ret.value:
                print(ret.value)
        except TclBreak:
            print('invoked "break" outside of a loop', file=sys.stderr)
        except TclContinue:
            print('invoked "continue" outside of a loop', file=sys.stderr)
        except SystemExit:
            break

    # Save history
    try:
        import readline

        readline.write_history_file(_HISTORY_FILE)
    except (ImportError, OSError):
        pass


# Curses REPL


def _curses_repl(interp: TclInterp) -> None:
    """Curses-based REPL with scrollable output."""
    import curses

    def _main(stdscr: curses.window) -> None:
        curses.use_default_colors()
        curses.init_pair(1, curses.COLOR_RED, -1)
        curses.init_pair(2, curses.COLOR_GREEN, -1)
        curses.init_pair(3, curses.COLOR_CYAN, -1)

        max_y, max_x = stdscr.getmaxyx()
        # Output area: all lines except the last 1
        output_lines: list[tuple[str, int]] = []  # (text, color_pair)
        history: list[str] = []
        history_idx = -1
        buf = ""
        cursor_pos = 0

        # Load history
        try:
            with open(_HISTORY_FILE) as f:
                history = [line.rstrip("\n") for line in f.readlines()][-_MAX_HISTORY:]
        except FileNotFoundError:
            pass

        interp.global_frame.set_var("tcl_interactive", "1")

        def _add_output(text: str, colour: int = 0) -> None:
            for line in text.split("\n"):
                output_lines.append((line, colour))

        def _refresh() -> None:
            stdscr.erase()
            max_y, max_x = stdscr.getmaxyx()

            # Draw output lines (scroll to bottom)
            visible = max_y - 1  # reserve last line for input
            start = max(0, len(output_lines) - visible)
            for i, (line, colour) in enumerate(output_lines[start : start + visible]):
                try:
                    stdscr.addnstr(i, 0, line, max_x - 1, curses.color_pair(colour))
                except curses.error:
                    pass

            # Draw input line
            prompt = "% " if not _is_continuation() else "> "
            input_line = prompt + buf
            try:
                stdscr.addnstr(max_y - 1, 0, input_line, max_x - 1, curses.color_pair(3))
            except curses.error:
                pass

            # Position cursor
            cx = len(prompt) + cursor_pos
            if cx < max_x:
                try:
                    stdscr.move(max_y - 1, cx)
                except curses.error:
                    pass

            stdscr.refresh()

        def _is_continuation() -> bool:
            return bool(buf) and not interp.is_complete(buf)

        _add_output("Tcl VM REPL (type Ctrl+D to exit)", 2)
        _refresh()

        while True:
            try:
                ch = stdscr.getch()
            except KeyboardInterrupt:
                buf = ""
                cursor_pos = 0
                _refresh()
                continue

            if ch == curses.KEY_RESIZE:
                _refresh()
                continue

            # Ctrl+D — exit
            if ch == 4:
                break

            # Ctrl+C — clear input
            if ch == 3:
                buf = ""
                cursor_pos = 0
                _refresh()
                continue

            # Enter
            if ch in (curses.KEY_ENTER, 10, 13):
                if buf and not interp.is_complete(buf):
                    buf += "\n"
                    cursor_pos = len(buf)
                    _refresh()
                    continue

                source = buf
                prompt_text = ("% " if "\n" not in source else "% ") + source
                _add_output(prompt_text, 3)

                if source.strip():
                    if not history or history[-1] != source:
                        history.append(source)
                    history_idx = -1

                    try:
                        result = interp.eval(source)
                        if result.value:
                            _add_output(result.value)
                    except TclError as e:
                        _add_output(f"error: {e.message}", 1)
                    except TclReturn as ret:
                        if ret.value:
                            _add_output(ret.value)
                    except TclBreak:
                        _add_output('invoked "break" outside of a loop', 1)
                    except TclContinue:
                        _add_output('invoked "continue" outside of a loop', 1)
                    except SystemExit:
                        return

                buf = ""
                cursor_pos = 0
                _refresh()
                continue

            # Backspace
            if ch in (curses.KEY_BACKSPACE, 127, 8):
                if cursor_pos > 0:
                    buf = buf[: cursor_pos - 1] + buf[cursor_pos:]
                    cursor_pos -= 1
                _refresh()
                continue

            # Delete
            if ch == curses.KEY_DC:
                if cursor_pos < len(buf):
                    buf = buf[:cursor_pos] + buf[cursor_pos + 1 :]
                _refresh()
                continue

            # Left arrow
            if ch == curses.KEY_LEFT:
                if cursor_pos > 0:
                    cursor_pos -= 1
                _refresh()
                continue

            # Right arrow
            if ch == curses.KEY_RIGHT:
                if cursor_pos < len(buf):
                    cursor_pos += 1
                _refresh()
                continue

            # Home
            if ch == curses.KEY_HOME:
                cursor_pos = 0
                _refresh()
                continue

            # End
            if ch == curses.KEY_END:
                cursor_pos = len(buf)
                _refresh()
                continue

            # Up arrow — history
            if ch == curses.KEY_UP:
                if history:
                    if history_idx < 0:
                        history_idx = len(history) - 1
                    elif history_idx > 0:
                        history_idx -= 1
                    buf = history[history_idx]
                    cursor_pos = len(buf)
                _refresh()
                continue

            # Down arrow — history
            if ch == curses.KEY_DOWN:
                if history_idx >= 0:
                    history_idx += 1
                    if history_idx >= len(history):
                        history_idx = -1
                        buf = ""
                    else:
                        buf = history[history_idx]
                    cursor_pos = len(buf)
                _refresh()
                continue

            # Ctrl+A — beginning of line
            if ch == 1:
                cursor_pos = 0
                _refresh()
                continue

            # Ctrl+E — end of line
            if ch == 5:
                cursor_pos = len(buf)
                _refresh()
                continue

            # Ctrl+K — kill to end of line
            if ch == 11:
                buf = buf[:cursor_pos]
                _refresh()
                continue

            # Ctrl+U — kill to beginning of line
            if ch == 21:
                buf = buf[cursor_pos:]
                cursor_pos = 0
                _refresh()
                continue

            # Printable character
            if 32 <= ch < 127:
                buf = buf[:cursor_pos] + chr(ch) + buf[cursor_pos:]
                cursor_pos += 1
                _refresh()
                continue

            _refresh()

        # Save history
        try:
            with open(_HISTORY_FILE, "w") as f:
                for h in history[-_MAX_HISTORY:]:
                    f.write(h + "\n")
        except OSError:
            pass

    curses.wrapper(_main)


# Public entry point


def run_repl(interp: TclInterp, *, use_curses: bool = True) -> None:
    """Launch the interactive REPL."""
    if use_curses and sys.stdin.isatty() and sys.stdout.isatty():
        try:
            import curses  # noqa: F811, F401

            _curses_repl(interp)
            return
        except ImportError:
            pass
        except Exception:
            # Fall back to readline if curses fails for any reason
            pass

    _readline_repl(interp)
