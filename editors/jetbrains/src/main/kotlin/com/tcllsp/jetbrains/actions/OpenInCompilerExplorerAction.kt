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

package com.tcllsp.jetbrains.actions

import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.components.service
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.wm.ToolWindowManager
import com.tcllsp.jetbrains.CompilerExplorerService
import com.tcllsp.jetbrains.TclFileType
import com.tcllsp.jetbrains.isJcefSupported

/**
 * Editor / project-view context-menu action that opens the current Tcl file in
 * the Compiler Explorer tool window and pushes its source for compilation.
 */
class OpenInCompilerExplorerAction : AnAction(), DumbAware {

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE)
        e.presentation.isEnabledAndVisible =
            e.project != null && file != null && !file.isDirectory &&
                TclFileType.isSupported(file) && isJcefSupported()
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
        if (!TclFileType.isSupported(file) || !isJcefSupported()) return

        // Make sure the file is open in an editor so highlight round-trips have
        // a document to target, then reveal the explorer and push the source.
        FileEditorManager.getInstance(project).openFile(file, true)

        val toolWindow = ToolWindowManager.getInstance(project)
            .getToolWindow("Tcl Compiler Explorer") ?: return
        toolWindow.activate {
            project.service<CompilerExplorerService>().pushFile(file)
        }
    }
}
