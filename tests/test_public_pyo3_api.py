"""Acceptance tests for the designed public PyO3 surface (track API-PYO3).

These exercise the ``tcl_lsp_py`` facades — ``parse_tcl`` / ``compile_tcl``
/ ``analyse_tcl`` / ``format_tcl`` / ``parse_bigip_config`` / ``query_bigip``
— and the typed error hierarchy rooted at ``TclLspError``.

The public surface is the *terminal* product of the Rust rewrite: a small,
semver-stable API for downstream embedders, deliberately distinct from the
legacy soft-dependency shims the in-tree Python still imports. It only
exists once the native wheel is built, so the whole module ``importorskip``s
``tcl_lsp_py`` and is a no-op on a fresh clone without ``make rust-build``
(the same soft-dependency philosophy the rest of the bridge uses). PR CI
builds the wheel and runs this file against it.
"""

from __future__ import annotations

import pytest

t = pytest.importorskip("tcl_lsp_py")


SAMPLE_TCL = 'proc greet {name} {\n  puts "hi $name"\n}\nset x 1\n'

SAMPLE_CONF = """ltm pool /Common/p1 {
    members {
        /Common/n1:80 {
            address 1.2.3.4
        }
    }
}
ltm virtual /Common/vs1 {
    destination /Common/1.1.1.1:443
    pool /Common/p1
}
"""


# --------------------------------------------------------------------------
# Error hierarchy
# --------------------------------------------------------------------------

EXCEPTIONS = (
    "TclLspError",
    "TclParseError",
    "TclCompileError",
    "TclAnalysisError",
    "BigipParseError",
    "BigipQueryError",
    "UnsupportedFeatureError",
)


@pytest.mark.parametrize("name", EXCEPTIONS)
def test_exception_type_exported(name: str) -> None:
    assert hasattr(t, name)
    assert issubclass(getattr(t, name), Exception)


@pytest.mark.parametrize(
    "name",
    [n for n in EXCEPTIONS if n != "TclLspError"],
)
def test_exceptions_subclass_base(name: str) -> None:
    assert issubclass(getattr(t, name), t.TclLspError)


# --------------------------------------------------------------------------
# parse_tcl
# --------------------------------------------------------------------------


def test_parse_tcl_tokens_and_commands() -> None:
    result = t.parse_tcl(SAMPLE_TCL)
    assert "ParseResult" in repr(result)
    assert len(result.tokens) > 0
    names = {cmd.name for cmd in result.commands}
    assert {"proc", "set"} <= names
    assert isinstance(result.warnings, list)

    tok = result.tokens[0]
    assert isinstance(tok.kind, str)
    assert isinstance(tok.text, str)
    assert isinstance(tok.start, tuple) and len(tok.start) == 2
    assert isinstance(tok.byte_range, tuple) and len(tok.byte_range) == 2


def test_parse_tcl_dialect_gating() -> None:
    # ``{*}`` expansion is 8.5+; it lexes to an EXPAND token under 9.0 but
    # is just a literal word under 8.4.
    nine = t.parse_tcl("puts {*}$args\n", dialect="tcl9.0")
    eight_four = t.parse_tcl("puts {*}$args\n", dialect="tcl8.4")
    assert any(tk.kind == "EXPAND" for tk in nine.tokens)
    assert not any(tk.kind == "EXPAND" for tk in eight_four.tokens)


# --------------------------------------------------------------------------
# compile_tcl
# --------------------------------------------------------------------------


def test_compile_tcl_returns_unit() -> None:
    unit = t.compile_tcl("proc a {} {return 1}\nproc b {} {return 2}\n")
    assert "CompilationUnit" in repr(unit)
    assert unit.procedure_count == 2
    assert unit.proc_names == sorted(unit.proc_names)
    assert len(unit.proc_names) == 2
    assert unit.has_interprocedural is True


def test_compile_tcl_skip_interprocedural() -> None:
    unit = t.compile_tcl("proc a {} {}", interprocedural=False)
    assert unit.has_interprocedural is False


def test_compile_tcl_irules_dialect_loads_when_handler() -> None:
    # The iRules dialect must build a *dialect-aware* registry so the `when`
    # event handler lowers to a `::when::*` procedure. Regression: the facade
    # previously used the plain default registry, which never loaded the iRules
    # specs (no `::when::*`) and also reported Tcl 8.x octal numeric semantics
    # for the tcl9.0 default (CommandRegistry.leading_zero_is_octal).
    unit = t.compile_tcl("when HTTP_REQUEST {\n  set x 1\n}\n", dialect="f5-irules")
    assert any(name.startswith("::when::") for name in unit.proc_names), unit.proc_names


