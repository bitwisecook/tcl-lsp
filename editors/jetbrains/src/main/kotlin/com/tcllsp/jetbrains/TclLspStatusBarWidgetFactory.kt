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

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.StatusBar
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.StatusBarWidgetFactory
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.options.ShowSettingsUtil
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.util.Consumer
import com.tcllsp.jetbrains.settings.TclLspSettings
import java.awt.event.MouseEvent

class TclLspStatusBarWidgetFactory : StatusBarWidgetFactory {

    override fun getId(): String = "TclLspStatusBar"

    override fun getDisplayName(): String = "Tcl Language Server"

    override fun isAvailable(project: Project): Boolean = true

    override fun createWidget(project: Project): StatusBarWidget = TclLspStatusBarWidget(project)

    override fun canBeEnabledOn(statusBar: StatusBar): Boolean = true
}

private class TclLspStatusBarWidget(private val project: Project) : StatusBarWidget, StatusBarWidget.TextPresentation {

    private var statusBar: StatusBar? = null

    override fun ID(): String = "TclLspStatusBar"

    override fun install(statusBar: StatusBar) {
        this.statusBar = statusBar

        // Listen for file editor changes to show/hide
        project.messageBus.connect().subscribe(
            FileEditorManagerListener.FILE_EDITOR_MANAGER,
            object : FileEditorManagerListener {
                override fun fileOpened(source: FileEditorManager, file: VirtualFile) {
                    statusBar.updateWidget(ID())
                }

                override fun fileClosed(source: FileEditorManager, file: VirtualFile) {
                    statusBar.updateWidget(ID())
                }
            }
        )
    }

    override fun dispose() {
        statusBar = null
    }

    override fun getPresentation(): StatusBarWidget.WidgetPresentation = this

    override fun getText(): String {
        val settings = TclLspSettings.getInstance()
        val dialectLabel = TclLspSettings.DIALECT_OPTIONS.firstOrNull { it.first == settings.dialect }?.second
            ?: settings.dialect
        return "tcl-lsp | $dialectLabel"
    }

    override fun getTooltipText(): String =
        "Tcl Language Server — click to open settings"

    override fun getAlignment(): Float = 0f

    override fun getClickConsumer(): Consumer<MouseEvent> = Consumer {
        ShowSettingsUtil.getInstance().showSettingsDialog(project, "Tcl Language Server")
    }
}
