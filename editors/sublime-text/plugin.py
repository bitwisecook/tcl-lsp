"""
Tcl Language Support — Sublime Text LSP helper plugin.

Provides automatic configuration of the tcl-lsp server for the
Sublime LSP package.  Standalone features (syntax, snippets, settings)
work even when the LSP package is not installed.

Constraint: runs in Sublime Text's embedded Python 3.10+ (plugin host 38).
"""

import os
import shutil
import zipfile

import sublime  # type: ignore[import-not-found]
import sublime_plugin  # type: ignore[import-not-found]

PACKAGE_NAME = "Tcl"
SETTINGS_KEY = "LSP-Tcl.sublime-settings"
SERVER_DIR = "server"
SERVER_ENTRY = "__main__.py"

# Dialects the server supports, keyed for the quick-panel.
DIALECTS = [
    ("tcl8.6", "Tcl 8.6 (default)"),
    ("tcl8.5", "Tcl 8.5"),
    ("tcl8.4", "Tcl 8.4"),
    ("tcl9.0", "Tcl 9.0"),
    ("f5-irules", "F5 iRules"),
    ("f5-iapps", "F5 iApps"),
    ("eda-tools", "EDA Tools"),
]

# Map syntax name → dialect ID for automatic syncing when the user
# selects a dialect-specific syntax from the language menu.
_SYNTAX_DIALECT_MAP = {
    "Tcl": "tcl8.6",
    "Tcl 8.4": "tcl8.4",
    "Tcl 8.5": "tcl8.5",
    "Tcl 9.0": "tcl9.0",
    "iRule": "f5-irules",
    "iApp": "f5-iapps",
    "EDA Tools": "eda-tools",
}

# Tracks the last-observed syntax name per view ID so that only
# genuine syntax changes trigger a dialect update (not tab switches).
_view_last_syntax = {}  # type: dict

# Set True once the LSP package is confirmed available.
_HAS_LSP = False


# ---------------------------------------------------------------------------
# Utility helpers
# ---------------------------------------------------------------------------

def _package_dir():
    # type: () -> str
    """Return the extracted Packages/Tcl directory."""
    return os.path.join(sublime.packages_path(), PACKAGE_NAME)


def _cache_dir():
    # type: () -> str
    """Return the Cache/Tcl directory for extracted assets."""
    cache = os.path.join(sublime.cache_path(), PACKAGE_NAME)
    os.makedirs(cache, exist_ok=True)
    return cache


def _find_bundled_server():
    # type: () -> str
    """Locate the bundled server entry point (__main__.py).

    First checks the extracted Packages/Tcl/server/ directory (normal for
    development or overridden-package installs).  If not found, checks
    Cache/Tcl/server/ (previously extracted).  Finally, extracts the
    server/ tree from the .sublime-package ZIP in Installed Packages/.
    """
    # 1. Extracted package directory (development / loose install)
    candidate = os.path.join(_package_dir(), SERVER_DIR, SERVER_ENTRY)
    if os.path.isfile(candidate):
        return candidate

    # 2. Cache (previously extracted)
    cached_dir = os.path.join(_cache_dir(), SERVER_DIR)
    cached_entry = os.path.join(cached_dir, SERVER_ENTRY)
    if os.path.isfile(cached_entry):
        return cached_entry

    # 3. Extract server/ tree from .sublime-package ZIP
    pkg_zip = os.path.join(
        sublime.installed_packages_path(),
        PACKAGE_NAME + ".sublime-package",
    )
    if os.path.isfile(pkg_zip):
        try:
            with zipfile.ZipFile(pkg_zip, "r") as zf:
                server_members = [
                    n for n in zf.namelist()
                    if n.startswith(SERVER_DIR + "/")
                ]
                if server_members:
                    dest = _cache_dir()
                    for member in server_members:
                        zf.extract(member, dest)
                    if os.path.isfile(cached_entry):
                        return cached_entry
        except (zipfile.BadZipFile, OSError):
            pass

    return ""


def _discover_python():
    # type: () -> str
    """Find a suitable Python 3.10+ interpreter on PATH."""
    candidates = [
        "python3.13", "python3.12", "python3.11", "python3.10", "python3",
    ]
    for name in candidates:
        path = shutil.which(name)
        if path is not None:
            return path
    return "python3"


