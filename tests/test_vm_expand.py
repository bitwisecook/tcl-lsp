"""Tests for {*} expansion in the Tcl VM.

Based on basic-47.x and basic-48.x from the C Tcl 9.0.3 test suite.
"""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestExpansionBasic:
    """basic-47.x: basic expansion tests."""

    def test_47_2_error_during_expansion(self) -> None:
        """Expansion of malformed list raises error."""
        interp = TclInterp()
        with pytest.raises(TclError, match="unmatched open brace"):
            interp.eval(r"list {*}\{")

    def test_47_4_no_expansion(self) -> None:
        """{*} followed by separator is a braced string '*', not expansion."""
        interp = TclInterp()
        result = interp.eval("list {*} {*}\t{*}")
        assert result.value == "* * *"

    def test_47_5_expansion_mixed(self) -> None:
        """Mixed expansion and non-expansion."""
        interp = TclInterp()
        result = interp.eval('list {*}{} {*}\t{*}x {*}"y z"')
        assert result.value == "* x y z"

    def test_47_6_expansion_to_zero_args(self) -> None:
        """{*}{} expands to zero arguments."""
        interp = TclInterp()
        result = interp.eval("list {*}{}")
        assert result.value == ""

    def test_47_7_expansion_to_one_arg(self) -> None:
        """{*}x expands to one argument."""
        interp = TclInterp()
        result = interp.eval("list {*}x")
        assert result.value == "x"

    def test_47_8_expansion_to_many_args(self) -> None:
        """{*}"y z" expands to multiple arguments."""
        interp = TclInterp()
        result = interp.eval('list {*}"y z"')
        assert result.value == "y z"

    def test_47_9_expansion_subst_order(self) -> None:
        """Substitution order with expansion is left-to-right."""
        interp = TclInterp()
        result = interp.eval(
            "set x 0; list [incr x] {*}[incr x] [incr x] {*}[list [incr x] [incr x]] [incr x]"
        )
        assert result.value == "1 2 3 4 5 6"

    def test_47_10_expand_empty_with_many_args(self) -> None:
        """Expansion of empty list with many following arguments."""
        interp = TclInterp()
        result = interp.eval("concat {*}{} a b c d e f g h i j k l m n o p q r")
        assert result.value == "a b c d e f g h i j k l m n o p q r"

    def test_47_11_expand_single_with_many_args(self) -> None:
        """Expansion of single element with many following arguments."""
        interp = TclInterp()
        result = interp.eval("concat {*}1 a b c d e f g h i j k l m n o p q r")
        assert result.value == "1 a b c d e f g h i j k l m n o p q r"

    def test_47_12_expand_two_with_many_args(self) -> None:
        """Expansion of two-element list with many following arguments."""
        interp = TclInterp()
        result = interp.eval("concat {*}{1 2} a b c d e f g h i j k l m n o p q r")
        assert result.value == "1 2 a b c d e f g h i j k l m n o p q r"

    def test_47_13_multiple_expansions_with_many_args(self) -> None:
        """Multiple expansions with many following arguments."""
        interp = TclInterp()
        result = interp.eval("concat {*}{} {*}{1 2} a b c d e f g h i j k l m n o p q")
        assert result.value == "1 2 a b c d e f g h i j k l m n o p q"


