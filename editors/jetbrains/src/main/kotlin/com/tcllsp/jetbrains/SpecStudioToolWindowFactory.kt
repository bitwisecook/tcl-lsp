// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.Gson
import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.platform.lsp.api.LspServerState
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter
import org.eclipse.lsp4j.DidChangeWatchedFilesParams
import org.eclipse.lsp4j.ExecuteCommandParams
import org.eclipse.lsp4j.FileChangeType
import org.eclipse.lsp4j.FileEvent
import java.nio.file.Files
import java.nio.file.Path
import java.util.Comparator
import java.util.concurrent.atomic.AtomicLong

private val SPEC_LOG = Logger.getInstance("com.tcllsp.jetbrains.SpecStudio")

class SpecStudioToolWindowFactory : ToolWindowFactory, DumbAware {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        if (!isJcefSupported()) return
        val panel = SpecStudioPanel(project)
        val content = ContentFactory.getInstance().createContent(panel.browser.component, "", false)
        content.setDisposer(panel)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project): Boolean =
        project.basePath != null && isJcefSupported()
}

internal class SpecStudioPanel(private val project: Project) : Disposable {
    val browser = JBCefBrowser()
    private val query = JBCefJSQuery.create(browser as JBCefBrowserBase)
    private val gson = Gson()
    private val sessionDir = Path.of(project.basePath!!, ".tcl-lsp", ".spec-studio", "jetbrains")
    private val paths = mapOf(
        "dsl" to sessionDir.resolve("active.tclspec"),
        "sample" to sessionDir.resolve("test.tcl"),
        "rust" to sessionDir.resolve("generated.rs"),
        "stub" to sessionDir.resolve("generated-stub.tcl"),
    )
    private var disposed = false

    /**
     * Which sample-dialect pin is the studio's current intent.
     *
     * Each pin now waits for the lazily-started LSP server, so two can be in
     * flight at once — the one sent when the studio mounts, and one from a
     * picker change made while that is still waiting.  They are independent
     * pooled tasks with no ordering between them, so without this the initial
     * dialect could be applied *after* the newer selection and leave the sample
     * disagreeing with the UI until the user changed it again (PR #1960
     * review).  A pin claims a generation before it is dispatched and rechecks
     * it before sending, so only the newest intent reaches the server.
     */
    private val dialectPinGeneration = AtomicLong()

    /** Serialises the recheck with the send it guards. */
    private val dialectPinLock = Any()

    init {
        Disposer.register(this, browser)
        Disposer.register(this, query)
        query.addHandler { message ->
            handleMessage(message)
            null
        }
        browser.jbCefClient.addLoadHandler(object : CefLoadHandlerAdapter() {
            override fun onLoadEnd(cefBrowser: CefBrowser?, frame: org.cef.browser.CefFrame?, httpStatusCode: Int) {
                if (frame?.isMain != true) return
                val script = """
                    window.__tclSpecStudioBridge=function(json){${query.inject("json")}};
                    (window.__tclSpecStudioQueue||[]).splice(0).forEach(window.__tclSpecStudioBridge);
                """.trimIndent()
                cefBrowser?.executeJavaScript(script, "", 0)
            }
        }, browser.cefBrowser)
        EditorFactory.getInstance().eventMulticaster.addDocumentListener(object : DocumentListener {
            override fun documentChanged(event: DocumentEvent) {
                if (disposed) return
                val file = FileDocumentManager.getInstance().getFile(event.document) ?: return
                val surface = paths.entries.firstOrNull { it.value == file.toNioPath() }?.key ?: return
                if (surface != "dsl" && surface != "sample") return
                sendUpdate(surface, event.document.text)
                ApplicationManager.getApplication().invokeLater {
                    if (!disposed) {
                        FileDocumentManager.getInstance().saveDocument(event.document)
                        if (surface == "dsl") notifyPack(paths.getValue("dsl"), FileChangeType.Changed)
                    }
                }
            }
        }, this)
        browser.loadHTML(getSpecStudioHtml())
    }

    private fun handleMessage(json: String) {
        try {
            val message = gson.fromJson(json, Map::class.java)
            val type = message["type"] as? String ?: return
            val surface = message["surface"] as? String
            when (type) {
                "surfaceUpdate" -> if (surface in paths && message["text"] is String) {
                    materialise(surface!!, message["text"] as String)
                }
                "openSurface" -> if (surface in paths) openSurface(surface!!)
                "dialectUpdate" -> applySampleDialect(message["dialect"] as? String)
            }
        } catch (error: Exception) {
            SPEC_LOG.warn("Could not handle Spec Studio message", error)
        }
    }

