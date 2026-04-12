"""Basic tests for the Tcl VM: set, puts, expr, and simple scripts."""

from __future__ import annotations

import io

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestVMSet:
    """Tests for the ``set`` command."""

    def test_set_and_read(self) -> None:
        interp = TclInterp()
        result = interp.eval("set x 42")
        assert result.value == "42"
        result = interp.eval("set x")
        assert result.value == "42"

    def test_set_string(self) -> None:
        interp = TclInterp()
        result = interp.eval('set greeting "hello world"')
        assert result.value == "hello world"

    def test_set_multiple(self) -> None:
        interp = TclInterp()
        interp.eval("set a 1")
        interp.eval("set b 2")
        result = interp.eval("set a")
        assert result.value == "1"
        result = interp.eval("set b")
        assert result.value == "2"

    def test_set_overwrite(self) -> None:
        interp = TclInterp()
        interp.eval("set x 1")
        interp.eval("set x 2")
        result = interp.eval("set x")
        assert result.value == "2"

    def test_read_undefined_variable(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="no such variable"):
            interp.eval("set undefined_var")


class TestVMPuts:
    """Tests for the ``puts`` command."""

    def test_puts_stdout(self) -> None:
        interp = TclInterp()
        buf = io.StringIO()
        interp.channels["stdout"] = buf
        interp.eval('puts "Hello, World!"')
        assert buf.getvalue() == "Hello, World!\n"

    def test_puts_nonewline(self) -> None:
        interp = TclInterp()
        buf = io.StringIO()
        interp.channels["stdout"] = buf
        interp.eval('puts -nonewline "hello"')
        assert buf.getvalue() == "hello"

    def test_puts_stderr(self) -> None:
        interp = TclInterp()
        buf = io.StringIO()
        interp.channels["stderr"] = buf
        interp.eval('puts stderr "error message"')
        assert buf.getvalue() == "error message\n"


