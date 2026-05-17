"""User-plugin discovery for the query engine.

External users can extend the engine without forking the project by
dropping Python files into the platform-conventional XDG plugin
directory:

- ``$XDG_CONFIG_HOME/f5q/plugins/*.py`` when ``XDG_CONFIG_HOME`` is set;
- ``~/.config/f5q/plugins/*.py`` otherwise.

Each Python file may register inputs (``@input_format``), builtins
(``@builtin``), and renderers (``@renderer``) — the same decorators
the in-tree built-ins use.  Registration happens by import-time
side-effect; the loader doesn't care which decorators the file
calls, only that importing the file succeeds.

The loader is **auto-triggered**: the first call to any registry's
``list_*`` / ``lookup`` / dispatch helper runs the XDG scan once.
The CLI and the Python API both go through the same registries, so
plugins are visible from both surfaces without any explicit "import
plugins" step.

Plugin import failures (syntax errors, missing dependencies, bad
decorator arguments) are written to stderr as a warning but never
raise — one broken plugin must not kill the CLI or a script that
doesn't use it.  The warning names the file and the exception so the
user can fix it quickly.
"""

from __future__ import annotations

import importlib.util
import os
import sys
from pathlib import Path
from typing import Iterable

_PLUGINS_LOADED = False
# Paths of plugin files that imported successfully on some prior call.
# Tracked so ``load_user_plugins(force=True)`` skips them on re-scan
# instead of re-executing their decorators — the second pass would
# trip the duplicate-registration guard in ``@renderer`` / ``@builtin``
# / ``@input_format`` and surface as a spurious "failed to load
# plugin" warning on stderr.  Files that previously *failed* to
# import stay out of this set, so a force-reload still retries them
# in case the user just fixed the syntax error.
_LOADED_FILES: set[str] = set()


def xdg_plugin_dir() -> Path:
    """Return the directory the loader scans for plugin files.

    Honours ``XDG_CONFIG_HOME``; falls back to ``~/.config`` per the
    `XDG Base Directory specification
    <https://specifications.freedesktop.org/basedir-spec/>`_.  The
    returned path may not exist — callers must handle the "no plugins
    installed" case.
    """
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    return Path(base) / "f5q" / "plugins"


def iter_plugin_files(directory: Path | None = None) -> Iterable[Path]:
    """Yield every ``*.py`` file the loader would import, in load order.

    Sorted by ``(relative depth, full path)`` so **top-level files
    load before sub-directory files**, with each tier sorted
    alphabetically.  This makes the order reproducible and matches
    the way users intuitively read the plugin tree: top-level
    plugins are first-class, sub-folders are for plugin-internal
    helpers.  Hidden files (``_*.py``) are skipped — they're
    conventionally helper modules a plugin file imports privately.
    """
    directory = directory if directory is not None else xdg_plugin_dir()
    if not directory.is_dir():
        return
    for path in sorted(
        directory.rglob("*.py"),
        key=lambda p: (len(p.relative_to(directory).parts), str(p)),
    ):
        if path.name.startswith("_"):
            continue
        yield path


def load_user_plugins(*, force: bool = False) -> list[Path]:
    """Import every plugin file under the XDG plugin directory.

    Returns the list of files that imported **on this call**.  Files
    that imported successfully on a prior call are skipped — the
    decorator registries reject duplicate names, so re-executing a
    plugin module would only emit spurious "failed to load" warnings
    and report it as failed.  Files that *failed* on a prior call
    are retried in case the user just fixed the syntax error.  Files
    that raise on this call are reported on stderr and skipped.

    Idempotent — subsequent calls return immediately unless *force*
    is set (used by tests that need to re-scan after dropping a new
    file).  Even with *force*, this only imports files the loader
    hasn't already imported, so the documented "drop a new file in
    ``~/.config/f5q/plugins/`` and re-scan" workflow surfaces just
    the new file in the return value rather than burying it among
    duplicate-registration warnings.
    """
    global _PLUGINS_LOADED
    if _PLUGINS_LOADED and not force:
        return []
    _PLUGINS_LOADED = True

    loaded: list[Path] = []
    for path in iter_plugin_files():
        if str(path) in _LOADED_FILES:
            continue
        if _import_plugin_file(path):
            loaded.append(path)
    return loaded


def _import_plugin_file(path: Path) -> bool:
    """Import *path* as a stand-alone module; return True on success.

    The module name is synthesised to be globally unique so a plugin
    re-loaded under ``force=True`` doesn't conflict with its prior
    incarnation in ``sys.modules`` (re-using the same name would
    leave both decorators having seen the registry).  Failures are
    reported on stderr but never raised.

    The plugin's parent directory is **temporarily prepended to
    ``sys.path``** for the duration of ``exec_module`` so a
    multi-file plugin can ``import sibling`` or
    ``from _helper import foo`` the way ordinary Python files in the
    same directory would.  The entry is removed afterwards so the
    plugin directory never lingers on ``sys.path`` outside its own
    import — the loader stays free of cross-contamination if two
    plugin directories share a helper name.
    """
    name = f"f5q_user_plugin_{path.stem}_{abs(hash(str(path))):x}"
    plugin_dir = str(path.parent)
    inserted = False
    if plugin_dir not in sys.path:
        sys.path.insert(0, plugin_dir)
        inserted = True
    try:
        spec = importlib.util.spec_from_file_location(name, path)
        if spec is None or spec.loader is None:
            print(f"f5q: warning: cannot load plugin {path}: no loader", file=sys.stderr)
            return False
        module = importlib.util.module_from_spec(spec)
        sys.modules[name] = module
        spec.loader.exec_module(module)
        _LOADED_FILES.add(str(path))
        return True
    except Exception as exc:  # noqa: BLE001  (any exception is a warn-not-crash event)
        print(f"f5q: warning: failed to load plugin {path}: {exc}", file=sys.stderr)
        return False
    finally:
        if inserted:
            try:
                sys.path.remove(plugin_dir)
            except ValueError:
                pass


def _reset_for_tests() -> None:
    """Drop the load-once flag so tests can re-scan against a fresh XDG dir.

    Tests parameterise ``XDG_CONFIG_HOME`` with ``monkeypatch.setenv``
    or ``tmp_path`` fixtures; without resetting the flag the second
    test in a run would skip the scan entirely.  Also clears the
    "already imported" set so the next test starts from a clean slate
    even when its plugin files live under the same paths as a prior
    test's fixtures.  Not part of the public API — call
    ``load_user_plugins(force=True)`` from script code instead.
    """
    global _PLUGINS_LOADED
    _PLUGINS_LOADED = False
    _LOADED_FILES.clear()
