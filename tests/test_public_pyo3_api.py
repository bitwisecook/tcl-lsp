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
    result = t.analyse_tcl("when HTTP_REQUEST {\n  pool /Common/p1\n}\n", dialect="f5-irules")
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
        t.parse_bigip_config("this is not bigip config", strict=True, uri="file:///x.conf")
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
    result = t.query_bigip([("conf1", SAMPLE_CONF)], ".ltm.pool[] | .name", output="json")
    assert isinstance(result.values[0].output, str)


def test_query_bigip_mutation() -> None:
    result = t.query_bigip([("conf1", SAMPLE_CONF)], '.ltm.pool["/Common/p1"].name = "/Common/p2"')
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


# --------------------------------------------------------------------------
# Refactoring facades (over tcl-lsp-core::refactor)
# --------------------------------------------------------------------------


def test_refactor_inline_variable() -> None:
    # Cursor on the `x` in the `set x 5` declaration.
    r = t.refactor_inline_variable("set x 5\nputs $x\n", 0, 4)
    assert r is not None
    assert r.title == "Inline variable '$x'"
    assert r.rewritten == "puts 5\n"
    assert r.edit_count == 2
    # Off-target (on `puts`) returns None.
    assert t.refactor_inline_variable("set x 5\nputs $x\n", 1, 0) is None


def test_refactor_if_to_switch() -> None:
    src = 'if {$x eq "a"} {\n  puts one\n} elseif {$x eq "b"} {\n  puts two\n}\n'
    r = t.refactor_if_to_switch(src, 0, 0)
    assert r is not None
    assert r.title == "Convert to switch on $x"
    assert r.rewritten.startswith("switch -exact -")


def test_refactor_extract_variable() -> None:
    r = t.refactor_extract_variable("puts [expr {1 + 2}]\n", 0, 5, 0, 19, "result")
    assert r is not None
    assert r.title == "Extract into variable '$result'"
    assert r.rewritten == "set result [expr {1 + 2}]\nputs $result\n"


def test_refactor_brace_expr() -> None:
    r = t.refactor_brace_expr('expr "$a + $b"\n', 0, 0)
    assert r is not None
    assert r.rewritten == "expr {$a + $b}\n"
    assert t.refactor_brace_expr("expr {$a + $b}\n", 0, 0) is None


def test_refactor_extract_datagroup() -> None:
    src = (
        "switch -exact -- $uri {\n"
        "    /api { set pool api_pool }\n"
        "    /web { set pool web_pool }\n"
        "    /cdn { set pool cdn_pool }\n"
        "}"
    )
    r = t.refactor_extract_datagroup(src, 0, 0, "uri_pool", dialect="f5-irules")
    assert r is not None
    assert r.name == "uri_pool"
    assert r.value_type == "string"
    assert r.records == [("/api", "api_pool"), ("/web", "web_pool"), ("/cdn", "cdn_pool")]
    assert r.rewritten == "set pool [class lookup $uri uri_pool]"
    assert r.data_group_definition.startswith("ltm data-group internal uri_pool {")


# --------------------------------------------------------------------------
# Analysis graphs (over tcl-lsp-core::graphs / ::diagram)
# --------------------------------------------------------------------------


def test_call_graph() -> None:
    src = "proc a {} { b }\nproc b {} {}\na\n"
    g = t.call_graph(src)
    names = {n["name"] for n in g["nodes"]}
    assert "::a" in names and "::b" in names
    assert isinstance(g["edges"], list)
    assert "<top-level>" in g["roots"]


def test_symbol_graph() -> None:
    g = t.symbol_graph("proc greet {name} { set x 1 }\n")
    assert g["summary"]["total_procs"] == 1
    assert isinstance(g["scopes"], list)


def test_dataflow_graph() -> None:
    g = t.dataflow_graph("proc f {} { set x 1 }\n")
    assert "taint_warnings" in g
    assert "proc_effects" in g
    assert "summary" in g


def test_diagram_data() -> None:
    src = "when HTTP_REQUEST {\n  if {[HTTP::uri] eq \"/\"} { pool web }\n}\n"
    d = t.diagram_data(src, dialect="f5-irules")
    assert isinstance(d["events"], list)
    assert isinstance(d["procedures"], list)
    assert any(e["name"] == "HTTP_REQUEST" for e in d["events"])


# --------------------------------------------------------------------------
# Optimiser (over tcl-compiler::optimiser)
# --------------------------------------------------------------------------


def test_optimise() -> None:
    src = "proc f {} {\n    set x [expr {1 + 2}]\n    return $x\n}\n"
    r = t.optimise(src, profile="full")
    assert r["profile"] == "full"
    assert r["multi_pass"] is False
    assert r["iterations"] == 1
    assert r["changed"] is True
    assert isinstance(r["optimizations"], list)
    for o in r["optimizations"]:
        assert set(o) >= {"code", "message", "range", "replacement"}


def test_optimise_aggressive_is_multipass() -> None:
    src = "proc f {} {\n    set x [expr {1 + 2}]\n    set y $x\n    return $y\n}\n"
    r = t.optimise(src, profile="aggressive")
    assert r["multi_pass"] is True
    assert r["iterations"] >= 1


# --------------------------------------------------------------------------
# Docstrings (over tcl-lsp-core::formatting::docstring)
# --------------------------------------------------------------------------


