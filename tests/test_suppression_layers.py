"""Comprehensive tests for diagnostic suppression at every layer.

The tcl-lsp precedence chain (highest priority first):

    1. Inline       ``# noqa`` / ``# noqa: CODE``
    2. File-level   ``# tcl-lsp: disable=CODE[,CODE...]`` (top of file)
    3. Project      ``.tcl-lsp.ini`` at the workspace root
    4. Editor       ``workspace/didChangeConfiguration`` payload
    5. Global       ``~/.config/tcl-lsp/config.ini``

Inline and file-level are per-document and applied after per-server
``disabled_diagnostics`` filtering.  Project, editor, and global all feed
into the same server-level filter via ``merge_settings_layers``; project
wins because it is the most project-specific source of truth.

See ``docs/kcs/kcs-howto-suppress-diagnostics.md``.
"""

from __future__ import annotations

import configparser
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import Analyser, analyse
from analyser._analyser._utils import (
    _FILE_DIRECTIVE_SCAN_LINES,
    parse_file_suppression,
    parse_noqa_line_suppressions,
)
from analyser.semantic_model import _FILE_SUPPRESS_KEY
from compiler.parsing.command_segmenter import segment_commands
from server.features.diagnostics import _is_suppressed, get_basic_diagnostics, get_deep_diagnostics
from shared.user_config import (
    PROJECT_CONFIG_FILENAME,
    find_project_config,
    get_all_settings,
    load_project_config,
    merge_settings_layers,
)


def _codes(diags) -> list[str]:
    """Return the ``code`` attribute (str) of each diagnostic."""
    return [str(d.code) for d in diags if d.code is not None]


def _config_from_string(text: str) -> configparser.ConfigParser:
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment]  # ty: ignore[invalid-assignment]  # preserve case (configparser docs sanction assigning a callable to optionxform)
    config.read_string(text)
    return config


# Layer 1: inline ``# <noqa>`` — the existing mechanism.  We re-verify it
# end-to-end here so the precedence tests below can cite it as a baseline.


class TestInlineSuppression:
    """``# <noqa>`` applies to the command on the following line only."""

    def test_bare_noqa_suppresses_all_codes_on_next_command(self):
        source = "# noqa\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W100" not in _codes(diags)

    def test_noqa_with_codes_is_selective(self):
        # ``expr $x + 1`` fires W100 (unbraced expr) and W210 ($x read before set).
        # Targeting only W210 leaves W100 untouched.
        source = "# noqa: W210\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W210" not in _codes(diags)
        assert "W100" in _codes(diags)

    def test_noqa_wildcard_equivalent_to_bare(self):
        source = "# noqa: *\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        codes = _codes(diags)
        assert "W100" not in codes
        assert "W210" not in codes

    def test_noqa_does_not_leak_to_next_command(self):
        source = "# noqa: W100\nset a 1\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W100" in _codes(diags)


