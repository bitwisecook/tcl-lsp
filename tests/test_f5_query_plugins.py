"""Tests for the user-plugin discovery and the public extension APIs.

Covers:

- :func:`f5q.load_user_plugins` — XDG scan, idempotent caching,
  warn-not-crash on broken files, ``force=True`` re-scan.
- :func:`f5q.xdg_plugin_dir` — XDG_CONFIG_HOME honoured, fallback to
  ``~/.config``.
- Public decorators (``@builtin``, ``@input_format``, ``@renderer``)
  reachable from ``f5q`` and round-tripping through the registries.
- End-to-end: a plugin file in ``$XDG_CONFIG_HOME/f5q/plugins/`` ships a
  custom renderer / builtin / input format the engine then dispatches.
"""

from __future__ import annotations

from pathlib import Path

import pytest

import f5q
from core.bigip.query.builtins import _REGISTRY as _BUILTIN_REGISTRY
from core.bigip.query.inputs import _REGISTRY as _INPUT_REGISTRY
from core.bigip.query.plugins import _reset_for_tests, iter_plugin_files, xdg_plugin_dir
from core.bigip.query.renderers import _REGISTRY as _RENDERER_REGISTRY


@pytest.fixture
def xdg(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Point the loader at a fresh ``$XDG_CONFIG_HOME`` per-test.

    Resets the load-once flag so each test starts with a clean
    scan; the registry dicts are *not* reset between tests, so each
    test must namespace its plugin names to avoid duplicate-
    registration collisions with siblings.
    """
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))
    plugin_dir = tmp_path / "f5q" / "plugins"
    plugin_dir.mkdir(parents=True)
    _reset_for_tests()
    yield plugin_dir
    _reset_for_tests()


# ---------------------------------------------------------------------------
# XDG path discovery
# ---------------------------------------------------------------------------


def test_xdg_dir_honours_env_var(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))
    assert xdg_plugin_dir() == tmp_path / "f5q" / "plugins"


def test_xdg_dir_falls_back_to_home(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
    monkeypatch.setenv("HOME", "/home/fake-user")
    # ``os.path.expanduser('~/.config')`` reads HOME on POSIX.
    assert xdg_plugin_dir() == Path("/home/fake-user/.config/f5q/plugins")


# ---------------------------------------------------------------------------
# Loader behaviour
# ---------------------------------------------------------------------------


def test_loader_returns_empty_when_dir_missing(monkeypatch, tmp_path) -> None:
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "no-such-dir"))
    _reset_for_tests()
    assert f5q.load_user_plugins(force=True) == []


def test_loader_imports_python_files(xdg: Path) -> None:
    (xdg / "ping.py").write_text(
        "from f5q import renderer\n"
        "@renderer('ping-test', summary='ping test', accepts='any')\n"
        "def _(values, **opts): return 'pong\\n'\n"
    )
    loaded = f5q.load_user_plugins(force=True)
    assert any(p.name == "ping.py" for p in loaded)
    try:
        assert f5q.render("ping-test", [1, 2, 3]) == "pong\n"
    finally:
        _RENDERER_REGISTRY.pop("ping-test", None)


def test_loader_skips_underscore_prefixed_files(xdg: Path) -> None:
    """Files starting with ``_`` are conventionally private helpers
    a plugin module imports; the scanner skips them."""
    (xdg / "_helper.py").write_text("raise RuntimeError('this would crash if imported')\n")
    loaded = f5q.load_user_plugins(force=True)
    assert not any(p.name == "_helper.py" for p in loaded)


def test_loader_warns_on_broken_file_without_crashing(
    xdg: Path, capsys: pytest.CaptureFixture
) -> None:
    (xdg / "broken.py").write_text("this is not valid python !!!\n")
    (xdg / "good.py").write_text(
        "from f5q import renderer\n"
        "@renderer('warn-test', summary='warn test', accepts='any')\n"
        "def _(values, **opts): return ''\n"
    )
    loaded = f5q.load_user_plugins(force=True)
    captured = capsys.readouterr()
    # Broken file produces a warning to stderr.
    assert "warning" in captured.err.lower()
    assert "broken.py" in captured.err
    # Good file still loads.
    assert any(p.name == "good.py" for p in loaded)
    assert not any(p.name == "broken.py" for p in loaded)
    _RENDERER_REGISTRY.pop("warn-test", None)


def test_loader_is_idempotent(xdg: Path) -> None:
    (xdg / "once.py").write_text(
        "from f5q import renderer\n"
        "@renderer('once-test', summary='once', accepts='any')\n"
        "def _(values, **opts): return ''\n"
    )
    _reset_for_tests()
    first = f5q.load_user_plugins()
    second = f5q.load_user_plugins()
    # First call returns the loaded list; second call (without
    # force=True) short-circuits and returns [] — the file is not
    # re-imported, so the second @renderer call would have raised
    # 'duplicate registration' if it had run.
    assert any(p.name == "once.py" for p in first)
    assert second == []
    _RENDERER_REGISTRY.pop("once-test", None)


def test_force_reload_skips_already_loaded_files(xdg: Path, capsys: pytest.CaptureFixture) -> None:
    """``load_user_plugins(force=True)`` must not re-execute a plugin
    that loaded successfully on a prior call.

    Re-executing would trip the duplicate-registration guard in
    every ``@renderer`` / ``@builtin`` / ``@input_format`` decorator
    the plugin used, and surface as misleading "failed to load
    plugin" warnings on stderr.  The documented use case — drop a
    new file in ``~/.config/f5q/plugins/`` and force-reload — should
    surface just the new file and stay quiet about the rest.
    """
    (xdg / "first.py").write_text(
        "from f5q import renderer\n"
        "@renderer('force-test-first', summary='x', accepts='any')\n"
        "def _(values, **opts): return ''\n"
    )
    first = f5q.load_user_plugins(force=True)
    assert any(p.name == "first.py" for p in first)
    capsys.readouterr()  # drain any first-load output

    # Drop a NEW plugin file and re-scan.  The already-loaded
    # ``first.py`` should be skipped silently; only ``second.py``
    # should appear in the return value, and stderr should stay
    # clean.
    (xdg / "second.py").write_text(
        "from f5q import renderer\n"
        "@renderer('force-test-second', summary='y', accepts='any')\n"
        "def _(values, **opts): return ''\n"
    )
    second = f5q.load_user_plugins(force=True)
    captured = capsys.readouterr()
    try:
        assert [p.name for p in second] == ["second.py"]
        assert "warning" not in captured.err.lower()
        assert "duplicate" not in captured.err.lower()
    finally:
        _RENDERER_REGISTRY.pop("force-test-first", None)
        _RENDERER_REGISTRY.pop("force-test-second", None)


def test_force_reload_retries_previously_broken_files(xdg: Path) -> None:
    """A plugin that failed to import on the first pass should be
    retried on subsequent force-reloads — the user might have just
    fixed the syntax error."""
    plugin = xdg / "fixable.py"
    plugin.write_text("this is not valid python !!!\n")
    f5q.load_user_plugins(force=True)
    # Now "fix" the file and re-scan.
    plugin.write_text(
        "from f5q import renderer\n"
        "@renderer('retry-test', summary='ok', accepts='any')\n"
        "def _(values, **opts): return ''\n"
    )
    second = f5q.load_user_plugins(force=True)
    try:
        assert any(p.name == "fixable.py" for p in second)
        # The decorator side-effect registered the renderer.
        assert any(s.name == "retry-test" for s in f5q.list_renderers())
    finally:
        _RENDERER_REGISTRY.pop("retry-test", None)


def test_iter_plugin_files_top_level_before_subdirs(xdg: Path) -> None:
    """Top-level files come first, sub-directory files second.

    The loader sorts by ``(depth, path)`` so a user can predict the
    order — top-level plugins (the first-class case) always load
    before their helpers in sub-folders.
    """
    (xdg / "z-top.py").write_text("")
    (xdg / "sub").mkdir()
    (xdg / "sub" / "a-nested.py").write_text("")
    files = [p.name for p in iter_plugin_files(xdg)]
    assert files.index("z-top.py") < files.index("a-nested.py")


def test_iter_plugin_files_supports_multifile_imports(xdg: Path, tmp_path: Path) -> None:
    """A plugin can ``import`` a sibling file in the same plugin directory.

    The loader temp-prepends the plugin directory to ``sys.path``
    for the duration of each plugin's import; the entry is removed
    afterward so ``sys.path`` stays clean.
    """
    # ``_helper.py`` is intentionally skipped by ``iter_plugin_files``
    # but ``import helper`` from a real plugin still resolves it.
    (xdg / "helper.py").write_text("MARK = 'imported-from-sibling'\n")
    (xdg / "main.py").write_text(
        "import helper\n"
        "from f5q import renderer\n"
        f"@renderer('multifile-test', summary={'helper.MARK'.upper()!r}, accepts='any')\n"
        "def _(values, **opts): return helper.MARK + '\\n'\n"
    )
    loaded = f5q.load_user_plugins(force=True)
    try:
        # Both files appear in the load result (both are top-level
        # *.py files, and neither name starts with ``_``).
        names = {p.name for p in loaded}
        assert "main.py" in names
        # The renderer's summary captures the sibling-imported value.
        spec = next(s for s in f5q.list_renderers() if s.name == "multifile-test")
        assert (
            spec.impl(
                [],
            )
            == "imported-from-sibling\n"
        )
    finally:
        _RENDERER_REGISTRY.pop("multifile-test", None)
    # sys.path stays clean afterward.
    assert str(xdg) not in __import__("sys").path


# ---------------------------------------------------------------------------
# Public decorators (@builtin, @input_format)
# ---------------------------------------------------------------------------


def test_builtin_decorator_registers_callable() -> None:
    @f5q.builtin("plugin-uppercase", summary="upper", min_args=1, max_args=1)
    def _u(s):
        return str(s).upper()

    try:
        spec = next(s for s in f5q.list_builtins() if s.name == "plugin-uppercase")
        assert spec.impl("hello") == "HELLO"
        assert spec.category == "user"
    finally:
        _BUILTIN_REGISTRY.pop("plugin-uppercase", None)


def test_builtin_decorator_rejects_duplicates() -> None:
    @f5q.builtin("plugin-dup-check", min_args=0, max_args=0)
    def _a():
        return 1

    try:
        with pytest.raises(RuntimeError, match="duplicate builtin"):

            @f5q.builtin("plugin-dup-check", min_args=0, max_args=0)
            def _b():
                return 2
    finally:
        _BUILTIN_REGISTRY.pop("plugin-dup-check", None)


def test_input_format_decorator_registers_parser() -> None:
    @f5q.input_format("plugin-list", summary="newline-list")
    def _parse(source, *, uri, options=()):
        return [line for line in source.splitlines() if line.strip()]

    try:
        spec = next(s for s in f5q.list_input_formats() if s.name == "plugin-list")
        result = spec.impl("a\nb\nc\n", uri="<test>")
        assert result == ["a", "b", "c"]
    finally:
        _INPUT_REGISTRY.pop("plugin-list", None)


def test_input_format_decorator_rejects_duplicates() -> None:
    @f5q.input_format("plugin-input-dup", summary="x")
    def _a(source, *, uri, options=()):
        return source

    try:
        with pytest.raises(RuntimeError, match="duplicate input format"):

            @f5q.input_format("plugin-input-dup", summary="y")
            def _b(source, *, uri, options=()):
                return source
    finally:
        _INPUT_REGISTRY.pop("plugin-input-dup", None)


# ---------------------------------------------------------------------------
# End-to-end: plugin file ships a builtin used by a query
# ---------------------------------------------------------------------------


def test_plugin_builtin_is_callable_from_query(xdg: Path, tmp_path: Path) -> None:
    (xdg / "uppercase_plugin.py").write_text(
        "from f5q import builtin\n"
        "@builtin('e2e-upper', summary='upper', min_args=1, max_args=1)\n"
        "def _u(s): return str(s).upper()\n"
    )
    f5q.load_user_plugins(force=True)
    try:
        conf = tmp_path / "tiny.conf"
        conf.write_text("ltm virtual /Common/web_vs { destination 1.1.1.1:443 }\n")
        result = f5q.Query(".ltm.virtual[] | e2e-upper(.name)").run(paths=[conf]).values()
        assert result == ["WEB_VS"]
    finally:
        _BUILTIN_REGISTRY.pop("e2e-upper", None)


def test_plugin_input_format_is_dispatchable(xdg: Path, tmp_path: Path) -> None:
    (xdg / "list_input.py").write_text(
        "from f5q import input_format\n"
        "@input_format('e2e-list', summary='newline list')\n"
        "def _p(source, *, uri, options=()):\n"
        "    return [line for line in source.splitlines() if line.strip()]\n"
    )
    f5q.load_user_plugins(force=True)
    try:
        data = tmp_path / "items.txt"
        data.write_text("alpha\nbeta\ngamma\n")
        conf = tmp_path / "tiny.conf"
        conf.write_text("ltm virtual /Common/web_vs { destination 1.1.1.1:443 }\n")
        result = (
            f5q.Query("$names[]")
            .run(
                paths=[conf],
                input_specs={str(data): f5q.InputSpec(kind="e2e-list")},
                names={"names": str(data)},
            )
            .values()
        )
        assert result == ["alpha", "beta", "gamma"]
    finally:
        _INPUT_REGISTRY.pop("e2e-list", None)


def test_plugin_renderer_is_dispatchable(xdg: Path, tmp_path: Path) -> None:
    (xdg / "tag_render.py").write_text(
        "from f5q import renderer\n"
        "@renderer('e2e-tag', summary='count tag', accepts='any')\n"
        "def _r(values, **opts):\n"
        "    tag = opts.get('tag', 'x')\n"
        "    return f'<{tag}>{len(values)}</{tag}>\\n'\n"
    )
    f5q.load_user_plugins(force=True)
    try:
        conf = tmp_path / "tiny.conf"
        conf.write_text(
            "ltm virtual /Common/a { destination 1.1.1.1:443 }\n"
            "ltm virtual /Common/b { destination 2.2.2.2:443 }\n"
        )
        out = f5q.Query(".ltm.virtual[] | .name").run(paths=[conf]).render("e2e-tag", tag="count")
        assert out == "<count>2</count>\n"
    finally:
        _RENDERER_REGISTRY.pop("e2e-tag", None)


def test_unknown_input_format_lists_registered_names(xdg: Path, tmp_path: Path) -> None:
    """Typo in a kind name should surface every registered format —
    much more helpful than a bare 'unknown' error."""
    conf = tmp_path / "tiny.conf"
    conf.write_text("ltm virtual /Common/web_vs { destination 1.1.1.1:443 }\n")
    extra = tmp_path / "x.json"
    extra.write_text("{}")
    with pytest.raises(f5q.QueryError) as exc_info:
        f5q.Query("$names[]").run(
            paths=[conf],
            input_specs={str(extra): f5q.InputSpec(kind="not-a-format")},
            names={"names": str(extra)},
        )
    msg = str(exc_info.value)
    assert "not-a-format" in msg
    # Every built-in kind appears in the help-style list.
    assert "json" in msg
    assert "csv" in msg
    assert "f5log" in msg
