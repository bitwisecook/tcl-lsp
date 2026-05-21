package com.tcllsp.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerEvent
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter
import java.util.concurrent.CompletableFuture

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.CompilerExplorer")

class CompilerExplorerToolWindowFactory : ToolWindowFactory, DumbAware {

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = CompilerExplorerPanel(project)
        val content = ContentFactory.getInstance().createContent(panel.browser.component, "", false)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project): Boolean = true
}

internal class CompilerExplorerPanel(private val project: Project) {

    val browser: JBCefBrowser = JBCefBrowser()
    private val jsQuery: JBCefJSQuery = JBCefJSQuery.create(browser as JBCefBrowserBase)
    private var lastSource: String = ""

    init {
        project.service<CompilerExplorerService>().register(this)

        // Set up JS → Kotlin bridge
        jsQuery.addHandler { message ->
            handleJsMessage(message)
            null
        }

        // Load HTML once JCEF is ready
        browser.jbCefClient.addLoadHandler(object : CefLoadHandlerAdapter() {
            override fun onLoadEnd(cefBrowser: CefBrowser?, frame: org.cef.browser.CefFrame?, httpStatusCode: Int) {
                if (frame?.isMain == true) {
                    // Install the JS→Kotlin bridge, then drain any messages
                    // (compile/highlight/etc.) that the page enqueued while it
                    // was loading via the shim in `adaptHtmlForJcef`.
                    val bridgeJs = """
                        window.__tcllspBridge = function(msg) {
                            ${jsQuery.inject("msg")}
                        };
                        if (typeof window.__tcllspFlushQueue === 'function') {
                            window.__tcllspFlushQueue();
                        }
                    """.trimIndent()
                    cefBrowser?.executeJavaScript(bridgeJs, "", 0)

                    // Push initial source. onLoadEnd runs on a JCEF thread, so
                    // the editor read must hop to the EDT — see pushFromActiveEditor.
                    pushFromActiveEditor()
                }
            }
        }, browser.cefBrowser)

        browser.loadHTML(getCompilerExplorerHtml())

        // Listen for file editor changes. fileOpened covers newly opened files;
        // selectionChanged covers switching between already-open tabs (including
        // the common case where the explorer is opened while a Tcl file is
        // already the active tab and no fileOpened event ever fires).
        project.messageBus.connect().subscribe(
            FileEditorManagerListener.FILE_EDITOR_MANAGER,
            object : FileEditorManagerListener {
                override fun fileOpened(source: FileEditorManager, file: VirtualFile) {
                    pushFromActiveEditor()
                }

                override fun selectionChanged(event: FileEditorManagerEvent) {
                    pushFromActiveEditor()
                }

                override fun fileClosed(source: FileEditorManager, file: VirtualFile) {}
            }
        )
    }

    /**
     * Read the active editor's Tcl source and push it to the webview.
     *
     * Editor access (`selectedTextEditor`, document text) is only valid on the
     * EDT, but this is invoked both from the EDT (editor listeners) and from a
     * JCEF callback thread (`onLoadEnd`). Hopping through `invokeLater`
     * normalises that — without it the initial load-time push silently fails
     * off-EDT and the IR pane stays stuck on "Waiting for source from editor...".
     */
    fun pushFromActiveEditor(force: Boolean = false) {
        ApplicationManager.getApplication().invokeLater {
            val manager = FileEditorManager.getInstance(project)
            val editor = manager.selectedTextEditor ?: return@invokeLater
            val file = manager.selectedFiles.firstOrNull() ?: return@invokeLater
            if (!TclFileType.isSupported(file)) return@invokeLater
            dispatchSource(editor.document.text, force)
        }
    }

    /** Push a specific file's source, used by the "Open In" action. */
    fun pushFile(file: VirtualFile, force: Boolean = true) {
        ApplicationManager.getApplication().invokeLater {
            if (!TclFileType.isSupported(file)) return@invokeLater
            val document = FileDocumentManager.getInstance().getDocument(file) ?: return@invokeLater
            dispatchSource(document.text, force)
        }
    }

    private fun dispatchSource(source: String, force: Boolean) {
        if (!force && source == lastSource) return
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

                val result = server.sendRequestSync { lsp4j ->
                    lsp4j.workspaceService.executeCommand(
                        org.eclipse.lsp4j.ExecuteCommandParams(
                            "tcl-lsp.compilerExplorer",
                            listOf(source, dialect)
                        )
                    )
                }

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
        ApplicationManager.getApplication().invokeLater {
            val editor = FileEditorManager.getInstance(project).selectedTextEditor ?: return@invokeLater
            val document = editor.document
            if (startOffset < 0 || endOffset > document.textLength) return@invokeLater

            val startPos = editor.offsetToLogicalPosition(startOffset)
            editor.selectionModel.setSelection(startOffset, endOffset)
            editor.scrollingModel.scrollTo(startPos, com.intellij.openapi.editor.ScrollType.CENTER_UP)
        }
    }

    private fun clearSourceHighlight() {
        ApplicationManager.getApplication().invokeLater {
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