def _load_settings():
    # type: () -> sublime.Settings
    return sublime.load_settings(SETTINGS_KEY)


def _set_dialect(dialect_id):
    # type: (str) -> None
    """Update the global LSP dialect setting."""
    settings = _load_settings()
    server_settings = settings.get("settings") or {}
    tcl_lsp = server_settings.get("tclLsp") or {}
    if tcl_lsp.get("dialect") == dialect_id:
        return
    tcl_lsp["dialect"] = dialect_id
    server_settings["tclLsp"] = tcl_lsp
    settings.set("settings", server_settings)
    sublime.save_settings(SETTINGS_KEY)
    sublime.status_message("Tcl dialect: " + dialect_id)


def _check_view_dialect(view):
    # type: (sublime.View) -> None
    """If the syntax on *view* changed, sync the LSP dialect."""
    if not _HAS_LSP:
        return
    syntax = view.syntax()
    if syntax is None:
        return
    name = syntax.name
    vid = view.id()
    prev = _view_last_syntax.get(vid)
    _view_last_syntax[vid] = name
    if prev == name:
        return  # no change
    dialect = _SYNTAX_DIALECT_MAP.get(name)
    if dialect is not None:
        _set_dialect(dialect)


# ---------------------------------------------------------------------------
# LSP AbstractPlugin — defined at module level so LSP can introspect it.
# Guarded by try/except so the plugin loads even without the LSP package.
# ---------------------------------------------------------------------------

try:
    from LSP.plugin import AbstractPlugin  # type: ignore[import-not-found]
    from LSP.plugin import register_plugin  # type: ignore[import-not-found]
    from LSP.plugin import unregister_plugin  # type: ignore[import-not-found]

    class TclLsp(AbstractPlugin):
        """LSP client configuration for the tcl-lsp server."""

        @classmethod
        def name(cls):
            # type: () -> str
            return PACKAGE_NAME

        @classmethod
        def configuration(cls):
            # type: () -> tuple
            """Return (settings, resource_path) for the LSP framework.

            The default AbstractPlugin.configuration() assumes the settings
            file lives at ``Packages/LSP-{name}/LSP-{name}.sublime-settings``,
            which only works when the plugin is its own ``LSP-{name}`` package.
            Because we bundle the LSP helper inside the ``Tcl`` syntax package
            the resource is actually at ``Packages/Tcl/LSP-Tcl.sublime-settings``.
            """
            basename = SETTINGS_KEY                     # "LSP-Tcl.sublime-settings"
            filepath = "Packages/{}/{}".format(PACKAGE_NAME, basename)
            settings = sublime.load_settings(basename)
            return (settings, filepath)

        @classmethod
        def additional_variables(cls):
            # type: () -> dict
            settings = _load_settings()
            # Allow user override of server path.
            user_path = settings.get("server_path")
            if user_path and os.path.isfile(user_path):
                server = user_path
            else:
                server = _find_bundled_server()

            # Allow user override of Python path.
            user_python = settings.get("python_path")
            if user_python and os.path.isfile(user_python):
                python = user_python
            else:
                python = _discover_python()

            return {
                "server_path": server,
                "python": python,
            }

        @classmethod
        def can_start(cls, window, initiating_view, workspace_folders, configuration):
            """Return an error string if the server cannot start."""
            variables = cls.additional_variables() or {}
            python = variables.get("python", "python3")
            server = variables.get("server_path", "")

            if not shutil.which(python):
                return "Python interpreter not found: {}".format(python)

            if not server or (not os.path.isfile(server) and not shutil.which(server)):
                return (
                    "tcl-lsp server not found.  "
                    "Download the .sublime-package from the GitHub Releases "
                    "page and install it as Tcl.sublime-package, or set "
                    "'server_path' in LSP-Tcl settings."
                )

            return None

except ImportError:
    TclLsp = None  # type: ignore[assignment,misc]


# ---------------------------------------------------------------------------
# Lifecycle
# ---------------------------------------------------------------------------