class TestNoqaNextLineSuppression:
    """``# noqa`` also suppresses the line immediately following the comment.

    This covers two cases the command-level ``preceding_comment`` mechanism
    cannot handle (issue #306):

    1. A noqa comment orphaned at the tail of a brace body — the diagnostic
       fires on the ``} elseif …`` line that immediately follows.
    2. A noqa comment placed before another comment line that itself generates
       a diagnostic (e.g. W115 on a ``\\``-continued comment).
    """

    def test_noqa_before_w115_comment_suppresses_it(self):
        # W115 fires on the ##nagelfar line (a backslash-continued comment).
        # The # noqa: W115 comment is on the line immediately before it.
        source = "# noqa: W115\n## continued comment\\\n    next line\nset x 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W115" not in _codes(diags)

    def test_noqa_before_w115_does_not_suppress_other_codes(self):
        source = "# noqa: W115\n## continued comment\\\n    next line\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W115" not in _codes(diags)
        assert "W100" in _codes(diags)

    def test_bare_noqa_before_comment_suppresses_all(self):
        source = "# noqa\n## continued comment\\\n    next line\nset x 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W115" not in _codes(diags)

    def test_parse_noqa_line_suppressions_unit(self):
        source = "set x 1\n# noqa: W306\n} elseif {cond} {\n"
        result = parse_noqa_line_suppressions(source)
        assert 2 in result
        assert "W306" in result[2]

    def test_noqa_orphaned_in_if_body_suppresses_elseif_diagnostic(self):
        # noqa inside the first body brace, W306 fires on the regexp in the
        # elseif condition (next line in source). Issue #306 case 1.
        source = (
            "if {[regexp -nocase {^([^#]+)} $name -> nameParsed]} {\n"
            '    set nameModif "result"\n'
            "# noqa: W306\n"
            '} elseif {[regexp -nocase "^i\\\\((\\\\[^)]+)\\\\)$" $name]} {\n'
            "    set nameModif $name\n"
            "} else {\n"
            "    set nameModif $name\n"
            "}\n"
        )
        diags, _, _ = get_basic_diagnostics(source)
        assert "W306" not in _codes(diags)

    def test_analyse_chunked_applies_next_line_suppression(self):
        # analyse_chunked is the primary LSP incremental path and must also
        # apply next-line noqa suppression.
        source = "# noqa: W115\n## continued comment\\\n    next line\nset x 1\n"
        commands = segment_commands(source)
        result, _ = Analyser().analyse_chunked(source, [list(commands)])
        assert 1 in result.suppressed_lines  # line 1 is the ##nagelfar line

    def test_analyse_commands_applies_next_line_suppression(self):
        # analyse_commands is the incremental restore path and must also apply
        # next-line noqa suppression.
        source = "# noqa: W115\n## continued comment\\\n    next line\nset x 1\n"
        commands = segment_commands(source)
        result = Analyser().analyse_commands(source, list(commands))
        assert 1 in result.suppressed_lines  # line 1 is the ##nagelfar line


# Layer 2: top-of-file ``# tcl-lsp: disable=...`` directive.


class TestParseFileSuppression:
    """Unit tests for the top-of-file directive parser."""

    def test_empty_source_returns_empty_set(self):
        assert parse_file_suppression("") == frozenset()

    def test_no_directive_returns_empty_set(self):
        assert parse_file_suppression("set x 1\nset y 2\n") == frozenset()

    def test_single_code(self):
        assert parse_file_suppression("# tcl-lsp: disable=W100\n") == frozenset({"W100"})

    def test_multiple_codes_comma_separated(self):
        got = parse_file_suppression("# tcl-lsp: disable=W100,W200,O109\n")
        assert got == frozenset({"W100", "W200", "O109"})

    def test_multiple_codes_with_whitespace(self):
        got = parse_file_suppression("# tcl-lsp: disable = W100 , W200 ,  O109\n")
        assert got == frozenset({"W100", "W200", "O109"})

    def test_wildcard(self):
        assert parse_file_suppression("# tcl-lsp: disable=*\n") == frozenset({"*"})

    def test_case_insensitive(self):
        assert parse_file_suppression("# TCL-LSP: DISABLE=W100\n") == frozenset({"W100"})

    def test_allowed_before_shebang_and_copyright(self):
        src = "#!/usr/bin/env tclsh\n# Copyright notice.\n# tcl-lsp: disable=W100\nset x 1\n"
        assert parse_file_suppression(src) == frozenset({"W100"})

    def test_allowed_after_blank_lines(self):
        src = "\n\n# tcl-lsp: disable=W100\n\nset x 1\n"
        assert parse_file_suppression(src) == frozenset({"W100"})

    def test_stops_at_first_non_comment_line(self):
        src = "set y 2\n# tcl-lsp: disable=W100\n"
        assert parse_file_suppression(src) == frozenset()

    def test_multiple_directives_accumulate(self):
        src = "# tcl-lsp: disable=W100\n# tcl-lsp: disable=O109\n"
        assert parse_file_suppression(src) == frozenset({"W100", "O109"})

    def test_scan_capped_at_line_limit(self):
        # Deep below the scan window — the directive must not be found.
        src = "\n" * (_FILE_DIRECTIVE_SCAN_LINES + 5) + "# tcl-lsp: disable=W100\n"
        assert parse_file_suppression(src) == frozenset()