    private fun materialise(surface: String, text: String) {
        val path = paths.getValue(surface)
        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                Files.createDirectories(sessionDir)
                val existed = Files.exists(path)
                Files.writeString(path, text)
                LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)?.refresh(false, false)
                if (surface == "dsl") notifyPack(path, if (existed) FileChangeType.Changed else FileChangeType.Created)
            } catch (error: Exception) {
                SPEC_LOG.warn("Could not materialise Spec Studio $surface", error)
            }
        }
    }

    private fun openSurface(surface: String) {
        val path = paths.getValue(surface)
        ApplicationManager.getApplication().executeOnPooledThread {
            if (!Files.exists(path)) {
                Files.createDirectories(sessionDir)
                Files.writeString(path, "")
            }
            val file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path) ?: return@executeOnPooledThread
            ApplicationManager.getApplication().invokeLater {
                FileEditorManager.getInstance(project).openFile(file, true)
            }
        }
    }

    private fun sendUpdate(surface: String, text: String) {
        val escapedSurface = gson.toJson(surface)
        val escapedText = gson.toJson(text)
        browser.cefBrowser.executeJavaScript(
            "window.dispatchEvent(new MessageEvent('message',{data:{type:'tclSpecStudioHostUpdate',surface:$escapedSurface,text:$escapedText}}));",
            "", 0,
        )
    }

    /**
     * Pin the sample surface to the studio's selected dialect, or release it
     * when [dialect] is null.
     *
     * The sample is always materialised as `test.tcl`, so without this the
     * server resolves it as generic Tcl however the studio's selector is set,
     * and a pack whose commands only exist in another dialect shows no
     * highlighting, completion or hover in the very buffer the studio exists
     * to give feedback on.  The per-document override is the seam that leaves
     * every other open buffer alone, unlike the two session-global dialect
     * commands (issue #1931).
     */
    @Suppress("UnstableApiUsage")
    private fun applySampleDialect(dialect: String?) {
        val uri = paths.getValue("sample").toUri().toString()
        // Releasing sends the URI alone rather than a null second argument:
        // the server reads an absent dialect as a clear, and this keeps the
        // argument list free of nulls across the Gson boundary.
        val args: List<Any> = if (dialect == null) listOf(uri) else listOf(uri, dialect)
        // Claimed here, not inside the task: the order pins are *requested* in
        // is the studio's intent, and the order the pooled tasks happen to run
        // in is not.
        val generation = dialectPinGeneration.incrementAndGet()
        ApplicationManager.getApplication().executeOnPooledThread {
            // Start the server and wait, rather than give up on one that has
            // not started yet. The server is launched lazily by the first Tcl
            // editor, so in a project with no Tcl file already open the studio
            // is the first thing to want it — and a dropped pin is never
            // retried, leaving the sample under generic Tcl for the rest of
            // the session. Same wait the Compiler Explorer has always used.
            val server = awaitRunningTclLspServer(project) ?: run {
                SPEC_LOG.warn("No Tcl LSP server to pin the Spec Studio sample dialect on")
                return@executeOnPooledThread
            }
            // Recheck under the lock rather than before it: a bare check
            // leaves the window between the check and the send, which is
            // exactly where a newer pin would overtake this one.
            synchronized(dialectPinLock) {
                if (dialectPinGeneration.get() != generation) {
                    return@executeOnPooledThread
                }
                try {
                    server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                        lsp4j.workspaceService.executeCommand(
                            ExecuteCommandParams("tcl-lsp.setDocumentDialectOverride", args)
                        )
                    }
                } catch (error: Exception) {
                    SPEC_LOG.warn("Could not set the Spec Studio sample dialect", error)
                }
            }
        }
    }

    @Suppress("UnstableApiUsage")
    private fun notifyPack(path: Path, type: FileChangeType) {
        ApplicationManager.getApplication().executeOnPooledThread {
            val server = LspServerManager.getInstance(project)
                .getServersForProvider(TclLspServerSupportProvider::class.java)
                .firstOrNull { it.state == LspServerState.Running } ?: return@executeOnPooledThread
            server.sendNotification { lsp4j ->
                lsp4j.workspaceService.didChangeWatchedFiles(
                    DidChangeWatchedFilesParams(listOf(FileEvent(path.toUri().toString(), type)))
                )
            }
        }
    }

    override fun dispose() {
        if (disposed) return
        disposed = true
        applySampleDialect(null)
        val pack = paths.getValue("dsl")
        val existed = Files.exists(pack)
        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                if (Files.exists(sessionDir)) {
                    Files.walk(sessionDir).use { paths ->
                        paths.sorted(Comparator.reverseOrder()).forEach(Files::deleteIfExists)
                    }
                }
                if (existed) notifyPack(pack, FileChangeType.Deleted)
            } catch (error: Exception) {
                SPEC_LOG.warn("Could not remove the Spec Studio override", error)
            }
        }
    }
}
