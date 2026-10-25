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
import com.google.gson.Gson
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import com.intellij.openapi.editor.Document
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
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.CompilerExplorer")


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

    // Source and request state is read and changed only on the EDT.
    private val panelLifetime = Any()
    private val panelId = java.util.UUID.randomUUID().toString()
    private var panelGeneration = 0L
    @Volatile private var panelDisposed = false
    private var sourceGeneration = 0L
    private var compileGeneration = 0L
    private var currentSourceSnapshot: CompilerExplorerSourceSnapshot? = null
    private var activeCompile: CompileLease? = null
    private var publishedCompile: CompileLease? = null
    private var resultCandidate: CompileLease? = null
    private var pageReady = false
    private var pendingSource: CompilerExplorerSourceSnapshot? = null

    private data class CompileLease(
        val request: ExplorerCompileRequest,
        val source: CompilerExplorerSourceSnapshot?,
        val panelGeneration: Long,
        val generation: Long,
    )

    /**
     * The file whose source the explorer last compiled — what a hover in the
     * webview refers to. Not the selected tab: the explorer keeps showing one
     * file while the user moves around the IDE.
     */
    private var sourceFile: VirtualFile? = null

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
                        if (panelDisposed || project.isDisposed) return@invokeLater
                        panelGeneration++
                        activeCompile = null
                        publishedCompile = null
                        resultCandidate = null
                        pageReady = true
                        val pending = pendingSource
                        pendingSource = null
                        currentSourceSnapshot = null
                        if (pending != null && pending.isCurrent()) {
                            dispatchSource(pending.file, pending.document, force = true)
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
        synchronized(panelLifetime) {
            ExplorerProjections.getInstance(project).cancelRequests(this)
            panelDisposed = true
        }
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
            if (FileDocumentManager.getInstance().getFile(editor.document) !== file) return@invokeLater
            sourceFile = file
            dispatchSource(file, editor.document, force)
        }
    }

    /** Push a specific file's source, used by the "Open In" action. */
    fun pushFile(file: VirtualFile, force: Boolean = true) {
        ApplicationManager.getApplication().invokeLater {
            if (!TclFileType.isSupported(file)) return@invokeLater
            val document = FileDocumentManager.getInstance().getDocument(file) ?: return@invokeLater
            sourceFile = file
            dispatchSource(file, document, force)
        }
    }

    private fun dispatchSource(file: VirtualFile, document: Document, force: Boolean) {
        if (panelDisposed || project.isDisposed || !file.isValid ||
            FileDocumentManager.getInstance().getDocument(file) !== document) return
        val source = document.text
        val dialect = TclLspSettings.getInstance().dialect
        val previous = currentSourceSnapshot
        if (!force && previous != null && previous.file === file && previous.document === document &&
            previous.modificationStamp == document.modificationStamp && previous.source == source &&
            previous.dialect == dialect) return
        val snapshot = CompilerExplorerSourceSnapshot(
            "$panelId:$panelGeneration:${++sourceGeneration}", file, document,
            source, document.modificationStamp, dialect,
        )
        currentSourceSnapshot = snapshot
        compileGeneration++
        activeCompile = null
        publishedCompile = null
        resultCandidate = null
        if (!pageReady) {
            pendingSource = snapshot
            return
        }
        sendSourceUpdate(snapshot)
    }

    private fun sendSourceUpdate(snapshot: CompilerExplorerSourceSnapshot) {
        if (panelDisposed || currentSourceSnapshot !== snapshot || !snapshot.isCurrent()) return
        postToWebview(mapOf("type" to "sourceUpdate", "source" to snapshot.source,
            "dialect" to snapshot.dialect, "sourceToken" to snapshot.token))
    }

    private fun postToWebview(message: Map<String, Any?>) {
        synchronized(panelLifetime) {
            if (panelDisposed || project.isDisposed) return
            val json = Gson().toJson(message.filterValues { it != null })
            browser.cefBrowser.executeJavaScript(
                "window.dispatchEvent(new MessageEvent('message', { data: $json }));", "", 0,
            )
        }
    }

    private fun handleJsMessage(message: String) {
        val parsed = runCatching { JsonParser.parseString(message) }.getOrNull()
            ?.takeIf { it.isJsonObject }?.asJsonObject ?: return
        ApplicationManager.getApplication().invokeLater {
            if (panelDisposed || project.isDisposed) return@invokeLater
            when (parsed.explorerString("type")) {
                "compileStarted" -> ExplorerCompileRequest.parse(parsed)?.let { request ->
                    if (activeCompile?.takeIf { it.request == request && compileIsCurrent(it) } == null) beginCompile(request)
                }
                "compile" -> ExplorerCompileRequest.parse(parsed)?.let { request ->
                    val lease = activeCompile?.takeIf { it.request == request && compileIsCurrent(it) }
                        ?: beginCompile(request) ?: return@let
                    runCompile(lease)
                }
                "compileCurrentnessCheck" -> {
                    val lease = activeCompile
                    val resultKind = parsed.explorerString("resultKind")
                    val accepted = lease != null && messageMatches(parsed, lease) && compileIsCurrent(lease) &&
                        (resultKind == "result" || resultKind == "error")
                    if (accepted) {
                        resultCandidate = if (resultKind == "result") lease else null
                        publishedCompile = null
                    }
                    postToWebview(mapOf("type" to if (accepted) "compileCurrentnessAccepted" else "compileCurrentnessRejected",
                        "compileToken" to parsed.explorerString("compileToken"),
                        "sourceToken" to parsed.explorerString("sourceToken")))
                }
                "compileComplete" -> {
                    val lease = activeCompile
                    val rendered = parsed.get("renderSucceeded")
                        ?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isBoolean }?.asBoolean == true
                    if (lease != null && resultCandidate === lease && rendered &&
                        messageMatches(parsed, lease) && compileIsCurrent(lease)) publishedCompile = lease
                }
                "highlightSource" -> ExplorerHighlightRequest.parse(parsed)?.let { highlightSourceRange(it) }
                "clearHighlight" -> clearSourceHighlight(parsed)
                "openProjection" -> {
                    if (parsed.explorerString("purpose") != "standalone-source") return@invokeLater
                    val source = parsed.explorerString("source") ?: return@invokeLater
                    val dialect = parsed.explorerString("dialect")?.takeIf { it.isNotEmpty() } ?: return@invokeLater
                    val token = parsed.explorerString("sourceToken")
                    val owner = currentSourceSnapshot
                    if (token != null) {
                        if (owner == null || owner.token != token || owner.source != source || !owner.isCurrent()) return@invokeLater
                    } else if (owner != null || parsed.has("sourceToken")) return@invokeLater
                    openProjection(
                        view = parsed.explorerString("view") ?: return@invokeLater,
                        label = parsed.explorerString("label") ?: return@invokeLater,
                        dialect = dialect,
                        source = source,
                    )
                }
            }
        }
    }

    private fun beginCompile(request: ExplorerCompileRequest): CompileLease? {
        val source = currentSourceSnapshot
        if (request.sourceToken != null) {
            if (source == null || source.token != request.sourceToken || source.source != request.source ||
                !source.isCurrent()) return null
        } else if (source != null) return null
        val lease = CompileLease(request, source, panelGeneration, ++compileGeneration)
        activeCompile = lease
        publishedCompile = null
        resultCandidate = null
        return lease
    }

    private fun compileIsCurrent(lease: CompileLease): Boolean =
        !panelDisposed && !project.isDisposed && activeCompile === lease &&
            lease.panelGeneration == panelGeneration && lease.generation == compileGeneration &&
            (if (lease.source != null) currentSourceSnapshot === lease.source && lease.source.isCurrent()
                else currentSourceSnapshot == null)

    private fun messageMatches(message: JsonObject, lease: CompileLease): Boolean =
        message.explorerString("purpose") == "standalone-source" &&
            message.explorerString("compileToken") == lease.request.compileToken &&
            (if (lease.request.sourceToken != null) message.explorerString("sourceToken") == lease.request.sourceToken
                else !message.has("sourceToken"))

    private fun publishCompileMessage(lease: CompileLease, type: String, data: Any?) {
        ApplicationManager.getApplication().invokeLater {
            if (!compileIsCurrent(lease)) return@invokeLater
            resultCandidate = if (type == "result") lease else null
            publishedCompile = null
            postToWebview(mapOf("type" to type, "data" to data,
                "compileToken" to lease.request.compileToken, "sourceToken" to lease.request.sourceToken))
        }
    }

    private fun runCompile(lease: CompileLease) {
        sendStatusToWebview("compiling", lease)
        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                val server = awaitRunningTclLspServer(project)
                if (server == null) {
                    publishCompileMessage(lease, "error", mapOf("error" to "Tcl LSP server did not become ready."))
                    return@executeOnPooledThread
                }

                // An explicit timeout uses the interface method rather than a synthetic
                // $default bridge whose declaring owner varies between supported IDEs.
                val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                    lsp4j.workspaceService.executeCommand(
                        org.eclipse.lsp4j.ExecuteCommandParams(
                            "tcl-lsp.compilerExplorer", listOf(lease.request.source, lease.request.dialect),
                        )
                    )
                }
                if (result != null) {
                    val json = Gson().toJsonTree(result)
                    val error = json.takeIf { it.isJsonObject }?.asJsonObject?.get("error")
                    if (error != null && !error.isJsonNull) {
                        publishCompileMessage(lease, "error", mapOf("error" to error))
                    } else publishCompileMessage(lease, "result", result)
                } else publishCompileMessage(lease, "error", mapOf("error" to "Compiler Explorer returned no result."))
            } catch (e: Exception) {
                LOG.warn("Compile failed", e)
                publishCompileMessage(lease, "error", mapOf("error" to (e.message ?: "Unknown error")))
            }
        }
    }

    /**
     * Render one explorer view through the server and show it in an ordinary
     * IntelliJ editor tab.
     *
     * The tool window keeps the interactive panes — the CFG's routed edges and
     * the WASM branch lanes do not survive as flat text — but everything that
     * *is* linear reads better in a real editor, where find, folding and the
     * colour scheme all work. The pane is read-only and carries a line map, so
     * moving the caret in it navigates back into the source file.
     */
    private fun openProjection(view: String, label: String, dialect: String, source: String) {
        if (panelDisposed || project.isDisposed) return
        val projections = ExplorerProjections.getInstance(project)
        val requestGeneration = projections.beginRequest(view, this)
        val requestPanelGeneration = panelGeneration
        val sourceOwner = currentSourceSnapshot?.takeIf {
            it.source == source && it.isCurrent()
        }
        ApplicationManager.getApplication().executeOnPooledThread {
            val server = awaitRunningTclLspServer(project)
            if (server == null) {
                ApplicationManager.getApplication().invokeLater {
                    if (!panelDisposed && !project.isDisposed &&
                        panelGeneration == requestPanelGeneration &&
                        projections.isCurrentRequest(view, this, requestGeneration)) {
                        sendErrorToWebview("Tcl LSP server did not become ready — cannot render $label.")
                    }
                }
                return@executeOnPooledThread
            }
            // The explicit timeout is load-bearing; see the note in runCompile
            // about `sendRequestSync$default`.
            val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                lsp4j.workspaceService.executeCommand(
                    org.eclipse.lsp4j.ExecuteCommandParams(
                        "tcl-lsp.compilerExplorerView",
                        listOf(source, dialect, view),
                    )
                )
            } ?: return@executeOnPooledThread

            val rendered = ExplorerProjection.parse(result)
            if (rendered == null) {
                ApplicationManager.getApplication().invokeLater {
                    if (!panelDisposed && !project.isDisposed &&
                        panelGeneration == requestPanelGeneration &&
                        projections.isCurrentRequest(view, this, requestGeneration)) {
                        sendErrorToWebview("Could not render the $label view.")
                    }
                }
                return@executeOnPooledThread
            }
            ApplicationManager.getApplication().invokeLater {
                synchronized(panelLifetime) {
                    if (panelDisposed || project.isDisposed ||
                        panelGeneration != requestPanelGeneration ||
                        !projections.isCurrentRequest(view, this, requestGeneration)) return@synchronized
                    val navigationOwner = sourceOwner?.takeIf {
                        currentSourceSnapshot === it && it.isCurrent()
                    }
                    projections.open(
                        label,
                        view,
                        rendered,
                        navigationOwner,
                        this,
                        requestGeneration,
                    )
                }
            }
        }
    }

    private fun sendStatusToWebview(status: String, lease: CompileLease) {
        if (!compileIsCurrent(lease)) return
        postToWebview(mapOf("type" to "status", "text" to status,
            "compileToken" to lease.request.compileToken, "sourceToken" to lease.request.sourceToken))
    }

    private fun sendErrorToWebview(error: String) {
        val escaped = escapeForJs(error)
        browser.cefBrowser.executeJavaScript(
            "window.dispatchEvent(new MessageEvent('message', { data: { type: 'error', data: { error: '$escaped' } } }));",
            "", 0
        )
    }

    /** Readonly source navigation belongs to the exact result's still-current editor input. */
    private fun highlightSourceRange(message: ExplorerHighlightRequest) {
        val lease = publishedCompile ?: return
        if (message.compileToken != lease.request.compileToken || message.sourceToken != lease.request.sourceToken ||
            !compileIsCurrent(lease)) return
        val editor = editorForCompiledSource(lease) ?: return
        val (start, end) = strictExplorerSourceOffsets(editor.document, message.range) ?: return
        synchronized(panelLifetime) {
            if (!compileIsCurrent(lease)) return
            editor.selectionModel.setSelection(start, end)
            editor.scrollingModel.scrollTo(editor.offsetToLogicalPosition(start),
                com.intellij.openapi.editor.ScrollType.CENTER_UP)
        }
    }

    private fun editorForCompiledSource(lease: CompileLease): com.intellij.openapi.editor.Editor? {
        if (!compileIsCurrent(lease)) return null
        val snapshot = lease.source ?: return null
        return com.intellij.openapi.editor.EditorFactory.getInstance()
            .getEditors(snapshot.document, project).firstOrNull { it.document === snapshot.document }
    }

    private fun clearSourceHighlight(message: JsonObject) {
        val lease = publishedCompile ?: return
        if (messageMatches(message, lease) && compileIsCurrent(lease)) {
            synchronized(panelLifetime) {
                if (compileIsCurrent(lease)) editorForCompiledSource(lease)?.selectionModel?.removeSelection()
            }
        }
    }

    private fun escapeForJs(s: String): String =
        s.replace("\\", "\\\\")
            .replace("'", "\\'")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
}