class TestExpansionAdvanced:
    """basic-48.x: advanced expansion tests."""

    def test_48_3_expansion_with_variables(self) -> None:
        """Expansion with variable references."""
        interp = TclInterp()
        interp.eval('set l1 [list a "b b" c d]')
        interp.eval('set l2 [list e f "g g" h]')
        interp.eval("proc l3 {} {return [list i j k {l l}]}")
        result = interp.eval("list {*}$l1 $l2 {*}[l3]")
        assert result.value == "a {b b} c d {e f {g g} h} i j k {l l}"

    def test_48_5_expansion_error_detection(self) -> None:
        """Expansion of malformed list raises error."""
        interp = TclInterp()
        interp.eval('set l "a {a b}x y"')
        with pytest.raises(TclError, match="list element in braces"):
            interp.eval("list x {*}$l")

    def test_48_6_expansion_concatenated_vars(self) -> None:
        """Expansion of concatenated variable values."""
        interp = TclInterp()
        interp.eval('set l1 [list a "b b" c d]')
        interp.eval('set l2 [list e f "g g" h]')
        result = interp.eval("list {*}$l1$l2")
        assert result.value == "a {b b} c de f {g g} h"

    def test_48_8_expansion_text_prefix(self) -> None:
        """Expansion with text prefix before variable."""
        interp = TclInterp()
        interp.eval('set l1 [list a "b b" c d]')
        result = interp.eval("list {*}hej$l1")
        assert result.value == "heja {b b} c d"

    def test_48_9_not_all_trigger(self) -> None:
        r"""Not all {*} in a command should trigger expansion."""
        interp = TclInterp()
        interp.eval('set l1 [list a "b b" c d]')
        interp.eval('set l2 [list e f "g g" h]')
        result = interp.eval(r'list {*}$l1 \{*\}$l2 "{*}$l1" {{*} i j k}')
        assert result.value == ("a {b b} c d {{*}e f {g g} h} {{*}a {b b} c d} {{*} i j k}")

    def test_48_10_expansion_of_command_word(self) -> None:
        """Expansion can produce the command name."""
        interp = TclInterp()
        interp.eval("set cmd [list string range jultomte]")
        result = interp.eval("{*}$cmd 2 6")
        assert result.value == "ltomt"

    def test_48_11_expansion_into_nothing(self) -> None:
        """Expansion of two empty lists produces empty result."""
        interp = TclInterp()
        interp.eval("set cmd {}")
        interp.eval("set bar {}")
        result = interp.eval("{*}$cmd {*}$bar")
        assert result.value == ""

    def test_48_12_expansion_with_quoted_list(self) -> None:
        """Expansion with quoted string."""
        interp = TclInterp()
        interp.eval('set l1 [list a "b b" c d]')
        interp.eval('set l2 [list e f "g g" h]')
        result = interp.eval('list {*}$l1 {*}"hej hopp" {*}$l2')
        assert result.value == "a {b b} c d hej hopp e f {g g} h"

    def test_48_13_expansion_with_braced_list(self) -> None:
        """Expansion with braced list."""
        interp = TclInterp()
        interp.eval('set l1 [list a "b b" c d]')
        interp.eval('set l2 [list e f "g g" h]')
        result = interp.eval("list {*}$l1 {*}{hej hopp} {*}$l2")
        assert result.value == "a {b b} c d hej hopp e f {g g} h"

    def test_48_14_hash_command(self) -> None:
        """Expansion producing '#' as command name is an error."""
        interp = TclInterp()
        interp.eval('set cmd "#"')
        with pytest.raises(TclError, match='invalid command name "#"'):
            interp.eval("{*}$cmd apa bepa")

    def test_48_28_expansion_braced_string(self) -> None:
        """{*}{a b c} expands to three arguments."""
        interp = TclInterp()
        result = interp.eval("list {*}{a b c}")
        assert result.value == "a b c"

    def test_expansion_preserves_list_structure(self) -> None:
        """Expansion preserves list element boundaries."""
        interp = TclInterp()
        result = interp.eval('list {*}{a "b c" d}')
        assert result.value == "a {b c} d"

    def test_expansion_multiple_in_one_command(self) -> None:
        """Multiple expansions in one command."""
        interp = TclInterp()
        interp.eval("set x [list a b]")
        interp.eval("set y [list c d]")
        result = interp.eval("list {*}$x {*}$y")
        assert result.value == "a b c d"

    def test_expansion_with_non_expanded_args(self) -> None:
        """Mix of expanded and non-expanded arguments."""
        interp = TclInterp()
        interp.eval("set x [list b c]")
        result = interp.eval("list a {*}$x d")
        assert result.value == "a b c d"

    def test_expansion_empty_list(self) -> None:
        """Expansion of empty list adds nothing."""
        interp = TclInterp()
        interp.eval("set x {}")
        result = interp.eval("list a {*}$x b")
        assert result.value == "a b"


