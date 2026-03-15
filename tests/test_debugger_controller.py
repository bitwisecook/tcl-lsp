"""Tests for the DebugController — breakpoints, stepping, state inspection."""

from __future__ import annotations

import threading

from debugger.controller import DebugController
from debugger.types import StepMode
from vm.interp import TclInterp


class TestBreakpoints:
    """Breakpoint management on the controller."""

    def test_add_breakpoint(self) -> None:
        ctrl = DebugController()
        result = ctrl.add_breakpoint(5)
        assert result == 5
        assert 5 in ctrl.get_breakpoints()

    def test_remove_breakpoint(self) -> None:
        ctrl = DebugController()
        ctrl.add_breakpoint(5)
        ctrl.remove_breakpoint(5)
        assert 5 not in ctrl.get_breakpoints()

    def test_set_breakpoints_replaces(self) -> None:
        ctrl = DebugController()
        ctrl.add_breakpoint(5)
        ctrl.set_breakpoints({10, 20})
        bps = ctrl.get_breakpoints()
        assert bps == [10, 20]


class TestStepping:
    """Verify step-in, step-over, step-out via the VM backend."""

    def test_step_in_stops_every_line(self) -> None:
        """STEP_IN should stop at every source line."""
        ctrl = DebugController()
        lines_stopped: list[int] = []

        def run_vm() -> None:
            interp = TclInterp(debug_hook=ctrl.debug_hook)
            interp.eval("set a 1\nset b 2\nset c 3")

        vm_thread = threading.Thread(target=run_vm, daemon=True)
        vm_thread.start()

        # Should stop at each line
        for _ in range(3):
            stopped = ctrl.wait_for_stop(timeout=5.0)
            if not stopped:
                break
            lines_stopped.append(ctrl.current_line)
            ctrl.resume(StepMode.STEP_IN)

        vm_thread.join(timeout=5.0)
        assert len(lines_stopped) >= 3

    def test_continue_skips_to_breakpoint(self) -> None:
        """CONTINUE should skip to the next breakpoint."""
        ctrl = DebugController()
        ctrl.set_breakpoints({3})
        lines_stopped: list[int] = []

        def run_vm() -> None:
            interp = TclInterp(debug_hook=ctrl.debug_hook)
            interp.eval("set a 1\nset b 2\nset c 3\nset d 4")

        vm_thread = threading.Thread(target=run_vm, daemon=True)
        vm_thread.start()

        # First stop: line 1 (step_in is the default mode)
        stopped = ctrl.wait_for_stop(timeout=5.0)
        assert stopped
        lines_stopped.append(ctrl.current_line)

        # Continue — should skip to breakpoint at line 3
        ctrl.resume(StepMode.CONTINUE)
        stopped = ctrl.wait_for_stop(timeout=5.0)
        if stopped:
            lines_stopped.append(ctrl.current_line)

        # Continue past breakpoint
        ctrl.resume(StepMode.CONTINUE)
        vm_thread.join(timeout=5.0)

        assert 3 in lines_stopped

    def test_step_over_skips_proc_body(self) -> None:
        """STEP_OVER should not stop inside a called procedure."""
        ctrl = DebugController()
        frames_at_stop: list[str] = []
        lines_stopped: list[int] = []

        def run_vm() -> None:
            interp = TclInterp(debug_hook=ctrl.debug_hook)
            interp.eval(
                "proc add {a b} {\n"
                "    set result [expr {$a + $b}]\n"
                "    return $result\n"
                "}\n"
                "set x [add 1 2]\n"
                "set y done\n"
            )

        vm_thread = threading.Thread(target=run_vm, daemon=True)
        vm_thread.start()

        # Step through the proc definition
        stopped = ctrl.wait_for_stop(timeout=5.0)
        assert stopped
        lines_stopped.append(ctrl.current_line)
        frames_at_stop.append(ctrl.get_stack_trace()[0].name)

        # Step to the call line
        ctrl.resume(StepMode.STEP_IN)
        stopped = ctrl.wait_for_stop(timeout=5.0)
        if stopped:
            lines_stopped.append(ctrl.current_line)
            # Now step OVER the call
            ctrl.resume(StepMode.STEP_OVER)
            stopped = ctrl.wait_for_stop(timeout=5.0)
            if stopped:
                lines_stopped.append(ctrl.current_line)
                # Should be at the line AFTER the call, not inside the proc
                frame = ctrl.get_stack_trace()[0]
                assert frame.name == "global"

        ctrl.resume(StepMode.CONTINUE)
        vm_thread.join(timeout=5.0)


class TestVariableInspection:
    """Verify variable state inspection while stopped."""

    def test_get_variables_shows_scalars(self) -> None:
        ctrl = DebugController()

        def run_vm() -> None:
            interp = TclInterp(debug_hook=ctrl.debug_hook)
            interp.eval("set name Alice\nset age 30")

        vm_thread = threading.Thread(target=run_vm, daemon=True)
        vm_thread.start()

        # Stop at first line
        stopped = ctrl.wait_for_stop(timeout=5.0)
        assert stopped
        ctrl.resume(StepMode.STEP_IN)

        # Stop at second line — "name" should be visible
        stopped = ctrl.wait_for_stop(timeout=5.0)
        assert stopped

        variables = ctrl.get_variables()
        var_names = [v.name for v in variables]
        assert "name" in var_names

        name_var = next(v for v in variables if v.name == "name")
        assert name_var.value == "Alice"
        assert name_var.type == "scalar"

        ctrl.resume(StepMode.CONTINUE)
        vm_thread.join(timeout=5.0)

    def test_get_stack_trace(self) -> None:
        ctrl = DebugController()

        def run_vm() -> None:
            interp = TclInterp(debug_hook=ctrl.debug_hook)
            interp.eval("proc inner {} { set x 1 }\nproc outer {} { inner }\nouter")

        vm_thread = threading.Thread(target=run_vm, daemon=True)
        vm_thread.start()

        # Step until we're inside 'inner'
        for _ in range(20):
            stopped = ctrl.wait_for_stop(timeout=5.0)
            if not stopped:
                break
            frames = ctrl.get_stack_trace()
            if len(frames) >= 3:  # inner -> outer -> global
                assert frames[0].name == "inner"
                assert frames[1].name == "outer"
                assert frames[2].name == "global"
                ctrl.resume(StepMode.CONTINUE)
                break
            ctrl.resume(StepMode.STEP_IN)

        vm_thread.join(timeout=5.0)