class TestFileLevelSuppression:
    """End-to-end: ``# tcl-lsp: disable=...`` suppresses across the file."""

    def test_directive_populates_sentinel_key(self):
        result = analyse("# tcl-lsp: disable=W100\nexpr $x + 1\n")
        assert result.suppressed_lines.get(_FILE_SUPPRESS_KEY) == frozenset({"W100"})

    def test_no_directive_leaves_sentinel_absent(self):
        result = analyse("expr $x + 1\n")
        assert _FILE_SUPPRESS_KEY not in result.suppressed_lines

    def test_is_suppressed_honours_sentinel(self):
        suppressed = {_FILE_SUPPRESS_KEY: frozenset({"W100"})}
        assert _is_suppressed("W100", 0, suppressed)
        assert _is_suppressed("W100", 42, suppressed)
        assert not _is_suppressed("W200", 0, suppressed)

    def test_wildcard_suppresses_everything(self):
        suppressed = {_FILE_SUPPRESS_KEY: frozenset({"*"})}
        assert _is_suppressed("W100", 0, suppressed)
        assert _is_suppressed("IRULE1005", 999, suppressed)

    def test_file_suppression_through_basic_pipeline(self):
        source = "# tcl-lsp: disable=W100\nexpr $x + 1\nexpr $y + 2\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W100" not in _codes(diags)

    def test_file_suppression_selective_leaves_other_codes(self):
        # Disabling only W210 still leaves W100 emitted.
        source = "# tcl-lsp: disable=W210\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        codes = _codes(diags)
        assert "W210" not in codes
        assert "W100" in codes

    def test_file_wildcard_suppresses_multiple_lines(self):
        source = "# tcl-lsp: disable=*\nexpr $x + 1\nexpr $y + 2\n"
        diags, _, _ = get_basic_diagnostics(source)
        codes = _codes(diags)
        assert "W100" not in codes
        assert "W210" not in codes

    def test_file_suppression_applies_to_deep_pass(self):
        # Deep passes consume the same ``suppressed`` dict, so file-level
        # suppression must transparently cover optimiser/shimmer/taint codes.
        source = "# tcl-lsp: disable=*\nexpr $x + 1\n"
        _basic, _result, suppressed = get_basic_diagnostics(source)
        deep = get_deep_diagnostics(source, suppressed)
        assert _codes(deep) == []

    def test_analyse_chunked_populates_sentinel_key(self):
        # The LSP uses analyse_chunked; file directives must be respected there.
        source = "# tcl-lsp: disable=W100\nexpr $x + 1\n"
        commands = segment_commands(source)
        result, _ = Analyser().analyse_chunked(source, [list(commands)])
        assert result.suppressed_lines.get(_FILE_SUPPRESS_KEY) == frozenset({"W100"})

    def test_analyse_commands_populates_sentinel_key(self):
        # The incremental path (analyse_commands) must also parse file directives.
        source = "# tcl-lsp: disable=W100\nexpr $x + 1\n"
        commands = segment_commands(source)
        result = Analyser().analyse_commands(source, list(commands))
        assert result.suppressed_lines.get(_FILE_SUPPRESS_KEY) == frozenset({"W100"})


# Layer 3 (unit): project config file discovery + parsing.


class TestProjectConfigFile:
    """Project config file is ``.tcl-lsp.ini`` at the workspace root."""

    def test_filename_constant(self):
        assert PROJECT_CONFIG_FILENAME == ".tcl-lsp.ini"

    def test_find_returns_none_when_absent(self, tmp_path: Path):
        assert find_project_config(tmp_path) is None

    def test_find_returns_path_when_present(self, tmp_path: Path):
        (tmp_path / PROJECT_CONFIG_FILENAME).write_text("[diagnostics]\ndisabled = W100\n")
        found = find_project_config(tmp_path)
        assert found is not None
        assert found.name == PROJECT_CONFIG_FILENAME

    def test_find_does_not_walk_upward(self, tmp_path: Path):
        (tmp_path / PROJECT_CONFIG_FILENAME).write_text("[diagnostics]\ndisabled = W100\n")
        nested = tmp_path / "src"
        nested.mkdir()
        # ``find_project_config`` is deliberately local; callers pass the
        # canonical workspace root rather than relying on upward walking.
        assert find_project_config(nested) is None

    def test_load_empty_when_missing(self, tmp_path: Path):
        config = load_project_config(tmp_path)
        assert isinstance(config, configparser.ConfigParser)
        assert config.sections() == []

    def test_load_parses_diagnostics_section(self, tmp_path: Path):
        (tmp_path / PROJECT_CONFIG_FILENAME).write_text("[diagnostics]\ndisabled = W100, O109\n")
        config = load_project_config(tmp_path)
        settings = get_all_settings(config)
        assert settings["diagnostics"]["W100"] is False
        assert settings["diagnostics"]["O109"] is False

    def test_malformed_file_is_ignored(self, tmp_path: Path):
        (tmp_path / PROJECT_CONFIG_FILENAME).write_text("not valid INI [[[[\n")
        config = load_project_config(tmp_path)
        # Should not raise; returns an empty-ish ConfigParser.
        assert config.sections() == []


