"""Tests for tcllib (Tcl Standard Library) support.

Verifies that tcllib commands are properly integrated:
- CommandSpec definitions are well-formed
- Registry package-to-command mapping works
- Completion includes/excludes tcllib commands based on ``package require``
- Hover shows docs (with import hint when package is missing)
- Diagnostics emit W120 when tcllib commands lack ``package require``
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.commands.registry import REGISTRY
from core.commands.registry.runtime import SIGNATURES, configure_signatures
from lsp.features.completion import get_completions
from lsp.features.diagnostics import get_diagnostics
from lsp.features.hover import get_hover


def _hover_text(result: types.Hover) -> str:
    """Extract the hover markdown text, handling the union return type."""
    contents = result.contents
    if isinstance(contents, types.MarkupContent):
        return contents.value
    if isinstance(contents, list):
        parts: list[str] = []
        for item in contents:
            if isinstance(item, types.MarkedStringWithLanguage):
                parts.append(item.value)
            else:
                parts.append(str(item))
        return "\n".join(parts)
    if isinstance(contents, types.MarkedStringWithLanguage):
        return contents.value
    return str(contents)


# Registry & CommandSpec well-formedness


class TestTcllibRegistry:
    """Verify tcllib CommandSpec entries and registry indices."""

    def test_all_tcllib_specs_have_tcllib_package(self):
        for name in REGISTRY.all_tcllib_command_names():
            spec = REGISTRY.get(name)
            assert spec is not None, f"missing spec for {name}"
            assert spec.tcllib_package is not None, f"{name} has no tcllib_package"

    def test_all_tcllib_specs_have_hover(self):
        for name in REGISTRY.all_tcllib_command_names():
            spec = REGISTRY.get(name)
            assert spec is not None, f"missing spec for {name}"
            assert spec.hover is not None, f"{name} has no hover snippet"
            assert spec.hover.summary, f"{name} hover has empty summary"

    def test_all_tcllib_specs_have_validation(self):
        for name in REGISTRY.all_tcllib_command_names():
            spec = REGISTRY.get(name)
            assert spec is not None, f"missing spec for {name}"
            assert spec.validation is not None, f"{name} has no validation"
            assert spec.validation.arity is not None, f"{name} has no arity"

    def test_known_packages_are_nonempty(self):
        packages = REGISTRY.known_tcllib_packages()
        assert len(packages) > 10, f"expected >10 packages, got {len(packages)}"

    def test_all_tcllib_command_names_nonempty(self):
        names = REGISTRY.all_tcllib_command_names()
        assert len(names) > 30, f"expected >30 commands, got {len(names)}"

    def test_package_to_command_mapping_is_consistent(self):
        for pkg in REGISTRY.known_tcllib_packages():
            cmds = REGISTRY.tcllib_command_names(frozenset({pkg}))
            assert len(cmds) > 0, f"package {pkg} has no commands"
            for cmd in cmds:
                assert REGISTRY.tcllib_package_for(cmd) == pkg

    def test_tcllib_package_for_returns_none_for_builtin(self):
        assert REGISTRY.tcllib_package_for("set") is None
        assert REGISTRY.tcllib_package_for("puts") is None

    def test_is_tcllib_command(self):
        assert REGISTRY.is_tcllib_command("json::json2dict")
        assert REGISTRY.is_tcllib_command("base64::encode")
        assert not REGISTRY.is_tcllib_command("set")
        assert not REGISTRY.is_tcllib_command("nonexistent")

    def test_tcllib_command_names_empty_for_empty_packages(self):
        result = REGISTRY.tcllib_command_names(frozenset())
        assert result == ()

    def test_tcllib_command_names_unknown_package(self):
        result = REGISTRY.tcllib_command_names(frozenset({"nonexistent_pkg"}))
        assert result == ()


# SIGNATURES integration


class TestTcllibSignatures:
    """Verify tcllib commands appear in SIGNATURES."""

    def test_json_commands_in_signatures(self):
        configure_signatures(dialect="tcl8.6")
        assert "json::json2dict" in SIGNATURES
        assert "json::dict2json" in SIGNATURES

    def test_base64_commands_in_signatures(self):
        configure_signatures(dialect="tcl8.6")
        assert "base64::encode" in SIGNATURES
        assert "base64::decode" in SIGNATURES

    def test_csv_commands_in_signatures(self):
        configure_signatures(dialect="tcl8.6")
        assert "csv::split" in SIGNATURES
        assert "csv::join" in SIGNATURES

    def test_struct_commands_in_signatures(self):
        configure_signatures(dialect="tcl8.6")
        assert "struct::list" in SIGNATURES
        assert "struct::set" in SIGNATURES

    def test_tcllib_commands_present_across_dialects(self):
        for dialect in ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"):
            configure_signatures(dialect=dialect)
            assert "json::json2dict" in SIGNATURES, f"missing in {dialect}"


# Known packages


class TestTcllibPackages:
    """Verify specific tcllib packages are registered."""

    def test_json_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"json"}))
        assert "json::json2dict" in cmds
        assert "json::dict2json" in cmds

    def test_base64_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"base64"}))
        assert "base64::encode" in cmds
        assert "base64::decode" in cmds

    def test_csv_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"csv"}))
        assert "csv::split" in cmds
        assert "csv::join" in cmds

    def test_fileutil_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"fileutil"}))
        assert "fileutil::cat" in cmds

    def test_uri_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"uri"}))
        assert "uri::split" in cmds
        assert "uri::join" in cmds

    def test_sha1_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"sha1"}))
        assert "sha1::sha1" in cmds

    def test_md5_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"md5"}))
        assert "md5::md5" in cmds

    def test_uuid_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"uuid"}))
        assert len(cmds) >= 1

    def test_cmdline_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"cmdline"}))
        assert "cmdline::getopt" in cmds

    def test_logger_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"logger"}))
        assert "logger::init" in cmds

    def test_ip_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"ip"}))
        assert "ip::normalize" in cmds

    def test_snit_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"snit"}))
        assert "snit::type" in cmds

    def test_multiple_packages(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"json", "base64"}))
        assert "json::json2dict" in cmds
        assert "base64::encode" in cmds


# Completion — per-document filtering via ``package require``


class TestTcllibCompletion:
    """Verify completion includes tcllib commands only when package is imported."""

    def test_completion_includes_json_with_package_require(self):
        source = "package require json\n"
        labels = {item.label for item in get_completions(source, 1, 0)}
        assert "json::json2dict" in labels
        assert "json::dict2json" in labels

    def test_completion_excludes_json_without_package_require(self):
        source = "set x 1\n"
        labels = {item.label for item in get_completions(source, 1, 0)}
        assert "json::json2dict" not in labels
        assert "json::dict2json" not in labels

    def test_completion_excludes_unrelated_package_commands(self):
        source = "package require json\n"
        labels = {item.label for item in get_completions(source, 1, 0)}
        # json is imported, but base64 is not
        assert "json::json2dict" in labels
        assert "base64::encode" not in labels

    def test_completion_multiple_packages(self):
        source = "package require json\npackage require base64\n"
        labels = {item.label for item in get_completions(source, 2, 0)}
        assert "json::json2dict" in labels
        assert "base64::encode" in labels

    def test_completion_detail_shows_tcllib_package(self):
        source = "package require json\n"
        items = get_completions(source, 1, 0)
        json_items = [i for i in items if i.label == "json::json2dict"]
        assert len(json_items) == 1
        detail = json_items[0].detail
        assert detail is not None
        assert "tcllib" in detail
        assert "json" in detail

    def test_completion_still_includes_builtins(self):
        source = "package require json\n"
        labels = {item.label for item in get_completions(source, 1, 0)}
        assert "set" in labels
        assert "puts" in labels
        assert "proc" in labels

    def test_empty_document_has_no_tcllib_commands(self):
        labels = {item.label for item in get_completions("", 0, 0)}
        all_tcllib = REGISTRY.all_tcllib_command_names()
        assert not labels.intersection(all_tcllib)


# Hover — documentation with import hints


class TestTcllibHover:
    """Verify hover shows docs for tcllib commands."""

    def test_hover_on_tcllib_command_with_import(self):
        source = "package require json\njson::json2dict $data"
        result = get_hover(source, 1, 5)
        assert result is not None
        text = _hover_text(result)
        assert "json2dict" in text.lower() or "JSON" in text or "json" in text
        # Should NOT have the import hint since package is imported
        assert "**Requires**" not in text

    def test_hover_on_tcllib_command_without_import(self):
        source = "json::json2dict $data"
        result = get_hover(source, 0, 5)
        assert result is not None
        text = _hover_text(result)
        # Should show docs
        assert "json" in text.lower() or "JSON" in text
        # Should show import hint
        assert "package require json" in text

    def test_hover_shows_synopsis(self):
        source = "package require base64\nbase64::encode $data"
        result = get_hover(source, 1, 5)
        assert result is not None
        text = _hover_text(result)
        assert "base64" in text.lower()

    def test_hover_import_hint_for_different_package(self):
        source = "package require json\nbase64::encode $data"
        result = get_hover(source, 1, 5)
        assert result is not None
        text = _hover_text(result)
        # json is imported but base64 is not
        assert "package require base64" in text


# Diagnostics — W120 missing ``package require``


class TestTcllibDiagnostics:
    """Verify W120 diagnostic for tcllib commands without ``package require``."""

    def test_w120_emitted_without_package_require(self):
        source = "json::json2dict $data"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        assert len(w120) == 1
        assert "json::json2dict" in w120[0].message
        assert "package require json" in w120[0].message

    def test_no_w120_with_package_require(self):
        source = "package require json\njson::json2dict $data"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        assert len(w120) == 0

    def test_w120_multiple_commands_same_package(self):
        source = "json::json2dict $a\njson::dict2json $b"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        # Should emit once per command, not once per invocation
        assert len(w120) == 2

    def test_w120_multiple_packages(self):
        source = "json::json2dict $a\nbase64::encode $b"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        assert len(w120) == 2
        messages = " ".join(d.message for d in w120)
        assert "json" in messages
        assert "base64" in messages

    def test_w120_partial_import(self):
        source = "package require json\njson::json2dict $a\nbase64::encode $b"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        # json is imported, base64 is not
        assert len(w120) == 1
        assert "base64" in w120[0].message

    def test_no_w120_for_builtin_commands(self):
        source = "set x 42\nputs $x"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        assert len(w120) == 0

    def test_w120_can_be_suppressed_with_noqa(self):
        # The noqa comment on line 0 is the preceding_comment
        # for the command on line 1, suppressing W120 on that line.
        source = "# noqa: W120\njson::json2dict $data"
        diags = get_diagnostics(source)
        w120 = [d for d in diags if d.code == "W120"]
        assert len(w120) == 0

    def test_w120_can_be_disabled_globally(self):
        source = "json::json2dict $data"
        diags = get_diagnostics(source, disabled_diagnostics={"W120"})
        w120 = [d for d in diags if d.code == "W120"]
        assert len(w120) == 0