def test_generate_docstring_stub() -> None:
    src = "proc greet {name {greeting hello} args} { return 1 }\n"
    stub = t.generate_docstring_stub(src, "greet")
    assert "# @brief TODO: describe greet" in stub
    assert "# @param name" in stub
    assert "# @param greeting - (default: hello)" in stub
    assert "# @param args - Additional arguments" in stub
    # Plain style differs.
    plain = t.generate_docstring_stub(src, "greet", style="plain")
    assert "# Arguments:" in plain
    # Unknown proc → None.
    assert t.generate_docstring_stub(src, "nope") is None


def test_parse_docstring() -> None:
    info = t.parse_docstring("@brief does x\n@param a - the a\n@param b\n@return sum")
    assert info["brief"] == "does x"
    assert info["params"] == [
        {"name": "a", "description": "the a"},
        {"name": "b", "description": ""},
    ]
    assert info["returns"] == "sum"


# --------------------------------------------------------------------------
# Minify / events (over tcl-lsp-core::minify, tcl-registry::events)
# --------------------------------------------------------------------------


def test_unminify_error() -> None:
    sym = "# Procs\na <- greet\n# Variables in ::greet\nb <- name\n"
    assert t.unminify_error('invalid command name "a"', sym) == 'invalid command name "greet"'
    assert (
        t.unminify_error('can\'t read "b": no such variable', sym)
        == 'can\'t read "name": no such variable'
    )


def test_event_multiplicity() -> None:
    assert t.event_multiplicity("RULE_INIT") == "init"
    assert t.event_multiplicity("CLIENT_ACCEPTED") == "once_per_connection"
    assert t.event_multiplicity("BOGUS") == "unknown"


def test_order_events_for_file() -> None:
    src = "when HTTP_RESPONSE {}\nwhen CLIENT_ACCEPTED {}\nwhen RULE_INIT {}\n"
    ordered = t.order_events_for_file(src)
    assert ordered.index("RULE_INIT") < ordered.index("CLIENT_ACCEPTED")
    assert ordered.index("CLIENT_ACCEPTED") < ordered.index("HTTP_RESPONSE")


# --------------------------------------------------------------------------
# iRule simulation (over tcl-irule-test, on the bytecode VM)
#
# Runtime-ready but not yet fully runnable: the VM lights this up as it grows
# the orchestrator command surface. The contract tested here is the *shape* and
# graceful degradation — never a raise, always the documented keys.
# --------------------------------------------------------------------------


def test_simulate_irule_shape_and_graceful() -> None:
    out = t.simulate_irule(
        "when HTTP_REQUEST { pool web }",
        method="GET",
        uri="/",
        profiles=["HTTP"],
        pools=[("web", ["10.0.0.1:80"])],
    )
    assert set(out) == {"pool", "node", "response_committed", "logs", "decisions", "error"}
    assert isinstance(out["logs"], list)
    assert isinstance(out["decisions"], list)
    assert isinstance(out["response_committed"], bool)
    # Either it ran (error empty) or it degraded with a reason — never a raise.
    if out["error"]:
        assert out["pool"] == "" and out["node"] == ""


# --------------------------------------------------------------------------
# detect_dialect (centralised heuristic — over tcl-registry)
# --------------------------------------------------------------------------


def test_detect_dialect_heuristics() -> None:
    assert t.detect_dialect("when HTTP_REQUEST { pool web }") == "f5-irules"
    assert t.detect_dialect("spawn ssh host\nexpect_before timeout") == "expect"
    assert t.detect_dialect("synth_design -top foo") == "xilinx-eda-tcl"
    assert t.detect_dialect("compile_ultra") == "synopsys-eda-tcl"
    assert t.detect_dialect("tmsh::create ltm pool p") == "f5-tmsh"
    # Extension beats generic content.
    assert t.detect_dialect("puts hi", filename="x.irule") == "f5-irules"
    assert t.detect_dialect("spawn x", filename="y.exp") == "expect"
    # Shebang.
    assert t.detect_dialect("#!/usr/bin/tclsh8.6\nputs hi") == "tcl8.6"
    # Fallback to the given default.
    assert t.detect_dialect("set x 1", default="tcl8.6") == "tcl8.6"


# --------------------------------------------------------------------------
# SSA graphs + command walk (over tcl-lsp-core / tcl-compiler)
# --------------------------------------------------------------------------


def test_def_use_chains() -> None:
    g = t.def_use_chains("proc f {x} { set y $x; return $y }\n")
    assert "functions" in g and "summary" in g
    assert {"totalDefs", "totalUses", "totalAliases", "functionCount"} <= set(g["summary"])
    for func in g["functions"]:
        assert {"name", "nodes", "edges", "aliases", "summary"} <= set(func)


def test_memory_aliases() -> None:
    g = t.memory_aliases("proc f {v} { upvar 1 $v local; set local 1 }\n")
    assert "functions" in g and "summary" in g
    assert {"total_alias_sets", "total_memory_ops", "function_count"} <= set(g["summary"])


def test_walk_commands_recurses_bodies() -> None:
    src = "when HTTP_REQUEST {\n  if {$x eq \"a\"} { pool p1 }\n}\n"
    cmds = t.walk_commands(src, dialect="f5-irules")
    firsts = [c["texts"][0] for c in cmds]
    # Nested `if` and `pool` inside the `when` body are walked.
    assert "when" in firsts and "if" in firsts and "pool" in firsts
    for c in cmds:
        assert isinstance(c["texts"], list)
        assert isinstance(c["line"], int) and isinstance(c["character"], int)
