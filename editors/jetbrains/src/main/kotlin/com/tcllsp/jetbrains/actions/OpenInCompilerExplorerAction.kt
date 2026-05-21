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

/**
 * Editor / project-view context-menu action that opens the current Tcl file in
 * the Compiler Explorer tool window and pushes its source for compilation.
 */
class OpenInCompilerExplorerAction : AnAction(), DumbAware {

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE)
        e.presentation.isEnabledAndVisible =
            e.project != null && file != null && !file.isDirectory && TclFileType.isSupported(file)
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
        if (!TclFileType.isSupported(file)) return

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
