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

            val result = server.sendRequestSync { lsp4j ->
                lsp4j.workspaceService.executeCommand(
                    ExecuteCommandParams(commandId, args)
                )
            }

            ApplicationManager.getApplication().invokeLater {
                presentResult(project, result)
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
