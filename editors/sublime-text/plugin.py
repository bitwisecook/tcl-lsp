# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""
TclLsp — Sublime Text language support for Tcl, iRules, iApps and EDA Tcl.

Standalone features (syntaxes, snippets, settings, symbol indexing) need
nothing but Sublime Text.  With the sublimelsp/LSP package installed, this
plugin also configures the tcl-lsp language server: the matching
`tcl-lsp-server` build for this platform is downloaded once into LSP's
package storage, checksum-verified against the release's signed
SHA256SUMS, and kept beside the `.tclspec` packs shipped in this package.

Constraint: runs in Sublime Text's plugin_host 3.8 (see `.python-version`).
"""

import functools
import hashlib
import json
import os
import re
import shutil
import tempfile
import urllib.request

import sublime  # type: ignore[import-not-found]
import sublime_plugin  # type: ignore[import-not-found]

PACKAGE_NAME = "TclLsp"

# The LSP client settings (user-editable, overridable from Packages/User).
SETTINGS_KEY = "LSP-Tcl.sublime-settings"

# Plugin state — first-run flags only.  Kept out of SETTINGS_KEY so a user's
# settings file stays theirs, and out of `Tcl.sublime-settings`, which
# Sublime applies to every view using the Tcl syntax.
STATE_KEY = "TclLsp.sublime-settings"

# Where the server binary and its spec packs come from.
GITHUB_REPO = "bitwisecook/tcl-lsp"
SERVER_BASENAME = "tcl-lsp-server"
CHECKSUM_ASSET = "SHA256SUMS"

# `tcl_spectcl::discovery` reads this to find the bundled `.tclspec` packs
# (the EDA vendor command libraries).  Without them the shipped EDA
# syntaxes highlight commands the server would report as unknown.
SPEC_PACK_DIR_ENV = "TCL_LSP_SPEC_PACK_DIR"
SPEC_PACK_SUBDIR = "specs"

# Package resource holding the release this package was built for.
VERSION_RESOURCE = "Packages/{}/server_version.json".format(PACKAGE_NAME)

# Sublime's platform/arch pair -> Rust target triple of the release asset.
# Keep in sync with SERVER_TARGET_MAP in the repository Makefile.
SERVER_TRIPLES = {
    ("linux", "x64"): "x86_64-unknown-linux-gnu",
    ("linux", "arm64"): "aarch64-unknown-linux-gnu",
    ("osx", "x64"): "x86_64-apple-darwin",
    ("osx", "arm64"): "aarch64-apple-darwin",
    ("windows", "x64"): "x86_64-pc-windows-msvc",
    ("windows", "arm64"): "aarch64-pc-windows-msvc",
}

# A release version — three numeric components, as `v<version>` tags carry.
# Anything else (a `git describe` string, the dev sentinel) has no release
# to download from.
_RELEASE_VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")

# Dialects the server supports, keyed for the quick-panel.
DIALECTS = [
    # @generated:dialects:begin
    ("bpf", "BPF"),
    ("cadence-eda-tcl", "Cadence EDA Tcl"),
    ("expect", "Expect"),
    ("f5-bigip", "F5 BIG-IP"),
    ("f5-iapps", "F5 iApps"),
    ("f5-irules", "F5 iRules"),
    ("f5-tmsh", "F5 tmsh Scripts"),
    ("intel-quartus-eda-tcl", "Intel Quartus EDA Tcl"),
    ("mentor-eda-tcl", "Mentor EDA Tcl"),
    ("microchip-libero-eda-tcl", "Microchip Libero EDA Tcl"),
    ("spectcl", "SpecTcl"),
    ("synopsys-eda-tcl", "Synopsys EDA Tcl"),
    ("tcl8.4", "Tcl 8.4"),
    ("tcl8.5", "Tcl 8.5"),
    ("tcl8.6", "Tcl 8.6 (default)"),
    ("tcl9.0", "Tcl 9.0"),
    ("tcl9.1", "Tcl 9.1"),
    ("xilinx-eda-tcl", "Xilinx EDA Tcl"),
    # @generated:dialects:end
]

# Map syntax name → dialect ID for automatic syncing when the user
# selects a dialect-specific syntax from the language menu.
_SYNTAX_DIALECT_MAP = {
    "Tcl": "tcl8.6",
    "Tcl 8.4": "tcl8.4",
    "Tcl 8.5": "tcl8.5",
    "Tcl 9.0": "tcl9.0",
    "Tcl 9.1": "tcl9.1",
    "iRule": "f5-irules",
    "iApp": "f5-iapps",
    "APL": "f5-iapps",
    "BIG-IP": "f5-bigip",
    "Synopsys EDA": "synopsys-eda-tcl",
    "Cadence EDA": "cadence-eda-tcl",
    "Xilinx EDA": "xilinx-eda-tcl",
    "Intel Quartus": "intel-quartus-eda-tcl",
    "Mentor EDA": "mentor-eda-tcl",
    "Expect": "expect",
}

# Tracks the last-observed syntax name per view ID so that only
# genuine syntax changes trigger a dialect update (not tab switches).
_view_last_syntax = {}  # type: dict

# Set True once the LSP package is confirmed available.
_HAS_LSP = False


# Utility helpers


def _package_dir():
    # type: () -> str
    """Return the extracted Packages/TclLsp directory (development installs)."""
    return os.path.join(sublime.packages_path(), PACKAGE_NAME)


def _server_filename():
    # type: () -> str
    """Return the server executable name for this platform."""
    if sublime.platform() == "windows":
        return SERVER_BASENAME + ".exe"
    return SERVER_BASENAME


def _server_triple():
    # type: () -> str
    """Return the release-asset target triple for this platform, or ''."""
    return SERVER_TRIPLES.get((sublime.platform(), sublime.arch()), "")


def _ensure_executable(path):
    # type: (str) -> str
    """Ensure *path* has the +x bit set and return the path unchanged."""
    if not path or os.name == "nt":
        return path
    try:
        mode = os.stat(path).st_mode
        if not (mode & 0o111):
            os.chmod(path, 0o755)
    except OSError:
        pass
    return path


def _load_settings():
    # type: () -> sublime.Settings
    return sublime.load_settings(SETTINGS_KEY)


def _state():
    # type: () -> sublime.Settings
    return sublime.load_settings(STATE_KEY)


def _save_state():
    # type: () -> None
    sublime.save_settings(STATE_KEY)


def _user_server_path():
    # type: () -> str
    """Return the user's `server_path` override when it names a real file."""
    path = _load_settings().get("server_path") or ""
    if path and os.path.isfile(path):
        return _ensure_executable(path)
    return ""


