// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.intellij.openapi.components.Service
import com.intellij.openapi.vfs.VirtualFile

/**
 * Project-level handle to the live Compiler Explorer tool-window panel.
 *
 * The panel registers itself on creation so actions (e.g. the editor
 * "Open In Tcl Compiler Explorer" entry) can push a specific file into the
 * already-loaded webview without reaching into the tool-window internals.
 */
@Service(Service.Level.PROJECT)
class CompilerExplorerService {

    @Volatile
    private var panel: CompilerExplorerPanel? = null

    internal fun register(panel: CompilerExplorerPanel) {
        this.panel = panel
    }

    /** Clear the reference when the panel is disposed, but only if it's ours. */
    internal fun unregister(panel: CompilerExplorerPanel) {
        if (this.panel === panel) {
            this.panel = null
        }
    }

    /** Push the given file's source into the explorer, if the panel is live. */
    fun pushFile(file: VirtualFile) {
        panel?.pushFile(file)
    }
}
