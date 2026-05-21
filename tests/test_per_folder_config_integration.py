"""Integration tests for per-folder config pull/dispatch (issue #230).

Beyond the unit-level resolver coverage in
``test_per_folder_config.py``, these tests pin the wiring between:

- ``_pull_and_apply_configuration`` and the pygls
  ``workspace_configuration`` request — request shape (one item per
  folder + an unscoped fallback last) and result-handling order.
- ``did_change_configuration`` dispatch (with and without a push
  payload).
- ``on_did_change_workspace_folders`` add/remove flows including
  ``drop_folder_configs`` and ``.tcl-lsp.ini`` loading.

A pygls ``LanguageServer`` is mocked: each test injects a fake whose
``workspace.workspace_folders`` returns a chosen list and whose
``workspace_configuration`` records calls and synchronously fires the
callback with caller-supplied results.
"""

from __future__ import annotations

import sys
from collections.abc import Callable
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import MagicMock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest
from lsprotocol import types

import lsp.lifecycle as _lifecycle
import lsp.settings as _lsp_settings
import lsp.state as _lsp_state
from core.formatting import FormatterConfig
from lsp.feature_config import FeatureConfig


@pytest.fixture
def reset_state(monkeypatch):
    """Snapshot/restore the module-level state mutated by these tests."""
    snap_feature = _lsp_state.feature_config
    snap_formatter = _lsp_state.formatter_config
    snap_per_folder_feature = dict(_lsp_state._per_folder_feature_configs)
    snap_per_folder_formatter = dict(_lsp_state._per_folder_formatter_configs)
    snap_editor_pf = dict(_lsp_state.editor_config_settings_per_folder)
    snap_project_pf = dict(_lsp_state.project_config_settings_per_folder)
    snap_editor = dict(_lsp_state.editor_config_settings)
    snap_project = dict(_lsp_state.project_config_settings)

    _lsp_state.feature_config = FeatureConfig()
    _lsp_state.formatter_config = FormatterConfig()
    _lsp_state._per_folder_feature_configs.clear()
    _lsp_state._per_folder_formatter_configs.clear()
    _lsp_state.editor_config_settings_per_folder.clear()
    _lsp_state.project_config_settings_per_folder.clear()
    _lsp_state.editor_config_settings.clear()
    _lsp_state.project_config_settings.clear()

    yield

    _lsp_state.feature_config = snap_feature
    _lsp_state.formatter_config = snap_formatter
    _lsp_state._per_folder_feature_configs.clear()
    _lsp_state._per_folder_feature_configs.update(snap_per_folder_feature)
    _lsp_state._per_folder_formatter_configs.clear()
    _lsp_state._per_folder_formatter_configs.update(snap_per_folder_formatter)
    _lsp_state.editor_config_settings_per_folder.clear()
    _lsp_state.editor_config_settings_per_folder.update(snap_editor_pf)
    _lsp_state.project_config_settings_per_folder.clear()
    _lsp_state.project_config_settings_per_folder.update(snap_project_pf)
    _lsp_state.editor_config_settings.clear()
    _lsp_state.editor_config_settings.update(snap_editor)
    _lsp_state.project_config_settings.clear()
    _lsp_state.project_config_settings.update(snap_project)


def _make_fake_server(folder_uris: list[str]):
    """Return a pygls-shaped MagicMock with the given workspace folders.

    pygls 2.x exposes ``Workspace.folders`` as a dict keyed by URI.
    Older releases used a ``workspace_folders`` list — populate both so
    the production code's compatibility shim can be exercised.

    ``workspace_configuration`` records its call args and exposes a
    helper to fire the supplied callback synchronously, mirroring how
    pygls dispatches the response from the client.
    """
    server = MagicMock(name="LanguageServer")
    folder_objs = [SimpleNamespace(uri=uri, name=uri.rsplit("/", 1)[-1]) for uri in folder_uris]
    server.workspace.folders = {uri: f for uri, f in zip(folder_uris, folder_objs, strict=False)}
    server.workspace.workspace_folders = folder_objs

    captured: dict[str, object] = {}

    def _record(params, callback=None):
        captured["params"] = params
        captured["callback"] = callback

    server.workspace_configuration.side_effect = _record
    server._captured = captured
    return server


# _pull_and_apply_configuration