def _development_server_path():
    # type: () -> str
    """Return a server staged inside the package directory, if any.

    `make build-editor-sublime` does not bundle a binary, but a development
    checkout symlinked into Packages/ may have one at `server/`, and that
    build should win over a download.
    """
    candidate = os.path.join(_package_dir(), "server", _server_filename())
    if os.path.isfile(candidate):
        return _ensure_executable(candidate)
    return ""


# Managed server install (downloaded on first use)


def _packaged_version():
    # type: () -> str
    """Return the release version this package was built for, or ''.

    `server_version.json` is written into the package by
    `make build-editor-sublime`; a plain source checkout has no stamp, and
    then the latest published release is resolved at install time instead.
    """
    try:
        raw = sublime.load_resource(VERSION_RESOURCE)
    except (OSError, ValueError):
        return ""
    try:
        version = (json.loads(raw) or {}).get("version") or ""
    except ValueError:
        return ""
    return version if _RELEASE_VERSION_RE.match(version) else ""


def _managed_dir(version):
    # type: (str) -> str
    """Return the install directory for *version* inside LSP's storage."""
    if TclLsp is None:
        return ""
    return os.path.join(TclLsp.storage_path(), PACKAGE_NAME, version)


def _managed_server_path(version):
    # type: (str) -> str
    """Return the managed server path for *version*, or '' when absent."""
    base = _managed_dir(version)
    if not base:
        return ""
    candidate = os.path.join(base, _server_filename())
    if os.path.isfile(candidate) and os.path.isdir(
        os.path.join(base, SPEC_PACK_SUBDIR)
    ):
        return _ensure_executable(candidate)
    return ""


