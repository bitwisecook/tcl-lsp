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

import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerEvent
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.platform.lsp.api.LspServerState
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.CompilerExplorer")

/** How long [CompilerExplorerPanel.awaitRunningServer] waits for the lazily
 *  started Tcl LSP server to reach Running before giving up. */
private const val SERVER_WAIT_TIMEOUT_MS = 10_000L

class CompilerExplorerToolWindowFactory : ToolWindowFactory, DumbAware {

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        if (!isJcefSupported()) return
        val panel = CompilerExplorerPanel(project)
        val content = ContentFactory.getInstance().createContent(panel.browser.component, "", false)
        // Disposing the content tears down the panel: it unregisters from the
        // project service and disposes the JCEF browser, JS query, and the
        // editor-listener connection (all registered as children of the panel).
        content.setDisposer(panel)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project): Boolean = isJcefSupported()
}

internal class CompilerExplorerPanel(private val project: Project) : Disposable {

    val browser: JBCefBrowser = JBCefBrowser()
    private val jsQuery: JBCefJSQuery = JBCefJSQuery.create(browser as JBCefBrowserBase)

    // All three fields are touched only on the EDT (every mutator hops through
    // invokeLater), so no extra synchronisation is needed.
    private var lastSource: String = ""
    private var pageReady = false
    private var pendingSource: String? = null

    init {
        // Tie the native browser, the JS bridge query, and the editor-listener
        // connection to this panel's lifetime so they're released when the
        // tool-window content is disposed (see content.setDisposer).
        Disposer.register(this, browser)
        Disposer.register(this, jsQuery)
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

                    // The page's `message` listener is registered by load-end, so
                    // it is now safe to deliver source. onLoadEnd runs on a JCEF
                    // thread, so flip the ready flag and flush on the EDT. Any
                    // push that arrived before now (e.g. the "Open In" action's
                    // pushFile) was parked in pendingSource; deliver it, else
                    // fall back to the active editor.
                    ApplicationManager.getApplication().invokeLater {
                        pageReady = true
                        val pending = pendingSource
                        pendingSource = null
                        if (pending != null) {
                            sendSourceUpdate(pending)
                        } else {
                            pushFromActiveEditor(force = true)
                        }
                    }
                }
            }
        }, browser.cefBrowser)

        browser.loadHTML(getCompilerExplorerHtml())

        // Listen for file editor changes. fileOpened covers newly opened files;
        // selectionChanged covers switching between already-open tabs (including
        // the common case where the explorer is opened while a Tcl file is
        // already the active tab and no fileOpened event ever fires). The
        // connection is bound to this panel so it disconnects on dispose.
        project.messageBus.connect(this).subscribe(
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

    override fun dispose() {
        project.service<CompilerExplorerService>().unregister(this)
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

        // A sourceUpdate dispatched before the page registers its message
        // listener is silently lost, and lastSource would then suppress the
        // load-end retry. Park it until onLoadEnd marks the page ready.
        if (!pageReady) {
            pendingSource = source
            return
        }
        sendSourceUpdate(source)
    }

    private fun sendSourceUpdate(source: String) {
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

    /**
     * Resolve a running Tcl LSP server, kicking it and waiting briefly if one
     * isn't ready yet. Returns null only after a real timeout.
     *
     * The LSP server starts lazily on the first Tcl editor (see
     * [TclLspServerSupportProvider]). When the explorer tool window is restored
     * on IDE startup it pushes its first compile before that server has
     * finished initialising, which previously surfaced a spurious "LSP server
     * not running" error in the output pane. Poll for a Running server instead;
     * this runs on the [runCompile] background thread, so the sleep is safe.
     */
    @Suppress("UnstableApiUsage")
    private fun awaitRunningServer(timeoutMs: Long = SERVER_WAIT_TIMEOUT_MS): LspServer? {
        val manager = LspServerManager.getInstance(project)
        fun running(): LspServer? =
            manager.getServersForProvider(TclLspServerSupportProvider::class.java)
                .firstOrNull { it.state == LspServerState.Running }

        running()?.let { return it }

        // Kick the lazily-started server before the clock starts, so a busy EDT
        // during startup can't eat the timeout budget before the start request
        // even runs. invokeAndWait is safe here: this runs on a pooled thread,
        // never the EDT, so it can't deadlock against the dispatch it waits on.
        ApplicationManager.getApplication().invokeAndWait {
            manager.startServersIfNeeded(TclLspServerSupportProvider::class.java)
        }

        // Monotonic clock: a wall-clock adjustment must not distort the wait.
        val deadlineNanos = System.nanoTime() + timeoutMs * 1_000_000
        while (System.nanoTime() < deadlineNanos) {
            if (project.isDisposed) return null
            try {
                Thread.sleep(150)
            } catch (e: InterruptedException) {
                // Preserve cancellation semantics for the pooled-thread task.
                Thread.currentThread().interrupt()
                return null
            }
            running()?.let { return it }
        }
        return null
    }

    private fun runCompile(source: String, dialect: String) {
        // IntelliJ's pooled executor rather than the FJP common pool: the
        // awaitRunningServer wait can block this task for several seconds, which
        // would otherwise starve unrelated common-pool work.
        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                sendStatusToWebview("compiling")

                val server = awaitRunningServer()
                if (server == null) {
                    sendErrorToWebview(
                        "Tcl LSP server did not become ready within " +
                            "${SERVER_WAIT_TIMEOUT_MS / 1000}s — it may still be " +
                            "starting up, or be disabled or not installed."
                    )
                    return@executeOnPooledThread
                }

                // Pass the timeout explicitly. Omitting it makes Kotlin emit a
                // call to the synthetic `LspServer.sendRequestSync$default`
                // bridge, which is bound to the exact class that declared the
                // method when we compiled (2024.1). In 2026.1+ `sendRequestSync`
                // moved up to the `LspClient` super-interface, so that bridge no
                // longer resolves as `LspServer.sendRequestSync$default` and the
                // plugin fails verification / throws NoSuchMethodError at runtime.
                // The default value is a compile-time const (10_000 ms) that
                // inlines, so behaviour is unchanged and there is no runtime
                // reference to the constant. Do not "simplify" this back to the
                // no-timeout form. See the jetbrains-plugin-compat skill.
                val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
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
