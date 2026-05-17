"""Per-workspace-folder configuration resolution (issue #230).

Covers:

- ``config_for_uri`` / ``formatter_config_for_uri`` longest-prefix
  matching and fallback when a document is outside every folder.
- ``_apply_settings_to_target`` writes feature toggles, formatter
  fields, line length, and disabled diagnostics to the right folder.
- The merged settings layers (``editor``, ``project``) honour the
  folder URI when resolving merged settings.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

import lsp.settings as _lsp_settings
import lsp.state as _lsp_state
from core.formatting import FormatterConfig
from lsp.feature_config import FeatureConfig


@pytest.fixture
def reset_per_folder_state():
    """Snapshot/restore the module-level per-folder maps and fallback configs."""
    snap_feature = _lsp_state.feature_config
    snap_formatter = _lsp_state.formatter_config
    snap_per_folder_feature = dict(_lsp_state._per_folder_feature_configs)
    snap_per_folder_formatter = dict(_lsp_state._per_folder_formatter_configs)
    snap_editor = dict(_lsp_state.editor_config_settings)
    snap_project = dict(_lsp_state.project_config_settings)
    snap_editor_pf = dict(_lsp_state.editor_config_settings_per_folder)
    snap_project_pf = dict(_lsp_state.project_config_settings_per_folder)

    _lsp_state.feature_config = FeatureConfig()
    _lsp_state.formatter_config = FormatterConfig()
    _lsp_state._per_folder_feature_configs.clear()
    _lsp_state._per_folder_formatter_configs.clear()
    _lsp_state.editor_config_settings.clear()
    _lsp_state.project_config_settings.clear()
    _lsp_state.editor_config_settings_per_folder.clear()
    _lsp_state.project_config_settings_per_folder.clear()

    yield

    _lsp_state.feature_config = snap_feature
    _lsp_state.formatter_config = snap_formatter
    _lsp_state._per_folder_feature_configs.clear()
    _lsp_state._per_folder_feature_configs.update(snap_per_folder_feature)
    _lsp_state._per_folder_formatter_configs.clear()
    _lsp_state._per_folder_formatter_configs.update(snap_per_folder_formatter)
    _lsp_state.editor_config_settings.clear()
    _lsp_state.editor_config_settings.update(snap_editor)
    _lsp_state.project_config_settings.clear()
    _lsp_state.project_config_settings.update(snap_project)
    _lsp_state.editor_config_settings_per_folder.clear()
    _lsp_state.editor_config_settings_per_folder.update(snap_editor_pf)
    _lsp_state.project_config_settings_per_folder.clear()
    _lsp_state.project_config_settings_per_folder.update(snap_project_pf)


class TestConfigResolution:
    """``config_for_uri`` / ``formatter_config_for_uri`` resolver behaviour."""

    def test_falls_back_when_no_per_folder_configs(self, reset_per_folder_state):
        cfg = _lsp_state.config_for_uri("file:///tmp/foo.tcl")
        assert cfg is _lsp_state.feature_config

    def test_falls_back_when_uri_outside_every_folder(self, reset_per_folder_state):
        _lsp_state.get_or_init_folder_feature_config("file:///home/user/proj-a")
        cfg = _lsp_state.config_for_uri("file:///elsewhere/foo.tcl")
        assert cfg is _lsp_state.feature_config

    def test_picks_matching_folder(self, reset_per_folder_state):
        a = _lsp_state.get_or_init_folder_feature_config("file:///home/user/proj-a")
        b = _lsp_state.get_or_init_folder_feature_config("file:///home/user/proj-b")
        a.line_length = 160
        b.line_length = 80

        assert _lsp_state.config_for_uri("file:///home/user/proj-a/foo.tcl").line_length == 160
        assert _lsp_state.config_for_uri("file:///home/user/proj-b/foo.tcl").line_length == 80

    def test_longest_prefix_wins_for_nested_folders(self, reset_per_folder_state):
        outer = _lsp_state.get_or_init_folder_feature_config("file:///home/user/repo")
        inner = _lsp_state.get_or_init_folder_feature_config("file:///home/user/repo/sub")
        outer.line_length = 100
        inner.line_length = 200

        outer_uri = "file:///home/user/repo/foo.tcl"
        inner_uri = "file:///home/user/repo/sub/bar.tcl"
        assert _lsp_state.config_for_uri(outer_uri).line_length == 100
        assert _lsp_state.config_for_uri(inner_uri).line_length == 200

    def test_does_not_match_sibling_path_with_shared_prefix(self, reset_per_folder_state):
        """``/home/foo`` should not match a doc under ``/home/foobar/...``."""
        _lsp_state.get_or_init_folder_feature_config("file:///home/foo").line_length = 99
        cfg = _lsp_state.config_for_uri("file:///home/foobar/x.tcl")
        assert cfg.line_length != 99
        assert cfg is _lsp_state.feature_config

    def test_uri_equal_to_folder_resolves_to_that_folder(self, reset_per_folder_state):
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///home/foo")
        cfg.line_length = 77
        assert _lsp_state.config_for_uri("file:///home/foo").line_length == 77

    def test_none_uri_falls_back(self, reset_per_folder_state):
        _lsp_state.get_or_init_folder_feature_config("file:///home/foo")
        assert _lsp_state.config_for_uri(None) is _lsp_state.feature_config

    def test_formatter_config_for_uri_picks_matching_folder(self, reset_per_folder_state):
        a = _lsp_state.get_or_init_folder_formatter_config("file:///home/user/proj-a")
        b = _lsp_state.get_or_init_folder_formatter_config("file:///home/user/proj-b")
        a.max_line_length = 160
        b.max_line_length = 80

        a_uri = "file:///home/user/proj-a/foo.tcl"
        b_uri = "file:///home/user/proj-b/foo.tcl"
        assert _lsp_state.formatter_config_for_uri(a_uri).max_line_length == 160
        assert _lsp_state.formatter_config_for_uri(b_uri).max_line_length == 80

    def test_formatter_config_falls_back_outside_folders(self, reset_per_folder_state):
        _lsp_state.get_or_init_folder_formatter_config("file:///home/user/proj-a")
        cfg = _lsp_state.formatter_config_for_uri("file:///tmp/x.tcl")
        assert cfg is _lsp_state.formatter_config


class TestApplyToTarget:
    """``_apply_settings_to_target`` writes to the right config object."""

    def test_fallback_target_mutates_global_feature_config(self, reset_per_folder_state):
        _lsp_settings._apply_settings_to_target(
            "", {"style": {"lineLength": 90}, "shimmer": {"enabled": False}}
        )
        assert _lsp_state.feature_config.line_length == 90
        assert _lsp_state.feature_config.shimmer_enabled is False

    def test_folder_target_creates_isolated_config(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_settings._apply_settings_to_target(folder_uri, {"style": {"lineLength": 200}})

        # Folder config sees the new value; fallback is unchanged.
        assert _lsp_state.config_for_uri(f"{folder_uri}/foo.tcl").line_length == 200
        assert _lsp_state.feature_config.line_length == 120

    def test_two_folders_independent(self, reset_per_folder_state):
        a_uri = "file:///home/user/proj-a"
        b_uri = "file:///home/user/proj-b"
        _lsp_settings._apply_settings_to_target(a_uri, {"style": {"lineLength": 200}})
        _lsp_settings._apply_settings_to_target(b_uri, {"style": {"lineLength": 60}})

        assert _lsp_state.config_for_uri(f"{a_uri}/x.tcl").line_length == 200
        assert _lsp_state.config_for_uri(f"{b_uri}/x.tcl").line_length == 60
        # Fallback untouched.
        assert _lsp_state.feature_config.line_length == 120

    def test_formatting_settings_applied_per_folder(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_settings._apply_settings_to_target(folder_uri, {"formatting": {"maxLineLength": 160}})

        fmt_cfg = _lsp_state.formatter_config_for_uri(f"{folder_uri}/foo.tcl")
        assert fmt_cfg.max_line_length == 160
        # Fallback formatter unchanged.
        assert _lsp_state.formatter_config.max_line_length == 120

    def test_disabled_diagnostics_per_folder(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_settings._apply_settings_to_target(folder_uri, {"diagnostics": {"W111": False}})

        a_cfg = _lsp_state.config_for_uri(f"{folder_uri}/foo.tcl")
        assert "W111" in a_cfg.disabled_diagnostics
        # Fallback should not have W111 disabled (default keeps it enabled).
        assert "W111" not in _lsp_state.feature_config.disabled_diagnostics


class TestMergedSettingsPerFolder:
    """``_merged_settings`` resolves per-folder editor/project layers."""

    def test_per_folder_layers_isolated_from_fallback(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_state.editor_config_settings_per_folder[folder_uri] = {"style": {"lineLength": 99}}
        _lsp_state.editor_config_settings["style"] = {"lineLength": 50}

        merged_folder = _lsp_settings._merged_settings(folder_uri)
        merged_fallback = _lsp_settings._merged_settings("")

        assert merged_folder["style"]["lineLength"] == 99
        assert merged_fallback["style"]["lineLength"] == 50

    def test_folder_layer_falls_back_when_unset(self, reset_per_folder_state):
        """An unknown folder URI falls through to workspace-level editor/project."""
        _lsp_state.editor_config_settings["style"] = {"lineLength": 50}
        merged = _lsp_settings._merged_settings("file:///home/user/unknown")
        assert merged["style"]["lineLength"] == 50


class TestPerFolderDialect:
    """Per-folder ``tclLsp.dialect`` resolution (issue #407).

    Documents in folders configured for different dialects must see the
    folder-resolved dialect — not a single process-wide setting — when
    the LSP handler opens a ``dialect_scope`` via
    ``_state.dialect_scope_for_uri``.  The lexer's ``{*}`` expansion flag
    and ``SIGNATURES`` proxy must reflect the scoped value.
    """

    def test_resolve_dialect_for_uri_picks_folder_dialect(self, reset_per_folder_state):
        a = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")
        b = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-b")
        a.dialect = "tcl8.4"
        b.dialect = "f5-irules"

        dia_a, _ = _lsp_state.resolve_dialect_for_uri("file:///workspaces/proj-a/foo.tcl")
        dia_b, _ = _lsp_state.resolve_dialect_for_uri("file:///workspaces/proj-b/bar.tcl")
        assert dia_a == "tcl8.4"
        assert dia_b == "f5-irules"

    def test_resolve_dialect_falls_back_to_workspace(self, reset_per_folder_state):
        _lsp_state.feature_config.dialect = "tcl9.0"
        _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")

        dia, _ = _lsp_state.resolve_dialect_for_uri("file:///elsewhere/foo.tcl")
        assert dia == "tcl9.0"

    def test_dialect_scope_swaps_lexer_expand_flag(self, reset_per_folder_state):
        from core.parsing.lexer import _expand_syntax_active

        a = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-84")
        b = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-86")
        a.dialect = "tcl8.4"
        b.dialect = "tcl8.6"

        with _lsp_state.dialect_scope_for_uri("file:///workspaces/proj-84/foo.tcl"):
            assert _expand_syntax_active() is False
        with _lsp_state.dialect_scope_for_uri("file:///workspaces/proj-86/foo.tcl"):
            assert _expand_syntax_active() is True

    def test_dialect_scope_swaps_signature_table(self, reset_per_folder_state):
        """The ``SIGNATURES`` proxy returns dialect-specific entries.

        ``when`` is an iRules-only command; it must be present under the
        f5-irules folder scope and absent under a vanilla tcl8.6 scope.
        """
        from core.commands.registry.runtime import SIGNATURES

        a = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/tcl-only")
        b = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/irules-only")
        a.dialect = "tcl8.6"
        b.dialect = "f5-irules"

        with _lsp_state.dialect_scope_for_uri("file:///workspaces/tcl-only/foo.tcl"):
            assert "when" not in SIGNATURES
        with _lsp_state.dialect_scope_for_uri("file:///workspaces/irules-only/foo.tcl"):
            assert "when" in SIGNATURES

    def test_apply_feature_settings_picks_up_dialect(self, reset_per_folder_state):
        """``_apply_feature_settings`` writes ``dialect`` to the FeatureConfig.

        This is the wiring that makes per-folder ``tclLsp.dialect`` settings
        actually land on the per-folder FeatureConfig (rather than being
        silently dropped under the old "process-wide singleton" model).
        """
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")
        changed = _lsp_settings._apply_feature_settings({"dialect": "f5-irules"}, target=cfg)
        assert changed
        assert cfg.dialect == "f5-irules"

        # Invalid dialect strings are silently ignored (not written).
        cfg.dialect = "tcl8.6"
        _lsp_settings._apply_feature_settings({"dialect": "nonsense"}, target=cfg)
        assert cfg.dialect == "tcl8.6"

    def test_apply_feature_settings_picks_up_extra_commands(self, reset_per_folder_state):
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")
        changed = _lsp_settings._apply_feature_settings(
            {"extraCommands": ["my-helper", "another"]}, target=cfg
        )
        assert changed
        assert cfg.extra_commands == ("another", "my-helper")

    def test_extra_commands_suppress_w123_unknown_command(self, reset_per_folder_state):
        """An ``extraCommands`` entry must suppress W123 for that name.

        Pre-#407 the analyser built ``registry_names`` from
        ``REGISTRY.command_names(dialect)`` alone, so the per-context
        ``_extra_commands_var`` never reached the unknown-command emitter
        and ``tclLsp.extraCommands`` did not actually mark its entries as
        known.  The unknown-command check now unions ``active_extra_commands()``
        into ``registry_names``.
        """
        from core.analysis import Analyser
        from core.common.dialect import dialect_scope

        src = "cmd_alpha foo bar\n"
        with dialect_scope("tcl8.6", extra_commands=["cmd_alpha"]):
            diags = Analyser().analyse(src).diagnostics
        w123 = [d for d in diags if d.code == "W123"]
        assert w123 == [], (
            f"W123 should be suppressed when name is in extra_commands; got {[d.message for d in w123]}"
        )

        with dialect_scope("tcl8.6", extra_commands=[]):
            diags = Analyser().analyse(src).diagnostics
        w123 = [d for d in diags if d.code == "W123"]
        assert len(w123) == 1, (
            f"W123 should still fire when name is not in extra_commands; got {len(w123)}"
        )
        assert "cmd_alpha" in w123[0].message

    def test_apply_feature_settings_sets_explicit_flag(self, reset_per_folder_state):
        """Setting ``tclLsp.dialect`` flips ``dialect_explicitly_set`` on the target."""
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")
        assert cfg.dialect_explicitly_set is False
        _lsp_settings._apply_feature_settings({"dialect": "tcl8.4"}, target=cfg)
        assert cfg.dialect_explicitly_set is True

        # Explicit null clears the override AND the flag.
        _lsp_settings._apply_feature_settings({"dialect": None}, target=cfg)
        assert cfg.dialect_explicitly_set is False

    # Race matrix for issue #407: every per-folder setting that bakes into
    # the cached ``AnalysisResult`` is exercised by ``did_open`` racing the
    # ``workspace/configuration`` pull.  Each row drives the apply ordering
    # directly (no LSP roundtrip), asserts the wrong diagnostic fires before
    # the pull, then asserts ``_apply_merged_settings_now`` re-analyses and
    # clears it once the per-folder setting arrives.  Adding a new
    # dialect-sensitive check?  Add a row here.
    @pytest.mark.parametrize(
        ("scenario", "source", "late_settings", "diagnostic_code"),
        [
            pytest.param(
                "dialect-flips-to-irules",
                "if { [active_members http_pool] >= 2 } {\n    puts \"ok\"\n}\n",
                {"dialect": "f5-irules"},
                "W002",
                id="dialect-W002-active_members",
            ),
            pytest.param(
                "non-ascii-mode-flips-off",
                'set greeting "“hello”"\n',
                {"style": {"nonAscii": "off"}},
                "W108",
                id="nonAscii-W108-smart-quotes",
            ),
        ],
    )
    def test_late_per_folder_setting_invalidates_cached_analysis(
        self,
        reset_per_folder_state,
        scenario,
        source,
        late_settings,
        diagnostic_code,
    ):
        """Per-folder dialect/non_ascii arriving after ``did_open`` clears stale diagnostics.

        Reproduces issue #407 follow-up: at session start ``did_open`` for
        the active editor races against the asynchronous
        ``workspace/configuration`` pull.  The first analyse runs under the
        workspace-fallback settings and bakes dialect-sensitive checks
        (W002 for iRules-only commands, W108 for non-ASCII, etc) into the
        cached analysis.  When the pull callback later applies the folder's
        real settings the workspace-level ``configure_signatures`` call is
        a no-op so the old ``signatures_changed``-only re-analyse trigger
        never fires — the user keeps seeing the stale warnings even though
        the per-folder config has resolved to the correct value.
        """
        import lsp.diagnostics_pipeline as _dp

        folder = "file:///workspaces/proj-b"
        file_uri = f"{folder}/test.tcl"
        _lsp_state.get_or_init_folder_feature_config(folder)
        _lsp_settings._apply_merged_settings_now()

        _lsp_state.workspace_state.open(file_uri, source, 1, language_id="tcl", analyse=False)

        captured: dict[str, list] = {}
        orig_publish = _dp._publish_diags_to_client

        def _capture(uri, diags, version=None):
            captured[uri] = list(diags)

        _dp._publish_diags_to_client = _capture
        _dp.configure(
            type("S", (), {"text_document_publish_diagnostics": lambda *a, **k: None})()
        )
        try:
            _dp._publish_diagnostics_sync(file_uri, source, 1)
            initial = [
                d for d in captured.get(file_uri, []) if d.code == diagnostic_code
            ]
            assert initial, (
                f"{scenario} precondition: expected {diagnostic_code} under "
                "the workspace-fallback settings (precondition for repro)"
            )

            # Pull arrives with the folder's real settings.
            _lsp_state.editor_config_settings_per_folder[folder] = late_settings
            _lsp_settings._apply_merged_settings_now()

            after = [
                d for d in captured.get(file_uri, []) if d.code == diagnostic_code
            ]
            assert after == [], (
                f"{scenario}: {diagnostic_code} must clear after the per-folder "
                f"setting {late_settings!r} applies; got: "
                + ", ".join(d.message for d in after)
            )
        finally:
            _dp._publish_diags_to_client = orig_publish


class TestPerFolderNonAscii:
    """Per-folder ``tclLsp.style.nonAscii`` resolution (issue #407)."""

    def test_apply_feature_settings_writes_non_ascii_mode(self, reset_per_folder_state):
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")
        changed = _lsp_settings._apply_feature_settings(
            {"style": {"nonAscii": "strict"}}, target=cfg
        )
        assert changed
        assert cfg.non_ascii_mode == "strict"

    def test_non_ascii_mode_scope_swaps_module_var(self, reset_per_folder_state):
        from core.analysis.checks._style import _non_ascii_mode_var, non_ascii_mode_scope

        baseline = _non_ascii_mode_var.get()
        with non_ascii_mode_scope("off"):
            assert _non_ascii_mode_var.get() == "off"
        assert _non_ascii_mode_var.get() == baseline


class TestPerFolderPackageResolver:
    """Per-folder ``tclLsp.libraryPaths`` resolution (issue #407)."""

    def test_package_resolver_for_uri_falls_back(self, reset_per_folder_state):
        resolver = _lsp_state.package_resolver_for_uri("file:///elsewhere/foo.tcl")
        assert resolver is _lsp_state.package_resolver

    def test_per_folder_resolver_returned_for_matching_uri(self, reset_per_folder_state):
        folder = "file:///workspaces/proj-a"
        folder_resolver = _lsp_state.get_or_init_folder_package_resolver(folder)
        # The resolver is only selected when the folder's FeatureConfig
        # has live ``library_paths`` — otherwise the stale-resolver guard
        # falls back to the workspace resolver.
        _lsp_state.get_or_init_folder_feature_config(folder).library_paths = ("/opt/tcllib",)
        assert _lsp_state.package_resolver_for_uri(f"{folder}/foo.tcl") is folder_resolver
        assert _lsp_state.package_resolver_for_uri("file:///elsewhere/foo.tcl") is (
            _lsp_state.package_resolver
        )

    def test_per_folder_resolver_skipped_when_library_paths_cleared(self, reset_per_folder_state):
        """When ``library_paths`` is unset the workspace resolver is used."""
        folder = "file:///workspaces/proj-a"
        _lsp_state.get_or_init_folder_package_resolver(folder)
        cfg = _lsp_state.get_or_init_folder_feature_config(folder)
        cfg.library_paths = ("/opt/tcllib",)
        # Now clear it.
        cfg.library_paths = None
        assert _lsp_state.package_resolver_for_uri(f"{folder}/foo.tcl") is (
            _lsp_state.package_resolver
        )

    def test_drop_folder_configs_prunes_resolver(self, reset_per_folder_state):
        folder = "file:///workspaces/proj-a"
        _lsp_state.get_or_init_folder_package_resolver(folder)
        assert folder in _lsp_state._per_folder_package_resolvers
        _lsp_state.drop_folder_configs(folder)
        assert folder not in _lsp_state._per_folder_package_resolvers

    def test_all_package_resolvers_includes_fallback_and_folders(self, reset_per_folder_state):
        folder = "file:///workspaces/proj-a"
        folder_resolver = _lsp_state.get_or_init_folder_package_resolver(folder)
        resolvers = _lsp_state.all_package_resolvers()
        assert _lsp_state.package_resolver in resolvers
        assert folder_resolver in resolvers

    def test_apply_feature_settings_writes_library_paths(self, reset_per_folder_state):
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///workspaces/proj-a")
        changed = _lsp_settings._apply_feature_settings(
            {"libraryPaths": ["/opt/tcllib", "/usr/local/lib/tcl"]}, target=cfg
        )
        assert changed
        assert cfg.library_paths == ("/opt/tcllib", "/usr/local/lib/tcl")


class TestDropFolderConfigs:
    def test_drop_clears_all_layers(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_state.get_or_init_folder_feature_config(folder_uri)
        _lsp_state.get_or_init_folder_formatter_config(folder_uri)
        _lsp_state.editor_config_settings_per_folder[folder_uri] = {"x": 1}
        _lsp_state.project_config_settings_per_folder[folder_uri] = {"y": 2}

        _lsp_state.drop_folder_configs(folder_uri)

        assert folder_uri not in _lsp_state._per_folder_feature_configs
        assert folder_uri not in _lsp_state._per_folder_formatter_configs
        assert folder_uri not in _lsp_state.editor_config_settings_per_folder
        assert folder_uri not in _lsp_state.project_config_settings_per_folder
