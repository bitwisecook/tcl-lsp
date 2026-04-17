package com.tcllsp.jetbrains

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefJSQuery
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter
import java.util.concurrent.CompletableFuture
import javax.swing.Timer

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.CompilerExplorer")

class CompilerExplorerToolWindowFactory : ToolWindowFactory, DumbAware {

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = CompilerExplorerPanel(project)
        val content = ContentFactory.getInstance().createContent(panel.browser.component, "", false)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project): Boolean = true
}

private class CompilerExplorerPanel(private val project: Project) {

    val browser: JBCefBrowser = JBCefBrowser()
    private val jsQuery: JBCefJSQuery = JBCefJSQuery.create(browser)
    private var debounceTimer: Timer? = null
    private var lastSource: String = ""

    init {
        // Set up JS → Kotlin bridge
        jsQuery.addHandler { message ->
            handleJsMessage(message)
            null
        }

        // Load HTML once JCEF is ready
        browser.jbCefClient.addLoadHandler(object : CefLoadHandlerAdapter() {
            override fun onLoadEnd(cefBrowser: CefBrowser?, frame: org.cef.browser.CefFrame?, httpStatusCode: Int) {
                if (frame?.isMain == true) {
                    // Inject the bridge function
                    val bridgeJs = """
                        window.__tcllspBridge = function(msg) {
                            ${jsQuery.inject("msg")}
                        };
                    """.trimIndent()
                    cefBrowser?.executeJavaScript(bridgeJs, "", 0)

                    // Push initial source
                    pushSourceFromActiveEditor()
                }
            }
        }, browser.cefBrowser)

        browser.loadHTML(getCompilerExplorerHtml())

        // Listen for file editor changes
        project.messageBus.connect().subscribe(
            FileEditorManagerListener.FILE_EDITOR_MANAGER,
            object : FileEditorManagerListener {
                override fun fileOpened(source: FileEditorManager, file: VirtualFile) {
                    pushSourceFromActiveEditor()
                }

                override fun fileClosed(source: FileEditorManager, file: VirtualFile) {}
            }
        )
    }

    private fun pushSourceFromActiveEditor() {
        val editor = FileEditorManager.getInstance(project).selectedTextEditor ?: return
        val document = editor.document
        val file = FileEditorManager.getInstance(project).selectedFiles.firstOrNull() ?: return
        if (!TclFileType.isSupported(file)) return

        val source = document.text
        if (source == lastSource) return
        lastSource = source

        val dialect = TclLspSettings.getInstance().dialect
        val escaped = escapeForJs(source)
        val dialectEscaped = escapeForJs(dialect)
        browser.cefBrowser.executeJavaScript(
            "window.dispatchEvent(new MessageEvent('message', { data: { type: 'sourceUpdate', source: '$escaped', dialect: '$dialectEscaped' } }));",
            "", 0
        )
    }

    private fun handleJsMessage(message: String) {
        try {
            // Simple JSON-like parsing for the message types
            when {
                message.startsWith("compile:") -> {
                    val payload = message.removePrefix("compile:")
                    val parts = payload.split("\u0000", limit = 2)
                    val source = parts.getOrElse(0) { "" }
                    val dialect = parts.getOrElse(1) { TclLspSettings.getInstance().dialect }
                    runCompile(source, dialect)
                }
                message.startsWith("highlightSource:") -> {
                    // Source highlighting in main editor
                    val payload = message.removePrefix("highlightSource:")
                    val parts = payload.split(",")
                    if (parts.size == 2) {
                        val start = parts[0].toIntOrNull() ?: return
                        val end = parts[1].toIntOrNull() ?: return
                        highlightSourceRange(start, end)
                    }
                }
                message == "clearHighlight" -> {
                    clearSourceHighlight()
                }
            }
        } catch (e: Exception) {
            LOG.warn("Error handling JS message: $message", e)
        }
    }

    private fun runCompile(source: String, dialect: String) {
        CompletableFuture.runAsync {
            try {
                sendStatusToWebview("compiling")

                @Suppress("UnstableApiUsage")
                val servers = LspServerManager.getInstance(project)
                    .getServersForProvider(TclLspServerSupportProvider::class.java)
                val server = servers.firstOrNull()
                if (server == null) {
                    sendErrorToWebview("LSP server not running")
                    return@runAsync
                }

                val lsp4jServer = server.lsp4jServer
                val result = lsp4jServer.workspaceService.executeCommand(
                    org.eclipse.lsp4j.ExecuteCommandParams(
                        "tcl-lsp.compilerExplorer",
                        listOf(source, dialect)
                    )
                ).get()

                if (result != null) {
                    val json = com.google.gson.Gson().toJson(result)
                    val escaped = escapeForJs(json)
                    browser.cefBrowser.executeJavaScript(
                        "window.dispatchEvent(new MessageEvent('message', { data: { type: 'result', data: JSON.parse('$escaped') } }));",
                        "", 0
                    )
                }
            } catch (e: Exception) {
                LOG.warn("Compile failed", e)
                sendErrorToWebview(e.message ?: "Unknown error")
            }
        }
    }

    private fun sendStatusToWebview(status: String) {
        browser.cefBrowser.executeJavaScript(
            "window.dispatchEvent(new MessageEvent('message', { data: { type: 'status', text: '$status' } }));",
            "", 0
        )
    }

    private fun sendErrorToWebview(error: String) {
        val escaped = escapeForJs(error)
        browser.cefBrowser.executeJavaScript(
            "window.dispatchEvent(new MessageEvent('message', { data: { type: 'error', data: { error: '$escaped' } } }));",
            "", 0
        )
    }

    private fun highlightSourceRange(startOffset: Int, endOffset: Int) {
        // Highlight in the main editor — run on EDT
        com.intellij.openapi.application.ApplicationManager.getApplication().invokeLater {
            val editor = FileEditorManager.getInstance(project).selectedTextEditor ?: return@invokeLater
            val document = editor.document
            if (startOffset < 0 || endOffset > document.textLength) return@invokeLater

            val startPos = editor.offsetToLogicalPosition(startOffset)
            val endPos = editor.offsetToLogicalPosition(endOffset)
            editor.selectionModel.setSelection(startOffset, endOffset)
            editor.scrollingModel.scrollTo(startPos, com.intellij.openapi.editor.ScrollType.CENTER_UP)
        }
    }

    private fun clearSourceHighlight() {
        com.intellij.openapi.application.ApplicationManager.getApplication().invokeLater {
            val editor = FileEditorManager.getInstance(project).selectedTextEditor ?: return@invokeLater
            editor.selectionModel.removeSelection()
        }
    }

    private fun escapeForJs(s: String): String =
        s.replace("\\", "\\\\")
            .replace("'", "\\'")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
}
