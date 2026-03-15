"""Tests for the VM debug hook insertion in machine.py."""

from __future__ import annotations

from debugger.types import DebugAction
from vm.interp import TclInterp


class TestDebugHookFiring:
    """Verify the debug hook fires at source line boundaries."""

    def test_hook_fires_at_line_boundaries(self) -> None:
        """The hook should fire once per distinct source line."""
        lines_seen: list[int] = []

        def hook(instr, pc, stack, frame):  # noqa: ANN001, ANN202
            lines_seen.append(instr.source_line)
            return DebugAction.CONTINUE

        interp = TclInterp(debug_hook=hook)
        interp.eval("set x 1\nset y 2\nset z 3")

        # Should have fired for each source line
        assert len(lines_seen) >= 3
        # Lines should be monotonically increasing for a simple sequential script
        for i in range(1, len(lines_seen)):
            assert lines_seen[i] >= lines_seen[i - 1]

    def test_no_hook_no_overhead(self) -> None:
        """Without a debug hook, execution should work normally."""
        interp = TclInterp()
        result = interp.eval("expr {2 + 3}")
        assert result.value == "5"

    def test_hook_none_no_crash(self) -> None:
        """Passing debug_hook=None should be equivalent to no hook."""
        interp = TclInterp(debug_hook=None)
        result = interp.eval("set x hello")
        assert result.value == "hello"

    def test_hook_stop_aborts_execution(self) -> None:
        """Returning STOP from the hook should abort execution."""
        call_count = 0

        def hook(instr, pc, stack, frame):  # noqa: ANN001, ANN202
            nonlocal call_count
            call_count += 1
            if call_count >= 2:
                return DebugAction.STOP
            return DebugAction.CONTINUE

        interp = TclInterp(debug_hook=hook)
        interp.eval("set x 1\nset y 2\nset z 3")
        # Execution should have stopped after the second line
        assert call_count == 2

    def test_hook_fires_in_proc(self) -> None:
        """The hook should fire inside proc bodies too."""
        lines_seen: list[int] = []

        def hook(instr, pc, stack, frame):  # noqa: ANN001, ANN202
            lines_seen.append(instr.source_line)
            return DebugAction.CONTINUE

        interp = TclInterp(debug_hook=hook)
        interp.eval(
            'proc greet {name} {\n    set msg "hello $name"\n    return $msg\n}\ngreet world'
        )

        # Hook should have fired for lines inside the proc body
        assert len(lines_seen) >= 3

    def test_hook_receives_frame(self) -> None:
        """The hook should receive the correct CallFrame."""
        frame_levels: list[int] = []

        def hook(instr, pc, stack, frame):  # noqa: ANN001, ANN202
            frame_levels.append(frame.level)
            return DebugAction.CONTINUE

        interp = TclInterp(debug_hook=hook)
        interp.eval("proc foo {} { set x 1 }\nfoo")

        # Should see level 0 for global and level 1 for inside the proc
        assert 0 in frame_levels
        assert 1 in frame_levels
