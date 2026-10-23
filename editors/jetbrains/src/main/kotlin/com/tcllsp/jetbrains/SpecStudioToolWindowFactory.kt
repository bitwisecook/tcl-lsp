// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.Gson
import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileDocumentManagerListener
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.platform.lsp.api.LspServer
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import org.cef.browser.CefBrowser
import org.cef.handler.CefLoadHandlerAdapter
import org.eclipse.lsp4j.ExecuteCommandParams
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicBoolean
import java.util.UUID
import java.util.concurrent.CompletableFuture

private val SPEC_LOG = Logger.getInstance("com.tcllsp.jetbrains.SpecStudio")

/** One live Spec Studio writer per project session path. */
internal object SpecStudioPanelGenerations {
    private val generations = ConcurrentHashMap<Path, AtomicLong>()
    private val activePaths = ConcurrentHashMap.newKeySet<Path>()
    private val activePanels = ConcurrentHashMap.newKeySet<SpecStudioPanel>()

    fun claim(path: Path): Long {
        activePaths.add(path)
        return generations.computeIfAbsent(path) { AtomicLong() }.incrementAndGet()
    }
    fun current(path: Path): Long = generations[path]?.get() ?: 0L
    fun isActive(path: Path): Boolean = path in activePaths
    fun retire(path: Path, generation: Long) {
        if (current(path) == generation) activePaths.remove(path)
    }
    fun register(panel: SpecStudioPanel) { activePanels.add(panel) }
    fun unregister(panel: SpecStudioPanel) { activePanels.remove(panel) }
    fun serverInitialized() { activePanels.toList().forEach { it.serverInitialized() } }
}

