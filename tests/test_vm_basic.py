"""Basic tests for the Tcl VM: set, puts, expr, and simple scripts."""

from __future__ import annotations

import io

import pytest

from tooling.vm.interp import TclInterp
from tooling.vm.types import TclError


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

    def test_float_div_by_positive_zero(self) -> None:
        # Verified against tclsh 9.0.3: 1.0 / 0.0 = Inf, -1.0 / 0.0 = -Inf
        interp = TclInterp()
        assert interp.eval("expr {1.0 / 0.0}").value == "Inf"
        assert interp.eval("expr {-1.0 / 0.0}").value == "-Inf"

    def test_float_div_by_negative_zero(self) -> None:
        # Verified against tclsh 9.0.3: sign flips when dividing by -0.0
        interp = TclInterp()
        assert interp.eval("expr {1.0 / -0.0}").value == "-Inf"
        assert interp.eval("expr {-1.0 / -0.0}").value == "Inf"

    def test_float_zero_div_zero_is_domain_error(self) -> None:
        # Verified against tclsh 9.0.3: 0.0 / ±0.0 raises ARITH DOMAIN
        interp = TclInterp()
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {0.0 / 0.0}")
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {0.0 / -0.0}")

    def test_nan_producing_ops_raise_domain_error(self) -> None:
        # Verified against tclsh 9.0.3: Inf-Inf, Inf*0, Inf/Inf all raise
        # ARITH DOMAIN instead of silently returning NaN.
        interp = TclInterp()
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {1.0/0.0 - 1.0/0.0}")
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {1.0/0.0 * 0.0}")
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {(1.0/0.0) / (1.0/0.0)}")

    def test_inf_arithmetic(self) -> None:
        # Verified against tclsh 9.0.3
        interp = TclInterp()
        assert interp.eval("expr {1.0/0.0 + 1}").value == "Inf"
        assert interp.eval("expr {1.0/0.0 * -1.0}").value == "-Inf"
        assert interp.eval("expr {1.0/0.0 + 1.0/0.0}").value == "Inf"
        assert interp.eval("expr {1.0 / (1.0/0.0)}").value == "0.0"

    def test_large_float_string(self) -> None:
        # Verified against tclsh 9.0.3: 1e308 should format as "1e+308"
        interp = TclInterp()
        assert interp.eval("expr {1e308}").value == "1e+308"
        assert interp.eval("expr {1e308 * 10}").value == "Inf"
        assert interp.eval("expr {-1e308 * 10}").value == "-Inf"

    def test_math_func_log_zero(self) -> None:
        # Verified against tclsh 9.0.3: log(0.0) = -Inf
        interp = TclInterp()
        assert interp.eval("expr {log(0.0)}").value == "-Inf"
        assert interp.eval("expr {log10(0.0)}").value == "-Inf"

    def test_math_func_pow_zero_neg_exp(self) -> None:
        # Verified against tclsh 9.0.3: pow(0,-n) = Inf, pow(-0,-odd) = -Inf
        interp = TclInterp()
        assert interp.eval("expr {pow(0.0,-1.0)}").value == "Inf"
        assert interp.eval("expr {pow(0.0,-2.0)}").value == "Inf"
        assert interp.eval("expr {pow(-0.0,-1.0)}").value == "-Inf"

    def test_math_func_pow_zero_neg_inf_exp(self) -> None:
        # IEEE 754: pow(±0, -Inf) = +Inf.  Regression guard: int(-Inf) raises
        # OverflowError in Python, so the sign-check must skip non-finite exps.
        interp = TclInterp()
        assert interp.eval("expr {pow(0.0, -Inf)}").value == "Inf"
        assert interp.eval("expr {pow(-0.0, -Inf)}").value == "Inf"
        assert interp.eval("expr {pow(0.0, -1.0/0.0)}").value == "Inf"

    def test_math_func_ceil_floor_inf(self) -> None:
        # Verified against tclsh 9.0.3: ceil/floor of ±Inf returns ±Inf
        interp = TclInterp()
        assert interp.eval("expr {ceil(1.0/0.0)}").value == "Inf"
        assert interp.eval("expr {floor(-1.0/0.0)}").value == "-Inf"
        assert interp.eval("expr {ceil(-1.0/0.0)}").value == "-Inf"
        assert interp.eval("expr {floor(1.0/0.0)}").value == "Inf"

    def test_math_func_isinf_isfinite(self) -> None:
        # Verified against tclsh 9.0.3
        interp = TclInterp()
        assert interp.eval("expr {isinf(1.0/0.0)}").value == "1"
        assert interp.eval("expr {isinf(1.0)}").value == "0"
        assert interp.eval("expr {isfinite(1.0/0.0)}").value == "0"
        assert interp.eval("expr {isfinite(1.0)}").value == "1"
        assert interp.eval("expr {isfinite(42)}").value == "1"

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
        from compiler.codegen.bytecode import format_module_asm
        from tooling.vm.compiler import compile_script

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
        from compiler.codegen.bytecode import format_module_asm
        from tooling.vm.compiler import compile_script

        module_asm, _ = compile_script("set x ${a(1)}")
        text = format_module_asm(module_asm)
        assert "loadStk" in text
        assert "loadArrayStk" not in text
        assert '"a(1)"' in text

    def test_unbraced_array_ref_uses_array_load(self) -> None:
        from compiler.codegen.bytecode import format_module_asm
        from tooling.vm.compiler import compile_script

        module_asm, _ = compile_script("set x $a(1)")
        text = format_module_asm(module_asm)
        assert "loadArrayStk" in text
        assert '"a"' in text
        assert '"1"' in text

    def test_namespaced_array_forms_distinguish_scalar_like_vs_array_ref(self) -> None:
        from compiler.codegen.bytecode import format_module_asm
        from tooling.vm.compiler import compile_script

        scalar_like, _ = compile_script("set x ${::ns::arr(k)}")
        array_ref, _ = compile_script("set x $::ns::arr(k)")
        scalar_text = format_module_asm(scalar_like)
        array_text = format_module_asm(array_ref)
        assert "loadStk" in scalar_text and "loadArrayStk" not in scalar_text
        assert "loadArrayStk" in array_text
        assert '"::ns::arr(k)"' in scalar_text
        assert '"::ns::arr"' in array_text
        assert '"k"' in array_text


