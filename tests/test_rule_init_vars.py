"""Tests for RULE_INIT cross-file variable sharing."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.irules_flow import (
    RuleInitExport,
    _find_when_bodies,
    extract_rule_init_vars,
)
from lsp.features.completion import get_completions
from lsp.workspace.scanner import BackgroundScanner
from lsp.workspace.workspace_index import WorkspaceIndex

# Priority extraction from _find_when_bodies


class TestFindWhenBodiesPriority:
    def test_default_priority_is_500(self):
        source = "when HTTP_REQUEST { set x 1 }"
        results = list(_find_when_bodies(source))
        assert len(results) == 1
        event, priority, body_text, body_tok, _event_tok = results[0]
        assert event == "HTTP_REQUEST"
        assert priority == 500

    def test_explicit_priority(self):
        source = "when RULE_INIT priority 100 { set ::x 1 }"
        results = list(_find_when_bodies(source))
        assert len(results) == 1
        event, priority, body_text, body_tok, _event_tok = results[0]
        assert event == "RULE_INIT"
        assert priority == 100

    def test_high_priority_number(self):
        source = "when HTTP_REQUEST priority 999 { set x 1 }"
        results = list(_find_when_bodies(source))
        assert len(results) == 1
        assert results[0][1] == 999

    def test_priority_with_timing(self):
        source = "when HTTP_REQUEST priority 200 timing enable { set x 1 }"
        results = list(_find_when_bodies(source))
        assert len(results) == 1
        assert results[0][1] == 200

    def test_timing_without_priority(self):
        source = "when HTTP_REQUEST timing enable { set x 1 }"
        results = list(_find_when_bodies(source))
        assert len(results) == 1
        # timing != priority, so default 500
        assert results[0][1] == 500

    def test_multiple_when_blocks(self):
        source = (
            "when RULE_INIT priority 100 { set ::x 1 }\n"
            "when HTTP_REQUEST priority 200 { set y 2 }\n"
        )
        results = list(_find_when_bodies(source))
        assert len(results) == 2
        assert results[0][0] == "RULE_INIT"
        assert results[0][1] == 100
        assert results[1][0] == "HTTP_REQUEST"
        assert results[1][1] == 200


# extract_rule_init_vars


class TestExtractRuleInitVars:
    def test_set_global_var(self):
        source = 'when RULE_INIT { set ::shared_var "hello" }'
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "::shared_var"
        assert exports[0].priority == 500
        assert exports[0].is_array is False

    def test_set_global_var_with_priority(self):
        source = 'when RULE_INIT priority 100 { set ::shared_var "hello" }'
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "::shared_var"
        assert exports[0].priority == 100

    def test_array_set_global(self):
        source = "when RULE_INIT { array set ::config { key1 val1 key2 val2 } }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "::config"
        assert exports[0].is_array is True

    def test_ignores_local_vars(self):
        source = 'when RULE_INIT { set local_var "hello" }'
        exports = extract_rule_init_vars(source)
        assert len(exports) == 0

    def test_ignores_non_rule_init_events(self):
        source = 'when HTTP_REQUEST { set ::shared_var "hello" }'
        exports = extract_rule_init_vars(source)
        assert len(exports) == 0

    def test_multiple_vars(self):
        source = (
            "when RULE_INIT {\n"
            '    set ::var1 "a"\n'
            '    set ::var2 "b"\n'
            "    array set ::arr1 { k v }\n"
            "}"
        )
        exports = extract_rule_init_vars(source)
        assert len(exports) == 3
        names = {e.name for e in exports}
        assert names == {"::var1", "::var2", "::arr1"}
        arrays = {e.name for e in exports if e.is_array}
        assert arrays == {"::arr1"}

    def test_mixed_events(self):
        source = 'when RULE_INIT { set ::init_var "x" }\nwhen HTTP_REQUEST { set ::req_var "y" }\n'
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "::init_var"

    def test_empty_rule_init(self):
        source = "when RULE_INIT { }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 0

    def test_no_when_blocks(self):
        source = "proc foo {} { return 1 }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 0


# WorkspaceIndex RULE_INIT var storage


class TestWorkspaceIndexRuleInitVars:
    def _make_export(
        self,
        name: str,
        priority: int = 500,
        is_array: bool = False,
    ) -> RuleInitExport:
        from core.analysis.semantic_model import Range
        from core.parsing.tokens import SourcePosition

        r = Range(
            start=SourcePosition(0, 0, 0),
            end=SourcePosition(0, 0, len(name)),
        )
        return RuleInitExport(name=name, priority=priority, range=r, is_array=is_array)

    def test_update_and_find(self):
        idx = WorkspaceIndex()
        exports = [self._make_export("::my_var")]
        idx.update_rule_init_vars("file:///a.irul", exports)
        results = idx.find_rule_init_var("::my_var")
        assert len(results) == 1
        assert results[0].name == "::my_var"
        assert results[0].source_uri == "file:///a.irul"

    def test_all_names(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::var1"),
                self._make_export("::var2"),
            ],
        )
        names = idx.all_rule_init_var_names()
        assert "::var1" in names
        assert "::var2" in names

    def test_update_replaces_old(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::old_var"),
            ],
        )
        assert len(idx.find_rule_init_var("::old_var")) == 1

        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::new_var"),
            ],
        )
        assert len(idx.find_rule_init_var("::old_var")) == 0
        assert len(idx.find_rule_init_var("::new_var")) == 1

    def test_multi_file(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::var_a"),
            ],
        )
        idx.update_rule_init_vars(
            "file:///b.irul",
            [
                self._make_export("::var_b"),
            ],
        )
        assert len(idx.find_rule_init_var("::var_a")) == 1
        assert len(idx.find_rule_init_var("::var_b")) == 1
        names = idx.all_rule_init_var_names()
        assert len(names) == 2

    def test_same_var_from_multiple_files(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::shared", priority=100),
            ],
        )
        idx.update_rule_init_vars(
            "file:///b.irul",
            [
                self._make_export("::shared", priority=200),
            ],
        )
        results = idx.find_rule_init_var("::shared")
        assert len(results) == 2

    def test_remove_cleans_rule_init_vars(self):
        from core.analysis.analyser import analyse

        idx = WorkspaceIndex()
        result = analyse("set x 1")
        idx.update("file:///a.irul", result)
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::my_var"),
            ],
        )
        assert len(idx.find_rule_init_var("::my_var")) == 1
        idx.remove("file:///a.irul")
        assert len(idx.find_rule_init_var("::my_var")) == 0

    def test_remove_background_entries_clears_rule_init(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::my_var"),
            ],
        )
        assert len(idx.find_rule_init_var("::my_var")) == 1
        idx.remove_background_entries()
        assert len(idx.find_rule_init_var("::my_var")) == 0

    def test_find_nonexistent_returns_empty(self):
        idx = WorkspaceIndex()
        assert idx.find_rule_init_var("::no_such_var") == []

    def test_array_flag_preserved(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::my_arr", is_array=True),
            ],
        )
        results = idx.find_rule_init_var("::my_arr")
        assert results[0].is_array is True


# BackgroundScanner RULE_INIT integration


class TestScannerRuleInitVars:
    def test_scanner_extracts_rule_init_vars(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "init.irul")).write_text(
                'when RULE_INIT priority 100 { set ::shared "hello" }'
            )
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            ri_vars = scanner.irules_rule_init_vars
            assert len(ri_vars) == 1
            uri = list(ri_vars.keys())[0]
            exports = ri_vars[uri]
            assert len(exports) == 1
            assert exports[0].name == "::shared"
            assert exports[0].priority == 100

    def test_scanner_no_rule_init_for_tcl_files(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "lib.tcl")).write_text("proc helper {} { return 1 }")
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            ri_vars = scanner.irules_rule_init_vars
            assert len(ri_vars) == 0

    def test_scan_result_has_rule_init_exports(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            f = os.path.join(tmpdir, "test.irul")
            Path(f).write_text("when RULE_INIT { set ::x 1\narray set ::cfg { k v } }")
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            ri_vars = scanner.irules_rule_init_vars
            uri = list(ri_vars.keys())[0]
            exports = ri_vars[uri]
            assert len(exports) == 2
            names = {e.name for e in exports}
            assert "::x" in names
            assert "::cfg" in names


# Completion with RULE_INIT vars


class TestCompletionRuleInitVars:
    def test_rule_init_vars_in_completion(self):
        source = "set val $"
        items = get_completions(
            source,
            0,
            len(source),
            workspace_rule_init_vars=["::shared_var", "::config"],
        )
        labels = {item.label for item in items}
        assert "$::shared_var" in labels
        assert "$::config" in labels

    def test_rule_init_vars_filtered_by_prefix(self):
        source = "set val $::sh"
        items = get_completions(
            source,
            0,
            len(source),
            workspace_rule_init_vars=["::shared_var", "::config"],
        )
        labels = {item.label for item in items}
        assert "$::shared_var" in labels
        assert "$::config" not in labels

    def test_rule_init_vars_have_detail(self):
        source = "set val $"
        items = get_completions(
            source,
            0,
            len(source),
            workspace_rule_init_vars=["::shared_var"],
        )
        ri_items = [i for i in items if i.label == "$::shared_var"]
        assert len(ri_items) == 1
        assert ri_items[0].detail == "RULE_INIT (cross-file)"

    def test_no_duplicate_with_local_var(self):
        source = 'set ::shared_var "hello"\nset val $'
        items = get_completions(
            source,
            1,
            len("set val $"),
            workspace_rule_init_vars=["::shared_var"],
        )
        # Should not have duplicate entries
        labels = [item.label for item in items if item.label == "$::shared_var"]
        assert len(labels) == 1


# static:: variable support


class TestExtractStaticVars:
    """extract_rule_init_vars should capture static:: variables."""

    def test_set_static_var(self):
        source = "when RULE_INIT { set static::debug 1 }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "static::debug"
        assert exports[0].is_array is False

    def test_set_static_var_with_priority(self):
        source = "when RULE_INIT priority 100 { set static::debug 1 }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "static::debug"
        assert exports[0].priority == 100

    def test_array_set_static(self):
        source = "when RULE_INIT { array set static::config { key1 val1 key2 val2 } }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 1
        assert exports[0].name == "static::config"
        assert exports[0].is_array is True

    def test_mixed_global_and_static(self):
        source = (
            "when RULE_INIT {\n"
            '    set ::global_var "a"\n'
            "    set static::debug 1\n"
            '    set static::pool_name "my_pool"\n'
            "}"
        )
        exports = extract_rule_init_vars(source)
        assert len(exports) == 3
        names = {e.name for e in exports}
        assert names == {"::global_var", "static::debug", "static::pool_name"}

    def test_ignores_local_vars(self):
        source = 'when RULE_INIT { set local_var "hello" }'
        exports = extract_rule_init_vars(source)
        assert len(exports) == 0

    def test_ignores_static_outside_rule_init(self):
        source = "when HTTP_REQUEST { set static::debug 1 }"
        exports = extract_rule_init_vars(source)
        assert len(exports) == 0


class TestWorkspaceIndexStaticVars:
    def _make_export(
        self,
        name: str,
        priority: int = 500,
        is_array: bool = False,
    ) -> RuleInitExport:
        from core.analysis.semantic_model import Range
        from core.parsing.tokens import SourcePosition

        r = Range(
            start=SourcePosition(0, 0, 0),
            end=SourcePosition(0, 0, len(name)),
        )
        return RuleInitExport(name=name, priority=priority, range=r, is_array=is_array)

    def test_static_var_stored_and_found(self):
        idx = WorkspaceIndex()
        exports = [self._make_export("static::debug")]
        idx.update_rule_init_vars("file:///a.irul", exports)
        results = idx.find_rule_init_var("static::debug")
        assert len(results) == 1
        assert results[0].name == "static::debug"

    def test_static_var_in_all_names(self):
        idx = WorkspaceIndex()
        idx.update_rule_init_vars(
            "file:///a.irul",
            [
                self._make_export("::global_var"),
                self._make_export("static::debug"),
            ],
        )
        names = idx.all_rule_init_var_names()
        assert "::global_var" in names
        assert "static::debug" in names


class TestCompletionStaticVars:
    def test_static_vars_in_completion(self):
        source = "set val $"
        items = get_completions(
            source,
            0,
            len(source),
            workspace_rule_init_vars=["static::debug", "static::pool"],
        )
        labels = {item.label for item in items}
        assert "$static::debug" in labels
        assert "$static::pool" in labels

    def test_static_vars_filtered_by_prefix(self):
        source = "set val $static::d"
        items = get_completions(
            source,
            0,
            len(source),
            workspace_rule_init_vars=["static::debug", "static::pool"],
        )
        labels = {item.label for item in items}
        assert "$static::debug" in labels
        assert "$static::pool" not in labels

    def test_static_vars_have_detail(self):
        source = "set val $"
        items = get_completions(
            source,
            0,
            len(source),
            workspace_rule_init_vars=["static::debug"],
        )
        ri_items = [i for i in items if i.label == "$static::debug"]
        assert len(ri_items) == 1
        assert ri_items[0].detail == "RULE_INIT (cross-file)"


class TestScannerStaticVars:
    def test_scanner_extracts_static_vars(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "init.irul")).write_text(
                "when RULE_INIT { set static::debug 1\nset ::global 2 }"
            )
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            ri_vars = scanner.irules_rule_init_vars
            assert len(ri_vars) == 1
            uri = list(ri_vars.keys())[0]
            exports = ri_vars[uri]
            assert len(exports) == 2
            names = {e.name for e in exports}
            assert "static::debug" in names
            assert "::global" in names
