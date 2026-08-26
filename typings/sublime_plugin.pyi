# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Type stubs for the `sublime_plugin` module, provided by the Sublime Text host.

Declares only the base classes `editors/sublime-text/plugin.py` subclasses.
"""

import sublime

class EventListener:
    def on_activated(self, view: sublime.View) -> None: ...
    def on_close(self, view: sublime.View) -> None: ...

class TextCommand:
    view: sublime.View
    def run(self, edit: sublime.Edit) -> None: ...

class WindowCommand:
    window: sublime.Window
    def run(self) -> None: ...

class ApplicationCommand:
    def run(self) -> None: ...
