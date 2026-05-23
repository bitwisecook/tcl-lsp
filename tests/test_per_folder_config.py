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
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

import server.settings as _lsp_settings
import server.state as _lsp_state
from server.feature_config import FeatureConfig
from tooling.formatter import FormatterConfig


def _install_diag_capture(captured: dict) -> tuple[Any, Any]:
    """Stub the diagnostics-pipeline publisher so tests can inspect diagnostics.

    Returns the original ``_publish_diags_to_client`` value so the caller
    can restore it in a ``finally`` block.  Centralised here so the
    ``# type: ignore`` for the deliberate signature-mismatch only appears
    once instead of in every test method.
    """
    import server.diagnostics_pipeline as _dp

    orig_publish = _dp._publish_diags_to_client
    orig_server = _dp._server

    def _capture(uri: str, diagnostics: list, version: int | None = None) -> None:
        captured[uri] = list(diagnostics)

    # Stubbing the module-level publisher so we can inspect what would have
    # been pushed.  ty correctly notices the diagnostic-list element type
    # widens from Diagnostic to Unknown via the test's loose list arg —
    # acceptable in a test stub.
    _dp._publish_diags_to_client = _capture  # type: ignore[assignment]  # ty: ignore[invalid-assignment]

    class _DummyServer:
        @staticmethod
        def text_document_publish_diagnostics(*_a: object, **_kw: object) -> None: ...

    # ``configure`` requires a real ``LanguageServer``; tests only use the
    # ``text_document_publish_diagnostics`` shim, so a structural stand-in
    # is fine.
    _dp.configure(_DummyServer())  # type: ignore[arg-type]  # ty: ignore[invalid-argument-type]
    return (orig_publish, orig_server)


def _restore_diag_capture(orig: tuple[Any, Any]) -> None:
    import server.diagnostics_pipeline as _dp

    orig_publish, orig_server = orig
    _dp._publish_diags_to_client = orig_publish  # type: ignore[assignment]
    _dp._server = orig_server


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
        from compiler.parsing.lexer import _expand_syntax_active

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
        from compiler.registry.runtime import SIGNATURES

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
        from analyser import Analyser
        from compiler.registry.dialect import dialect_scope

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
                'if { [active_members http_pool] >= 2 } {\n    puts "ok"\n}\n',
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
        import server.diagnostics_pipeline as _dp

        folder = "file:///workspaces/proj-b"
        file_uri = f"{folder}/test.tcl"
        _lsp_state.get_or_init_folder_feature_config(folder)
        _lsp_settings._apply_merged_settings_now()

        _lsp_state.workspace_state.open(file_uri, source, 1, language_id="tcl", analyse=False)

        captured: dict[str, list] = {}
        orig_publish = _install_diag_capture(captured)
        try:
            _dp._publish_diagnostics_sync(file_uri, source, 1)
            initial = [d for d in captured.get(file_uri, []) if d.code == diagnostic_code]
            assert initial, (
                f"{scenario} precondition: expected {diagnostic_code} under "
                "the workspace-fallback settings (precondition for repro)"
            )

            # Pull arrives with the folder's real settings.
            _lsp_state.editor_config_settings_per_folder[folder] = late_settings
            _lsp_settings._apply_merged_settings_now()

            after = [d for d in captured.get(file_uri, []) if d.code == diagnostic_code]
            assert after == [], (
                f"{scenario}: {diagnostic_code} must clear after the per-folder "
                f"setting {late_settings!r} applies; got: " + ", ".join(d.message for d in after)
            )
        finally:
            _restore_diag_capture(orig_publish)