class TestStringIsDouble:
    """Tests for ``string is double`` — verified against tclsh 9.0.3."""

    def test_non_numeric_strings_return_zero(self) -> None:
        # Verified against tclsh 9.0.3: string is double abc = 0
        interp = TclInterp()
        assert interp.eval("string is double abc").value == "0"
        assert interp.eval("string is double {1.2.3}").value == "0"
        assert interp.eval("string is double hello").value == "0"

    def test_valid_doubles_return_one(self) -> None:
        # Verified against tclsh 9.0.3
        interp = TclInterp()
        assert interp.eval("string is double 3.14").value == "1"
        assert interp.eval("string is double 42").value == "1"
        assert interp.eval("string is double 1e5").value == "1"
        assert interp.eval("string is double {-3.14}").value == "1"

    def test_negative_zero_is_valid_double(self) -> None:
        # Verified against tclsh 9.0.3: string is double -0.0 = 1
        interp = TclInterp()
        assert interp.eval("string is double {-0.0}").value == "1"
        assert interp.eval("string is double 0.0").value == "1"

    def test_inf_and_nan_are_valid_doubles(self) -> None:
        # Verified against tclsh 9.0.3: Inf, -Inf, NaN are valid doubles
        interp = TclInterp()
        assert interp.eval("string is double Inf").value == "1"
        assert interp.eval("string is double {-Inf}").value == "1"
        assert interp.eval("string is double NaN").value == "1"

    def test_empty_string_nonstrict_is_one(self) -> None:
        # Verified against tclsh 9.0.3: non-strict empty string = 1
        interp = TclInterp()
        assert interp.eval("string is double {}").value == "1"

    def test_strict_empty_string_is_zero(self) -> None:
        # Verified against tclsh 9.0.3: strict empty string = 0
        interp = TclInterp()
        assert interp.eval("string is double -strict {}").value == "0"


class TestScanFloatSpecials:
    """Tests for ``scan`` with %f format — verified against tclsh 9.0.3."""

    def test_scan_normal_float(self) -> None:
        # Verified against tclsh 9.0.3
        interp = TclInterp()
        assert interp.eval("scan 3.14 %f").value == "3.14"
        assert interp.eval("scan 42.0 %f").value == "42.0"

    def test_scan_inf(self) -> None:
        # Verified against tclsh 9.0.3: scan Inf %f = Inf
        interp = TclInterp()
        assert interp.eval("scan Inf %f").value == "Inf"
        assert interp.eval("scan {-Inf} %f").value == "-Inf"

    def test_scan_inf_lowercase(self) -> None:
        # Verified against tclsh 9.0.3: scan accepts lowercase inf
        interp = TclInterp()
        assert interp.eval("scan inf %f").value == "Inf"
        assert interp.eval("scan {-inf} %f").value == "-Inf"

    def test_scan_nan(self) -> None:
        # Verified against tclsh 9.0.3: scan NaN %f = NaN
        interp = TclInterp()
        assert interp.eval("scan NaN %f").value == "NaN"

    def test_scan_with_variable(self) -> None:
        # Verified against tclsh 9.0.3: scan into variable, returns count
        interp = TclInterp()
        assert interp.eval("scan Inf %f x").value == "1"
        assert interp.eval("set x").value == "Inf"
        assert interp.eval("scan {-Inf} %f y").value == "1"
        assert interp.eval("set y").value == "-Inf"