def _installed_versions():
    # type: () -> list
    """Return every fully-installed managed version, name-sorted.

    Ordering is by directory name, not semver — the pinned version is
    tried first and `_install_server` prunes the rest, so this list only
    ever decides between leftovers on an unstamped source install, where
    any complete install will do.
    """
    if TclLsp is None:
        return []
    root = os.path.join(TclLsp.storage_path(), PACKAGE_NAME)
    try:
        names = sorted(os.listdir(root))
    except OSError:
        return []
    return [name for name in names if _managed_server_path(name)]


def _fetch(url, timeout=30):
    # type: (str, int) -> bytes
    """GET *url* and return its body."""
    request = urllib.request.Request(url, headers={"User-Agent": _user_agent()})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def _user_agent():
    # type: () -> str
    return "{}/Sublime-Text (+https://github.com/{})".format(PACKAGE_NAME, GITHUB_REPO)


def _latest_release_version():
    # type: () -> str
    """Ask GitHub for the newest non-pre-release version."""
    url = "https://api.github.com/repos/{}/releases/latest".format(GITHUB_REPO)
    tag = (json.loads(_fetch(url).decode("utf-8")) or {}).get("tag_name") or ""
    version = tag[1:] if tag.startswith("v") else tag
    if not _RELEASE_VERSION_RE.match(version):
        raise RuntimeError(
            "the latest tcl-lsp release ({}) is not a plain version tag".format(
                tag or "none"
            )
        )
    return version


def _asset_url(version, asset):
    # type: (str, str) -> str
    return "https://github.com/{}/releases/download/v{}/{}".format(
        GITHUB_REPO, version, asset
    )


def _expected_checksum(version, asset):
    # type: (str, str) -> str
    """Return the SHA256SUMS digest for *asset* in release *version*.

    Every release carries a SHA256SUMS covering each attached artefact
    (signed with cosign alongside it), so a download is never trusted on
    the strength of the transport alone.
    """
    sums = _fetch(_asset_url(version, CHECKSUM_ASSET)).decode("utf-8")
    for line in sums.splitlines():
        parts = line.split()
        if len(parts) == 2 and os.path.basename(parts[1]) == asset:
            return parts[0]
    raise RuntimeError(
        "release v{} has no {} entry for {}".format(version, CHECKSUM_ASSET, asset)
    )


def _download_verified(url, expected_sha256, dest):
    # type: (str, str, str) -> None
    """Stream *url* to *dest*, refusing content that fails the checksum."""
    digest = hashlib.sha256()
    request = urllib.request.Request(url, headers={"User-Agent": _user_agent()})
    with urllib.request.urlopen(request, timeout=120) as response:
        with open(dest, "wb") as handle:
            while True:
                chunk = response.read(256 * 1024)
                if not chunk:
                    break
                digest.update(chunk)
                handle.write(chunk)
    actual = digest.hexdigest()
    if actual != expected_sha256:
        os.remove(dest)
        raise RuntimeError(
            "checksum mismatch for {}: expected {}, got {}".format(
                os.path.basename(url), expected_sha256, actual
            )
        )


def _stage_spec_packs(dest_dir):
    # type: (str) -> None
    """Copy this package's `.tclspec` packs into *dest_dir*.

    The packs ship as package resources, so they may live inside the
    `.sublime-package` ZIP; the server needs real files on disk.
    """
    os.makedirs(dest_dir, exist_ok=True)
    prefix = "Packages/{}/{}/".format(PACKAGE_NAME, SPEC_PACK_SUBDIR)
    for resource in sublime.find_resources("*.tclspec"):
        if not resource.startswith(prefix):
            continue
        target = os.path.join(dest_dir, os.path.basename(resource))
        with open(target, "wb") as handle:
            handle.write(sublime.load_binary_resource(resource))


