package com.tcllsp.jetbrains.actions

import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.Project

// ---------------------------------------------------------------------------
// Document-modifying commands. The LSP server returns a WorkspaceEdit that
// lsp4j applies in-place, so the action just dispatches the command.
// ---------------------------------------------------------------------------

class OptimiseDocumentAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.optimiseDocument"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri, "full")
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Optimisations applied", NotificationType.INFORMATION)
    }
}

class MinifyDocumentAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.minifyDocument"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Document minified", NotificationType.INFORMATION)
    }
}

class FixAllSafeIssuesAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.fixAllSafeIssues"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Safe quick fixes applied", NotificationType.INFORMATION)
    }
}

// ---------------------------------------------------------------------------
// Source-as-input commands that return generated content. The result is
// opened in a scratch editor so the user can save / iterate on it.
// ---------------------------------------------------------------------------

class TkPreviewAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.tkPreview"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val source = documentText ?: return null
        return listOf(source)
    }
    override fun resultExtension(result: Any?): String = "html"
}

class TranslateXcAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.xcTranslate"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val source = documentText ?: return null
        return listOf(source, "both")
    }
    override fun resultExtension(result: Any?): String = "json"
}

class DiagramDataAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.diagramData"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val source = documentText ?: return null
        return listOf(source)
    }
    override fun resultExtension(result: Any?): String = "mmd"
}

// ---------------------------------------------------------------------------
// URI-as-input commands. extractLinkedObjects + bigipCleanup both want the
// current document's URI plus a default options dict; the server fills in
// the rest from the parsed config.
// ---------------------------------------------------------------------------

class BigipCleanupAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.bigipCleanup"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun resultExtension(result: Any?): String = "tmsh"
}

class ExtractLinkedObjectsAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.extractLinkedObjects"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun resultExtension(result: Any?): String = "json"
}

// ---------------------------------------------------------------------------
// Information / catalogue queries. No document context required.
// ---------------------------------------------------------------------------

class ListIruleEventsAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.listIruleEvents"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> = emptyList()
    override fun resultExtension(result: Any?): String = "json"
}

class ListKnownPackagesAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.listKnownPackages"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> = emptyList()
    override fun resultExtension(result: Any?): String = "json"
}

class GetEffectiveConfigAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.getEffectiveConfig"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return if (documentUri != null) listOf(documentUri) else listOf("")
    }
    override fun resultExtension(result: Any?): String = "json"
}

// ---------------------------------------------------------------------------
// Prompt-driven catalogue lookups.
// ---------------------------------------------------------------------------

class DescribeIruleEventAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.describeIruleEvent"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "iRule event name (e.g. HTTP_REQUEST):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class DescribeIruleCommandAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.describeIruleCommand"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "iRule command name (e.g. HTTP::redirect):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class SearchHelpAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.searchHelp"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "Search Tcl LSP help for:"), false)
    }
    override fun resultExtension(result: Any?): String = "json"
}

class SuggestPackagesForSymbolAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.suggestPackagesForSymbol"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "Symbol (e.g. ::json::parse):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class ListSubcommandsAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.listSubcommands"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "Ensemble command (e.g. string, dict, array):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class RenamePartitionAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.renamePartition"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        val oldName = prompt(project, "Current partition name (e.g. Common):", "Common")
        val newName = prompt(project, "New partition name:")
        return listOf(uri, oldName, newName)
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Partition renamed", NotificationType.INFORMATION)
    }
}