# Layer-merge precedence: project > editor > global.


class TestMergeSettingsLayers:
    """Unit tests for ``merge_settings_layers`` — the precedence engine."""

    def test_empty_inputs(self):
        assert merge_settings_layers() == {}
        assert merge_settings_layers({}, {}, {}) == {}

    def test_single_layer_passthrough(self):
        layer = {"diagnostics": {"W100": False}}
        assert merge_settings_layers(layer) == layer

    def test_second_layer_overrides_first(self):
        a = {"diagnostics": {"W100": False}}
        b = {"diagnostics": {"W100": True}}
        assert merge_settings_layers(a, b) == {"diagnostics": {"W100": True}}

    def test_later_layer_wins_per_key_not_section(self):
        # The two layers touch different codes; the merge must keep both.
        a = {"diagnostics": {"W100": False}}
        b = {"diagnostics": {"W200": False}}
        merged = merge_settings_layers(a, b)
        assert merged == {"diagnostics": {"W100": False, "W200": False}}

    def test_project_overrides_editor_overrides_global(self):
        global_ = {"diagnostics": {"W100": False, "W200": False}}
        editor = {"diagnostics": {"W100": True}}  # tries to re-enable
        project = {"diagnostics": {"W100": False}}  # enforces off again
        merged = merge_settings_layers(global_, editor, project)
        assert merged["diagnostics"]["W100"] is False  # project wins
        assert merged["diagnostics"]["W200"] is False  # inherited from global

    def test_editor_wins_over_global_when_no_project(self):
        global_ = {"diagnostics": {"W100": False}}
        editor = {"diagnostics": {"W100": True}}
        merged = merge_settings_layers(global_, editor, {})
        assert merged["diagnostics"]["W100"] is True

    def test_non_dict_section_is_replaced_wholesale(self):
        # Sanity: a scalar or list value replaces whatever was there before.
        a = {"dialect": "tcl8.6"}
        b = {"dialect": "tcl9.0"}
        assert merge_settings_layers(a, b)["dialect"] == "tcl9.0"

    def test_ignores_non_dict_layers(self):
        # Robustness: passing ``None`` or a non-dict layer should not crash.
        merged = merge_settings_layers({"diagnostics": {"W100": False}}, None)
        assert merged == {"diagnostics": {"W100": False}}


# Layer interactions: each pair of layers, walking the precedence chain.


