# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Type stubs for `LSP.plugin`, provided by the sublimelsp/LSP package.

An optional third-party dependency of the Sublime plugin: `plugin.py` imports it
lazily and degrades gracefully when it is absent, so it is never installed in a
type-checking environment.
"""

from typing import Any

import sublime

class AbstractPlugin:
    @classmethod
    def name(cls) -> str: ...
    @classmethod
    def configuration(cls) -> tuple[sublime.Settings, str]: ...
    @classmethod
    def additional_variables(cls) -> dict[str, str]: ...
    @classmethod
    def storage_path(cls) -> str: ...
    @classmethod
    def needs_update_or_installation(cls) -> bool: ...
    @classmethod
    def install_or_update(cls) -> None: ...
    @classmethod
    def can_start(
        cls,
        window: sublime.Window,
        initiating_view: sublime.View,
        workspace_folders: list[Any],
        configuration: Any,
    ) -> str | None: ...

def register_plugin(plugin: type[AbstractPlugin]) -> None: ...
def unregister_plugin(plugin: type[AbstractPlugin]) -> None: ...
