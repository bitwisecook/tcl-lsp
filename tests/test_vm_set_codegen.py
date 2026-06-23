"""Regression tests for the ``set`` codegen / errorInfo fixes proven by
the Tcl 9 ``set.test`` core slice (VM side).

Each test pins a contract that previously failed in
``tmp/tcl9.0.3/tests/set.test`` (set-1.15, set-1.25, set-1.26, set-2.1,
set-2.4, set-4.1, set-4.4) so a future codegen change can't silently
regress it.  The full slice runs in ``test_vm_tcl9_core_baseline.py``.
"""

from __future__ import annotations

from tooling.vm.interp import TclInterp


class TestNestedArraySet:
    """``set x [set a(foo) v]`` must store through the array element
    (``storeArrayStk``), not collapse to a scalar ``storeStk`` chain."""

    def test_nested_array_set_propagates_value(self) -> None:
        interp = TclInterp()
        interp.eval("catch {unset a}")
        interp.eval("set x 27")
        interp.eval("set x [set a(foo) 11]")
        assert interp.eval("set x").value == "11"
        assert interp.eval("set a(foo)").value == "11"

    def test_nested_array_set_two_levels(self) -> None:
        interp = TclInterp()
        interp.eval("catch {unset a}; catch {unset b}")
        interp.eval("set r [set x [set a(foo) [set b(bar) 9]]]")
        assert interp.eval("set x").value == "9"
        assert interp.eval("set a(foo)").value == "9"
        assert interp.eval("set b(bar)").value == "9"
        assert interp.eval("set r").value == "9"

    def test_all_scalar_nested_set_unaffected(self) -> None:
        interp = TclInterp()
        interp.eval("set r [set x [set y [set z 42]]]")
        assert interp.eval("set z").value == "42"
        assert interp.eval("set y").value == "42"
        assert interp.eval("set x").value == "42"
        assert interp.eval("set r").value == "42"


class TestBracedArrayKeyLiteral:
    """A brace-suppressed array element key keeps its ``$`` literal —
    ``set {arr($foo)} 5`` creates the element literally named ``$foo``."""

    def test_braced_key_dollar_is_literal(self) -> None:
        interp = TclInterp()
        interp.eval("catch {unset arr}")
        assert interp.eval("set {arr($foo)} 5").value == "5"
        # The element is literally ``$foo``; ``array names`` list-quotes a
        # ``$``-leading element as ``{$foo}`` (matches tclsh 9.0).
        assert interp.eval("array names arr").value == "{$foo}"

    def test_braced_key_literal_in_proc(self) -> None:
        interp = TclInterp()
        assert interp.eval("apply {{} {set a x; set {be($a)} 1; array names be}}").value == "{$a}"

    def test_live_array_index_still_substitutes(self) -> None:
        # The fix must NOT make a live (non-braced) index literal.
        interp = TclInterp()
        interp.eval("set i 5; set a($i) hi")
        assert interp.eval("set a(5)").value == "hi"

    def test_dynamic_read_index_substitutes(self) -> None:
        # tcltest's ``$testConstraints($constraint)`` shape — a live read
        # index must resolve, not be taken literally.
        interp = TclInterp()
        out = interp.eval("apply {{} {set c foo; set arr(foo) 0; expr {!$arr($c)}}}")
        assert out.value == "1"


class TestArrayIndexInSubstTemplate:
    """An array read embedded in a larger word (``$be($a,hej)`` inside a
    key/value template) must resolve the element, not the whole array."""

    def test_nested_array_read_in_key_template(self) -> None:
        interp = TclInterp()
        body = (
            "apply {{} {set a x; set be(x,hej) 1;set out(hej,$be($a,hej),hej) 9;array names out}}"
        )
        assert interp.eval(body).value == "hej,1,hej"

    def test_backslash_then_live_index_read(self) -> None:
        # ``b\e($a)`` → array read be($a); the ``\e`` collapses to ``e``
        # while the live ``$a`` index still substitutes.
        interp = TclInterp()
        assert (
            interp.eval(r"apply {{} {set a x; set be(x) 7; set r [set b\e($a)]; set r}}").value
            == "7"
        )


class TestSetErrorInfo:
    """``catch`` + ``$::errorInfo`` must carry the ``while executing``
    frame for a failing compiled / dynamic ``set`` and for a write-trace
    rejection."""

    def test_compiled_set_read_error_frame(self) -> None:
        interp = TclInterp()
        interp.eval('catch {set {"foo}} msg')
        ei = interp.eval("set ::errorInfo").value
        assert 'can\'t read ""foo": no such variable' in ei
        assert "while executing" in ei
        assert 'set {"foo}' in ei

    def test_dynamic_set_read_error_frame_keeps_source(self) -> None:
        interp = TclInterp()
        interp.eval("set z set")
        interp.eval('catch {$z {"foo}} msg')
        ei = interp.eval("set ::errorInfo").value
        # The frame must show the original source spelling, not a
        # reconstructed ``${z} "foo"``.
        assert 'while executing\n"$z {"foo}"' in ei

    def test_write_trace_rejection_chain(self) -> None:
        interp = TclInterp()
        interp.eval('proc readonly args {error "variable is read-only"}')
        interp.eval("set x 123")
        interp.eval("trace add var x write readonly")
        interp.eval("catch {set x 1} msg")
        ei = interp.eval("set ::errorInfo").value
        # The readonly callback's traceback survives, ending with the
        # triggering command frame.
        assert "variable is read-only" in ei
        assert "while executing" in ei
        assert ei.rstrip().endswith('"set x 1"')

    def test_write_trace_rejection_dynamic_invoke(self) -> None:
        interp = TclInterp()
        interp.eval("set z set")
        interp.eval('proc readonly args {error "variable is read-only"}')
        interp.eval("$z x 123")
        interp.eval("trace add var x write readonly")
        interp.eval("catch {$z x 1} msg")
        ei = interp.eval("set ::errorInfo").value
        assert "variable is read-only" in ei
        assert ei.rstrip().endswith('"$z x 1"')
