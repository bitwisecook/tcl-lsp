"""Tests for Tcl stdlib command registry support."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.analysis.analyser import analyse
from core.commands.registry import REGISTRY
from lsp.features.completion import get_completions
from lsp.features.hover import get_hover


def _hover_text(result: types.Hover) -> str:
    """Extract markdown text from a Hover result."""
    contents = result.contents
    if isinstance(contents, types.MarkupContent):
        return contents.value
    if isinstance(contents, list):
        return "\n".join(str(c) for c in contents)
    return str(contents)


class TestStdlibRegistration:
    """Stdlib commands are present in the global registry."""

    def test_http_geturl_registered(self):
        spec = REGISTRY.get("http::geturl")
        assert spec is not None
        assert spec.name == "http::geturl"
        assert spec.required_package == "http"

    def test_msgcat_mc_registered(self):
        spec = REGISTRY.get("msgcat::mc")
        assert spec is not None
        assert spec.required_package == "msgcat"

    def test_tcltest_test_registered(self):
        spec = REGISTRY.get("tcltest::test")
        assert spec is not None
        assert spec.required_package == "tcltest"

    def test_platform_identify_registered(self):
        spec = REGISTRY.get("platform::identify")
        assert spec is not None
        assert spec.required_package == "platform"

    def test_safe_interp_create_registered(self):
        spec = REGISTRY.get("safe::interpCreate")
        assert spec is not None
        assert spec.required_package == "safe"

    def test_opt_proc_registered(self):
        spec = REGISTRY.get("tcl::OptProc")
        assert spec is not None
        assert spec.required_package == "opt"

    def test_history_registered_no_package(self):
        spec = REGISTRY.get("history")
        assert spec is not None
        assert spec.required_package is None

    def test_pkg_mkindex_registered_no_package(self):
        spec = REGISTRY.get("pkg_mkIndex")
        assert spec is not None
        assert spec.required_package is None

    def test_tcl_tm_path_registered_no_package(self):
        spec = REGISTRY.get("tcl::tm::path")
        assert spec is not None
        assert spec.required_package is None

    def test_http_cookiejar_registered(self):
        spec = REGISTRY.get("http::cookiejar")
        assert spec is not None
        assert spec.required_package == "cookiejar"

    def test_tcl_word_break_registered(self):
        spec = REGISTRY.get("tcl_wordBreakAfter")
        assert spec is not None
        assert spec.required_package is None


class TestPackageFiltering:
    """The active_packages parameter controls stdlib command visibility."""

    def test_http_visible_with_package(self):
        spec = REGISTRY.get("http::geturl", active_packages=frozenset({"http"}))
        assert spec is not None

    def test_http_hidden_without_package(self):
        spec = REGISTRY.get("http::geturl", active_packages=frozenset())
        assert spec is None

    def test_http_hidden_with_wrong_package(self):
        spec = REGISTRY.get("http::geturl", active_packages=frozenset({"msgcat"}))
        assert spec is None

    def test_http_visible_when_no_filter(self):
        """active_packages=None means 'no filtering' — show everything."""
        spec = REGISTRY.get("http::geturl", active_packages=None)
        assert spec is not None

    def test_non_package_commands_always_visible(self):
        """Commands without required_package are visible regardless."""
        spec = REGISTRY.get("history", active_packages=frozenset())
        assert spec is not None

    def test_builtins_unaffected_by_package_filter(self):
        """Core Tcl builtins (set, puts, etc.) are unaffected."""
        spec = REGISTRY.get("set", active_packages=frozenset())
        assert spec is not None

    def test_command_names_filtered(self):
        all_names = set(REGISTRY.command_names())
        filtered_names = set(REGISTRY.command_names(active_packages=frozenset()))
        # Filtered should exclude package-gated commands
        assert "http::geturl" in all_names
        assert "http::geturl" not in filtered_names
        # Non-gated commands should be in both
        assert "set" in all_names
        assert "set" in filtered_names

    def test_command_names_with_http_package(self):
        names = set(REGISTRY.command_names(active_packages=frozenset({"http"})))
        assert "http::geturl" in names
        assert "http::config" in names
        assert "http::wait" in names
        # Other packages still excluded
        assert "msgcat::mc" not in names

    def test_validation_filtered_by_package(self):
        val = REGISTRY.validation("http::geturl", active_packages=frozenset({"http"}))
        assert val is not None
        val_none = REGISTRY.validation("http::geturl", active_packages=frozenset())
        assert val_none is None

    def test_switches_filtered_by_package(self):
        """Switch names respect package filtering."""
        sw = REGISTRY.switches("http::geturl", active_packages=frozenset({"http"}))
        # http::geturl doesn't have switches registered (no forms with options)
        assert isinstance(sw, tuple)


class TestStdlibHover:
    """Hover provider returns documentation for stdlib commands."""

    def test_hover_http_geturl_with_package(self):
        source = "package require http\nhttp::geturl http://example.com"
        hover = get_hover(source, 1, 5)
        assert hover is not None
        assert "geturl" in _hover_text(hover).lower() or "http" in _hover_text(hover).lower()

    def test_hover_http_geturl_without_package(self):
        source = "http::geturl http://example.com"
        hover = get_hover(source, 0, 5)
        # Without package require the command still shows docs (with an
        # import hint) so the developer can discover the missing import.
        assert hover is not None
        text = _hover_text(hover)
        assert "package require http" in text

    def test_hover_http_data_with_package(self):
        source = "package require http\nhttp::data $tok"
        hover = get_hover(source, 1, 5)
        assert hover is not None
        assert "body" in _hover_text(hover).lower() or "response" in _hover_text(hover).lower()

    def test_hover_history_always_visible(self):
        source = "history info"
        hover = get_hover(source, 0, 2)
        assert hover is not None
        assert "history" in _hover_text(hover).lower()

    def test_hover_msgcat_mc_with_package(self):
        source = 'package require msgcat\nmsgcat::mc "hello"'
        hover = get_hover(source, 1, 5)
        assert hover is not None
        assert "translate" in _hover_text(hover).lower() or "msgcat" in _hover_text(hover).lower()

    def test_hover_source_attribution(self):
        source = "package require http\nhttp::geturl http://example.com"
        hover = get_hover(source, 1, 5)
        assert hover is not None
        assert "stdlib" in _hover_text(hover).lower() or "http" in _hover_text(hover).lower()


class TestStdlibCompletion:
    """Completion provider includes/excludes stdlib commands based on package require."""

    def test_completion_includes_http_with_package(self):
        source = "package require http\nhttp::get"
        items = get_completions(source, 1, 9)
        labels = {item.label for item in items}
        assert "http::geturl" in labels

    def test_completion_excludes_http_without_package(self):
        source = "http::get"
        items = get_completions(source, 0, 9)
        labels = {item.label for item in items}
        assert "http::geturl" not in labels

    def test_completion_includes_history_always(self):
        source = "hist"
        items = get_completions(source, 0, 4)
        labels = {item.label for item in items}
        assert "history" in labels

    def test_completion_includes_pkg_mkindex_always(self):
        source = "pkg_mk"
        items = get_completions(source, 0, 6)
        labels = {item.label for item in items}
        assert "pkg_mkIndex" in labels


class TestStdlibAnalysis:
    """The analyser tracks package requires correctly."""

    def test_package_requires_tracked(self):
        source = "package require http\npackage require msgcat"
        result = analyse(source)
        names = result.active_package_names()
        assert "http" in names
        assert "msgcat" in names

    def test_no_package_requires(self):
        source = "set x 1"
        result = analyse(source)
        names = result.active_package_names()
        assert len(names) == 0

    def test_package_require_with_version(self):
        source = "package require http 2.9"
        result = analyse(source)
        names = result.active_package_names()
        assert "http" in names
