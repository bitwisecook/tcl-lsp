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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Minimal SublimeLSP helper for tcl-lsp.

Sublime Text already ships a Tcl syntax and snippets. This package leaves
those resources alone, registers tcl-lsp with the base LSP package, and
downloads the matching native server on first use. Released packages pin the
SHA-256 digest of every downloadable server in ``server_version.json``.

The plugin runs in Sublime Text's Python 3.8 plugin host.
"""

import hashlib
import json
import os
import re
import shutil
import tempfile
import urllib.request

import sublime  # type: ignore[import-not-found]
import sublime_plugin  # type: ignore[import-not-found]

PACKAGE_NAME = "LSP-Tcl"
SETTINGS_KEY = "LSP-Tcl.sublime-settings"
STATE_KEY = "LSP-Tcl-state.sublime-settings"
GITHUB_REPO = "bitwisecook/tcl-lsp"
SERVER_BASENAME = "tcl-lsp-server"
CHECKSUM_ASSET = "SHA256SUMS"
VERSION_RESOURCE = "Packages/{}/server_version.json".format(PACKAGE_NAME)
BUILTIN_TCL_SYNTAX = "Packages/TCL/Tcl.sublime-syntax"

SERVER_TRIPLES = {
    ("linux", "x64"): "x86_64-unknown-linux-gnu",
    ("linux", "arm64"): "aarch64-unknown-linux-gnu",
    ("osx", "x64"): "x86_64-apple-darwin",
    ("osx", "arm64"): "aarch64-apple-darwin",
    ("windows", "x64"): "x86_64-pc-windows-msvc",
    ("windows", "arm64"): "aarch64-pc-windows-msvc",
}

# File types understood by tcl-lsp that Sublime's built-in TCL package may not
# claim. We assign the built-in syntax only when the view is still plain text,
# so a user's explicit or third-party syntax always wins.
TCL_FILE_EXTENSIONS = {
    # @generated:file-extensions:begin
    "tcl",
    "tk",
    "itcl",
    "tm",
    "test",
    "globals",
    "exp",
    "expect",
    "scf",
    "iapp",
    "iappimpl",
    "impl",
    "irul",
    "irule",
    "irules",
    "tmsh",
    "qsf",
    "qpf",
    "qip",
    "do",
    "tclspec",
    "sslictcl",
    "sdc",
    "upf",
    "xdc",
    "apl",
    # @generated:file-extensions:end
}

_RELEASE_VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")
_LSP_PLUGIN_CLASS = None  # type: ignore[var-annotated]


def _package_dir():
    # type: () -> str
    return os.path.join(sublime.packages_path(), PACKAGE_NAME)


def _server_filename():
    # type: () -> str
    if sublime.platform() == "windows":
        return SERVER_BASENAME + ".exe"
    return SERVER_BASENAME


def _server_triple():
    # type: () -> str
    return SERVER_TRIPLES.get((sublime.platform(), sublime.arch()), "")


def _ensure_executable(path):
    # type: (str) -> str
    if path and os.name != "nt":
        try:
            mode = os.stat(path).st_mode
            if not (mode & 0o111):
                os.chmod(path, 0o755)
        except OSError:
            pass
    return path


def _settings():
    # type: () -> sublime.Settings
    return sublime.load_settings(SETTINGS_KEY)


def _state():
    # type: () -> sublime.Settings
    return sublime.load_settings(STATE_KEY)


def _user_server_path():
    # type: () -> str
    path = _settings().get("server_path") or ""
    if path and os.path.isfile(path):
        return _ensure_executable(path)
    return ""


def _development_server_path():
    # type: () -> str
    candidate = os.path.join(_package_dir(), "server", _server_filename())
    if os.path.isfile(candidate):
        return _ensure_executable(candidate)
    return ""


def _version_manifest():
    # type: () -> dict
    try:
        raw = sublime.load_resource(VERSION_RESOURCE)
        return json.loads(raw) or {}
    except (OSError, ValueError):
        return {}


def _packaged_version():
    # type: () -> str
    version = _version_manifest().get("version") or ""
    return version if _RELEASE_VERSION_RE.match(version) else ""


def _pinned_digest(triple):
    # type: (str) -> str
    return (_version_manifest().get("servers") or {}).get(triple) or ""


def _managed_dir(version):
    # type: (str) -> str
    if _LSP_PLUGIN_CLASS is None:
        return ""
    return os.path.join(_LSP_PLUGIN_CLASS.storage_path(), PACKAGE_NAME, version)


def _managed_server_path(version):
    # type: (str) -> str
    candidate = os.path.join(_managed_dir(version), _server_filename())
    if os.path.isfile(candidate):
        return _ensure_executable(candidate)
    return ""


def _installed_versions():
    # type: () -> list
    if _LSP_PLUGIN_CLASS is None:
        return []
    root = os.path.join(_LSP_PLUGIN_CLASS.storage_path(), PACKAGE_NAME)
    try:
        names = sorted(os.listdir(root))
    except OSError:
        return []
    return [name for name in names if _managed_server_path(name)]


def _user_agent():
    # type: () -> str
    return "{}/Sublime-Text (+https://github.com/{})".format(PACKAGE_NAME, GITHUB_REPO)


def _fetch(url, timeout=30):
    # type: (str, int) -> bytes
    request = urllib.request.Request(url, headers={"User-Agent": _user_agent()})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def _latest_release_version():
    # type: () -> str
    url = "https://api.github.com/repos/{}/releases/latest".format(GITHUB_REPO)
    tag = (json.loads(_fetch(url).decode("utf-8")) or {}).get("tag_name") or ""
    version = tag[1:] if tag.startswith("v") else tag
    if not _RELEASE_VERSION_RE.match(version):
        raise RuntimeError("latest tcl-lsp release is not a plain version tag")
    return version


def _asset_url(version, asset):
    # type: (str, str) -> str
    return "https://github.com/{}/releases/download/v{}/{}".format(
        GITHUB_REPO, version, asset
    )


def _release_checksum(version, asset):
    # type: (str, str) -> str
    sums = _fetch(_asset_url(version, CHECKSUM_ASSET)).decode("utf-8")
    for line in sums.splitlines():
        parts = line.split()
        if len(parts) == 2 and os.path.basename(parts[1]) == asset:
            return parts[0]
    raise RuntimeError(
        "release v{} has no {} entry for {}".format(version, CHECKSUM_ASSET, asset)
    )


def _download_verified(url, expected_sha256, destination):
    # type: (str, str, str) -> None
    digest = hashlib.sha256()
    request = urllib.request.Request(url, headers={"User-Agent": _user_agent()})
    with urllib.request.urlopen(request, timeout=120) as response:
        with open(destination, "wb") as handle:
            while True:
                chunk = response.read(256 * 1024)
                if not chunk:
                    break
                digest.update(chunk)
                handle.write(chunk)
    actual = digest.hexdigest()
    if actual != expected_sha256:
        os.remove(destination)
        raise RuntimeError(
            "checksum mismatch: expected {}, got {}".format(expected_sha256, actual)
        )


def _prune_managed_versions(keep):
    # type: (str) -> None
    if _LSP_PLUGIN_CLASS is None:
        return
    root = os.path.join(_LSP_PLUGIN_CLASS.storage_path(), PACKAGE_NAME)
    try:
        versions = os.listdir(root)
    except OSError:
        return
    for version in versions:
        if version != keep:
            shutil.rmtree(os.path.join(root, version), ignore_errors=True)


def _install_server():
    # type: () -> None
    triple = _server_triple()
    if not triple:
        raise RuntimeError(
            "no tcl-lsp-server build for {}-{}; set 'server_path' in "
            "LSP-Tcl settings".format(sublime.platform(), sublime.arch())
        )

    version = _packaged_version() or _latest_release_version()
    suffix = ".exe" if sublime.platform() == "windows" else ""
    asset = "{}-{}{}".format(SERVER_BASENAME, triple, suffix)
    target_dir = _managed_dir(version)
    if not target_dir:
        raise RuntimeError("the LSP package is not available")

    expected = _pinned_digest(triple)
    pinned = bool(expected)
    if not expected:
        expected = _release_checksum(version, asset)

    parent = os.path.dirname(target_dir)
    os.makedirs(parent, exist_ok=True)
    staging = tempfile.mkdtemp(prefix=".{}-".format(version), dir=parent)
    try:
        binary = os.path.join(staging, _server_filename())
        _download_verified(_asset_url(version, asset), expected, binary)
        _ensure_executable(binary)
        shutil.rmtree(target_dir, ignore_errors=True)
        os.rename(staging, target_dir)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise

    _prune_managed_versions(version)
    integrity = "package-pinned digest" if pinned else "release SHA256SUMS fallback"
    print(
        "{}: installed tcl-lsp-server {} ({}; {})".format(
            PACKAGE_NAME, version, triple, integrity
        )
    )


def _resolve_server():
    # type: () -> str
    path = _user_server_path() or _development_server_path()
    if path:
        return path
    version = _packaged_version()
    candidates = [version] if version else []
    candidates.extend(reversed(_installed_versions()))
    for candidate in candidates:
        managed = _managed_server_path(candidate)
        if managed:
            return managed
    return ""


try:
    from LSP.plugin import AbstractPlugin, register_plugin, unregister_plugin
except ImportError:

    def register_plugin(plugin):
        # type: (type) -> None
        raise RuntimeError("sublimelsp/LSP is not installed")

    def unregister_plugin(plugin):
        # type: (type) -> None
        raise RuntimeError("sublimelsp/LSP is not installed")

else:

    class _LspTclPlugin(AbstractPlugin):
        @classmethod
        def name(cls):
            # type: () -> str
            return "Tcl"

        @classmethod
        def additional_variables(cls):
            # type: () -> dict
            return {"server_path": _resolve_server()}

        @classmethod
        def needs_update_or_installation(cls):
            # type: () -> bool
            if _user_server_path() or _development_server_path():
                return False
            version = _packaged_version()
            if version:
                return not _managed_server_path(version)
            return not _installed_versions()

        @classmethod
        def install_or_update(cls):
            # type: () -> None
            _install_server()

        @classmethod
        def can_start(cls, window, initiating_view, workspace_folders, configuration):
            if _resolve_server():
                return None
            return (
                "tcl-lsp-server is unavailable; check network access or set "
                "'server_path' in Preferences > Package Settings > LSP > "
                "Servers > LSP-Tcl."
            )

    _LSP_PLUGIN_CLASS = _LspTclPlugin


def _suggest_lsp_install():
    # type: () -> None
    state = _state()
    if state.get("lsp_suggestion_shown"):
        return
    state.set("lsp_suggestion_shown", True)
    sublime.save_settings(STATE_KEY)
    sublime.message_dialog(
        "LSP-Tcl needs the base LSP package.\n\n"
        "Install it with Package Control: Install Package > LSP.\n\n"
        "Sublime Text's built-in TCL package continues to provide syntax "
        "highlighting and snippets."
    )


def _assign_builtin_tcl_syntax(view):
    # type: (sublime.View) -> None
    filename = view.file_name()
    if not filename or "." not in os.path.basename(filename):
        return
    extension = filename.rsplit(".", 1)[1].lower()
    syntax = view.syntax()
    if extension in TCL_FILE_EXTENSIONS and syntax and syntax.scope == "text.plain":
        view.assign_syntax(BUILTIN_TCL_SYNTAX)


class LspTclFileAssociationListener(sublime_plugin.EventListener):
    """Use Sublime's built-in Tcl syntax for otherwise-unclaimed Tcl files."""

    def on_load_async(self, view):
        # type: (sublime.View) -> None
        _assign_builtin_tcl_syntax(view)

    def on_post_save_async(self, view):
        # type: (sublime.View) -> None
        _assign_builtin_tcl_syntax(view)


def plugin_loaded():
    # type: () -> None
    if _LSP_PLUGIN_CLASS is None:
        sublime.set_timeout(_suggest_lsp_install, 2000)
        return
    register_plugin(_LSP_PLUGIN_CLASS)
    print("{}: registered tcl-lsp".format(PACKAGE_NAME))


def plugin_unloaded():
    # type: () -> None
    if _LSP_PLUGIN_CLASS is not None:
        unregister_plugin(_LSP_PLUGIN_CLASS)