class TestVMExpr:
    """Tests for the ``expr`` command and expression evaluation."""

    def test_basic_arithmetic(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {1 + 2}")
        assert result.value == "3"

    def test_multiplication(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {6 * 7}")
        assert result.value == "42"

    def test_division(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {10 / 3}")
        assert result.value == "3"

    def test_float_division(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {10.0 / 3}")
        # Should be approximately 3.333...
        assert result.value.startswith("3.333")

    def test_modulo(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {10 % 3}")
        assert result.value == "1"

    def test_comparison(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {5 > 3}")
        assert result.value == "1"
        result = interp.eval("expr {5 < 3}")
        assert result.value == "0"

    def test_expr_with_variables(self) -> None:
        interp = TclInterp()
        interp.eval("set x 10")
        interp.eval("set y 20")
        result = interp.eval("expr {$x + $y}")
        assert result.value == "30"

    def test_expr_nested(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {(2 + 3) * 4}")
        assert result.value == "20"

    def test_expr_boolean(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {1 && 1}")
        assert result.value == "1"
        result = interp.eval("expr {1 && 0}")
        assert result.value == "0"
        result = interp.eval("expr {0 || 1}")
        assert result.value == "1"

    def test_expr_string_compare(self) -> None:
        interp = TclInterp()
        result = interp.eval('expr {"abc" eq "abc"}')
        assert result.value == "1"
        result = interp.eval('expr {"abc" ne "def"}')
        assert result.value == "1"

    def test_math_functions(self) -> None:
        interp = TclInterp()
        result = interp.eval("expr {abs(-5)}")
        assert result.value == "5"
        result = interp.eval("expr {max(3, 7)}")
        assert result.value == "7"
        result = interp.eval("expr {min(3, 7)}")
        assert result.value == "3"


class TestVMControlFlow:
    """Tests for control flow: if, for, while, foreach."""

    def test_if_true(self) -> None:
        interp = TclInterp()
        interp.eval('if {1} { set x "yes" }')
        result = interp.eval("set x")
        assert result.value == "yes"

    def test_if_false(self) -> None:
        interp = TclInterp()
        interp.eval('if {0} { set x "yes" } else { set x "no" }')
        result = interp.eval("set x")
        assert result.value == "no"

    def test_if_elseif(self) -> None:
        interp = TclInterp()
        interp.eval("set x 2")
        interp.eval("if {$x == 1} { set r a } elseif {$x == 2} { set r b } else { set r c }")
        result = interp.eval("set r")
        assert result.value == "b"

    def test_for_loop(self) -> None:
        interp = TclInterp()
        interp.eval("set sum 0")
        interp.eval("for {set i 0} {$i < 5} {incr i} { set sum [expr {$sum + $i}] }")
        result = interp.eval("set sum")
        assert result.value == "10"

    def test_while_loop(self) -> None:
        interp = TclInterp()
        interp.eval("set i 0")
        interp.eval("set sum 0")
        interp.eval("while {$i < 5} { set sum [expr {$sum + $i}]; incr i }")
        result = interp.eval("set sum")
        assert result.value == "10"

    def test_foreach(self) -> None:
        interp = TclInterp()
        interp.eval("set sum 0")
        interp.eval("foreach x {1 2 3 4 5} { set sum [expr {$sum + $x}] }")
        result = interp.eval("set sum")
        assert result.value == "15"

    def test_break_in_for(self) -> None:
        interp = TclInterp()
        interp.eval("set i 0")
        interp.eval("for {set i 0} {$i < 10} {incr i} { if {$i == 5} break }")
        result = interp.eval("set i")
        assert result.value == "5"

    def test_continue_in_for(self) -> None:
        interp = TclInterp()
        interp.eval("set sum 0")
        interp.eval(
            "for {set i 0} {$i < 5} {incr i} { if {$i == 2} continue; set sum [expr {$sum + $i}] }"
        )
        result = interp.eval("set sum")
        # 0 + 1 + 3 + 4 = 8  (skip 2)
        assert result.value == "8"


class TestVMProcs:
    """Tests for procedure definition and calling."""

    def test_simple_proc(self) -> None:
        interp = TclInterp()
        interp.eval("proc double {x} { expr {$x * 2} }")
        result = interp.eval("double 21")
        assert result.value == "42"

    def test_proc_with_return(self) -> None:
        interp = TclInterp()
        interp.eval('proc greet {name} { return "Hello, $name" }')
        result = interp.eval('greet "World"')
        assert result.value == "Hello, World"

    def test_proc_default_args(self) -> None:
        interp = TclInterp()
        interp.eval('proc greet {{name "World"}} { return "Hello, $name" }')
        result = interp.eval("greet")
        assert result.value == "Hello, World"
        result = interp.eval('greet "Tcl"')
        assert result.value == "Hello, Tcl"

    def test_recursive_proc(self) -> None:
        interp = TclInterp()
        interp.eval(
            "proc factorial {n} { "
            "if {$n <= 1} { return 1 }; "
            "return [expr {$n * [factorial [expr {$n - 1}]]}] }"
        )
        result = interp.eval("factorial 5")
        assert result.value == "120"

    def test_proc_wrong_args(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {a b} { expr {$a + $b} }")
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval("f 1")


class TestVMErrorHandling:
    """Tests for error handling: catch, error."""

    def test_catch_error(self) -> None:
        interp = TclInterp()
        result = interp.eval('catch { error "boom" } msg')
        assert result.value == "1"  # TCL_ERROR
        result = interp.eval("set msg")
        assert result.value == "boom"

    def test_catch_ok(self) -> None:
        interp = TclInterp()
        result = interp.eval("catch { set x 42 } msg")
        assert result.value == "0"  # TCL_OK

    def test_error_command(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="something went wrong"):
            interp.eval('error "something went wrong"')


class TestVMStringOps:
    """Tests for string command."""

    def test_string_length(self) -> None:
        interp = TclInterp()
        result = interp.eval('string length "hello"')
        assert result.value == "5"

    def test_string_index(self) -> None:
        interp = TclInterp()
        result = interp.eval('string index "hello" 0')
        assert result.value == "h"

    def test_string_range(self) -> None:
        interp = TclInterp()
        result = interp.eval('string range "hello world" 0 4')
        assert result.value == "hello"

    def test_string_tolower(self) -> None:
        interp = TclInterp()
        result = interp.eval('string tolower "HELLO"')
        assert result.value == "hello"

    def test_string_toupper(self) -> None:
        interp = TclInterp()
        result = interp.eval('string toupper "hello"')
        assert result.value == "HELLO"

    def test_string_trim(self) -> None:
        interp = TclInterp()
        result = interp.eval('string trim "  hello  "')
        assert result.value == "hello"

    def test_string_match(self) -> None:
        interp = TclInterp()
        result = interp.eval('string match "hel*" "hello"')
        assert result.value == "1"
        result = interp.eval('string match "xyz*" "hello"')
        assert result.value == "0"


class TestVMListOps:
    """Tests for list commands."""

    def test_list_create(self) -> None:
        interp = TclInterp()
        result = interp.eval("list a b c")
        assert result.value == "a b c"

    def test_llength(self) -> None:
        interp = TclInterp()
        result = interp.eval('llength "a b c d"')
        assert result.value == "4"

    def test_lindex(self) -> None:
        interp = TclInterp()
        result = interp.eval('lindex "a b c d" 2')
        assert result.value == "c"

    def test_lrange(self) -> None:
        interp = TclInterp()
        result = interp.eval('lrange "a b c d e" 1 3')
        assert result.value == "b c d"

    def test_lsort(self) -> None:
        interp = TclInterp()
        result = interp.eval('lsort "d b a c"')
        assert result.value == "a b c d"

    def test_join(self) -> None:
        interp = TclInterp()
        result = interp.eval('join "a b c" ","')
        assert result.value == "a,b,c"

    def test_split(self) -> None:
        interp = TclInterp()
        result = interp.eval('split "a,b,c" ","')
        assert result.value == "a b c"


class TestVMIncr:
    """Tests for incr command."""

    def test_incr_default(self) -> None:
        interp = TclInterp()
        interp.eval("set x 5")
        result = interp.eval("incr x")
        assert result.value == "6"

    def test_incr_amount(self) -> None:
        interp = TclInterp()
        interp.eval("set x 5")
        result = interp.eval("incr x 10")
        assert result.value == "15"

    def test_incr_negative(self) -> None:
        interp = TclInterp()
        interp.eval("set x 5")
        result = interp.eval("incr x -3")
        assert result.value == "2"

    def test_incr_undefined(self) -> None:
        interp = TclInterp()
        result = interp.eval("incr newvar")
        assert result.value == "1"


class TestVMDisassemble:
    """Tests for bytecode generation / disassembly."""

    def test_simple_set_disassembly(self) -> None:
        from core.compiler.codegen import format_module_asm
        from vm.compiler import compile_script

        module_asm, _ = compile_script("set x 42")
        text = format_module_asm(module_asm)
        assert "push1" in text.lower() or "push4" in text.lower()
        assert "storeStk" in text or "storeScalar1" in text

    def test_bytecode_roundtrip(self) -> None:
        """Compile and execute a script, verify the result matches."""
        interp = TclInterp()
        result = interp.eval("set x 42; expr {$x + 8}")
        assert result.value == "50"


class TestVariableShapeBytecodeIdentity:
    """Variable-shape forms should compile to distinct bytecode paths."""

    def test_braced_scalar_like_array_name_uses_scalar_load(self) -> None:
        from core.compiler.codegen import format_module_asm
        from vm.compiler import compile_script

        module_asm, _ = compile_script("set x ${a(1)}")
        text = format_module_asm(module_asm)
        assert "loadStk" in text
        assert "loadArrayStk" not in text
        assert '"a(1)"' in text

    def test_unbraced_array_ref_uses_array_load(self) -> None:
        from core.compiler.codegen import format_module_asm
        from vm.compiler import compile_script

        module_asm, _ = compile_script("set x $a(1)")
        text = format_module_asm(module_asm)
        assert "loadArrayStk" in text
        assert '"a"' in text
        assert '"1"' in text

    def test_namespaced_array_forms_distinguish_scalar_like_vs_array_ref(self) -> None:
        from core.compiler.codegen import format_module_asm
        from vm.compiler import compile_script

        scalar_like, _ = compile_script("set x ${::ns::arr(k)}")
        array_ref, _ = compile_script("set x $::ns::arr(k)")
        scalar_text = format_module_asm(scalar_like)
        array_text = format_module_asm(array_ref)
        assert "loadStk" in scalar_text and "loadArrayStk" not in scalar_text
        assert "loadArrayStk" in array_text
        assert '"::ns::arr(k)"' in scalar_text
        assert '"::ns::arr"' in array_text
        assert '"k"' in array_text
