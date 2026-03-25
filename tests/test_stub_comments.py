"""Tests for dialect stub comment parsing."""

from pathlib import Path

from core.analysis.semantic_model import ProcArgTrait, Range
from core.analysis.stub_comments import (
    is_stubs_begin,
    is_stubs_end,
    parse_expr_stub_line,
    parse_stub_line,
    parse_stubs_file,
)

_ZERO = Range.zero()


class TestParseStubLine:
    def test_simple_command(self):
        stub = parse_stub_line("# tcl-lsp: stub my_cmd {arg1 arg2}", _ZERO)
        assert stub is not None
        assert stub.name == "my_cmd"
        assert len(stub.args) == 2
        assert stub.args[0].name == "arg1"
        assert stub.args[0].role == "value"
        assert stub.args[1].name == "arg2"

    def test_with_roles(self):
        stub = parse_stub_line(
            "# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop",
            _ZERO,
        )
        assert stub is not None
        assert stub.name == "foreach_in_collection"
        assert stub.args[0].role == "var"
        assert stub.args[1].role == "value"
        assert stub.args[2].role == "body"
        assert stub.loop is True
        assert stub.barrier is False

    def test_all_flags(self):
        stub = parse_stub_line(
            "# tcl-lsp: stub dangerous {script:body} -barrier -unsafe -mutator -scope_alias",
            _ZERO,
        )
        assert stub is not None
        assert stub.barrier is True
        assert stub.unsafe is True
        assert stub.mutator is True
        assert stub.scope_alias is True
        assert stub.pure is False
        assert stub.loop is False

    def test_optional_args(self):
        stub = parse_stub_line(
            "# tcl-lsp: stub redirect {?-file? target body:body}",
            _ZERO,
        )
        assert stub is not None
        assert stub.args[0].optional is True
        assert stub.args[0].name == "-file"
        assert stub.args[1].optional is False

    def test_bare_format(self):
        stub = parse_stub_line("stub get_cells {pattern:pattern} -pure", _ZERO)
        assert stub is not None
        assert stub.name == "get_cells"
        assert stub.pure is True

    def test_invalid_role_returns_none(self):
        stub = parse_stub_line("# tcl-lsp: stub bad {arg:invalid_role}", _ZERO)
        assert stub is None

    def test_not_a_stub_returns_none(self):
        stub = parse_stub_line("# just a regular comment", _ZERO)
        assert stub is None

    def test_empty_args(self):
        stub = parse_stub_line("# tcl-lsp: stub no_args {} -pure", _ZERO)
        assert stub is not None
        assert len(stub.args) == 0


class TestParseExprStubLine:
    def test_expr_func(self):
        stub = parse_expr_stub_line("# tcl-lsp: stub expr-func sizeof 1", _ZERO)
        assert stub is not None
        assert stub.name == "sizeof"
        assert stub.kind == "function"
        assert stub.arity == 1

    def test_expr_op(self):
        stub = parse_expr_stub_line("# tcl-lsp: stub expr-op contains 2", _ZERO)
        assert stub is not None
        assert stub.name == "contains"
        assert stub.kind == "operator"
        assert stub.arity == 2

    def test_default_arity(self):
        stub = parse_expr_stub_line("stub expr-func myfunc", _ZERO)
        assert stub is not None
        assert stub.arity == 1

    def test_default_op_arity(self):
        stub = parse_expr_stub_line("stub expr-op myop", _ZERO)
        assert stub is not None
        assert stub.arity == 2

    def test_not_expr_stub(self):
        stub = parse_expr_stub_line("stub get_cells {pattern:pattern}", _ZERO)
        assert stub is None


class TestStubsBlockMarkers:
    def test_stubs_begin(self):
        assert is_stubs_begin("# tcl-lsp: stubs-begin")
        assert is_stubs_begin("#  tcl-lsp:  stubs-begin")
        assert not is_stubs_begin("# tcl-lsp: stub my_cmd {}")

    def test_stubs_end(self):
        assert is_stubs_end("# tcl-lsp: stubs-end")
        assert is_stubs_end("#  tcl-lsp:  stubs-end")
        assert not is_stubs_end("# tcl-lsp: stubs-begin")


class TestParseStubsFile:
    def test_parse_file(self, tmp_path: Path):
        stub_file = tmp_path / "test.tcl.stubs"
        stub_file.write_text(
            "# Comment line\n"
            "stub foreach_in_collection {varName:var collection body:body} -loop\n"
            "stub get_cells {?-filter? pattern:pattern} -pure\n"
            "\n"
            "stub expr-func sizeof 1\n"
            "stub expr-op contains 2\n"
        )
        cmd_stubs, expr_stubs = parse_stubs_file(stub_file)
        assert len(cmd_stubs) == 2
        assert cmd_stubs[0].name == "foreach_in_collection"
        assert cmd_stubs[1].name == "get_cells"
        assert len(expr_stubs) == 2
        assert expr_stubs[0].name == "sizeof"
        assert expr_stubs[1].name == "contains"

    def test_missing_file(self, tmp_path: Path):
        cmd_stubs, expr_stubs = parse_stubs_file(tmp_path / "nonexistent.tcl.stubs")
        assert cmd_stubs == []
        assert expr_stubs == []


class TestAnalyserIntegration:
    def test_inline_stubs_parsed(self):
        from core.analysis.analyser import Analyser

        source = (
            "# tcl-lsp: stubs-begin\n"
            "# tcl-lsp: stub my_cmd {arg1:var body:body} -loop\n"
            "# tcl-lsp: stub expr-func sizeof 1\n"
            "# tcl-lsp: stubs-end\n"
            "set x 1\n"
        )
        analyser = Analyser()
        analyser.analyse(source)
        assert len(analyser.result.stub_commands) == 1
        assert analyser.result.stub_commands[0].name == "my_cmd"
        assert len(analyser.result.stub_expr_defs) == 1
        assert analyser.result.stub_expr_defs[0].name == "sizeof"

    def test_stub_outside_block_ignored(self):
        from core.analysis.analyser import Analyser

        source = "# tcl-lsp: stub my_cmd {arg1 arg2}\nset x 1\n"
        analyser = Analyser()
        analyser.analyse(source)
        assert len(analyser.result.stub_commands) == 0

    def test_multiple_stubs_blocks(self):
        from core.analysis.analyser import Analyser

        source = (
            "# tcl-lsp: stubs-begin\n"
            "# tcl-lsp: stub cmd_a {arg1}\n"
            "# tcl-lsp: stubs-end\n"
            "set x 1\n"
            "# tcl-lsp: stubs-begin\n"
            "# tcl-lsp: stub cmd_b {arg1 arg2}\n"
            "# tcl-lsp: stubs-end\n"
            "set y 2\n"
        )
        analyser = Analyser()
        analyser.analyse(source)
        assert len(analyser.result.stub_commands) == 2
        names = {s.name for s in analyser.result.stub_commands}
        assert names == {"cmd_a", "cmd_b"}

    def test_proc_arg_traits_populated(self):
        from core.analysis.analyser import Analyser

        source = (
            "proc safe_incr {varName {amount 1}} {\n"
            "    upvar 1 $varName var\n"
            "    incr var $amount\n"
            "}\n"
        )
        analyser = Analyser()
        analyser.analyse(source)
        proc = analyser.result.all_procs.get("::safe_incr")
        assert proc is not None
        assert ProcArgTrait.VAR_WRITE in proc.param_traits.get("varName", frozenset())