def _prune_managed_versions(keep):
    # type: (str) -> None
    """Remove managed installs other than *keep* (best effort)."""
    if TclLsp is None:
        return
    root = os.path.join(TclLsp.storage_path(), PACKAGE_NAME)
    try:
        names = os.listdir(root)
    except OSError:
        return
    for name in names:
        if name == keep:
            continue
        shutil.rmtree(os.path.join(root, name), ignore_errors=True)


def _install_server():
    # type: () -> None
    """Download and verify the server build for this platform."""
    triple = _server_triple()
    if not triple:
        raise RuntimeError(
            "no tcl-lsp-server build for {}-{}; set 'server_path' in the "
            "TclLsp LSP settings to a server you built yourself".format(
                sublime.platform(), sublime.arch()
            )
        )

    version = _packaged_version() or _latest_release_version()
    asset = "{}-{}{}".format(
        SERVER_BASENAME, triple, ".exe" if sublime.platform() == "windows" else ""
    )
    target_dir = _managed_dir(version)
    if not target_dir:
        raise RuntimeError("the LSP package is not available")

    expected = _expected_checksum(version, asset)

    # Build the install in a sibling directory and move it into place, so an
    # interrupted download never leaves a half-installed version behind.
    parent = os.path.dirname(target_dir)
    os.makedirs(parent, exist_ok=True)
    staging = tempfile.mkdtemp(prefix=".{}-".format(version), dir=parent)
    try:
        binary = os.path.join(staging, _server_filename())
        _download_verified(_asset_url(version, asset), expected, binary)
        _ensure_executable(binary)
        _stage_spec_packs(os.path.join(staging, SPEC_PACK_SUBDIR))
        shutil.rmtree(target_dir, ignore_errors=True)
        os.rename(staging, target_dir)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise

    _prune_managed_versions(version)
    print("{}: installed tcl-lsp-server {} ({})".format(PACKAGE_NAME, version, triple))


def _resolve_server():
    # type: () -> tuple
    """Return `(server_path, spec_pack_dir)` for the LSP client config.

    Resolution order: the user's `server_path`, a server staged in a
    development checkout, then the managed download.  Only the managed
    install carries a spec-pack directory — a server the user points at
    keeps whatever packs sit beside it.
    """
    path = _user_server_path() or _development_server_path()
    if path:
        return (path, "")

    version = _packaged_version()
    candidates = [version] if version else []
    candidates.extend(reversed(_installed_versions()))
    for candidate in candidates:
        managed = _managed_server_path(candidate)
        if managed:
            return (managed, os.path.join(_managed_dir(candidate), SPEC_PACK_SUBDIR))
    return ("", "")


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


# LSP AbstractPlugin — defined at module level so LSP can introspect it.
# The sublimelsp/LSP package is optional: without it the syntax/commands half of
# this plugin still loads, `TclLsp` stays None, and `_suggest_lsp_install` nudges
# the user.  The class is bound to `TclLsp` in the `else` branch rather than
# defined directly in the `try`, so the name has one type rather than two.
TclLsp = None  # type: ignore[var-annotated]

try:
    from LSP.plugin import (
        AbstractPlugin,
        register_plugin,
        unregister_plugin,
    )
except ImportError:

    def register_plugin(plugin):
        # type: (type) -> None
        raise RuntimeError("sublimelsp/LSP is not installed")

    def unregister_plugin(plugin):
        # type: (type) -> None
        raise RuntimeError("sublimelsp/LSP is not installed")