# --------------------------------------------------------------------------
# analyse_tcl
# --------------------------------------------------------------------------


def test_analyse_tcl_symbols_and_diagnostics() -> None:
    result = t.analyse_tcl(SAMPLE_TCL)
    assert "AnalysisResult" in repr(result)
    assert any(p.endswith("greet") for p in result.procs)
    assert isinstance(result.diagnostics, list)
    assert isinstance(result.classes, list)
    assert isinstance(result.variables, list)
    for diag in result.diagnostics:
        assert isinstance(diag.code, str)
        assert diag.severity in ("error", "warning", "hint")
        assert isinstance(diag.start, tuple) and len(diag.start) == 2


def test_analyse_tcl_irules_dialect() -> None:
    # Dialect-gated analysis should accept the iRules dialect without error.
    result = t.analyse_tcl(
        'when HTTP_REQUEST {\n  pool /Common/p1\n}\n', dialect="f5-irules"
    )
    assert isinstance(result.diagnostics, list)


# --------------------------------------------------------------------------
# format_tcl
# --------------------------------------------------------------------------


def test_format_tcl_default() -> None:
    out = t.format_tcl("proc  x { }  {\nputs hi\n}\n")
    assert isinstance(out, str) and out


def test_format_tcl_with_options() -> None:
    opts = t.FormatOptions(indent_size=2, max_line_length=100)
    assert opts.indent_size == 2
    assert opts.max_line_length == 100
    out = t.format_tcl("proc x {} {\nputs hi\n}\n", options=opts)
    assert isinstance(out, str)


def test_format_options_rejects_unknown_indent_style() -> None:
    with pytest.raises(t.UnsupportedFeatureError):
        t.FormatOptions(indent_style="curly")


# --------------------------------------------------------------------------
# parse_bigip_config
# --------------------------------------------------------------------------


def test_parse_bigip_config() -> None:
    config = t.parse_bigip_config(SAMPLE_CONF)
    assert "BigipConfig" in repr(config)
    assert config.object_count >= 2
    assert len(config.object_keys) == config.object_count
    assert config.default_partition == "Common"
    assert config.to_json().lstrip().startswith("{")


def test_parse_bigip_config_strict_ok_on_valid() -> None:
    config = t.parse_bigip_config(SAMPLE_CONF, strict=True)
    assert config.object_count >= 2


def test_parse_bigip_config_strict_raises_on_garbage() -> None:
    with pytest.raises(t.BigipParseError) as excinfo:
        t.parse_bigip_config(
            "this is not bigip config", strict=True, uri="file:///x.conf"
        )
    err = excinfo.value
    assert err.code == "BIGIP_PARSE"
    assert err.uri == "file:///x.conf"
    assert isinstance(err, t.TclLspError)


# --------------------------------------------------------------------------
# query_bigip
# --------------------------------------------------------------------------


def test_query_bigip_read() -> None:
    result = t.query_bigip([("conf1", SAMPLE_CONF)], ".ltm.pool[] | .name")
    assert "QueryResult" in repr(result)
    assert result.has_mutation is False
    assert len(result.values) == 1
    assert result.values[0].uri == "conf1"
    assert "p1" in result.values[0].output


def test_query_bigip_json_output() -> None:
    result = t.query_bigip(
        [("conf1", SAMPLE_CONF)], ".ltm.pool[] | .name", output="json"
    )
    assert isinstance(result.values[0].output, str)


def test_query_bigip_mutation() -> None:
    result = t.query_bigip(
        [("conf1", SAMPLE_CONF)], '.ltm.pool["/Common/p1"].name = "/Common/p2"'
    )
    assert result.has_mutation is True
    assert len(result.edits) == 1
    assert result.edits[0].uri == "conf1"
    assert result.edits[0].changed is True
    assert "p2" in result.edits[0].new_source


def test_query_bigip_unknown_output_mode() -> None:
    with pytest.raises(t.UnsupportedFeatureError) as excinfo:
        t.query_bigip([("c", SAMPLE_CONF)], ".ltm.pool[]", output="nonsense")
    assert excinfo.value.code == "UNSUPPORTED_FEATURE"


def test_query_bigip_malformed_query() -> None:
    with pytest.raises(t.BigipQueryError) as excinfo:
        t.query_bigip([("c", SAMPLE_CONF)], ".ltm.pool[ %% bad")
    err = excinfo.value
    assert str(err.code).startswith("BIGIP_QUERY")
    assert isinstance(err.message, str)
    # Positional (lex/parse) errors carry a resolved range.
    if err.code in ("BIGIP_QUERY_LEX", "BIGIP_QUERY_PARSE"):
        assert err.range is not None