private data class StudioSourceLease(val issuer: List<Int>, val token: Long)

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
    private val sessionDir = Path.of(
        project.basePath!!, ".tcl-lsp", ".spec-studio", "jetbrains", "session-${UUID.randomUUID()}",
    )
    private val paths = mapOf(
        "dsl" to sessionDir.resolve("active.tclspec"),
        "sample" to sessionDir.resolve("test.tcl"),
        "rust" to sessionDir.resolve("generated.rs"),
        "stub" to sessionDir.resolve("generated-stub.tcl"),
    )
    @Volatile private var disposed = false
    private val panelGeneration = SpecStudioPanelGenerations.claim(sessionDir)
    private val surfaceGenerations = paths.keys.associateWith { AtomicLong() }
    /** Host-issued source image revisions carried across the browser round trip. */
    private val sourceRevisions = paths.keys.associateWith { AtomicLong() }
    private val lastPageWriteGenerations = paths.keys.associateWith { AtomicLong() }
    @Volatile private var pendingDslGeneration: Long? = null
    private val dslPublicationInFlight = ConcurrentHashMap.newKeySet<Long>()
  private val dslPublicationAttempt = AtomicLong()
    private val openGenerations = paths.keys.associateWith { AtomicLong() }
    private val generatedReceipts = SpecStudioGeneratedFiles.readReceipts(sessionDir)
    private val generatedSnapshots = mutableMapOf<String, SpecStudioGeneratedFiles.Snapshot>()
    private val unresolvedSourceImages = mutableMapOf<String, SourceImage>()
    private var materialisingDocument: com.intellij.openapi.editor.Document? = null
    private val studioSourceLock = Any()
    @Volatile private var studioSourceLease: StudioSourceLease? = null
    private var studioSourceRevision = 0L
    private val studioSourceGeneration = AtomicLong(1L)
    private val webviewLoaded = AtomicBoolean(false)

    /**
     * Pins wait for the lazily-started server on independent pooled tasks.
     * Claim a generation before dispatch and recheck under the send lock to
     * drop waiting requests superseded by a newer dialect selection.
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
                if (event.document === materialisingDocument || disposed ||
                    SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return
                val file = FileDocumentManager.getInstance().getFile(event.document) ?: return
                val surface = paths.entries.firstOrNull { it.value == file.toNioPath() }?.key ?: return
                if (surface != "dsl" && surface != "sample") return
                val generation = surfaceGenerations.getValue(surface).incrementAndGet()
                lastPageWriteGenerations.getValue(surface).set(0L)
                val publicationAttempt = if (surface == "dsl") dslPublicationAttempt.incrementAndGet() else null
                if (surface == "dsl") pendingDslGeneration = -1L
                val revision = sourceRevisions.getValue(surface).incrementAndGet()
                sendUpdate(surface, event.document.text, revision,
                    publicationPending = if (surface == "dsl") true else null,
                    publicationAttempt = publicationAttempt)
                ApplicationManager.getApplication().invokeLater {
                    // IDE edits remain authoritative even if the browser panel
                    // closes before this queued save runs.
                    FileDocumentManager.getInstance().saveDocument(event.document)
                    if (!disposed && SpecStudioPanelGenerations.current(sessionDir) == panelGeneration &&
                        surface == "dsl" && surfaceGenerations.getValue("dsl").get() == generation) {
                        publishCurrentDslAfterSave(
                            event.document, event.document.text,
                            sourceRevisions.getValue("dsl").get(), publicationAttempt!!,
                        )
                    }
                }
            }
        }, this)
        ApplicationManager.getApplication().messageBus.connect(this).subscribe(FileDocumentManagerListener.TOPIC, object : FileDocumentManagerListener {
            override fun beforeDocumentSaving(document: com.intellij.openapi.editor.Document) {
                val file = FileDocumentManager.getInstance().getFile(document) ?: return
                if (file.toNioPath() != paths.getValue("dsl") || disposed) return
                ApplicationManager.getApplication().invokeLater {
                    if (disposed || SpecStudioPanelGenerations.current(sessionDir) != panelGeneration ||
                        FileDocumentManager.getInstance().isDocumentUnsaved(document) || pendingDslGeneration != -1L) return@invokeLater
                    val revision = sourceRevisions.getValue("dsl").get()
                    val attempt = dslPublicationAttempt.get()
                    publishCurrentDslAfterSave(document, document.text, revision, attempt)
                }
            }
        })
        SpecStudioPanelGenerations.register(this)
        val generation = studioSourceGeneration.get()
        ApplicationManager.getApplication().executeOnPooledThread {
            val server = awaitRunningTclLspServer(project)
            if (server != null) synchronized(studioSourceLock) {
                acquireStudioSource(server, generation)
                val source = readSavedUtf8(paths.getValue("dsl"))
                if (source != null) publishStudioSource(server, paths.getValue("dsl"), source, generation)
            }
            loadWebviewIfCurrent(generation)
        }
    }

    private fun publishCurrentDslAfterSave(
        document: com.intellij.openapi.editor.Document,
        expectedText: String,
        expectedRevision: Long,
        publicationAttempt: Long,
    ) {
        if (pendingDslGeneration != -1L || !dslPublicationInFlight.add(publicationAttempt)) return
        val surfaceGeneration = surfaceGenerations.getValue("dsl").get()
        val leaseGeneration = studioSourceGeneration.get()
        ApplicationManager.getApplication().executeOnPooledThread {
            var published = false
            try {
                if (disposed || surfaceGenerations.getValue("dsl").get() != surfaceGeneration ||
                    sourceRevisions.getValue("dsl").get() != expectedRevision || document.text != expectedText ||
                    publicationAttempt != dslPublicationAttempt.get()) return@executeOnPooledThread
                val server = awaitRunningTclLspServer(project) ?: return@executeOnPooledThread
                published = publishStudioSource(server, paths.getValue("dsl"), expectedText, leaseGeneration)
            } finally {
                ApplicationManager.getApplication().invokeLater {
                    try {
                        if (published && !disposed && surfaceGenerations.getValue("dsl").get() == surfaceGeneration &&
                            sourceRevisions.getValue("dsl").get() == expectedRevision &&
                            pendingDslGeneration == -1L && document.text == expectedText &&
                            !FileDocumentManager.getInstance().isDocumentUnsaved(document) &&
                            publicationAttempt == dslPublicationAttempt.get()) {
                            pendingDslGeneration = null
                            sendUpdate("dsl", expectedText, expectedRevision, accepted = true,
                                publicationPending = false, publicationAttempt = publicationAttempt)
                        }
                    } finally {
                        dslPublicationInFlight.remove(publicationAttempt)
                    }
                }
            }
        }
    }

    internal fun serverInitialized() {
        if (disposed) return
        pendingDslGeneration = -1L
        val publicationAttempt = dslPublicationAttempt.incrementAndGet()
        sendCurrentSurface("dsl", null, accepted = false, publicationPending = true,
            publicationAttempt = publicationAttempt)
        val generation = studioSourceGeneration.incrementAndGet()
        ApplicationManager.getApplication().executeOnPooledThread {
            val server = awaitRunningTclLspServer(project) ?: run {
                loadWebviewIfCurrent(generation)
                return@executeOnPooledThread
            }
            synchronized(studioSourceLock) {
                if (disposed || studioSourceGeneration.get() != generation) return@synchronized
                acquireStudioSource(server, generation)
                val path = paths.getValue("dsl")
                val source = readSavedUtf8(path)
                val published = source != null && publishStudioSource(server, path, source, generation)
                if (published && source != null) {
                    val revision = sourceRevisions.getValue("dsl").get()
                    ApplicationManager.getApplication().invokeLater {
                        if (disposed || studioSourceGeneration.get() != generation ||
                            sourceRevisions.getValue("dsl").get() != revision || pendingDslGeneration != -1L ||
                            dslPublicationAttempt.get() != publicationAttempt ||
                            SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return@invokeLater
                        val file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path) ?: return@invokeLater
                        val document = FileDocumentManager.getInstance().getCachedDocument(file)
                        if (document != null && (document.text != source || FileDocumentManager.getInstance().isDocumentUnsaved(document))) return@invokeLater
                        if (document == null && readSavedUtf8(path) != source) return@invokeLater
                        pendingDslGeneration = null
                        sendUpdate("dsl", source, revision, accepted = true, publicationPending = false,
                            publicationAttempt = publicationAttempt)
                    }
                }
            }
            loadWebviewIfCurrent(generation)
        }
    }

    private fun loadWebviewIfCurrent(generation: Long) {
        ApplicationManager.getApplication().invokeLater {
            if (!disposed && studioSourceGeneration.get() == generation &&
                SpecStudioPanelGenerations.current(sessionDir) == panelGeneration &&
                webviewLoaded.compareAndSet(false, true)) {
                browser.loadHTML(getSpecStudioHtml())
            }
        }
    }

    private fun handleMessage(json: String) {
        if (disposed || SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return
        try {
            val message = gson.fromJson(json, Map::class.java)
            val type = message["type"] as? String ?: return
            val surface = message["surface"] as? String
            when (type) {
                "surfaceUpdate" -> if (surface in paths && message["text"] is String) {
                    val expectedRaw = message["expectedSourceRevision"]
                    val expected = exactWireRevision(expectedRaw)
                    val webGeneration = exactWireRevision(message["webGeneration"])
                    val basedOnDsl = exactWireRevision(message["basedOnDslRevision"])
                    val bootstrap = message["bootstrap"] == true
                    if (surface == "dsl" || surface == "sample") {
                        if (webGeneration == null || (expectedRaw != null && expected == null)) return
                        if (bootstrap) {
                            val path = paths.getValue(surface!!)
                            if (Files.exists(path)) {
                                surfaceGenerations.getValue(surface).incrementAndGet()
                                if (surface == "dsl") {
                                    // Existing disk content is not proof that the
                                    // open buffer matches the active publication.
                                    // Keep generated outputs blocked until the
                                    // exact saved image is accepted and still
                                    // matches the current document.
                                    pendingDslGeneration = -1L
                                    surfaceGenerations.getValue("rust").incrementAndGet()
                                    surfaceGenerations.getValue("stub").incrementAndGet()
                                }
                                val revision = sourceRevisions.getValue(surface).incrementAndGet()
                                lastPageWriteGenerations.getValue(surface).set(0L)
                                val publicationAttempt = if (surface == "dsl") dslPublicationAttempt.incrementAndGet() else null
                                sendCurrentSurface(surface, webGeneration, accepted = false,
                                    publicationPending = if (surface == "dsl") true else null,
                                    publicationAttempt = publicationAttempt)
                                if (surface == "dsl") republishBootstrapDsl(revision, webGeneration, publicationAttempt!!)
                                return
                            }
                            if (sourceRevisions.getValue(surface).get() != 0L) return
                        }
                    } else if (basedOnDsl == null) return
                    materialise(surface!!, message["text"] as String, expected, webGeneration, basedOnDsl)
                }
                "openSurface" -> if (surface in paths) openSurface(surface!!)
                "dialectUpdate" -> applySampleDialect(message["dialect"] as? String)
            }
        } catch (error: Exception) {
            SPEC_LOG.warn("Could not handle Spec Studio message", error)
        }
    }

    private fun republishBootstrapDsl(revision: Long, webGeneration: Long?, publicationAttempt: Long) {
        val generation = studioSourceGeneration.get()
        val path = paths.getValue("dsl")
        ApplicationManager.getApplication().executeOnPooledThread {
            val server = awaitRunningTclLspServer(project) ?: return@executeOnPooledThread
            val saved = readSavedUtf8(path) ?: return@executeOnPooledThread
            if (!publishStudioSource(server, path, saved, generation)) return@executeOnPooledThread
            ApplicationManager.getApplication().invokeLater {
                if (disposed || sourceRevisions.getValue("dsl").get() != revision ||
                    studioSourceGeneration.get() != generation || pendingDslGeneration != -1L ||
                    dslPublicationAttempt.get() != publicationAttempt ||
                    SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return@invokeLater
                val file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path) ?: return@invokeLater
                val document = FileDocumentManager.getInstance().getCachedDocument(file)
                if (document != null) {
                    if (document.text != saved || FileDocumentManager.getInstance().isDocumentUnsaved(document)) return@invokeLater
                } else if (readSavedUtf8(path) != saved) {
                    return@invokeLater
                }
                pendingDslGeneration = null
                sendUpdate("dsl", saved, revision, webGeneration, accepted = true,
                    publicationPending = false, publicationAttempt = publicationAttempt)
            }
        }
    }

    private fun exactWireRevision(value: Any?): Long? {
        val number = value as? Number ?: return null
        val double = number.toDouble()
        if (!double.isFinite() || double < 0.0 || double > 9_007_199_254_740_991.0) return null
        val long = double.toLong()
        return long.takeIf { it.toDouble() == double }
    }

    @Suppress("UnstableApiUsage")
    private fun acquireStudioSource(server: LspServer, generation: Long): StudioSourceLease? {
        if (disposed || studioSourceGeneration.get() != generation) return null
        val path = paths.getValue("dsl")
        return try {
            val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                lsp4j.workspaceService.executeCommand(
                    ExecuteCommandParams("tcl-lsp.studioSource", listOf(
                        mapOf("operation" to "acquire", "uri" to path.toUri().toString()),
                    ))
                )
            } ?: return null
            val root = Gson().toJsonTree(result).takeIf { it.isJsonObject }?.asJsonObject ?: return null
            if (root.get("accepted")?.asBoolean != true) return null
            val leaseJson = root.getAsJsonObject("lease") ?: return null
            val issuerJson = leaseJson.getAsJsonArray("issuer") ?: return null
            if (issuerJson.size() != 16) return null
            val issuer = mutableListOf<Int>()
            for (element in issuerJson) {
                val primitive = element.asJsonPrimitive.takeIf { it.isNumber } ?: return null
                val numeric = primitive.asDouble
                val number = primitive.asInt
                if (!numeric.isFinite() || numeric != number.toDouble() || number !in 0..255) return null
                issuer.add(number)
            }
            val tokenPrimitive = leaseJson.get("token")?.asJsonPrimitive?.takeIf { it.isNumber } ?: return null
            val tokenDouble = tokenPrimitive.asDouble
            val token = tokenPrimitive.asLong
            if (!tokenDouble.isFinite() || tokenDouble != token.toDouble() || token !in 1L..0xffff_ffffL) return null
            val acquired = StudioSourceLease(issuer, token)
            if (disposed || studioSourceGeneration.get() != generation) {
                retireStudioSourceHandle(server, path, acquired)
                return null
            }
            studioSourceLease = acquired
            studioSourceRevision = 0L
            acquired
        } catch (error: Exception) {
            SPEC_LOG.warn("Could not acquire the Spec Studio source lease", error)
            null
        }
    }

    /** Publish only the exact UTF-8 bytes now saved at the leased pack URI. */
    @Suppress("UnstableApiUsage")
    private fun publishStudioSource(
        server: LspServer,
        path: Path,
        expectedSource: String,
        generation: Long,
    ): Boolean = synchronized(studioSourceLock) {
        if (disposed || studioSourceGeneration.get() != generation) return@synchronized false
        val bytes = try { Files.readAllBytes(path) } catch (_: Exception) { return@synchronized false }
        val decoder = java.nio.charset.StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
            .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
        val saved = try { decoder.decode(java.nio.ByteBuffer.wrap(bytes)).toString() }
        catch (_: Exception) { return@synchronized false }
        if (saved != expectedSource) return@synchronized false
        var lease = studioSourceLease
        if (lease == null) lease = acquireStudioSource(server, generation) ?: return@synchronized false
        val nextRevision = studioSourceRevision + 1L
        if (nextRevision !in 1L..0xffff_ffffL) return@synchronized false
        try {
            val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                lsp4j.workspaceService.executeCommand(
                    ExecuteCommandParams("tcl-lsp.studioSource", listOf(
                        mapOf(
                            "operation" to "publish",
                            "uri" to path.toUri().toString(),
                            "lease" to mapOf("issuer" to lease!!.issuer, "token" to lease!!.token),
                            "revision" to nextRevision,
                            "source" to saved,
                        ),
                    ))
                )
            }
            val accepted = result != null && Gson().toJsonTree(result).let {
                it.isJsonObject && it.asJsonObject.get("accepted")?.asBoolean == true
            }
            if (!accepted) {
                // A refused publication can leave an earlier source active.
                // Replace this handle when possible, otherwise retain it so
                // disposal can still issue an exact retire request.
                val replacement = acquireStudioSource(server, generation)
                if (replacement == null) retireStudioSourceHandle(server, path, lease)
                return@synchronized false
            }
            if (disposed || studioSourceGeneration.get() != generation || studioSourceLease !== lease) {
                return@synchronized false
            }
            studioSourceRevision = nextRevision
            true
        } catch (error: Exception) {
            SPEC_LOG.warn("Could not publish the saved Spec Studio pack", error)
            // The server may have processed the request before the transport
            // failed. Keep this handle for disposal's exact retire attempt.
            false
        }
    }

    @Suppress("UnstableApiUsage")
    private fun retireStudioSource(server: LspServer, generation: Long): Boolean {
        synchronized(studioSourceLock) {
            val lease = studioSourceLease ?: return true
            if (studioSourceGeneration.get() != generation) return false
            val path = paths.getValue("dsl")
            return retireStudioSourceHandle(server, path, lease)
        }
    }

    @Suppress("UnstableApiUsage")
    private fun retireStudioSourceHandle(server: LspServer, path: Path, lease: StudioSourceLease): Boolean {
        try {
            val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
                lsp4j.workspaceService.executeCommand(
                    ExecuteCommandParams("tcl-lsp.studioSource", listOf(
                        mapOf(
                            "operation" to "retire",
                            "uri" to path.toUri().toString(),
                            "lease" to mapOf("issuer" to lease.issuer, "token" to lease.token),
                        ),
                    ))
                )
            }
            if (result != null && Gson().toJsonTree(result).let {
                    it.isJsonObject && it.asJsonObject.get("accepted")?.asBoolean == true
                } && studioSourceLease === lease) {
                studioSourceLease = null
                return true
            }
            return false
        } catch (error: Exception) {
            SPEC_LOG.warn("Could not retire the Spec Studio source lease", error)
            // Preserve the exact handle after an uncertain transport outcome;
            // disposal can retry it, and a later same-URI acquire supersedes it.
            return false
        }
    }

    private fun readSavedUtf8(path: Path): String? = try {
        val decoder = java.nio.charset.StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
            .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
        decoder.decode(java.nio.ByteBuffer.wrap(Files.readAllBytes(path))).toString()
    } catch (_: Exception) { null }

    private fun materialise(
        surface: String,
        text: String,
        expectedSourceRevision: Long?,
        webGeneration: Long?,
        basedOnDslRevision: Long?,
    ) {
        if (surface !in paths) return
        if (SpecStudioGeneratedFiles.utf8(text) == null) {
            if (surface == "dsl" || surface == "sample") {
                sendCurrentSurface(surface, webGeneration, accepted = false)
            }
            return
        }
        if (surface == "dsl" || surface == "sample") {
            val currentRevision = sourceRevisions.getValue(surface).get()
            if ((expectedSourceRevision ?: 0L) != currentRevision) {
                val retryable = lastPageWriteGenerations.getValue(surface).get() != 0L
                sendCurrentSurface(surface, webGeneration, accepted = false, retryable = retryable)
                return
            }
        } else if (pendingDslGeneration != null || basedOnDslRevision != sourceRevisions.getValue("dsl").get()) {
            return
        }
        val generation = surfaceGenerations.getValue(surface).incrementAndGet()
        val publicationAttempt = if (surface == "dsl") dslPublicationAttempt.incrementAndGet() else null
        if (surface == "dsl") {
            pendingDslGeneration = generation
            surfaceGenerations.getValue("rust").incrementAndGet()
            surfaceGenerations.getValue("stub").incrementAndGet()
        }
        val path = paths.getValue(surface)
        ApplicationManager.getApplication().invokeLater {
            if (!isCurrent(surface, generation) || !sourceRevisionCurrent(surface, expectedSourceRevision) ||
                !dslBasisCurrent(surface, basedOnDslRevision)) return@invokeLater
            WriteCommandAction.runWriteCommandAction(project) {
                if (!isCurrent(surface, generation) || !sourceRevisionCurrent(surface, expectedSourceRevision) ||
                    !dslBasisCurrent(surface, basedOnDslRevision)) return@runWriteCommandAction
                if (!resolveUnresolvedSourceMutation(surface) || !sourceRevisionCurrent(surface, expectedSourceRevision)) {
                    refuseMaterialisation(surface, generation, webGeneration)
                    return@runWriteCommandAction
                }
                try {
                    Files.createDirectories(sessionDir)
                    val bytes = SpecStudioGeneratedFiles.utf8(text) ?: return@runWriteCommandAction
                    val existed = Files.exists(path)
                    var file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)
                    val documents = FileDocumentManager.getInstance()
                    var document = file?.let(documents::getCachedDocument)
                    var previous = generatedSnapshots[surface]
                    if (previous != null && previous.document != null && document == null && file != null) {
                        previous = recoverGeneratedSnapshot(surface, file, null)
                        if (previous != null) generatedSnapshots[surface] = previous
                    }
                    if (previous == null && file != null) previous = recoverGeneratedSnapshot(surface, file, document)
                    if (file != null) {
                        val expectedReceipt = generatedReceipts[surface]
                        val savedBytesMatch = try {
                            expectedReceipt != null &&
                                SpecStudioGeneratedFiles.digest(file.contentsToByteArray()) == expectedReceipt
                        } catch (_: Exception) {
                            false
                        }
                        if (!savedBytesMatch) {
                            SPEC_LOG.info("Preserving Spec Studio $surface file whose saved bytes no longer match its receipt")
                            if (surface == "dsl" || surface == "sample") {
                                refuseMaterialisation(surface, generation, webGeneration)
                            }
                            return@runWriteCommandAction
                        }
                        if (previous == null || previous.file !== file) {
                            SPEC_LOG.info("Preserving unowned Spec Studio $surface file")
                            if (surface == "dsl" || surface == "sample") {
                                refuseMaterialisation(surface, generation, webGeneration)
                            }
                            return@runWriteCommandAction
                        }
                        val currentText = SpecStudioGeneratedFiles.currentText(file, document)
                        if (currentText != previous.text ||
                            (previous.document != null && document !== previous.document) ||
                            (previous.document != null && document?.modificationStamp != previous.modificationStamp)) {
                            generatedSnapshots.remove(surface)
                            SPEC_LOG.info("Preserving edited Spec Studio $surface document")
                            if (surface == "dsl" || surface == "sample") {
                                refuseMaterialisation(surface, generation, webGeneration)
                            }
                            return@runWriteCommandAction
                        }
                        if (document != null) {
                            val before = captureSourceImage(path)
                            var reconciled = false
                            materialisingDocument = document
                            try {
                                document.setText(text)
                                documents.saveDocument(document)
                                if (documents.isDocumentUnsaved(document)) {
                                    refuseMaterialisation(surface, generation, webGeneration)
                                    return@runWriteCommandAction
                                }
                            } finally {
                                materialisingDocument = null
                                reconciled = reconcileSourceMutation(surface, path, before)
                            }
                            if (!reconciled) {
                                refuseMaterialisation(surface, generation, webGeneration)
                                return@runWriteCommandAction
                            }
                        } else {
                            val before = captureSourceImage(path)
                            var reconciled = false
                            try {
                                file.setBinaryContent(bytes)
                            } finally {
                                reconciled = reconcileSourceMutation(surface, path, before)
                            }
                            if (!reconciled) {
                                refuseMaterialisation(surface, generation, webGeneration)
                                return@runWriteCommandAction
                            }
                        }
                    } else {
                        if (existed) {
                            if (surface == "dsl" || surface == "sample") {
                                refuseMaterialisation(surface, generation, webGeneration)
                            }
                            return@runWriteCommandAction
                        }
                        val before = captureSourceImage(path)
                        var reconciled = false
                        try {
                            Files.write(path, bytes)
                        } finally {
                            reconciled = reconcileSourceMutation(surface, path, before)
                        }
                        if (!reconciled) {
                            refuseMaterialisation(surface, generation, webGeneration)
                            return@runWriteCommandAction
                        }
                        file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)
                            ?: return@runWriteCommandAction
                        document = documents.getCachedDocument(file)
                    }
                    if (!isCurrent(surface, generation)) return@runWriteCommandAction
                    if (file == null) {
                        if (surface == "dsl" || surface == "sample") {
                            refuseMaterialisation(surface, generation, webGeneration)
                        }
                        return@runWriteCommandAction
                    }
                    val writtenText = SpecStudioGeneratedFiles.currentText(file, document)
                        ?: return@runWriteCommandAction
                    if (writtenText != text) {
                        if (surface == "dsl" || surface == "sample") {
                            refuseMaterialisation(surface, generation, webGeneration)
                        }
                        return@runWriteCommandAction
                    }
                    generatedSnapshots[surface] = SpecStudioGeneratedFiles.Snapshot(
                        file, document, document?.modificationStamp, text,
                    )
                    generatedReceipts[surface] = SpecStudioGeneratedFiles.digest(file.contentsToByteArray())
                    SpecStudioGeneratedFiles.writeReceipts(sessionDir, generatedReceipts)
                    if (surface == "dsl") {
                        val revision = sourceRevisions.getValue(surface).get()
                        lastPageWriteGenerations.getValue(surface).set(webGeneration ?: 0L)
                        val leaseGeneration = studioSourceGeneration.get()
                        ApplicationManager.getApplication().executeOnPooledThread {
                            val server = awaitRunningTclLspServer(project)
                            val accepted = server != null && publishStudioSource(
                                server, path, writtenText, leaseGeneration,
                            )
                            ApplicationManager.getApplication().invokeLater {
                                if (!isCurrent(surface, generation) ||
                                    sourceRevisions.getValue(surface).get() != revision) return@invokeLater
                                if (accepted) {
                                    if (pendingDslGeneration == generation) pendingDslGeneration = null
                                    sendUpdate(surface, writtenText, revision, webGeneration, accepted = true,
                                        publicationPending = if (surface == "dsl") false else null,
                                        publicationAttempt = publicationAttempt)
                                } else {
                                    refuseMaterialisation(surface, generation, webGeneration)
                                }
                            }
                        }
                    } else if (surface == "sample") {
                        val revision = sourceRevisions.getValue(surface).get()
                        lastPageWriteGenerations.getValue(surface).set(webGeneration ?: 0L)
                        sendUpdate(surface, writtenText, revision, webGeneration, accepted = true)
                    }
                } catch (error: Exception) {
                    SPEC_LOG.warn("Could not materialise Spec Studio $surface", error)
                    if (isCurrent(surface, generation) && (surface == "dsl" || surface == "sample")) {
                        refuseMaterialisation(surface, generation, webGeneration)
                    }
                }
            }
        }
    }

    private fun openSurface(surface: String) {
        if (surface !in paths) return
        val generation = openGenerations.getValue(surface).incrementAndGet()
        val path = paths.getValue(surface)
        ApplicationManager.getApplication().invokeLater {
            if (!isOpenCurrent(surface, generation)) return@invokeLater
            WriteCommandAction.runWriteCommandAction(project) {
                if (!isOpenCurrent(surface, generation)) return@runWriteCommandAction
                try {
                    Files.createDirectories(sessionDir)
                    var file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)
                    if (file == null && !Files.exists(path)) {
                        Files.write(path, byteArrayOf())
                        file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)
                        if (file != null) {
                            val document = FileDocumentManager.getInstance().getCachedDocument(file)
                            generatedSnapshots[surface] = SpecStudioGeneratedFiles.Snapshot(
                                file, document, document?.modificationStamp, "",
                            )
                            generatedReceipts[surface] = SpecStudioGeneratedFiles.digest(file.contentsToByteArray())
                            SpecStudioGeneratedFiles.writeReceipts(sessionDir, generatedReceipts)
                        }
                    }
                    if (file != null && isOpenCurrent(surface, generation)) {
                        if (surface == "dsl") {
                            val source = readSavedUtf8(path)
                            if (source != null) {
                                val leaseGeneration = studioSourceGeneration.get()
                                ApplicationManager.getApplication().executeOnPooledThread {
                                    val server = awaitRunningTclLspServer(project)
                                    if (server != null) publishStudioSource(server, path, source, leaseGeneration)
                                }
                            }
                        }
                        FileEditorManager.getInstance(project).openFile(file, true)
                    }
                } catch (error: Exception) {
                    SPEC_LOG.warn("Could not open Spec Studio $surface", error)
                }
            }
        }
    }

    private fun recoverGeneratedSnapshot(
        surface: String,
        file: com.intellij.openapi.vfs.VirtualFile,
        document: com.intellij.openapi.editor.Document?,
    ): SpecStudioGeneratedFiles.Snapshot? {
        val expected = generatedReceipts[surface] ?: return null
        val bytes = try { file.contentsToByteArray() } catch (_: Exception) { return null }
        if (SpecStudioGeneratedFiles.digest(bytes) != expected) return null
        if (document != null && FileDocumentManager.getInstance().isDocumentUnsaved(document)) return null
        val text = SpecStudioGeneratedFiles.currentText(file, document) ?: return null
        return SpecStudioGeneratedFiles.Snapshot(file, document, document?.modificationStamp, text)
    }

    private fun isCurrent(surface: String, generation: Long): Boolean =
        !disposed && SpecStudioPanelGenerations.current(sessionDir) == panelGeneration &&
            surfaceGenerations[surface]?.get() == generation

    private fun isOpenCurrent(surface: String, generation: Long): Boolean =
        !disposed && SpecStudioPanelGenerations.current(sessionDir) == panelGeneration &&
            openGenerations[surface]?.get() == generation

    private fun sourceRevisionCurrent(surface: String, expected: Long?): Boolean =
        (surface != "dsl" && surface != "sample") ||
            sourceRevisions.getValue(surface).get() == (expected ?: 0L)

    private fun dslBasisCurrent(surface: String, basis: Long?): Boolean =
        if (surface == "dsl" || surface == "sample") true
        else pendingDslGeneration == null && unresolvedSourceImages["dsl"] == null &&
            basis == sourceRevisions.getValue("dsl").get()

    private fun currentSurfaceSnapshot(surface: String): Pair<String, Long>? {
        val path = paths[surface] ?: return null
        val file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path) ?: return null
        val document = FileDocumentManager.getInstance().getCachedDocument(file)
        val text = SpecStudioGeneratedFiles.currentText(file, document) ?: return null
        return text to sourceRevisions.getValue(surface).get()
    }

    private fun sendCurrentSurface(
        surface: String,
        webGeneration: Long?,
        accepted: Boolean,
        retryable: Boolean = false,
        publicationPending: Boolean? = null,
        publicationAttempt: Long? = null,
    ) {
        ApplicationManager.getApplication().invokeLater {
            if (disposed || SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return@invokeLater
            val sourceResolved = ApplicationManager.getApplication().runReadAction<Boolean> {
                resolveUnresolvedSourceMutation(surface)
            }
            if (!sourceResolved) return@invokeLater
            val snapshot = ApplicationManager.getApplication().runReadAction<Pair<String, Long>?> {
                currentSurfaceSnapshot(surface)
            } ?: return@invokeLater
            sendUpdate(surface, snapshot.first, snapshot.second, webGeneration, accepted, retryable, publicationPending, publicationAttempt)
        }
    }

    private data class SourceImage(
        val file: VirtualFile?,
        val document: com.intellij.openapi.editor.Document?,
        val documentText: String?,
        val modificationStamp: Long?,
        val savedDigest: String?,
    )

    private fun captureSourceImage(path: Path): SourceImage {
        val file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)
        if (file == null && Files.exists(path)) {
            throw IllegalStateException("Spec Studio source exists but is not indexed")
        }
        val document = file?.let { FileDocumentManager.getInstance().getCachedDocument(it) }
        val savedDigest = when {
            file == null -> null
            !file.isValid -> throw IllegalStateException("Spec Studio source file is unavailable")
            else -> SpecStudioGeneratedFiles.digest(file.contentsToByteArray())
        }
        return SourceImage(
            file = file,
            document = document,
            documentText = document?.text,
            modificationStamp = document?.modificationStamp,
            savedDigest = savedDigest,
        )
    }

    /** Issue one new host revision only when the actual file/document image changed. */
    private fun reconcileSourceMutation(surface: String, path: Path, before: SourceImage): Boolean {
        if (surface != "dsl" && surface != "sample") return true
        val after = try { captureSourceImage(path) } catch (_: Exception) {
            unresolvedSourceImages[surface] = before
            return false
        }
        applySourceImageChange(surface, before, after)
        unresolvedSourceImages.remove(surface)
        return true
    }

    private fun resolveUnresolvedSourceMutation(surface: String): Boolean {
        val before = unresolvedSourceImages[surface] ?: return true
        val path = paths[surface] ?: return false
        val after = try { captureSourceImage(path) } catch (_: Exception) { return false }
        applySourceImageChange(surface, before, after)
        unresolvedSourceImages.remove(surface)
        return true
    }

    private fun applySourceImageChange(surface: String, before: SourceImage, after: SourceImage) {
        val changed = before.file !== after.file ||
            before.document !== after.document ||
            before.documentText != after.documentText ||
            before.modificationStamp != after.modificationStamp ||
            before.savedDigest != after.savedDigest
        if (changed) {
            sourceRevisions.getValue(surface).incrementAndGet()
            lastPageWriteGenerations.getValue(surface).set(0L)
        }
    }

    private fun refuseMaterialisation(surface: String, generation: Long, webGeneration: Long?) {
        lastPageWriteGenerations.getValue(surface).set(0L)
        val publicationAttempt = if (surface == "dsl") dslPublicationAttempt.incrementAndGet() else null
        if (surface == "dsl") pendingDslGeneration = -1L
        sendCurrentSurface(surface, webGeneration, accepted = false,
            publicationPending = if (surface == "dsl") true else null,
            publicationAttempt = publicationAttempt)
    }

    private fun sendUpdate(
        surface: String,
        text: String,
        sourceRevision: Long,
        webGeneration: Long? = null,
        accepted: Boolean? = null,
        retryable: Boolean? = null,
        publicationPending: Boolean? = null,
        publicationAttempt: Long? = null,
    ) {
        val escapedSurface = gson.toJson(surface)
        val escapedText = gson.toJson(text)
        val escapedWebGeneration = gson.toJson(webGeneration)
        val escapedAccepted = gson.toJson(accepted)
        val escapedRetryable = gson.toJson(retryable)
        val escapedPublicationPending = gson.toJson(publicationPending)
        val escapedPublicationAttempt = gson.toJson(publicationAttempt)
        browser.cefBrowser.executeJavaScript(
            "window.dispatchEvent(new MessageEvent('message',{data:{type:'tclSpecStudioHostUpdate',surface:$escapedSurface,text:$escapedText,sourceRevision:$sourceRevision,webGeneration:$escapedWebGeneration,accepted:$escapedAccepted,retryable:$escapedRetryable,publicationPending:$escapedPublicationPending,publicationAttempt:$escapedPublicationAttempt}}));",
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
     * commands.
     */
    @Suppress("UnstableApiUsage")
    private fun applySampleDialect(dialect: String?): CompletableFuture<Unit> {
        val uri = paths.getValue("sample").toUri().toString()
        // Releasing sends the URI alone rather than a null second argument:
        // the server reads an absent dialect as a clear, and this keeps the
        // argument list free of nulls across the Gson boundary.
        val args: List<Any> = if (dialect == null) listOf(uri) else listOf(uri, dialect)
        // Claimed here, not inside the task: the order pins are *requested* in
        // is the studio's intent, and the order the pooled tasks happen to run
        // in is not.
        val generation = dialectPinGeneration.incrementAndGet()
        val completed = CompletableFuture<Unit>()
        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                if (SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return@executeOnPooledThread
                // Wait for the lazily-started server so the studio can pin its
                // sample before any Tcl editor has opened.
                val server = awaitRunningTclLspServer(project) ?: run {
                    SPEC_LOG.warn("No Tcl LSP server to pin the Spec Studio sample dialect on")
                    return@executeOnPooledThread
                }
                // Recheck under the lock rather than before it: a bare check
                // leaves the window between the check and the send, which is
                // exactly where a newer pin would overtake this one.
                synchronized(dialectPinLock) {
                    if (dialectPinGeneration.get() != generation ||
                        SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) {
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
            } finally {
                completed.complete(Unit)
            }
        }
        return completed
    }

    override fun dispose() {
        if (disposed) return
        disposed = true
        val leaseGeneration = studioSourceGeneration.incrementAndGet()
        surfaceGenerations.values.forEach { it.incrementAndGet() }
        val dialectClear = applySampleDialect(null)
        SpecStudioPanelGenerations.retire(sessionDir, panelGeneration)
        SpecStudioPanelGenerations.unregister(this)
        ApplicationManager.getApplication().executeOnPooledThread {
            try { dialectClear.get(LspServer.DEFAULT_REQUEST_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS) }
            catch (error: Exception) { SPEC_LOG.warn("Spec Studio sample dialect clear did not complete before source retirement", error) }
            val server = awaitRunningTclLspServer(project)
            var retired = false
            if (server != null) {
                repeat(3) { attempt ->
                    if (!retired) retired = retireStudioSource(server, leaseGeneration)
                    if (!retired && attempt < 2) Thread.sleep(100L)
                }
            }
            if (!retired) {
                SPEC_LOG.warn("Retaining Spec Studio files because the active source lease could not be retired")
                return@executeOnPooledThread
            }
            ApplicationManager.getApplication().invokeLater {
                if (SpecStudioPanelGenerations.current(sessionDir) != panelGeneration) return@invokeLater
                WriteCommandAction.runWriteCommandAction(project) {
                try {
                    val documents = FileDocumentManager.getInstance()
                    val editors = FileEditorManager.getInstance(project)
                    for ((surface, receipt) in generatedSnapshots.toMap()) {
                        val file = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(paths.getValue(surface))
                            ?: continue
                        if (file !== receipt.file || editors.isFileOpen(file)) continue
                        val document = documents.getCachedDocument(file)
                        if (receipt.document != null && document != null && document !== receipt.document) continue
                        val expectedReceipt = generatedReceipts[surface] ?: continue
                        val savedBytesMatch = try {
                            SpecStudioGeneratedFiles.digest(file.contentsToByteArray()) == expectedReceipt
                        } catch (_: Exception) {
                            false
                        }
                        if (!savedBytesMatch) continue
                        val currentText = SpecStudioGeneratedFiles.currentText(file, document) ?: continue
                        if (currentText != receipt.text ||
                            (document != null && receipt.document != null &&
                                document.modificationStamp != receipt.modificationStamp)) continue
                        file.delete(this)
                        generatedReceipts.remove(surface)
                    }
                    if (generatedReceipts.isEmpty()) {
                        Files.deleteIfExists(SpecStudioGeneratedFiles.receiptPath(sessionDir))
                        if (Files.isDirectory(sessionDir) && Files.list(sessionDir).use { !it.findAny().isPresent }) {
                            Files.deleteIfExists(sessionDir)
                        }
                    } else {
                        SpecStudioGeneratedFiles.writeReceipts(sessionDir, generatedReceipts)
                    }
                } catch (error: Exception) {
                    SPEC_LOG.warn("Could not remove unchanged Spec Studio generated files", error)
                }
                }
            }
        }
    }
}