class TestExpansionBytecode:
    """Verify bytecode matches C Tcl for expansion."""

    def test_bytecode_simple_expand(self) -> None:
        """Bytecode for cmd {*}$x matches C Tcl."""
        from core.compiler.codegen import Op
        from vm.compiler import compile_script

        module_asm, _ = compile_script("cmd {*}$x")
        asm = module_asm.top_level
        instrs = asm.instructions

        # expandStart → push1 "cmd" → push1 "x" → loadStk → expandStkTop 2 → invokeExpanded → done
        ops = [i.op for i in instrs]
        assert ops[0] == Op.EXPAND_START
        assert ops[1] == Op.PUSH1  # "cmd"
        assert ops[2] == Op.PUSH1  # "x"
        assert ops[3] == Op.LOAD_STK
        assert ops[4] == Op.EXPAND_STKTOP
        assert instrs[4].operands[0] == 2  # word count
        assert ops[5] == Op.INVOKE_EXPANDED
        assert ops[6] == Op.DONE

    def test_bytecode_mixed_expand(self) -> None:
        """Bytecode for cmd a {*}$x b matches C Tcl."""
        from core.compiler.codegen import Op
        from vm.compiler import compile_script

        module_asm, _ = compile_script("cmd a {*}$x b")
        asm = module_asm.top_level
        instrs = asm.instructions

        ops = [i.op for i in instrs]
        assert ops[0] == Op.EXPAND_START
        assert ops[1] == Op.PUSH1  # "cmd"
        assert ops[2] == Op.PUSH1  # "a"
        assert ops[3] == Op.PUSH1  # "x"
        assert ops[4] == Op.LOAD_STK
        assert ops[5] == Op.EXPAND_STKTOP
        assert instrs[5].operands[0] == 3  # word count (cmd, a, $x)
        assert ops[6] == Op.PUSH1  # "b"
        assert ops[7] == Op.INVOKE_EXPANDED
        assert ops[8] == Op.DONE

    def test_bytecode_multiple_expand(self) -> None:
        """Bytecode for cmd {*}$a {*}$b matches C Tcl."""
        from core.compiler.codegen import Op
        from vm.compiler import compile_script

        module_asm, _ = compile_script("cmd {*}$a {*}$b")
        asm = module_asm.top_level
        instrs = asm.instructions

        ops = [i.op for i in instrs]
        assert ops[0] == Op.EXPAND_START
        assert ops[1] == Op.PUSH1  # "cmd"
        assert ops[2] == Op.PUSH1  # "a"
        assert ops[3] == Op.LOAD_STK
        assert ops[4] == Op.EXPAND_STKTOP
        assert instrs[4].operands[0] == 2  # first expand: word count = 2
        assert ops[5] == Op.PUSH1  # "b"
        assert ops[6] == Op.LOAD_STK
        assert ops[7] == Op.EXPAND_STKTOP
        assert instrs[7].operands[0] == 3  # second expand: word count = 3
        assert ops[8] == Op.INVOKE_EXPANDED
        assert ops[9] == Op.DONE

    def test_no_expand_bytecode_for_braced_star(self) -> None:
        """cmd {*} (at EOF) should produce normal invokeStk, not expansion."""
        from core.compiler.codegen import Op
        from vm.compiler import compile_script

        module_asm, _ = compile_script("cmd {*}")
        asm = module_asm.top_level
        ops = [i.op for i in asm.instructions]
        assert Op.EXPAND_START not in ops
        assert Op.EXPAND_STKTOP not in ops
        assert Op.INVOKE_EXPANDED not in ops