def _check_package_name():
    # type: () -> None
    """Warn if the .sublime-package file is not named Tcl.sublime-package."""
    expected = os.path.join(
        sublime.installed_packages_path(),
        PACKAGE_NAME + ".sublime-package",
    )
    if os.path.isfile(expected):
        return  # Correctly named
    ip_dir = sublime.installed_packages_path()
    if os.path.isdir(ip_dir):
        for fname in os.listdir(ip_dir):
            if fname.endswith(".sublime-package") and "tcl-lsp" in fname.lower():
                sublime.error_message(
                    "Tcl Language Support\n\n"
                    "The package file must be named 'Tcl.sublime-package' "
                    "to work correctly, but it is currently named:\n\n"
                    "  " + fname + "\n\n"
                    "Please rename it to 'Tcl.sublime-package' and restart "
                    "Sublime Text."
                )
                return


def _disable_builtin_tcl():
    # type: () -> None
    """Disable the shipped TCL package to avoid duplicate syntax entries.

    Sublime Text ships a built-in package named 'TCL' (uppercase).
    Since our package is named 'Tcl' (mixed case), they are treated as
    separate packages and both syntaxes appear in the language menu.
    Adding 'TCL' to ignored_packages hides the built-in.
    """
    prefs = sublime.load_settings("Preferences.sublime-settings")
    ignored = prefs.get("ignored_packages") or []
    if "TCL" not in ignored:
        ignored.append("TCL")
        prefs.set("ignored_packages", ignored)
        sublime.save_settings("Preferences.sublime-settings")


def _enable_semantic_highlighting():
    # type: () -> None
    """Enable semantic highlighting in the global LSP settings if not already on."""
    lsp_settings = sublime.load_settings("LSP.sublime-settings")
    if not lsp_settings.get("semantic_highlighting"):
        lsp_settings.set("semantic_highlighting", True)
        sublime.save_settings("LSP.sublime-settings")


def plugin_loaded():
    # type: () -> None
    """Called by Sublime Text after all packages are loaded."""
    global _HAS_LSP

    # Defer these so they don't interfere with the current load cycle.
    sublime.set_timeout(_check_package_name, 2000)
    sublime.set_timeout(_disable_builtin_tcl, 1000)
    sublime.set_timeout(_enable_semantic_highlighting, 1500)

    if TclLsp is not None:
        _HAS_LSP = True
        register_plugin(TclLsp)
        print("Tcl: registered LSP server plugin")
    else:
        sublime.set_timeout(lambda: _suggest_lsp_install(), 3000)


def plugin_unloaded():
    # type: () -> None
    """Called by Sublime Text when the plugin is unloaded."""
    if TclLsp is not None:
        unregister_plugin(TclLsp)


def _suggest_lsp_install():
    # type: () -> None
    """Show a one-time message suggesting LSP package installation."""
    settings = sublime.load_settings("Tcl.sublime-settings")
    if settings.get("_lsp_suggestion_shown"):
        return
    settings.set("_lsp_suggestion_shown", True)
    sublime.save_settings("Tcl.sublime-settings")

    sublime.message_dialog(
        "Tcl Language Support\n\n"
        "For full language server features (diagnostics, completions, "
        "hover, formatting, code actions, and more), install the LSP "
        "package from Package Control:\n\n"
        "  Command Palette > Package Control: Install Package > LSP\n\n"
        "Syntax highlighting, snippets, and settings work without LSP."
    )


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

class TclSelectDialectCommand(sublime_plugin.WindowCommand):
    """Quick panel to choose the Tcl dialect for the LSP server."""

    def run(self):
        # type: () -> None
        items = [label for _, label in DIALECTS]
        self.window.show_quick_panel(items, self._on_done)

    def _on_done(self, index):
        # type: (int) -> None
        if index < 0:
            return
        _set_dialect(DIALECTS[index][0])

    def is_enabled(self):
        # type: () -> bool
        return _HAS_LSP


class TclRestartServerCommand(sublime_plugin.WindowCommand):
    """Restart the tcl-lsp language server."""

    def run(self):
        # type: () -> None
        self.window.run_command("lsp_restart_server", {"config_name": PACKAGE_NAME})

    def is_enabled(self):
        # type: () -> bool
        return _HAS_LSP