class TestExprInfLiteral:
    """Tests for Inf/-Inf/NaN as expression literals — verified against tclsh 9.0.3."""

    def test_inf_literal(self) -> None:
        # Verified against tclsh 9.0.3: expr {Inf} = Inf
        interp = TclInterp()
        assert interp.eval("expr {Inf}").value == "Inf"
        assert interp.eval("expr {-Inf}").value == "-Inf"

    def test_inf_arithmetic(self) -> None:
        # Verified against tclsh 9.0.3
        interp = TclInterp()
        assert interp.eval("expr {Inf+0}").value == "Inf"
        assert interp.eval("expr {Inf*2}").value == "Inf"
        assert interp.eval("expr {-Inf*2}").value == "-Inf"
        assert interp.eval("expr {Inf + 1.0}").value == "Inf"
        assert interp.eval("expr {-Inf - 1.0}").value == "-Inf"

    def test_inf_comparisons(self) -> None:
        # Verified against tclsh 9.0.3
        interp = TclInterp()
        assert interp.eval("expr {Inf==Inf}").value == "1"
        assert interp.eval("expr {Inf > 1e308}").value == "1"
        assert interp.eval("expr {-Inf < -1e308}").value == "1"
        assert interp.eval("expr {Inf != -Inf}").value == "1"

    def test_inf_isinf(self) -> None:
        # Verified against tclsh 9.0.3: isinf(Inf) = 1
        interp = TclInterp()
        assert interp.eval("expr {isinf(Inf)}").value == "1"
        assert interp.eval("expr {isinf(-Inf)}").value == "1"
        assert interp.eval("expr {isfinite(Inf)}").value == "0"

    def test_nan_domain_error_in_ops(self) -> None:
        # Verified against tclsh 9.0.3: NaN-producing ops raise ARITH DOMAIN
        interp = TclInterp()
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {Inf - Inf}")
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {Inf * 0.0}")


class TestMathFuncIntegerOverflow:
    """Tests for wide/entier/int with Inf — verified against tclsh 9.0.3."""

    def test_wide_inf_raises_ioverflow(self) -> None:
        # Verified against tclsh 9.0.3: wide(Inf) = integer value too large
        interp = TclInterp()
        with pytest.raises(TclError, match="integer value too large to represent"):
            interp.eval("expr {wide(Inf)}")
        with pytest.raises(TclError, match="integer value too large to represent"):
            interp.eval("expr {wide(-Inf)}")

    def test_entier_inf_raises_ioverflow(self) -> None:
        # Verified against tclsh 9.0.3: entier(Inf) = integer value too large
        interp = TclInterp()
        with pytest.raises(TclError, match="integer value too large to represent"):
            interp.eval("expr {entier(Inf)}")
        with pytest.raises(TclError, match="integer value too large to represent"):
            interp.eval("expr {entier(-Inf)}")

    def test_int_inf_raises_ioverflow(self) -> None:
        # Verified against tclsh 9.0.3: int(Inf) = integer value too large
        interp = TclInterp()
        with pytest.raises(TclError, match="integer value too large to represent"):
            interp.eval("expr {int(Inf)}")
        with pytest.raises(TclError, match="integer value too large to represent"):
            interp.eval("expr {int(-Inf)}")

    def test_wide_normal_float_truncates(self) -> None:
        # Verified against tclsh 9.0.3: wide(3.7) = 3 (truncates toward zero)
        interp = TclInterp()
        assert interp.eval("expr {wide(3.7)}").value == "3"
        assert interp.eval("expr {wide(-3.7)}").value == "-3"

    def test_entier_normal_float_truncates(self) -> None:
        # Verified against tclsh 9.0.3: entier truncates toward zero
        interp = TclInterp()
        assert interp.eval("expr {entier(3.7)}").value == "3"
        assert interp.eval("expr {entier(-3.7)}").value == "-3"

    def test_int_normal_float_truncates(self) -> None:
        # Verified against tclsh 9.0.3: int() truncates toward zero
        interp = TclInterp()
        assert interp.eval("expr {int(3.7)}").value == "3"
        assert interp.eval("expr {int(-3.7)}").value == "-3"