class TestPullAndApplyConfiguration:
    def test_request_one_item_per_folder_plus_unscoped_fallback(self, reset_state, monkeypatch):
        folder_a = "file:///home/user/proj-a"
        folder_b = "file:///home/user/proj-b"
        server = _make_fake_server([folder_a, folder_b])
        monkeypatch.setattr(_lsp_settings, "_server", server)

        _lsp_settings._pull_and_apply_configuration()

        params = server._captured["params"]
        assert isinstance(params, types.ConfigurationParams)
        assert len(params.items) == 3
        assert params.items[0].scope_uri == folder_a
        assert params.items[0].section == "tclLsp"
        assert params.items[1].scope_uri == folder_b
        assert params.items[1].section == "tclLsp"
        # Last item is the unscoped fallback.
        assert params.items[2].scope_uri is None
        assert params.items[2].section == "tclLsp"

    def test_no_workspace_folders_pulls_only_fallback(self, reset_state, monkeypatch):
        server = _make_fake_server([])
        monkeypatch.setattr(_lsp_settings, "_server", server)

        _lsp_settings._pull_and_apply_configuration()

        params = server._captured["params"]
        assert len(params.items) == 1
        assert params.items[0].scope_uri is None

    def test_callback_routes_per_folder_results_to_their_targets(self, reset_state, monkeypatch):
        folder_a = "file:///home/user/proj-a"
        folder_b = "file:///home/user/proj-b"
        server = _make_fake_server([folder_a, folder_b])
        monkeypatch.setattr(_lsp_settings, "_server", server)
        # Avoid debounced re-application reaching into asyncio.
        monkeypatch.setattr(_lsp_settings, "_schedule_apply_merged", lambda: None)

        _lsp_settings._pull_and_apply_configuration()
        callback = server._captured["callback"]
        assert callback is not None

        callback(
            [
                {"style": {"lineLength": 200}},  # folder A
                {"style": {"lineLength": 60}},  # folder B
                {"style": {"lineLength": 100}},  # fallback
            ]
        )

        assert _lsp_state.editor_config_settings_per_folder[folder_a] == {
            "style": {"lineLength": 200}
        }
        assert _lsp_state.editor_config_settings_per_folder[folder_b] == {
            "style": {"lineLength": 60}
        }
        # Fallback writes to the legacy attribute, not the per-folder map.
        assert _lsp_state.editor_config_settings == {"style": {"lineLength": 100}}

    def test_callback_handles_empty_result(self, reset_state, monkeypatch):
        server = _make_fake_server(["file:///home/user/proj-a"])
        monkeypatch.setattr(_lsp_settings, "_server", server)
        monkeypatch.setattr(_lsp_settings, "_schedule_apply_merged", lambda: None)

        _lsp_settings._pull_and_apply_configuration()
        callback = server._captured["callback"]
        # Empty/None results are valid client responses — they must not crash
        # and must not register any per-folder config.
        callback([])
        callback(None)
        assert "file:///home/user/proj-a" not in _lsp_state.editor_config_settings_per_folder

    def test_callback_skips_non_dict_items(self, reset_state, monkeypatch):
        folder_a = "file:///home/user/proj-a"
        server = _make_fake_server([folder_a])
        monkeypatch.setattr(_lsp_settings, "_server", server)
        monkeypatch.setattr(_lsp_settings, "_schedule_apply_merged", lambda: None)

        _lsp_settings._pull_and_apply_configuration()
        callback = server._captured["callback"]
        # Some clients return null for an unconfigured section.
        callback([None, None])

        assert folder_a not in _lsp_state.editor_config_settings_per_folder
        assert _lsp_state.editor_config_settings == {}


# did_change_configuration handler dispatch


class TestDidChangeConfigurationDispatch:
    def test_empty_payload_triggers_pull(self, reset_state, monkeypatch):
        called: list[bool] = []
        monkeypatch.setattr(
            _lsp_settings,
            "_pull_and_apply_configuration",
            lambda: called.append(True),
        )

        # Build a minimal pygls-shaped server and grab the registered handler.
        server = MagicMock()
        registered: dict[str, Callable] = {}

        def _feature(name, *_args, **_kwargs):
            def _decorator(fn):
                registered[name] = fn
                return fn

            return _decorator

        server.feature.side_effect = _feature
        _lsp_settings.register(server)

        handler = registered[types.WORKSPACE_DID_CHANGE_CONFIGURATION]
        handler(types.DidChangeConfigurationParams(settings=None))
        assert called == [True]

    def test_payload_applies_to_fallback_then_pulls(self, reset_state, monkeypatch):
        called: list[bool] = []
        monkeypatch.setattr(
            _lsp_settings,
            "_pull_and_apply_configuration",
            lambda: called.append(True),
        )
        monkeypatch.setattr(_lsp_settings, "_schedule_apply_merged", lambda: None)

        server = MagicMock()
        registered: dict[str, Callable] = {}

        def _feature(name, *_args, **_kwargs):
            def _decorator(fn):
                registered[name] = fn
                return fn

            return _decorator

        server.feature.side_effect = _feature
        _lsp_settings.register(server)

        handler = registered[types.WORKSPACE_DID_CHANGE_CONFIGURATION]
        handler(
            types.DidChangeConfigurationParams(settings={"tclLsp": {"style": {"lineLength": 95}}})
        )

        # Fallback layer was updated from the push payload.
        assert _lsp_state.editor_config_settings == {"style": {"lineLength": 95}}
        # And we still pulled afterwards to populate per-folder configs.
        assert called == [True]


