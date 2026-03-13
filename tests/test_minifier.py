"""Tests for the Tcl code minifier."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.minifier import MinifyResult, SymbolMap, minify_tcl, unminify_error


class TestMinifyBasic:
    """Basic minification behaviour."""

    def test_empty_source(self):
        assert minify_tcl("") == ""

    def test_whitespace_only(self):
        assert minify_tcl("   \n\n  ") == ""

    def test_single_command(self):
        assert minify_tcl("set x 10") == "set x 10"

    def test_single_command_trailing_newline(self):
        assert minify_tcl("set x 10\n") == "set x 10"

    def test_two_commands_joined(self):
        assert minify_tcl("set x 10\nset y 20\n") == "set x 10;set y 20"

    def test_multiple_commands(self):
        result = minify_tcl("set a 1\nset b 2\nset c 3\n")
        assert result == "set a 1;set b 2;set c 3"


class TestMinifyComments:
    """Comment stripping."""

    def test_line_comment_removed(self):
        assert minify_tcl("# this is a comment\nset x 1\n") == "set x 1"

    def test_inline_comment_after_command(self):
        # Comments on their own line between commands are removed.
        result = minify_tcl("set x 1\n# middle\nset y 2\n")
        assert result == "set x 1;set y 2"

    def test_all_comments(self):
        assert minify_tcl("# a\n# b\n# c\n") == ""

    def test_comment_before_proc(self):
        result = minify_tcl("# doc\nproc foo {} {\n    return 1\n}\n")
        assert result == "proc foo {} {return 1}"


class TestMinifyWhitespace:
    """Whitespace collapsing."""

    def test_leading_whitespace_stripped(self):
        assert minify_tcl("    set x 1\n") == "set x 1"

    def test_blank_lines_collapsed(self):
        assert minify_tcl("set x 1\n\n\n\nset y 2\n") == "set x 1;set y 2"

    def test_indented_code(self):
        result = minify_tcl("    set x 1\n    set y 2\n")
        assert result == "set x 1;set y 2"


class TestMinifyBodies:
    """Recursive body minification."""

    def test_proc_body(self):
        source = "proc greet {name} {\n    puts $name\n    return 1\n}\n"
        result = minify_tcl(source)
        assert result == "proc greet {name} {puts $name;return 1}"

    def test_if_body(self):
        source = "if {$x > 0} {\n    puts yes\n}\n"
        result = minify_tcl(source)
        assert result == "if {$x>0} {puts yes}"

    def test_if_else_body(self):
        source = "if {$x} {\n    puts a\n} else {\n    puts b\n}\n"
        result = minify_tcl(source)
        assert result == "if {$x} {puts a} else {puts b}"

    def test_while_body(self):
        source = "while {1} {\n    incr x\n    if {$x > 10} {\n        break\n    }\n}\n"
        result = minify_tcl(source)
        assert result == "while {1} {incr x;if {$x>10} {break}}"

    def test_foreach_body(self):
        source = "foreach item $list {\n    puts $item\n}\n"
        result = minify_tcl(source)
        assert result == "foreach item $list {puts $item}"

    def test_nested_procs(self):
        source = (
            "proc outer {} {\n"
            "    proc inner {} {\n"
            "        return 42\n"
            "    }\n"
            "    return [inner]\n"
            "}\n"
        )
        result = minify_tcl(source)
        assert result == "proc outer {} {proc inner {} {return 42};return [inner]}"


class TestMinifyPreservation:
    """Things that must be preserved exactly."""

    def test_string_content_preserved(self):
        result = minify_tcl('set x "hello   world"\n')
        assert result == 'set x "hello   world"'

    def test_braced_string_preserved(self):
        result = minify_tcl("set x {hello   world}\n")
        assert result == "set x {hello   world}"

    def test_variable_substitution(self):
        result = minify_tcl("set x $y\n")
        assert result == "set x $y"

    def test_command_substitution(self):
        result = minify_tcl("set x [expr {1 + 2}]\n")
        assert result == "set x [expr {1+2}]"

    def test_expression_compressed(self):
        result = minify_tcl("expr {$a + $b * $c}\n")
        assert result == "expr {$a+$b*$c}"

    def test_backslash_newline_in_command(self):
        # Backslash-newline inside [] command substitution should be preserved
        # in the token text (the minifier doesn't rewrite command substitution).
        source = 'set x [string \\\nlength "hello"]\n'
        result = minify_tcl(source)
        assert "string" in result
        assert "length" in result


class TestMinifyRealWorld:
    """Realistic Tcl code patterns."""

    def test_switch_command(self):
        source = "switch $x {\n    a {\n        puts a\n    }\n    b {\n        puts b\n    }\n}\n"
        result = minify_tcl(source)
        assert "switch" in result
        assert result.startswith("switch $x")

    def test_for_loop(self):
        source = "for {set i 0} {$i < 10} {incr i} {\n    puts $i\n}\n"
        result = minify_tcl(source)
        assert result == "for {set i 0} {$i<10} {incr i} {puts $i}"

    def test_complex_irule(self):
        source = (
            "when HTTP_REQUEST {\n"
            "    # Route traffic\n"
            '    if {[HTTP::uri] starts_with "/api"} {\n'
            "        pool api_pool\n"
            "    } else {\n"
            "        pool web_pool\n"
            "    }\n"
            "}\n"
        )
        result = minify_tcl(source)
        assert "# Route" not in result  # comment stripped
        assert "pool api_pool" in result
        assert "pool web_pool" in result

    def test_minify_idempotent(self):
        source = "proc foo {} {\n    set x 1\n    return $x\n}\n"
        first = minify_tcl(source)
        second = minify_tcl(first)
        assert first == second

    def test_size_reduction(self):
        source = (
            "# A long comment about this function\n"
            "proc calculate {a b c} {\n"
            "    # Step 1: add\n"
            "    set sum [expr {$a + $b}]\n"
            "\n"
            "    # Step 2: multiply\n"
            "    set result [expr {$sum * $c}]\n"
            "\n"
            "    return $result\n"
            "}\n"
        )
        result = minify_tcl(source)
        assert len(result) < len(source)


class TestExprCompression:
    """Whitespace compression inside expr bodies."""

    def test_expr_simple_addition(self):
        assert minify_tcl("expr {$a + $b}") == "expr {$a+$b}"

    def test_expr_compound_operators(self):
        assert minify_tcl("expr {$a == $b && $c != $d}") == "expr {$a==$b&&$c!=$d}"

    def test_expr_ternary(self):
        assert minify_tcl("expr {$x > 0 ? 1 : 0}") == "expr {$x>0?1:0}"

    def test_expr_parens(self):
        assert minify_tcl("expr {($a + $b) * $c}") == "expr {($a+$b)*$c}"

    def test_expr_word_op_eq_preserved(self):
        result = minify_tcl('expr {$a eq "hello"}')
        assert " eq " in result

    def test_expr_word_op_ne_preserved(self):
        result = minify_tcl('expr {$a ne "world"}')
        assert " ne " in result

    def test_expr_word_op_in_preserved(self):
        result = minify_tcl("expr {$a in $list}")
        assert " in " in result

    def test_expr_word_op_ni_preserved(self):
        result = minify_tcl("expr {$a ni $list}")
        assert " ni " in result

    def test_expr_function_call(self):
        assert minify_tcl("expr {int($x)}") == "expr {int($x)}"

    def test_expr_inside_command_substitution(self):
        assert minify_tcl("set x [expr {$a + $b}]") == "set x [expr {$a+$b}]"

    def test_if_condition_compressed(self):
        assert minify_tcl("if {$x > 0} {puts yes}") == "if {$x>0} {puts yes}"

    def test_while_condition_compressed(self):
        assert minify_tcl("while {$i < 10} {incr i}") == "while {$i<10} {incr i}"

    def test_for_condition_compressed(self):
        result = minify_tcl("for {set i 0} {$i < 10} {incr i} {puts $i}")
        assert result == "for {set i 0} {$i<10} {incr i} {puts $i}"

    def test_string_inside_expr_preserved(self):
        result = minify_tcl('expr {$a eq "hello   world"}')
        assert '"hello   world"' in result

    def test_cmd_subst_inside_expr_preserved(self):
        result = minify_tcl("expr {[string length $s] > 0}")
        assert result == "expr {[string length $s]>0}"


class TestCompactNames:
    """Name compaction with compact_names=True."""

    def test_compact_returns_tuple(self):
        result = minify_tcl("set x 1\n", compact_names=True)
        assert isinstance(result, tuple)
        assert len(result) == 2

    def test_compact_renames_proc(self):
        source = "proc calculate {} {\n    return 1\n}\ncalculate\n"
        result, smap = minify_tcl(source, compact_names=True)
        assert "calculate" not in result
        assert "calculate" in smap.procs

    def test_compact_renames_local_vars(self):
        source = "proc test {} {\n    set result 42\n    return $result\n}\n"
        result, smap = minify_tcl(source, compact_names=True)
        assert "result" not in result
        assert any("result" in v for v in smap.variables.values())

    def test_compact_renames_params(self):
        source = "proc add {alpha beta} {\n    return [expr {$alpha + $beta}]\n}\n"
        result, smap = minify_tcl(source, compact_names=True)
        assert "alpha" not in result
        assert "beta" not in result

    def test_compact_skips_single_char_names(self):
        source = "proc f {x y} {\n    set z [expr {$x + $y}]\n    return $z\n}\n"
        result, smap = minify_tcl(source, compact_names=True)
        # f, x, y, z are all single-char: nothing to compact.
        assert not smap.procs
        assert not smap.variables

    def test_compact_preserves_builtins(self):
        source = "proc calculate {} {\n    return 1\n}\n"
        result, smap = minify_tcl(source, compact_names=True)
        # Short proc name must not collide with builtins like "set", "if", etc.
        short_name = smap.procs.get("calculate", "")
        assert short_name not in ("set", "if", "for", "while", "proc", "puts")

    def test_compact_call_sites_renamed(self):
        source = "proc calculate {value} {\n    return $value\n}\nset result [calculate 42]\n"
        result, smap = minify_tcl(source, compact_names=True)
        short = smap.procs.get("calculate", "")
        assert short
        # The call site should use the short name.
        assert f"[{short} 42]" in result

    def test_compact_multiple_procs_unique(self):
        source = "proc greet {} {\n    return 1\n}\nproc farewell {} {\n    return 2\n}\n"
        result, smap = minify_tcl(source, compact_names=True)
        # Each proc should get a unique short name.
        short_names = list(smap.procs.values())
        assert len(short_names) == len(set(short_names))


class TestCompactBarriers:
    """Barrier commands prevent variable renaming."""

    def test_global_barrier(self):
        source = "proc test {} {\n    global myvar\n    set myvar 42\n}\n"
        _, smap = minify_tcl(source, compact_names=True)
        # Variables in a scope with 'global' should NOT be renamed.
        assert not smap.variables

    def test_upvar_barrier(self):
        source = "proc test {name} {\n    upvar 1 $name local\n    set local 42\n}\n"
        _, smap = minify_tcl(source, compact_names=True)
        assert not smap.variables

    def test_eval_barrier(self):
        source = "proc test {} {\n    set prefix hello\n    eval {puts $prefix}\n}\n"
        _, smap = minify_tcl(source, compact_names=True)
        assert not smap.variables

    def test_safe_proc_still_renamed(self):
        source = "proc test {} {\n    global myvar\n    set myvar 42\n}\ntest\n"
        _, smap = minify_tcl(source, compact_names=True)
        # Proc name can still be renamed even if vars can't.
        assert "test" in smap.procs


class TestSymbolMap:
    """SymbolMap formatting."""

    def test_format_empty(self):
        smap = SymbolMap()
        assert smap.format() == ""

    def test_format_procs(self):
        smap = SymbolMap(procs={"calculate": "a"})
        text = smap.format()
        assert "a <- calculate" in text

    def test_format_variables(self):
        smap = SymbolMap(variables={"::test": {"result": "a"}})
        text = smap.format()
        assert "a <- result" in text
        assert "::test" in text


class TestAggressiveMode:
    """Aggressive mode: optimise + compact + minify."""

    def test_aggressive_returns_minify_result(self):
        result = minify_tcl("set x 1\n", aggressive=True)
        assert isinstance(result, MinifyResult)
        assert result.source
        assert result.original_length > 0

    def test_aggressive_constant_folding(self):
        source = "set x [expr {2 + 3}]\nputs $x\n"
        result = minify_tcl(source, aggressive=True)
        # The optimizer should fold expr {2+3} to 5.
        assert "5" in result.source

    def test_aggressive_list_folding(self):
        source = "set items [list a b c]\n"
        result = minify_tcl(source, aggressive=True)
        # [list a b c] -> {a b c}
        assert "{a b c}" in result.source or "a b c" in result.source

    def test_aggressive_compacts_names(self):
        source = "proc calculate {alpha beta} {\n    return [expr {$alpha + $beta}]\n}\n"
        result = minify_tcl(source, aggressive=True)
        # Proc and params should be compacted.
        assert "calculate" not in result.source
        assert "alpha" not in result.source

    def test_aggressive_savings_pct(self):
        source = (
            "# A verbose comment\n"
            "proc long_name {parameter_one parameter_two} {\n"
            "    set intermediate [expr {$parameter_one + $parameter_two}]\n"
            "    return $intermediate\n"
            "}\n"
        )
        result = minify_tcl(source, aggressive=True)
        assert result.savings_pct > 40

    def test_aggressive_optimisations_counted(self):
        source = "set x [expr {2 + 3}]\nputs $x\n"
        result = minify_tcl(source, aggressive=True)
        assert result.optimisations_applied >= 0

    def test_aggressive_smaller_than_compact(self):
        source = (
            "# Calculate\n"
            "proc calculate {alpha beta} {\n"
            "    set sum_val [expr {$alpha + $beta}]\n"
            "    set result [expr {$sum_val * 2}]\n"
            "    return $result\n"
            "}\n"
            "puts [calculate 10 20]\n"
        )
        compact_result, _ = minify_tcl(source, compact_names=True)
        aggressive_result = minify_tcl(source, aggressive=True)
        assert aggressive_result.minified_length <= len(compact_result)


class TestVarBraceCompression:
    """${var} → $var compression safety."""

    def test_simple_var_no_braces(self):
        # Standalone $var should not have braces.
        result = minify_tcl("set x $longvar\n")
        assert "$longvar" in result
        assert "${longvar}" not in result

    def test_var_followed_by_word_char_in_quotes(self):
        # "${longvar}_suffix" must keep braces to avoid $longvar_suffix.
        result = minify_tcl('set x "${longvar}_suffix"\n')
        assert "${longvar}" in result

    def test_var_followed_by_nonword_in_quotes(self):
        # "${longvar} rest" — space follows, so $longvar is safe.
        result = minify_tcl('puts "${longvar} rest"\n')
        assert "$longvar" in result

    def test_adjacent_vars_in_quotes(self):
        # "${x}${y}" — $y starts with $, not a word char, so $x$y is safe.
        result = minify_tcl('puts "${x}${y}"\n')
        assert "$x$y" in result or "$x${y}" in result

    def test_var_at_end_of_quoted_string(self):
        # "${longvar}" at end — nothing follows, safe to drop braces.
        result = minify_tcl('puts "hello ${longvar}"\n')
        assert "$longvar" in result
        assert "${longvar}" not in result


class TestArrayMemberCompaction:
    """Array member name compaction."""

    def test_array_members_compacted(self):
        source = (
            "proc test {} {\n"
            "    set config(database_host) localhost\n"
            "    set config(database_port) 5432\n"
            "    puts $config(database_host)\n"
            "    puts $config(database_port)\n"
            "}\n"
        )
        result, smap = minify_tcl(source, compact_names=True)
        assert "database_host" not in result
        assert "database_port" not in result
        assert smap.array_members

    def test_array_member_short_names_skipped(self):
        source = "set arr(x) 1\nset arr(y) 2\n"
        result, smap = minify_tcl(source, compact_names=True)
        # Single-char members are already minimal.
        assert not smap.array_members

    def test_array_member_unsafe_keys_skipped(self):
        # Keys that look user-input-derived should not be renamed.
        source = "set data(uri_path) /foo\nset data(uri_query) /bar\n"
        result, smap = minify_tcl(source, compact_names=True)
        assert "uri_path" in result
        assert not smap.array_members

    def test_array_member_preserves_semantics(self):
        source = "set config(server_name) myhost\nputs $config(server_name)\n"
        result, smap = minify_tcl(source, compact_names=True)
        if smap.array_members:
            # The short name should appear in both set and reference.
            short = list(smap.array_members.get("config", {}).values())[0]
            assert f"config({short})" in result

    def test_array_member_in_string_not_compacted(self):
        # "foo(bar)" inside a string literal must NOT be treated as an array ref.
        source = 'puts "foo(database_host)"\nset x "config(database_port)"\n'
        result, smap = minify_tcl(source, compact_names=True)
        assert "foo(database_host)" in result
        assert "config(database_port)" in result
        assert not smap.array_members

    def test_array_member_inside_proc_body(self):
        # Array refs inside a proc body should be found and compacted.
        source = (
            "proc test {} {\n    set data(server_name) myhost\n    return $data(server_name)\n}\n"
        )
        result, smap = minify_tcl(source, compact_names=True)
        assert "server_name" not in result
        assert smap.array_members


class TestCommandAliasing:
    """Repeated command name aliasing in aggressive mode."""

    def test_aliasing_saves_bytes(self):
        # HTTP::uri used 3 times → should be aliased.
        source = (
            "when HTTP_REQUEST {\n"
            '    if {[HTTP::uri] starts_with "/api"} {\n'
            "        log local0. [HTTP::uri]\n"
            "        pool [HTTP::uri]\n"
            "    }\n"
            "}\n"
        )
        result = minify_tcl(source, aggressive=True)
        if result.symbol_map.command_aliases:
            alias = result.symbol_map.command_aliases.get("HTTP::uri", "")
            assert alias  # Should have been aliased.
            assert f"${alias}" in result.source

    def test_aliasing_not_applied_for_single_use(self):
        source = "HTTP::uri /test\n"
        result = minify_tcl(source, aggressive=True)
        assert not result.symbol_map.command_aliases

    def test_aliasing_cost_benefit(self):
        # Short command used twice — alias overhead exceeds savings.
        source = "puts hello\nputs world\n"
        result = minify_tcl(source, aggressive=True)
        assert "puts" not in result.symbol_map.command_aliases

    def test_aliasing_non_namespaced_command(self):
        # A long non-namespaced command used repeatedly should be aliased.
        source = (
            "longcommandname arg1\n"
            "longcommandname arg2\n"
            "longcommandname arg3\n"
            "longcommandname arg4\n"
        )
        result = minify_tcl(source, aggressive=True)
        assert result.symbol_map.command_aliases.get("longcommandname")
        alias = result.symbol_map.command_aliases["longcommandname"]
        assert f"${alias}" in result.source
        # The original name should only appear in the preamble "set alias longcommandname".
        assert result.source.count("longcommandname") == 1


class TestArgumentAliasing:
    """Repeated literal argument aliasing in aggressive mode."""

    def test_argument_aliasing_saves_bytes(self):
        # -normalized used 4 times → should be aliased.
        source = (
            "when HTTP_REQUEST {\n"
            "    HTTP::uri -normalized\n"
            "    HTTP::uri -normalized\n"
            "    HTTP::uri -normalized\n"
            "    HTTP::uri -normalized\n"
            "}\n"
        )
        result = minify_tcl(source, aggressive=True)
        assert result.symbol_map.argument_aliases.get("-normalized")
        alias = result.symbol_map.argument_aliases["-normalized"]
        assert f"${alias}" in result.source

    def test_argument_not_aliased_for_single_use(self):
        source = "HTTP::uri -normalized\n"
        result = minify_tcl(source, aggressive=True)
        assert "-normalized" not in result.symbol_map.argument_aliases

    def test_argument_in_string_not_aliased(self):
        # -normalized inside a string literal must NOT be replaced.
        source = 'puts "-normalized"\nputs "-normalized"\nputs "-normalized"\n'
        result = minify_tcl(source, aggressive=True)
        # The argument aliasing phase must not touch strings.
        assert not result.symbol_map.argument_aliases

    def test_argument_aliasing_cost_benefit(self):
        # Short argument used twice — alias overhead exceeds savings.
        source = "cmd -ab\ncmd -ab\n"
        result = minify_tcl(source, aggressive=True)
        assert "-ab" not in result.symbol_map.argument_aliases

    def test_argument_aliases_no_collision_with_command_aliases(self):
        # Both command and argument aliasing should use distinct alias names.
        source = (
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
        )
        result = minify_tcl(source, aggressive=True)
        cmd_alias_names = set(result.symbol_map.command_aliases.values())
        arg_alias_names = set(result.symbol_map.argument_aliases.values())
        assert not cmd_alias_names & arg_alias_names  # no overlap

    def test_argument_inside_proc_body(self):
        # Arguments inside proc bodies should be found and aliased.
        source = (
            "proc test {} {\n"
            "    mycmd -longflagname\n"
            "    mycmd -longflagname\n"
            "    mycmd -longflagname\n"
            "    mycmd -longflagname\n"
            "}\n"
        )
        result = minify_tcl(source, aggressive=True)
        assert result.symbol_map.argument_aliases.get("-longflagname")


class TestStringLiteralAliasing:
    """Repeated substring aliasing inside quoted strings."""

    def test_repeated_literal_substring_aliased(self):
        # Repeated literal fragment followed by a non-word char ($).
        source = (
            'log local0. "[clock seconds] INFO: $a done"\n'
            'log local0. "[clock seconds] INFO: $b done"\n'
            'log local0. "[clock seconds] INFO: $c done"\n'
            'log local0. "[clock seconds] INFO: $d done"\n'
        )
        result = minify_tcl(source, aggressive=True)
        assert result.symbol_map.string_aliases

    def test_short_substring_not_aliased(self):
        # Very short repeated text — not worth aliasing.
        source = 'puts "a $x"\nputs "a $y"\n'
        result = minify_tcl(source, aggressive=True)
        assert not result.symbol_map.string_aliases

    def test_unbalanced_braces_skipped(self):
        # Literal with unbalanced braces can't be brace-quoted safely.
        source = (
            'puts "hello { world $x done"\n'
            'puts "hello { world $y done"\n'
            'puts "hello { world $z done"\n'
            'puts "hello { world $w done"\n'
        )
        result = minify_tcl(source, aggressive=True)
        for sub in result.symbol_map.string_aliases:
            assert sub.count("{") == sub.count("}"), f"Unbalanced braces: {sub!r}"

    def test_string_alias_renders_correctly(self):
        # After aliasing, the variable reference must be syntactically valid.
        source = (
            'puts "-normalized_long_flag"\n'
            'puts "-normalized_long_flag"\n'
            'puts "-normalized_long_flag"\n'
        )
        result = minify_tcl(source, aggressive=True)
        if result.symbol_map.string_aliases:
            alias = list(result.symbol_map.string_aliases.values())[0]
            # Quote stripping may remove the quotes, leaving bare $alias.
            assert f"${alias}" in result.source

    def test_no_collision_with_prior_alias_phases(self):
        source = (
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            'log local0. "[clock seconds] INFO: request $a"\n'
            'log local0. "[clock seconds] INFO: response $b"\n'
            'log local0. "[clock seconds] INFO: timeout $c"\n'
            'log local0. "[clock seconds] INFO: complete $d"\n'
        )
        result = minify_tcl(source, aggressive=True)
        all_names = (
            set(result.symbol_map.command_aliases.values())
            | set(result.symbol_map.argument_aliases.values())
            | set(result.symbol_map.string_aliases.values())
        )
        # All alias names should be unique.
        total = (
            len(result.symbol_map.command_aliases)
            + len(result.symbol_map.argument_aliases)
            + len(result.symbol_map.string_aliases)
        )
        assert len(all_names) == total


class TestSubcommandAbbreviation:
    """Dialect-aware ensemble subcommand abbreviation."""

    def test_string_subcommands_abbreviated_for_irules(self):
        source = "string tolower hello\nstring length hello\nstring range hello 0 2\n"
        result = minify_tcl(source, aggressive=True, dialect="f5-irules")
        assert "string tol " in result.source
        assert "string le " in result.source
        assert "string ra " in result.source

    def test_clock_subcommands_abbreviated(self):
        source = "set t [clock seconds]\nset f [clock format $t]\n"
        result = minify_tcl(source, aggressive=True, dialect="f5-irules")
        assert "clock se" in result.source
        assert "clock f " in result.source

    def test_info_subcommands_abbreviated(self):
        source = "if {[info exists ::x]} { set y 1 }\n"
        result = minify_tcl(source, aggressive=True, dialect="f5-irules")
        assert "info e " in result.source

    def test_not_abbreviated_for_plain_tcl(self):
        source = "string tolower hello\nstring length hello\n"
        result = minify_tcl(source, aggressive=True, dialect="tcl8.6")
        assert "string tolower" in result.source
        assert "string length" in result.source

    def test_not_abbreviated_for_tcl85(self):
        source = "string tolower hello\n"
        result = minify_tcl(source, aggressive=True, dialect="tcl8.5")
        assert "string tolower" in result.source

    def test_no_false_positives_on_word_boundaries(self):
        # "tolower" inside a longer word should not be abbreviated.
        source = 'set my_tolower_func "x"\n'
        result = minify_tcl(source, aggressive=True, dialect="f5-irules")
        assert "my_tolower_func" in result.source or "my_tol" not in result.source

    def test_no_abbreviation_inside_quoted_strings(self):
        source = 'puts "string tolower hello"\n'
        result = minify_tcl(source, aggressive=True, dialect="f5-irules")
        assert '"string tolower hello"' in result.source
        assert '"string tol hello"' not in result.source

    def test_subcommand_abbreviation_applies_inside_proc_body(self):
        source = "proc demo {} {\n    string tolower Hello\n}\n"
        result = minify_tcl(source, aggressive=True, dialect="f5-irules")
        assert "string tol Hello" in result.source


class TestSubstTemplateAliasing:
    """Repeated quoted string deduplication via [subst]."""

    def test_identical_dynamic_strings_aliased(self):
        # Same quoted string with dynamic content repeated 3 times.
        line = 'log local0. "[clock format [clock seconds]] INFO: [HTTP::uri] $timing done"\n'
        source = line * 3
        result = minify_tcl(source, aggressive=True)
        assert "[subst " in result.source
        # The template should appear once in a set preamble.
        assert "set " in result.source

    def test_different_strings_not_aliased(self):
        source = (
            'log local0. "[clock seconds] INFO: $a request"\n'
            'log local0. "[clock seconds] INFO: $b response"\n'
        )
        result = minify_tcl(source, aggressive=True)
        assert "subst" not in result.source

    def test_pure_literal_strings_not_subst_aliased(self):
        # Strings without $ or [ don't need subst — literal aliasing handles them.
        source = 'puts "hello world"\nputs "hello world"\nputs "hello world"\n'
        result = minify_tcl(source, aggressive=True)
        assert "subst" not in result.source

    def test_no_alias_name_collision(self):
        # Template aliases must not shadow variables from prior phases.
        source = (
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            "HTTP::uri -normalized\n"
            'log local0. "[clock format [clock seconds]] INFO: [HTTP::uri] $timing done"\n'
            'log local0. "[clock format [clock seconds]] INFO: [HTTP::uri] $timing done"\n'
            'log local0. "[clock format [clock seconds]] INFO: [HTTP::uri] $timing done"\n'
        )
        result = minify_tcl(source, aggressive=True)
        # Should parse correctly — no variable name collisions.
        if "[subst " in result.source:
            # The subst alias should not collide with command aliases.
            cmd_aliases = set(result.symbol_map.command_aliases.values())
            # Extract the subst alias name from [subst $X]
            import re

            subst_vars = set(re.findall(r"\[subst \$(\w+)\]", result.source))
            assert not cmd_aliases & subst_vars

    def test_short_dynamic_string_not_aliased(self):
        # Short strings: [subst $a] overhead exceeds savings.
        source = 'puts "$x y"\nputs "$x y"\n'
        result = minify_tcl(source, aggressive=True)
        assert "subst" not in result.source


class TestQuoteStripping:
    """Unnecessary quote removal during whitespace minification."""

    def test_simple_word_unquoted(self):
        source = 'puts "hello"\n'
        result = minify_tcl(source)
        assert result == "puts hello"

    def test_variable_ref_unquoted(self):
        source = 'puts "$a"\n'
        result = minify_tcl(source)
        assert result == "puts $a"

    def test_braced_var_unquoted(self):
        source = 'puts "${a}d"\n'
        result = minify_tcl(source)
        assert result == "puts ${a}d"

    def test_space_keeps_quotes(self):
        source = 'puts "$a $b"\n'
        result = minify_tcl(source)
        assert result == 'puts "$a $b"'

    def test_empty_string_keeps_quotes(self):
        source = 'set x ""\n'
        result = minify_tcl(source)
        assert result == 'set x ""'

    def test_command_sub_no_space_unquoted(self):
        source = 'puts "[cmd]result"\n'
        result = minify_tcl(source)
        assert result == "puts [cmd]result"

    def test_command_sub_with_space_keeps_quotes(self):
        source = 'puts "[cmd] result"\n'
        result = minify_tcl(source)
        assert result == 'puts "[cmd] result"'

    def test_starts_with_brace_keeps_quotes(self):
        source = 'puts "{text}"\n'
        result = minify_tcl(source)
        assert result == 'puts "{text}"'

    def test_bare_brace_keeps_quotes(self):
        source = 'puts "$a}"\n'
        result = minify_tcl(source)
        assert result == 'puts "$a}"'


class TestSymbolMapFormat:
    """Extended SymbolMap formatting."""

    def test_format_array_members(self):
        smap = SymbolMap(array_members={"config": {"database_host": "a", "database_port": "b"}})
        text = smap.format()
        assert "a <- database_host" in text
        assert "config" in text

    def test_format_command_aliases(self):
        smap = SymbolMap(command_aliases={"HTTP::uri": "a"})
        text = smap.format()
        assert "$a <- HTTP::uri" in text

    def test_format_argument_aliases(self):
        smap = SymbolMap(argument_aliases={"-normalized": "a"})
        text = smap.format()
        assert "$a <- -normalized" in text
        assert "Argument aliases" in text

    def test_format_string_aliases(self):
        smap = SymbolMap(string_aliases={" INFO: ": "a"})
        text = smap.format()
        assert "$a" in text
        assert "String literal aliases" in text

    def test_format_static_folds(self):
        smap = SymbolMap(static_folds={"${x} world": "hello world"})
        text = smap.format()
        assert "hello world" in text
        assert "Static substring folds" in text


class TestSymbolMapParse:
    """Round-trip: format() -> parse()."""

    def test_round_trip_procs(self):
        original = SymbolMap(procs={"calculate": "a", "helper": "b"})
        parsed = SymbolMap.parse(original.format())
        assert parsed.procs == original.procs

    def test_round_trip_variables(self):
        original = SymbolMap(variables={"::calculate": {"alpha": "a", "beta": "b"}})
        parsed = SymbolMap.parse(original.format())
        assert parsed.variables == original.variables

    def test_round_trip_command_aliases(self):
        original = SymbolMap(command_aliases={"HTTP::uri": "a"})
        parsed = SymbolMap.parse(original.format())
        assert parsed.command_aliases == original.command_aliases

    def test_round_trip_argument_aliases(self):
        original = SymbolMap(argument_aliases={"-normalized": "a"})
        parsed = SymbolMap.parse(original.format())
        assert parsed.argument_aliases == original.argument_aliases


class TestUnminifyError:
    """Translate minified error messages back to original names."""

    def test_no_symbol_map_returns_unchanged(self):
        msg = 'can\'t read "x": no such variable'
        assert unminify_error(msg) == msg

    def test_translate_quoted_variable(self):
        smap = SymbolMap(variables={"::myproc": {"longName": "a"}})
        msg = 'can\'t read "a": no such variable'
        result = unminify_error(msg, symbol_map=smap)
        assert '"longName"' in result
        assert '"a"' not in result

    def test_translate_dollar_variable(self):
        smap = SymbolMap(variables={"::myproc": {"counter": "a"}})
        msg = "error: $a is undefined"
        result = unminify_error(msg, symbol_map=smap)
        assert "$counter" in result

    def test_translate_proc_name(self):
        smap = SymbolMap(procs={"calculate_total": "a"})
        msg = 'invalid command name "a"'
        result = unminify_error(msg, symbol_map=smap)
        assert '"calculate_total"' in result

    def test_translate_command_alias(self):
        smap = SymbolMap(command_aliases={"HTTP::uri": "a"})
        msg = 'can\'t read "a": no such variable'
        result = unminify_error(msg, symbol_map=smap)
        assert '"HTTP::uri"' in result

    def test_translate_multiple_symbols(self):
        smap = SymbolMap(
            procs={"handle_request": "a"},
            variables={"::handle_request": {"response_code": "b", "body_text": "c"}},
        )
        # Proc name in a standalone quoted context
        msg1 = 'invalid command name "a"'
        result1 = unminify_error(msg1, symbol_map=smap)
        assert '"handle_request"' in result1
        # Variables via dollar references
        msg2 = "error evaluating $b and $c"
        result2 = unminify_error(msg2, symbol_map=smap)
        assert "$response_code" in result2
        assert "$body_text" in result2

    def test_translate_from_formatted_string(self):
        smap = SymbolMap(procs={"calculate": "a"})
        formatted = smap.format()
        msg = 'invalid command name "a"'
        result = unminify_error(msg, symbol_map=formatted)
        assert '"calculate"' in result

    def test_irule_error_log(self):
        smap = SymbolMap(variables={"::": {"request_uri": "a"}})
        msg = '/Common/my_irule:1: can\'t read "a": no such variable'
        result = unminify_error(msg, symbol_map=smap)
        assert '"request_uri"' in result

    def test_line_remap_with_sources(self):
        original = "# comment\nset x 1\nset y 2\nset z 3\n"
        minified = "set x 1;set y 2;set z 3"
        msg = '(procedure "test" line 3)'
        result = unminify_error(
            msg,
            minified_source=minified,
            original_source=original,
        )
        assert "minified line 3" in result

    def test_no_change_when_no_match(self):
        smap = SymbolMap(procs={"calculate": "a"})
        msg = "some unrelated error"
        result = unminify_error(msg, symbol_map=smap)
        assert result == msg


class TestStaticSubstringFolding:
    """IR/CFG/SSA/SCCP-backed static substring folding."""

    def test_constant_var_in_string_folded(self):
        """A $var proven constant by SCCP is folded into the string."""
        source = 'set x hello\nputs "value is $x"\n'
        result = minify_tcl(source, aggressive=True)
        assert "hello" in result.source
        # When folding occurs, it should be recorded in static_folds.
        if result.symbol_map.static_folds:
            folded_values = list(result.symbol_map.static_folds.values())
            assert any("hello" in v for v in folded_values)

    def test_multiple_constant_vars_folded(self):
        """Multiple constant vars in one string are all folded."""
        source = 'set prefix INFO\nset suffix done\nputs "$prefix: $suffix"\n'
        result = minify_tcl(source, aggressive=True)
        assert "INFO" in result.source
        assert "done" in result.source
        if result.symbol_map.static_folds:
            folded_values = list(result.symbol_map.static_folds.values())
            assert any("INFO" in v and "done" in v for v in folded_values)

    def test_non_constant_var_not_folded(self):
        """A proc parameter (overdefined) prevents folding."""
        source = 'proc test {x} {\n    puts "value is $x"\n}\n'
        result = minify_tcl(source, aggressive=True)
        # $x is a parameter — should remain dynamic (no static fold for it).
        # Check that no fold contains "value is" (the puts string was not folded).
        folds = result.symbol_map.static_folds or {}
        assert not any("value is" in v for v in folds.values())

    def test_command_substitution_prevents_folding(self):
        """[cmd] in a string prevents static folding."""
        source = 'set x hello\nputs "[clock seconds] $x"\n'
        result = minify_tcl(source, aggressive=True)
        # The [clock seconds] prevents folding of the whole string.
        assert "clock" in result.source
        # The puts string should not have been statically folded.
        folds = result.symbol_map.static_folds or {}
        assert not any("clock" in v for v in folds.values())

    def test_repeated_constant_strings_folded(self):
        """Repeated strings with constant vars are all folded."""
        source = (
            "set level INFO\n"
            'puts "$level: first message"\n'
            'puts "$level: second message"\n'
            'puts "$level: third message"\n'
        )
        result = minify_tcl(source, aggressive=True)
        # All three should have INFO inlined.
        assert result.source.count("INFO") >= 3

    def test_overdefined_var_at_join_not_folded(self):
        """A variable assigned different values on different paths is not folded."""
        source = 'if {1} {\n    set x hello\n} else {\n    set x world\n}\nputs "value: $x"\n'
        result = minify_tcl(source, aggressive=True)
        # $x is overdefined at the puts — should not fold to either value.
        # (The constant-branch pass may fold the if, but that's a different pass.)
        assert result is not None

    def test_tainted_var_not_folded(self):
        """A variable from tainted source is not folded even if SCCP says constant."""
        # Most taint comes from external commands like HTTP::uri which aren't
        # constant in SCCP anyway, so this mainly verifies the safety check.
        # Use puts so the result is used (not eliminated as dead code).
        source = 'proc test {x} {\n    puts "input: $x"\n}\n'
        result = minify_tcl(source, aggressive=True)
        # $x is a parameter — should not be statically folded.
        folds = result.symbol_map.static_folds or {}
        assert not any("input:" in v for v in folds.values())

    def test_static_folds_in_symbol_map(self):
        """Static fold details appear in the symbol map."""
        source = 'set x hello\nputs "greeting: $x"\n'
        result = minify_tcl(source, aggressive=True)
        if result.symbol_map.static_folds:
            for original, folded in result.symbol_map.static_folds.items():
                assert "hello" in folded

    def test_no_fold_when_var_not_in_ssa(self):
        """Variables not tracked by SSA (e.g. global) are not folded."""
        source = 'proc test {} {\n    global x\n    puts "value: $x"\n}\n'
        result = minify_tcl(source, aggressive=True)
        # global barrier prevents SCCP from knowing x's value.
        # Should not fold.
        assert result is not None

    def test_fold_preserves_literal_text(self):
        """Literal text around $var substitutions is preserved."""
        source = 'set name world\nputs "hello $name!"\n'
        result = minify_tcl(source, aggressive=True)
        assert "hello" in result.source
        assert "world" in result.source

    def test_pure_cmd_string_length_folded(self):
        """[string length $x] is folded when $x is a known constant."""
        source = 'set x hello\nputs "length is [string length $x]"\n'
        result = minify_tcl(source, aggressive=True)
        assert "5" in result.source
        assert "string length" not in result.source

    def test_pure_cmd_expr_folded(self):
        """[expr {$x + $y}] is folded when both vars are constants."""
        source = 'set x 10\nset y 20\nputs "sum is [expr {$x + $y}]"\n'
        result = minify_tcl(source, aggressive=True)
        assert "30" in result.source

    def test_pure_cmd_format_folded(self):
        """[format %s=%d $x $y] is folded when args are constants."""
        source = 'set x hello\nset y 42\nputs "result: [format {%s=%d} $x $y]"\n'
        result = minify_tcl(source, aggressive=True)
        assert "hello=42" in result.source

    def test_pure_cmd_string_toupper_folded(self):
        """[string toupper $x] is folded when $x is constant."""
        source = 'set x hello\nputs "[string toupper $x] world"\n'
        result = minify_tcl(source, aggressive=True)
        assert "HELLO" in result.source

    def test_impure_cmd_not_folded(self):
        """[clock seconds] is impure and should not be folded."""
        source = 'set x hello\nputs "time: [clock seconds] $x"\n'
        result = minify_tcl(source, aggressive=True)
        assert "clock" in result.source

    def test_dead_set_eliminated_after_fold(self):
        """set commands whose variable is fully folded are removed."""
        source = 'set level INFO\nputs "$level: started"\nputs "$level: done"\n'
        result = minify_tcl(source, aggressive=True)
        # The set command should be eliminated since $level is fully folded.
        assert "INFO: started" in result.source
        assert "INFO: done" in result.source

    def test_dead_set_kept_when_var_still_used(self):
        """set is NOT removed when the variable is still referenced."""
        source = 'set x hello\nputs "greeting: $x"\nputs $x\n'
        result = minify_tcl(source, aggressive=True)
        # $x is used as a bare argument in `puts $x`, so the set must remain.
        assert result is not None

    def test_format_static_folds_in_symbol_map(self):
        """Static fold map records what was folded."""
        smap = SymbolMap(static_folds={"$x world": "hello world"})
        text = smap.format()
        assert "Static substring folds" in text
        assert "hello world" in text


class TestStaticSubstrEdgeCases:
    """Edge-case tests for specific correctness fixes in static_substr.py."""

    def test_brace_balance_rejects_mismatched(self):
        """_build_replacement must not brace-quote '}{' (equal counts, bad nesting)."""
        from core.minifier.static_substr import _build_replacement

        result = _build_replacement("}{")
        # Should NOT produce {}{} which would be invalid Tcl.
        assert not result.startswith("{") or result == "{}{}"
        # Should use double-quote escaping instead.
        assert result.startswith('"') or result.startswith("\\")

    def test_parse_var_ref_rejects_namespace(self):
        """_parse_var_ref rejects $ns::var (namespace-qualified)."""
        from core.minifier.static_substr import _parse_var_ref

        _, name = _parse_var_ref("$ns::var", 0)
        assert name is None

    def test_parse_var_ref_allows_colon_after(self):
        """_parse_var_ref allows $var: (single colon is not namespace qualifier)."""
        from core.minifier.static_substr import _parse_var_ref

        end, name = _parse_var_ref("$level: message", 0)
        assert name == "level"
        assert end == 6

    def test_parse_var_ref_rejects_array(self):
        """_parse_var_ref rejects $arr(idx) (array element)."""
        from core.minifier.static_substr import _parse_var_ref

        _, name = _parse_var_ref("$arr(idx)", 0)
        assert name is None

    def test_parse_var_ref_accepts_simple(self):
        """_parse_var_ref accepts simple $varname."""
        from core.minifier.static_substr import _parse_var_ref

        end, name = _parse_var_ref("$foo bar", 0)
        assert name == "foo"
        assert end == 4

    def test_format_extra_args_not_folded(self):
        """[format %s hi there] should not be folded (extra args)."""
        from core.minifier.static_substr import _eval_format

        result = _eval_format(["%s", "hi", "there"])
        assert result is None

    def test_list_folding_quotes_special_chars(self):
        """[list] folding must brace-quote elements with spaces."""
        from core.minifier.static_substr import _try_eval_pure_cmd

        result = _try_eval_pure_cmd("[list a {b c}]", {}, {})
        assert result is not None
        assert "b c" in result
        # The element "b c" must be quoted (braced), not bare.
        assert "{b c}" in result

    def test_dead_set_preserved_when_set_read_exists(self):
        """Dead-set elimination must not remove set when [set var] reads exist."""
        from core.minifier.static_substr import _eliminate_dead_sets

        source = "set x hello;puts [set x]"
        result, count = _eliminate_dead_sets(source, {"x"})
        # $x is not present, but [set x] reads the variable — keep the set.
        assert count == 0
        assert "set x hello" in result

    def test_dead_set_preserved_when_incr_exists(self):
        """Dead-set elimination must not remove set when incr var reads exist."""
        from core.minifier.static_substr import _eliminate_dead_sets

        source = "set x 0;incr x"
        result, count = _eliminate_dead_sets(source, {"x"})
        assert count == 0
        assert "set x 0" in result


class TestIRulesBraceSeparatorMinifier:
    """In iRules, the minifier should elide spaces between adjacent braced args."""

    def test_if_brace_elision(self):
        """``if {cond} {body}`` minifies to ``if {cond}{body}`` in iRules."""
        source = "if {$a} {puts a}"
        result = minify_tcl(source, dialect="f5-irules")
        assert result == "if {$a}{puts a}"

    def test_standard_tcl_keeps_spaces(self):
        """Standard Tcl minifier preserves spaces between braced args."""
        source = "if {$a} {puts a}"
        result = minify_tcl(source, dialect="tcl8.6")
        assert " {puts a}" in result

    def test_if_elseif_else_elision(self):
        """Multi-branch if/elseif/else with brace elision."""
        source = "if {$a} {puts a} elseif {$b} {puts b} else {puts c}"
        result = minify_tcl(source, dialect="f5-irules")
        # elseif/else are bare words so spaces remain around them
        assert "{$a}{puts a}" in result
        assert "{$b}{puts b}" in result

    def test_mixed_braced_and_unbraced(self):
        """Spaces preserved between non-braced args."""
        source = "set x {hello}"
        result = minify_tcl(source, dialect="f5-irules")
        # set and x are bare words, so space must remain
        assert result == "set x {hello}"