class TestAnalyserOptInDiagnostics:
    """Opt-in (``default=False``) diagnostics must reach the emit path.

    Codes that have ``default=False`` registered via ``@diag(default=False)``
    -- currently W123 (unresolved command) and W242 (loop termination not
    provable) -- are emitted from the analyser only when the code is *not*
    in the analyser's ``_disabled_diagnostics`` set.  The LSP path used to
    hardcode ``Analyser(disabled_diagnostics=default_disabled_diagnostics())``
    which permanently disabled them: the user's ``tclLsp.diagnostics.W123:
    true`` updated ``feature_config`` but the analyser ignored it.  The
    fix routes ``cfg.disabled_diagnostics`` through to the Analyser via
    ``_effective_disabled_diagnostics(uri)``.
    """

    def test_w123_does_not_fire_with_default_config(self, reset_per_folder_state):
        """Sanity: W123 stays opt-in (off by default in the LSP path)."""
        import server.diagnostics_pipeline as _dp

        folder = "file:///workspaces/proj"
        _lsp_state.get_or_init_folder_feature_config(folder)
        _lsp_settings._apply_merged_settings_now()

        file_uri = f"{folder}/file.tcl"
        source = "my_unknown_helper foo bar\n"
        _lsp_state.workspace_state.open(file_uri, source, 1, language_id="tcl", analyse=False)

        captured: dict[str, list] = {}
        orig = _install_diag_capture(captured)
        try:
            _dp._publish_diagnostics_sync(file_uri, source, 1)
            w123 = [d for d in captured.get(file_uri, []) if d.code == "W123"]
            assert w123 == [], "W123 must stay off by default; got: " + ", ".join(
                d.message for d in w123
            )
        finally:
            _restore_diag_capture(orig)

    def test_w123_fires_when_user_enables_it(self, reset_per_folder_state):
        """``tclLsp.diagnostics.W123: true`` makes the analyser emit W123."""
        import server.diagnostics_pipeline as _dp

        folder = "file:///workspaces/proj"
        _lsp_state.get_or_init_folder_feature_config(folder)
        _lsp_state.editor_config_settings_per_folder[folder] = {"diagnostics": {"W123": True}}
        _lsp_settings._apply_merged_settings_now()

        file_uri = f"{folder}/file.tcl"
        source = "my_unknown_helper foo bar\n"
        _lsp_state.workspace_state.open(file_uri, source, 1, language_id="tcl", analyse=False)

        captured: dict[str, list] = {}
        orig = _install_diag_capture(captured)
        try:
            _dp._publish_diagnostics_sync(file_uri, source, 1)
            w123 = [d for d in captured.get(file_uri, []) if d.code == "W123"]
            assert len(w123) == 1, (
                f"W123 must fire when opted in via folder config; got {len(w123)} "
                f"diagnostics: {[(d.code, d.message) for d in captured.get(file_uri, [])]}"
            )
            assert "my_unknown_helper" in w123[0].message
        finally:
            _restore_diag_capture(orig)

    def test_toggling_w123_at_runtime_re_analyses(self, reset_per_folder_state):
        """Flipping ``tclLsp.diagnostics.W123`` invalidates the cached analysis.

        The analyser bakes W123 into ``AnalysisResult.diagnostics`` at
        analyse time -- a post-filter can't *add* it after the fact.  So
        when the user enables W123 the cached analysis must be re-run.
        ``_apply_merged_settings_now`` tracks per-doc resolved
        ``disabled_diagnostics`` and forces ``force_reanalyse=True`` for
        any doc whose effective set changed.
        """
        import server.diagnostics_pipeline as _dp

        folder = "file:///workspaces/proj"
        _lsp_state.get_or_init_folder_feature_config(folder)
        _lsp_settings._apply_merged_settings_now()

        file_uri = f"{folder}/file.tcl"
        source = "my_unknown_helper foo\n"
        _lsp_state.workspace_state.open(file_uri, source, 1, language_id="tcl", analyse=False)

        captured: dict[str, list] = {}
        orig = _install_diag_capture(captured)
        try:
            _dp._publish_diagnostics_sync(file_uri, source, 1)
            initial = [d for d in captured.get(file_uri, []) if d.code == "W123"]
            assert initial == [], "precondition: W123 off by default"

            # User opts in to W123 via folder settings.
            _lsp_state.editor_config_settings_per_folder[folder] = {"diagnostics": {"W123": True}}
            _lsp_settings._apply_merged_settings_now()

            after = [d for d in captured.get(file_uri, []) if d.code == "W123"]
            assert len(after) == 1, (
                f"W123 must fire after the user opts in via folder config; "
                f"got {len(after)} diagnostics, all codes: "
                + ", ".join(d.code for d in captured.get(file_uri, []))
            )

            # Toggling back off must clear W123 again.
            _lsp_state.editor_config_settings_per_folder[folder] = {"diagnostics": {"W123": False}}
            _lsp_settings._apply_merged_settings_now()
            cleared = [d for d in captured.get(file_uri, []) if d.code == "W123"]
            assert cleared == [], "W123 must clear after the user opts back out; got: " + ", ".join(
                d.message for d in cleared
            )
        finally:
            _restore_diag_capture(orig)


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
        from analyser.checks._style import _non_ascii_mode_var, non_ascii_mode_scope

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