# on_did_change_workspace_folders


def _folders_event(
    added: list[str] | None = None, removed: list[str] | None = None
) -> types.DidChangeWorkspaceFoldersParams:
    """Build a minimal DidChangeWorkspaceFoldersParams."""
    added_folders = [types.WorkspaceFolder(uri=u, name=u.rsplit("/", 1)[-1]) for u in (added or [])]
    removed_folders = [
        types.WorkspaceFolder(uri=u, name=u.rsplit("/", 1)[-1]) for u in (removed or [])
    ]
    return types.DidChangeWorkspaceFoldersParams(
        event=types.WorkspaceFoldersChangeEvent(added=added_folders, removed=removed_folders)
    )


class TestDidChangeWorkspaceFolders:
    def test_added_folder_initialises_per_folder_configs(self, reset_state, monkeypatch, tmp_path):
        folder_uri = f"file://{tmp_path}/proj-a"
        monkeypatch.setattr(_lsp_settings, "_pull_and_apply_configuration", lambda: None)
        monkeypatch.setattr("core.common.user_config.load_project_config", lambda _path: None)

        _lifecycle.on_did_change_workspace_folders(_folders_event(added=[folder_uri]))

        assert folder_uri in _lsp_state._per_folder_feature_configs
        assert folder_uri in _lsp_state._per_folder_formatter_configs

    def test_removed_folder_drops_all_layers(self, reset_state, monkeypatch):
        folder_uri = "file:///home/user/proj-a"
        _lsp_state.get_or_init_folder_feature_config(folder_uri)
        _lsp_state.get_or_init_folder_formatter_config(folder_uri)
        _lsp_state.editor_config_settings_per_folder[folder_uri] = {"x": 1}
        _lsp_state.project_config_settings_per_folder[folder_uri] = {"y": 2}

        monkeypatch.setattr(_lsp_settings, "_pull_and_apply_configuration", lambda: None)

        _lifecycle.on_did_change_workspace_folders(_folders_event(removed=[folder_uri]))

        assert folder_uri not in _lsp_state._per_folder_feature_configs
        assert folder_uri not in _lsp_state._per_folder_formatter_configs
        assert folder_uri not in _lsp_state.editor_config_settings_per_folder
        assert folder_uri not in _lsp_state.project_config_settings_per_folder

    def test_change_triggers_re_pull(self, reset_state, monkeypatch, tmp_path):
        called: list[bool] = []
        monkeypatch.setattr(
            _lsp_settings,
            "_pull_and_apply_configuration",
            lambda: called.append(True),
        )
        monkeypatch.setattr("core.common.user_config.load_project_config", lambda _path: None)

        _lifecycle.on_did_change_workspace_folders(
            _folders_event(added=[f"file://{tmp_path}/added"])
        )
        assert called == [True]

    def test_added_folder_loads_project_config_ini(self, reset_state, monkeypatch, tmp_path):
        folder_path = tmp_path / "proj-a"
        folder_path.mkdir()
        (folder_path / ".tcl-lsp.ini").write_text("[style]\nline_length = 137\n", encoding="utf-8")
        folder_uri = folder_path.as_uri()

        monkeypatch.setattr(_lsp_settings, "_pull_and_apply_configuration", lambda: None)

        _lifecycle.on_did_change_workspace_folders(_folders_event(added=[folder_uri]))

        # Project layer for the new folder reflects the .ini contents.
        # ``get_all_settings`` normalises ``line_length`` -> ``lineLength``.
        proj = _lsp_state.project_config_settings_per_folder.get(folder_uri, {})
        assert proj.get("style", {}).get("lineLength") == 137

    def test_empty_event_is_a_noop(self, reset_state, monkeypatch):
        called: list[bool] = []
        monkeypatch.setattr(
            _lsp_settings,
            "_pull_and_apply_configuration",
            lambda: called.append(True),
        )
        _lifecycle.on_did_change_workspace_folders(_folders_event())
        # Even an empty change re-pulls — that's fine, the client is signalling
        # that something might have changed.  Just verify we didn't blow up.
        assert called == [True]