else:

    class _TclLspPlugin(AbstractPlugin):
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
            Because the LSP helper is bundled inside the ``TclLsp`` language
            package the resource is at
            ``Packages/TclLsp/LSP-Tcl.sublime-settings``.
            """
            basename = SETTINGS_KEY  # "LSP-Tcl.sublime-settings"
            filepath = "Packages/{}/{}".format(PACKAGE_NAME, basename)
            settings = sublime.load_settings(basename)
            return (settings, filepath)

        @classmethod
        def additional_variables(cls):
            # type: () -> dict
            server, spec_dir = _resolve_server()
            return {
                "server_path": server,
                "spec_pack_dir": spec_dir,
            }

        @classmethod
        def needs_update_or_installation(cls):
            # type: () -> bool
            """Report whether the managed server has to be fetched.

            Deliberately free of network calls: LSP calls this before it
            spawns the installation thread.
            """
            if _user_server_path() or _development_server_path():
                return False
            version = _packaged_version()
            if version:
                return not _managed_server_path(version)
            # An unstamped (source) install has no release to pin to, so any
            # previously downloaded server is left alone.
            return not _installed_versions()

        @classmethod
        def install_or_update(cls):
            # type: () -> None
            _install_server()

        @classmethod
        def can_start(cls, window, initiating_view, workspace_folders, configuration):
            """Return an error string if the server cannot start."""
            server, _ = _resolve_server()
            if not server or not os.path.isfile(server):
                return (
                    "tcl-lsp server not available.  TclLsp downloads the "
                    "matching build from "
                    "https://github.com/{}/releases on first use; if this "
                    "machine has no access to it, install the server "
                    "yourself and set 'server_path' in "
                    "Preferences > Package Settings > TclLsp > LSP "
                    "Settings.".format(GITHUB_REPO)
                )
            return None

    TclLsp = _TclLspPlugin


# Lifecycle


def _legacy_package_installed():
    # type: () -> str
    """Return the filename of a pre-rename install still on disk, if any.

    Before this package was submitted to Package Control it was installed
    by hand as `Tcl.sublime-package`.  Left in place beside `TclLsp` it
    would double up every syntax and LSP client config.
    """
    directory = sublime.installed_packages_path()
    for name in ("Tcl.sublime-package",):
        if os.path.isfile(os.path.join(directory, name)):
            return name
    return ""


def _warn_about_legacy_package():
    # type: () -> None
    legacy = _legacy_package_installed()
    if not legacy:
        return
    state = _state()
    if state.get("legacy_package_warning_shown"):
        return
    state.set("legacy_package_warning_shown", True)
    _save_state()
    sublime.message_dialog(
        "Tcl Language Support (TclLsp)\n\n"
        "An older hand-installed copy of this package is still present as\n\n"
        "  " + os.path.join(sublime.installed_packages_path(), legacy) + "\n\n"
        "Delete it and restart Sublime Text — otherwise every Tcl syntax "
        "and the language server are registered twice."
    )


def _pending_setup_steps():
    # type: () -> list
    """Return the recommended-setup steps that have not been applied."""
    steps = []
    ignored = sublime.load_settings("Preferences.sublime-settings").get(
        "ignored_packages"
    )
    if "TCL" not in (ignored or []):
        steps.append(
            "disable Sublime Text's built-in TCL package, so each Tcl syntax "
            "appears once in the language menu"
        )
    if _HAS_LSP and not sublime.load_settings("LSP.sublime-settings").get(
        "semantic_highlighting"
    ):
        steps.append(
            "turn on LSP's semantic_highlighting, so tcl-lsp's semantic "
            "tokens reach the buffer"
        )
    return steps


def _apply_setup_steps():
    # type: () -> None
    """Apply the recommended setup.  Only ever called with consent."""
    prefs = sublime.load_settings("Preferences.sublime-settings")
    ignored = prefs.get("ignored_packages") or []
    if "TCL" not in ignored:
        ignored.append("TCL")
        prefs.set("ignored_packages", ignored)
        sublime.save_settings("Preferences.sublime-settings")
    if _HAS_LSP:
        lsp_settings = sublime.load_settings("LSP.sublime-settings")
        if not lsp_settings.get("semantic_highlighting"):
            lsp_settings.set("semantic_highlighting", True)
            sublime.save_settings("LSP.sublime-settings")


def _offer_recommended_setup(interactive):
    # type: (bool) -> None
    """Offer the recommended setup, editing settings only on a yes.

    `interactive` is True for the palette command, which reports "nothing
    to do" rather than staying silent.
    """
    steps = _pending_setup_steps()
    if not steps:
        if interactive:
            sublime.message_dialog(
                "Tcl Language Support (TclLsp)\n\n"
                "The recommended setup is already applied."
            )
        return
    prompt = (
        "Tcl Language Support (TclLsp)\n\n"
        "Apply the recommended setup?  This will:\n\n"
        + "".join("  • " + step + "\n" for step in steps)
        + "\nBoth are ordinary preferences you can change back at any "
        "time, and nothing else in your settings is touched."
    )
    if sublime.ok_cancel_dialog(prompt, "Apply"):
        _apply_setup_steps()


def _first_run_setup():
    # type: () -> None
    """Ask once, on the first load after installation."""
    state = _state()
    if state.get("setup_prompt_shown"):
        return
    state.set("setup_prompt_shown", True)
    _save_state()
    _offer_recommended_setup(interactive=False)


def _suggest_lsp_install():
    # type: () -> None
    """Show a one-time message suggesting LSP package installation."""
    state = _state()
    if state.get("lsp_suggestion_shown"):
        return
    state.set("lsp_suggestion_shown", True)
    _save_state()

    sublime.message_dialog(
        "Tcl Language Support (TclLsp)\n\n"
        "For full language server features (diagnostics, completions, "
        "hover, formatting, code actions, and more), install the LSP "
        "package from Package Control:\n\n"
        "  Command Palette > Package Control: Install Package > LSP\n\n"
        "Syntax highlighting, snippets, and settings work without LSP."
    )


def plugin_loaded():
    # type: () -> None
    """Called by Sublime Text after all packages are loaded."""
    global _HAS_LSP

    # Defer these so they don't interfere with the current load cycle.
    sublime.set_timeout(_warn_about_legacy_package, 2000)

    if TclLsp is not None:
        _HAS_LSP = True
        register_plugin(TclLsp)
        print("{}: registered LSP server plugin".format(PACKAGE_NAME))
        sublime.set_timeout(_first_run_setup, 3000)
    else:
        sublime.set_timeout(_suggest_lsp_install, 3000)
        sublime.set_timeout(_first_run_setup, 5000)


def plugin_unloaded():
    # type: () -> None
    """Called by Sublime Text when the plugin is unloaded."""
    if TclLsp is not None:
        unregister_plugin(TclLsp)


# Commands


class TclRecommendedSetupCommand(sublime_plugin.ApplicationCommand):
    """Offer the two settings changes this package recommends."""

    def run(self):
        # type: () -> None
        _offer_recommended_setup(interactive=True)


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


class TclFormatDocumentCommand(sublime_plugin.TextCommand):
    """Scope-gated wrapper around lsp_format_document.

    Sublime gates context-menu visibility through a command's
    is_visible() (the ``context`` key works for key bindings, not menus),
    so the menu uses this wrapper to keep Format Document out of non-Tcl
    buffers while the palette can still call lsp_format_document directly.
    """

    def run(self, edit):
        # type: (sublime.Edit) -> None
        self.view.run_command("lsp_format_document")

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
        map_path = map_path.strip()
        if not map_path or not os.path.isfile(map_path):
            sublime.error_message("Symbol map file not found: " + map_path)
            return
        with open(map_path, "r", encoding="utf-8") as f:
            map_text = f.read()
        # Send to LSP
        view = self.window.active_view()
        if view is None:
            sublime.error_message("No active view to unminify into.")
            return
        view.run_command(
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

    def is_visible(self):
        # type: () -> bool
        return _is_tcl_view(self.window.active_view())


# Dialect sync — automatically update the LSP dialect when the user
# selects a dialect-specific syntax from View > Syntax.


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
            "tcl_dialect", functools.partial(_check_view_dialect, view)
        )


# Helpers


def _is_tcl_view(view):
    # type: (sublime.View | None) -> bool
    """Return True if the view holds one of our package's syntaxes.

    Matches by scope rather than syntax-file path so it covers every
    dialect the package ships — the EDA, Expect, iApp and Tcl-version
    grammars all declare ``source.tcl``; iRules use ``source.irule``,
    F5 iApp APL uses ``source.tcl-apl`` and a BIG-IP config uses
    ``source.tcl-bigip`` (the same scope names the VS Code grammars use,
    so every editor agrees).
    """
    if view is None:
        return False
    sel = view.sel()
    point = sel[0].b if sel else 0
    return view.match_selector(
        point, "source.tcl, source.irule, source.tcl-apl, source.tcl-bigip"
    )
