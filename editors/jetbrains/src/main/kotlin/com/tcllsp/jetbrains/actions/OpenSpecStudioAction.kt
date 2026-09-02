// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.wm.ToolWindowManager
import com.tcllsp.jetbrains.isJcefSupported

class OpenSpecStudioAction : AnAction(), DumbAware {
    override fun actionPerformed(event: AnActionEvent) {
        val project = event.project ?: return
        if (!isJcefSupported()) return
        ToolWindowManager.getInstance(project).getToolWindow("Tcl Spec Studio")?.show()
    }

    override fun update(event: AnActionEvent) {
        event.presentation.isEnabledAndVisible =
            event.project?.basePath != null && isJcefSupported()
    }
}
