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

import com.google.gson.Gson
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.testFramework.LightVirtualFile
import com.tcllsp.jetbrains.TclFileType
import com.tcllsp.jetbrains.TclLspServerSupportProvider
import org.eclipse.lsp4j.ExecuteCommandParams

/**
 * Base class for actions that dispatch an LSP `workspace/executeCommand`
 * to tcl-lsp. Subclasses provide:
 *
 *  - the LSP command id (`commandId`)
 *  - how to build the argument list (`buildArguments`)
 *  - how to present the result (`presentResult`, default: open as
 *    a scratch file when the result is a non-empty string)
 *
 * `getActionUpdateThread()` returns BGT because we only read the
 * current editor's virtual file in `update()` — cheap, but the IDE
 * still wants the declaration to be explicit for 2024.x+.
 */
abstract class TclLspActionBase : AnAction() {
    /** Fully-qualified LSP command identifier (e.g. `tcl-lsp.optimiseDocument`). */
    protected abstract val commandId: String

    /**
     * Human-readable label for the result scratch file when one is
     * created. Default uses the action's text.
     */
    protected open val resultLabel: String
        get() = templatePresentation.text ?: commandId

    /**
     * Whether this action requires an active Tcl editor. Set to false
     * for queries that don't take a document (e.g. listIruleEvents).
     */
    protected open val needsEditor: Boolean = true

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        if (!needsEditor) {
            e.presentation.isEnabledAndVisible = e.project != null
            return
        }
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE)
        e.presentation.isEnabledAndVisible = e.project != null &&
            file != null &&
            file.fileType is TclFileType
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE)
        val editor = e.getData(CommonDataKeys.EDITOR)
        val args = try {
            buildArguments(project, file?.url, editor?.document?.text, e)
        } catch (cancel: ActionCancelledException) {
            return
        }
        if (args == null) return

        object : Task.Backgroundable(project, "Running ${templatePresentation.text}", true) {
            override fun run(indicator: ProgressIndicator) {
                runCommand(project, args)
            }
        }.queue()
    }

    private fun runCommand(project: Project, args: List<Any>) {
        try {
            @Suppress("UnstableApiUsage")
            val servers = LspServerManager.getInstance(project)
                .getServersForProvider(TclLspServerSupportProvider::class.java)
            val server = servers.firstOrNull()
            if (server == null) {
                notify(project, "LSP server not running", NotificationType.ERROR)
                return
            }

            // Pass the timeout explicitly. Omitting it makes Kotlin emit a call
            // to the synthetic `LspServer.sendRequestSync$default` bridge, bound
            // to the class that declared the method when we compiled (2024.1).
            // In 2026.1+ `sendRequestSync` moved up to the `LspClient`
            // super-interface, so that bridge no longer resolves as
            // `LspServer.sendRequestSync$default` and the plugin fails
            // verification / throws NoSuchMethodError at runtime. The default is
            // a compile-time const (10_000 ms) that inlines, so behaviour is
            // unchanged. Do not "simplify" this back to the no-timeout form.
            // See the jetbrains-plugin-compat skill.
            val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                lsp4j.workspaceService.executeCommand(
                    ExecuteCommandParams(commandId, args)
                )
            }

            ApplicationManager.getApplication().invokeLater {
                if (acceptResult(project, result, args)) {
                    presentResult(project, result)
                }
            }
        } catch (ex: Exception) {
            notify(project, ex.message ?: "Command failed", NotificationType.ERROR)
        }
    }

    /**
     * Build the argument list for the LSP command, or return null to
     * cancel the dispatch. Throw `ActionCancelledException` from a
     * prompt callback to abort silently.
     */
    protected abstract fun buildArguments(
        project: Project,
        documentUri: String?,
        documentText: String?,
        event: AnActionEvent,
    ): List<Any>?

    /**
     * Default presentation: a non-empty string or stringifiable result
     * goes into a scratch editor tab so the user can read / save it.
     * Override to send the result to a tool window or a notification.
     */
    protected open fun presentResult(project: Project, result: Any?) {
        if (result == null) return
        val text = stringifyResult(result) ?: return
        if (text.isBlank()) return
        val ext = resultExtension(result)
        val name = scratchFileName(ext)
        val virtual = LightVirtualFile(name, text)
        FileEditorManager.getInstance(project)
            .openTextEditor(OpenFileDescriptor(project, virtual), true)
    }

    /**
     * Gives a source-backed action a final chance to reject a response before
     * it is presented. Most command results are not snapshot-sensitive, but a
     * static preview must never open a model for an obsolete document.
     */
    protected open fun acceptResult(project: Project, result: Any?, args: List<Any>): Boolean = true

    /** File extension for the scratch result, without the leading dot. */
    protected open fun resultExtension(result: Any?): String = "txt"

    private fun scratchFileName(ext: String): String {
        val slug = resultLabel
            .lowercase()
            .replace(Regex("[^a-z0-9]+"), "-")
            .trim('-')
            .ifBlank { "tcl-lsp-result" }
        return "$slug.$ext"
    }

    protected open fun stringifyResult(result: Any?): String? {
        if (result == null) return null
        if (result is String) return result
        return try {
            Gson().toJson(result)
        } catch (_: Exception) {
            result.toString()
        }
    }

    protected fun prompt(project: Project, label: String, default: String = ""): String {
        val value = Messages.showInputDialog(project, label, templatePresentation.text, null, default, null)
            ?: throw ActionCancelledException()
        if (value.isBlank()) throw ActionCancelledException()
        return value.trim()
    }

    protected fun notify(project: Project, message: String, type: NotificationType) {
        com.intellij.notification.NotificationGroupManager.getInstance()
            .getNotificationGroup("Tcl LSP")
            .createNotification("Tcl LSP", message, type)
            .notify(project)
    }
}

class ActionCancelledException : RuntimeException()
