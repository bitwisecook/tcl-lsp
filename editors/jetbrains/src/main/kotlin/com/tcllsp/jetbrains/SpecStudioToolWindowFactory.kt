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
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.platform.lsp.api.LspServerState
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter
import org.eclipse.lsp4j.DidChangeWatchedFilesParams
import org.eclipse.lsp4j.FileChangeType
import org.eclipse.lsp4j.FileEvent
import java.nio.file.Files
import java.nio.file.Path
import java.util.Comparator

private val SPEC_LOG = Logger.getInstance("com.tcllsp.jetbrains.SpecStudio")

class SpecStudioToolWindowFactory : ToolWindowFactory, DumbAware {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = SpecStudioPanel(project)
        val content = ContentFactory.getInstance().createContent(panel.browser.component, "", false)
        content.setDisposer(panel)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project): Boolean = project.basePath != null
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