class TestPrecedenceInteractions:
    """Smoke tests that walk the precedence chain one pair at a time."""

    # 1. Inline overrides file-level by union: both disable the code, so the
    #    diagnostic is suppressed even if only one directive is present.

    def test_inline_suppresses_even_without_file_directive(self):
        # Inline ``# <noqa>`` goes on the line *before* the command it suppresses.
        source = "# noqa: W100\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W100" not in _codes(diags)

    def test_file_suppresses_even_without_inline(self):
        source = "# tcl-lsp: disable=W100\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W100" not in _codes(diags)

    def test_inline_and_file_coexist(self):
        # Both directives target W100 — no crash, still suppressed.
        source = "# tcl-lsp: disable=W100\n# noqa: W100\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(source)
        assert "W100" not in _codes(diags)

    # 2. File-level vs project-level: file suppresses regardless of project
    #    config, because file is checked after server-level filtering.

    def test_file_suppresses_even_when_project_enables(self):
        # Simulate: project config has W100 enabled; file still disables it.
        # File-level suppression happens in the per-doc pipeline, not via the
        # server-level ``disabled_diagnostics`` set, so it always wins.
        source = "# tcl-lsp: disable=W100\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(
            source,
            disabled_diagnostics=set(),  # project has nothing disabled
        )
        assert "W100" not in _codes(diags)

    def test_project_disables_without_file_directive(self):
        # Simulate: project config disables W100 via ``disabled_diagnostics``.
        source = "expr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(
            source,
            disabled_diagnostics={"W100"},
        )
        assert "W100" not in _codes(diags)

    # 3. Project vs editor: project wins.  Exercised via merge_settings_layers
    #    above; here we just verify the same behavior at the settings level.

    def test_project_wins_over_editor_in_merge(self):
        global_ = {}
        editor = {"diagnostics": {"W100": True}}
        project = {"diagnostics": {"W100": False}}
        assert merge_settings_layers(global_, editor, project)["diagnostics"]["W100"] is False

    def test_editor_can_disable_when_project_silent(self):
        global_ = {}
        editor = {"diagnostics": {"W100": False}}
        project = {}
        assert merge_settings_layers(global_, editor, project)["diagnostics"]["W100"] is False

    # 4. Editor vs global: editor wins.

    def test_editor_wins_over_global_in_merge(self):
        global_ = {"diagnostics": {"W100": False}}
        editor = {"diagnostics": {"W100": True}}
        project = {}
        assert merge_settings_layers(global_, editor, project)["diagnostics"]["W100"] is True

    def test_global_default_when_no_overrides(self):
        global_ = {"diagnostics": {"W100": False}}
        merged = merge_settings_layers(global_, {}, {})
        assert merged["diagnostics"]["W100"] is False

    # 5. Full chain: project > editor > global, and inline/file always
    #    suppresses regardless of what the server-level layers say.

    def test_full_chain_project_trumps_everything_below_it(self):
        global_ = {"diagnostics": {"W100": True, "W200": True}}
        editor = {"diagnostics": {"W100": True}}
        project = {"diagnostics": {"W100": False, "W200": False}}
        merged = merge_settings_layers(global_, editor, project)
        assert merged["diagnostics"]["W100"] is False
        assert merged["diagnostics"]["W200"] is False

    def test_inline_overrides_an_enabled_code_from_project(self):
        # Even if every server layer has W100 enabled, inline wins locally.
        source = "# noqa: W100\nexpr $x + 1\n"
        diags, _, _ = get_basic_diagnostics(
            source,
            disabled_diagnostics=set(),  # project+editor+global all enable W100
        )
        assert "W100" not in _codes(diags)


# Deep-pass coverage: each suppression layer must reach the expensive passes
# (optimiser, shimmer, taint, etc.) that run after basic diagnostics.


class TestDeepPassRespectsSuppression:
    """Deep diagnostics consume the same ``suppressed`` dict."""

    def test_inline_suppresses_deep_pass(self):
        source = "set x 1  ;# noqa: *\n"
        suppressed = {0: frozenset({"*"})}
        deep = get_deep_diagnostics(source, suppressed)
        assert deep == []

    def test_file_level_suppresses_deep_pass(self):
        source = "# tcl-lsp: disable=*\nset x 1\nset y 2\n"
        _basic, _result, suppressed = get_basic_diagnostics(source)
        deep = get_deep_diagnostics(source, suppressed)
        assert deep == []

    def test_disabled_diagnostics_filter_propagates(self):
        # Project/editor/global all funnel into this set for the deep pass.
        source = "expr $x + 1\n"
        _basic, _result, suppressed = get_basic_diagnostics(source)
        deep = get_deep_diagnostics(
            source,
            suppressed,
            disabled_diagnostics={"W100"},
            disabled_optimisations={"O111"},
        )
        # Nothing we can reliably assert about *other* codes fired, just that
        # the two we disabled don't show up.
        codes = set(_codes(deep))
        assert "W100" not in codes
        assert "O111" not in codes


@pytest.mark.parametrize(
    "directive,expected_codes",
    [
        ("# tcl-lsp: disable=W100", {"W100"}),
        ("# tcl-lsp: disable = W100, W200", {"W100", "W200"}),
        ("# tcl-lsp: disable=*", {"*"}),
        ("#  tcl-lsp:disable=W100", {"W100"}),
        ("# tcl-lsp:disable= W100 ", {"W100"}),
    ],
)
def test_file_directive_formats(directive: str, expected_codes: set[str]):
    """Directive parser tolerates whitespace variants and spacing."""
    assert parse_file_suppression(directive + "\n") == frozenset(expected_codes)