class TestWorkspacePushPropagation:
    """Workspace-level ``workspace/didChangeConfiguration`` push propagates
    into per-folder editor caches so per-folder ``FeatureConfig`` attributes
    update synchronously, without waiting for the async
    ``workspace/configuration`` pull (the gap that produced the three
    ``configSettings.test.ts`` flakes on PR #415).

    Companion to ``TestPerFolderDialect.test_late_per_folder_setting_invalidates_cached_analysis``
    on the #407 branch: that matrix covers settings baked into
    ``AnalysisResult`` (W002, W108).  This one covers settings read at
    handler entry via ``config_for_uri(uri).X_enabled``, which bypass the
    analysis cache entirely and so are *not* fixed by #407's
    force-re-analyse trigger.
    """

    @pytest.mark.parametrize(
        ("baseline_value", "push_payload", "attr", "expected"),
        [
            pytest.param(
                True,
                {"features": {"references": False}},
                "references_enabled",
                False,
                id="features.references=false",
            ),
            pytest.param(
                True,
                {"features": {"folding": False}},
                "folding_enabled",
                False,
                id="features.folding=false",
            ),
            pytest.param(
                True,
                {"features": {"selectionRange": False}},
                "selection_range_enabled",
                False,
                id="features.selectionRange=false",
            ),
            pytest.param(
                True,
                {"features": {"hover": False}},
                "hover_enabled",
                False,
                id="features.hover=false",
            ),
            pytest.param(
                120,
                {"style": {"lineLength": 200}},
                "line_length",
                200,
                id="style.lineLength=200",
            ),
        ],
    )
    def test_push_propagates_to_per_folder_cfg_before_pull(
        self, reset_per_folder_state, baseline_value, push_payload, attr, expected
    ):
        """Without the propagation fix the per-folder cfg stays at the
        previous-pull value until the next ``workspace/configuration``
        response arrives.  With the fix the push reaches the per-folder
        cfg synchronously via the leading-edge apply."""
        folder = "file:///home/user/proj-a"
        doc = f"{folder}/x.tcl"

        # Build the initial-pull payload from the same shape as ``push_payload``,
        # substituting ``baseline_value`` for the leaf so the per-folder cfg
        # starts on the opposite side of the push.
        def _with_leaf(payload, leaf):
            ((section, inner),) = payload.items()
            ((key, _),) = inner.items()
            return {section: {key: leaf}}

        baseline_settings = _with_leaf(push_payload, baseline_value)
        _lsp_state.editor_config_settings.update(baseline_settings)
        _lsp_state.editor_config_settings_per_folder[folder] = dict(baseline_settings)
        _lsp_settings._apply_merged_settings_now()
        assert getattr(_lsp_state.config_for_uri(doc), attr) == baseline_value

        # Drive the push path directly, leaving
        # ``editor_config_settings_per_folder`` untouched by the would-be
        # pull -- this is the race window the test guards.
        _lsp_settings._propagate_push_to_folder_caches(push_payload)
        _lsp_settings._apply_all_settings(push_payload, folder_uri="")
        _lsp_settings._apply_merged_settings_now()

        # Workspace fallback always updates -- regression guard.
        assert getattr(_lsp_state.feature_config, attr) == expected
        # The interesting bit: per-folder cfg flipped without waiting for
        # the pull.  Pre-fix this assertion fails.
        actual = getattr(_lsp_state.config_for_uri(doc), attr)
        assert actual == expected, (
            f"push of {push_payload!r} did not propagate to per-folder "
            f"cfg; per-folder.{attr} = {actual!r}, expected {expected!r}.  "
            f"Without the fix the previous-pull value sticks until the "
            f"matching pull response arrives."
        )

    def test_push_preserves_unrelated_folder_overrides(self, reset_per_folder_state):
        """A workspace-level features push must not clobber a folder's
        dialect / extraCommands override.  Those are per-folder concerns
        that the pull refreshes; the push only carries the workspace-
        merged value, and our propagation is keyed per top-level
        setting so unrelated keys in the folder cache survive untouched.
        """
        folder = "file:///home/user/proj-a"
        doc = f"{folder}/x.tcl"

        folder_settings = {
            "features": {"references": True},
            "dialect": "f5-irules",
            "extraCommands": ["folder-helper"],
        }
        _lsp_state.editor_config_settings.update(
            {"features": {"references": True}, "dialect": "tcl8.6"}
        )
        _lsp_state.editor_config_settings_per_folder[folder] = dict(folder_settings)
        _lsp_settings._apply_merged_settings_now()

        cfg = _lsp_state.config_for_uri(doc)
        assert cfg.dialect == "f5-irules"
        assert list(cfg.extra_commands or ()) == ["folder-helper"]
        assert cfg.references_enabled is True

        push_payload = {"features": {"references": False}}
        _lsp_settings._propagate_push_to_folder_caches(push_payload)
        _lsp_settings._apply_all_settings(push_payload, folder_uri="")
        _lsp_settings._apply_merged_settings_now()

        cfg = _lsp_state.config_for_uri(doc)
        assert cfg.dialect == "f5-irules", (
            f"features push clobbered folder dialect override; got {cfg.dialect!r}"
        )
        assert list(cfg.extra_commands or ()) == ["folder-helper"], (
            f"features push clobbered folder extraCommands override; got {cfg.extra_commands!r}"
        )
        assert cfg.references_enabled is False

    def test_push_with_no_per_folder_caches_is_noop(self, reset_per_folder_state):
        """Pushes arriving before any pull has populated a per-folder
        editor cache must update the workspace fallback and not crash."""
        assert _lsp_state.editor_config_settings_per_folder == {}
        push_payload = {"features": {"references": False}}
        _lsp_settings._propagate_push_to_folder_caches(push_payload)
        _lsp_settings._apply_all_settings(push_payload, folder_uri="")
        _lsp_settings._apply_merged_settings_now()

        assert _lsp_state.feature_config.references_enabled is False
        assert _lsp_state.editor_config_settings_per_folder == {}

    def test_deep_merge_preserves_sibling_features_keys(self, reset_per_folder_state):
        """A push that touches one feature toggle must not erase the
        siblings living in the same per-folder cache entry."""
        folder = "file:///home/user/proj-a"
        _lsp_state.editor_config_settings_per_folder[folder] = {
            "features": {"references": True, "hover": False, "folding": True}
        }
        _lsp_settings._propagate_push_to_folder_caches({"features": {"references": False}})
        assert _lsp_state.editor_config_settings_per_folder[folder] == {
            "features": {"references": False, "hover": False, "folding": True}
        }

    @pytest.mark.parametrize(
        "per_folder_only_key",
        ["dialect", "extraCommands", "extra_commands", "libraryPaths", "library_paths"],
    )
    def test_per_folder_only_keys_are_not_propagated(
        self, reset_per_folder_state, per_folder_only_key
    ):
        """The push carries workspace-level values for keys that may be
        overridden per-folder.  Propagating those keys into the per-folder
        cache would clobber the folder's existing override AND flip
        ``FeatureConfig.dialect_explicitly_set`` via
        ``_apply_feature_settings``'s ``looks_inherited`` branch -- which
        was exactly the regression the ``Dialect Detection defaults .tcl
        files to Tcl 8.6`` VS Code test caught during the initial draft of
        this race fix on PR #415.  Skip them in the deep-merge so the
        ``workspace/configuration`` pull stays the authoritative source.
        """
        folder = "file:///home/user/proj-a"
        # Folder pull settled with a per-folder override on the
        # per-folder-only key.
        sample_value: object
        if per_folder_only_key == "dialect":
            sample_value = "f5-irules"
        else:
            sample_value = ["folder-helper"]
        original_payload = {per_folder_only_key: sample_value}
        _lsp_state.editor_config_settings_per_folder[folder] = dict(original_payload)

        # Workspace-level push tries to push a different value for the same key.
        workspace_value: object
        if per_folder_only_key == "dialect":
            workspace_value = "tcl8.6"
        else:
            workspace_value = []
        _lsp_settings._propagate_push_to_folder_caches({per_folder_only_key: workspace_value})

        # Folder cache keeps its original per-folder value untouched --
        # the pull will refresh it with the authoritative per-folder
        # merged value.
        assert _lsp_state.editor_config_settings_per_folder[folder] == original_payload

    def test_set_server_dialect_push_does_not_flip_folder_explicit(self, reset_per_folder_state):
        """``setServerDialect("f5-irules")`` fires a dialect-only push.
        Before the fix that excludes per-folder-only keys, the deep-merge
        wrote ``dialect`` into every folder cache, and the subsequent
        ``_apply_merged_settings_now`` then flipped per-folder
        ``dialect_explicitly_set`` to True via the ``looks_inherited``
        branch -- which blocked the iRules / iApps auto-switch in
        ``did_open`` and made the ``Dialect Detection defaults .tcl
        files to Tcl 8.6`` VS Code test fail.  Drive the same path
        directly and assert per-folder ``dialect_explicitly_set`` stays
        False.
        """
        folder = "file:///home/user/proj-a"
        doc = f"{folder}/x.tcl"

        # Simulate a folder seeded by did_open's auto-switch: dialect is
        # set, but explicit stays False.
        folder_cfg = _lsp_state.get_or_init_folder_feature_config(folder)
        folder_cfg.dialect = "f5-irules"
        assert folder_cfg.dialect_explicitly_set is False
        # Seed the per-folder cache from a prior pull so the propagation
        # path's "folders to update" loop sees this folder.
        _lsp_state.editor_config_settings_per_folder[folder] = {"features": {"references": True}}

        # setServerDialect-style push: hand-crafted dialect-only payload.
        push_payload = {"dialect": "f5-irules"}
        _lsp_settings._propagate_push_to_folder_caches(push_payload)
        _lsp_settings._apply_all_settings(push_payload, folder_uri="")
        _lsp_settings._apply_merged_settings_now()

        cfg = _lsp_state.config_for_uri(doc)
        assert cfg.dialect == "f5-irules"
        assert cfg.dialect_explicitly_set is False, (
            "setServerDialect's dialect-only push must not flip "
            "per-folder dialect_explicitly_set -- doing so blocks "
            "did_open's iRules / iApps auto-switch (see #415)."
        )