class TclOptimiseDocumentCommand(sublime_plugin.TextCommand):
    """Apply all optimisation suggestions to the current document."""

    def run(self, edit):
        # type: (sublime.Edit) -> None
        self.view.run_command(
            "lsp_execute",
            {
                "command_name": "tcl-lsp.optimiseDocument",
                "command_args": {
                    "uri": self.view.settings().get("lsp_uri"),
                },
            },
        )

    def is_enabled(self):
        # type: () -> bool
        return _HAS_LSP

    def is_visible(self):
        # type: () -> bool
        return _is_tcl_view(self.view)


class TclFixAllSafeIssuesCommand(sublime_plugin.TextCommand):
    """Apply all safe quick-fixes to the current document."""

    def run(self, edit):
        # type: (sublime.Edit) -> None
        self.view.run_command(
            "lsp_execute",
            {
                "command_name": "tcl-lsp.fixAllSafeIssues",
                "command_args": {
                    "uri": self.view.settings().get("lsp_uri"),
                },
            },
        )

    def is_enabled(self):
        # type: () -> bool
        return _HAS_LSP

    def is_visible(self):
        # type: () -> bool
        return _is_tcl_view(self.view)


class TclMinifyDocumentCommand(sublime_plugin.TextCommand):
    """Minify the current Tcl document."""

    def run(self, edit):
        # type: (sublime.Edit) -> None
        self.view.run_command(
            "lsp_execute",
            {
                "command_name": "tcl-lsp.minifyDocument",
                "command_args": {
                    "uri": self.view.settings().get("lsp_uri"),
                },
            },
        )

    def is_enabled(self):
        # type: () -> bool
        return _HAS_LSP

    def is_visible(self):
        # type: () -> bool
        return _is_tcl_view(self.view)


class TclUnminifyErrorCommand(sublime_plugin.WindowCommand):
    """Translate a minified-code error message back to original names."""

    def run(self):
        # type: () -> None
        self.window.show_input_panel(
            "Error message:",
            "",
            self._on_error_text,
            None,
            None,
        )

    def _on_error_text(self, error_text):
        # type: (str) -> None
        if not error_text:
            return
        self._error_text = error_text
        self.window.show_input_panel(
            "Symbol map file path:",
            "",
            self._on_symbol_map,
            None,
            None,
        )

    def _on_symbol_map(self, map_path):
        # type: (str) -> None
        import os
        map_path = map_path.strip()
        if not map_path or not os.path.isfile(map_path):
            sublime.error_message("Symbol map file not found: " + map_path)
            return
        with open(map_path, "r", encoding="utf-8") as f:
            map_text = f.read()
        # Send to LSP
        self.window.active_view().run_command(
            "lsp_execute",
            {
                "command_name": "tcl-lsp.unminifyError",
                "command_args": {
                    "error_message": self._error_text,
                    "symbol_map": map_text,
                },
            },
        )

    def is_enabled(self):
        # type: () -> bool
        return _HAS_LSP


# ---------------------------------------------------------------------------
# Dialect sync — automatically update the LSP dialect when the user
# selects a dialect-specific syntax from View > Syntax.
# ---------------------------------------------------------------------------

class TclDialectSyncListener(sublime_plugin.EventListener):
    """Sync LSP dialect when the user switches to a dialect syntax."""

    def on_activated(self, view):
        # type: (sublime.View) -> None
        self._ensure_settings_listener(view)
        _check_view_dialect(view)

    def on_close(self, view):
        # type: (sublime.View) -> None
        _view_last_syntax.pop(view.id(), None)

    def _ensure_settings_listener(self, view):
        # type: (sublime.View) -> None
        """Attach a settings-change callback so we catch syntax changes
        while the view already has focus (e.g. from the language menu)."""
        if view.settings().get("_tcl_lsp_syn"):
            return
        syntax = view.syntax()
        if syntax is None or syntax.name not in _SYNTAX_DIALECT_MAP:
            return
        view.settings().set("_tcl_lsp_syn", True)
        view.settings().add_on_change(
            "tcl_dialect", lambda: _check_view_dialect(view)
        )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _is_tcl_view(view):
    # type: (sublime.View) -> bool
    """Return True if the view is a Tcl or iRules file."""
    if view is None:
        return False
    syntax = view.settings().get("syntax", "")
    return "Tcl" in syntax or "iRule" in syntax
