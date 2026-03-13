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
