// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.Gson
import com.google.gson.JsonObject
import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.google.gson.JsonPrimitive
import com.intellij.openapi.editor.Document
import com.intellij.openapi.application.ReadAction
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.platform.lsp.api.LspServerState
import org.eclipse.lsp4j.InitializeResult
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.eclipse.lsp4j.ExecuteCommandParams

private fun exactWireInteger(value: JsonElement?): java.math.BigInteger? =
    value?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isNumber }?.let {
        runCatching { it.asBigDecimal.toBigIntegerExact() }.getOrNull()
    }

private fun exactWireInt(value: JsonElement?): Int? = exactWireInteger(value)?.let {
    runCatching { it.intValueExact() }.getOrNull()
}

private fun canonicalWireIssuer(value: JsonElement?): JsonArray? {
    val values = value?.takeIf { it.isJsonArray }?.asJsonArray ?: return null
    if (values.size() != 16) return null
    val result = JsonArray()
    for (entry in values) {
        val integer = exactWireInt(entry)?.takeIf { it in 0..255 } ?: return null
        result.add(integer)
    }
    return result
}

/** Host document identity and image; its stamp is never an LSP version. */
internal class CommandSourceSnapshot private constructor(
    val file: VirtualFile,
    val document: Document,
    val source: String,
    private val originalUrl: String,
    private val stamp: Long,
    private val configuration: String,
) {
    fun isCurrent(project: Project): Boolean = !project.isDisposed && file.isValid && file.url == originalUrl &&
        FileDocumentManager.getInstance().getFile(document) === file &&
        document.modificationStamp == stamp && document.text == source &&
        Gson().toJson(TclLspSettings.getInstance().toServerSettings()) == configuration

    companion object {
        fun capture(file: VirtualFile, document: Document): CommandSourceSnapshot? {
            if (!file.isValid || FileDocumentManager.getInstance().getFile(document) !== file) return null
            val source = document.text
            if (!Charsets.UTF_8.newEncoder().canEncode(source)) return null
            return CommandSourceSnapshot(file, document, source, file.url, document.modificationStamp,
                Gson().toJson(TclLspSettings.getInstance().toServerSettings()))
        }
    }
}

/** Actual selected SDK sync publication, separate from the local editor stamp. */
@Suppress("UnstableApiUsage")
internal class CommandSyncPublication private constructor(
    val project: Project,
    val server: LspServer,
    val snapshot: CommandSourceSnapshot,
    val uri: String,
    val version: Int,
    private val descriptor: Any,
    private val initialized: InitializeResult,
) {
    /** Called inside a read or write action immediately before a host sink. */
    fun isCurrent(): Boolean = snapshot.isCurrent(project) &&
        LspServerManager.getInstance(project)
            .getServersForProvider(TclLspServerSupportProvider::class.java).singleOrNull() === server &&
        server.project === project && server.state == LspServerState.Running &&
        server.descriptor === descriptor && server.initializeResult === initialized &&
        server.getDocumentIdentifier(snapshot.file).uri == uri &&
        server.getDocumentVersion(snapshot.document) == version

    fun readCurrent(): Boolean = ReadAction.compute<Boolean, RuntimeException> { isCurrent() }

    companion object {
        fun capture(project: Project, server: LspServer, snapshot: CommandSourceSnapshot): CommandSyncPublication? =
            ReadAction.compute<CommandSyncPublication?, RuntimeException> {
                if (!snapshot.isCurrent(project) || server.project !== project || server.state != LspServerState.Running) return@compute null
                val initialized = server.initializeResult ?: return@compute null
                val result = CommandSyncPublication(project, server, snapshot,
                    server.getDocumentIdentifier(snapshot.file).uri,
                    server.getDocumentVersion(snapshot.document), server.descriptor, initialized)
                result.takeIf { it.isCurrent() }
            }
    }
}

/** Metadata issued by the actual sync publication. It supplies no edit permission. */
internal class PublishedCommandSource private constructor(
    val wire: JsonObject,
    val uri: String,
    val version: Int,
    val source: String,
    val analysisSource: String,
    val dialect: String,
) {
    fun transformArgument(): Map<String, Any> = mapOf("uri" to uri, "version" to version, "source" to source)

    companion object {
        fun parse(wire: JsonObject, uri: String, source: String): PublishedCommandSource? {
            fun string(key: String) = wire.get(key)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString
            fun integer(key: String) = exactWireInt(wire.get(key))
            if (integer("schema") != 1 || string("purpose") != "current-document-source-context" ||
                string("uri") != uri || string("source") != source || string("id").isNullOrEmpty()) return null
            val issuer = canonicalWireIssuer(wire.get("issuer")) ?: return null
            val epoch = exactWireInteger(wire.get("inputsEpoch"))?.takeIf { it.signum() >= 0 && it.bitLength() <= 64 }
                ?: return null
            val version = integer("version") ?: return null
            val analysis = string("analysisSource") ?: return null
            val dialect = string("dialect")?.takeIf { it.isNotEmpty() } ?: return null
            if (!Charsets.UTF_8.newEncoder().canEncode(source) ||
                !Charsets.UTF_8.newEncoder().canEncode(analysis)) return null
            val canonical = wire.deepCopy()
            canonical.addProperty("schema", 1)
            canonical.addProperty("version", version)
            canonical.add("issuer", issuer)
            canonical.add("inputsEpoch", JsonPrimitive(epoch))
            return PublishedCommandSource(canonical, uri, version, source, analysis, dialect)
        }
    }
}

@Suppress("UnstableApiUsage")
internal fun capturePublishedCommandSource(publication: CommandSyncPublication): PublishedCommandSource? {
    if (!publication.readCurrent()) return null
    val result = publication.server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { client ->
        client.workspaceService.executeCommand(ExecuteCommandParams("tcl-lsp.captureDocumentSourceContext",
            listOf(publication.uri, publication.version, publication.snapshot.source)))
    }
    if (!publication.readCurrent()) return null
    val json = Gson().toJsonTree(result).takeIf { it.isJsonObject }?.asJsonObject ?: return null
    return PublishedCommandSource.parse(json, publication.uri, publication.snapshot.source)
        ?.takeIf { it.version == publication.version }
}

@Suppress("UnstableApiUsage")
internal fun publishedCommandSourceIsCurrent(server: LspServer, source: PublishedCommandSource): Boolean =
    server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { client ->
        client.workspaceService.executeCommand(ExecuteCommandParams("tcl-lsp.validateDocumentSourceContext", listOf(source.wire)))
    } == true

/** A worker-produced whole-source proposal, independently validated by the server. */
internal fun commandTransformProposal(result: Any?, source: PublishedCommandSource): Pair<String, JsonObject>? {
    val json = Gson().toJsonTree(result).takeIf { it.isJsonObject }?.asJsonObject ?: return null
    val replacement = json.get("source")?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString ?: return null
    if (!Charsets.UTF_8.newEncoder().canEncode(replacement)) return null
    val receipt = json.get("editReceipt")?.takeIf { it.isJsonObject }?.asJsonObject ?: return null
    if (receipt.get("uri")?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString != source.uri ||
        receipt.get("source")?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString != source.source ||
        exactWireInt(receipt.get("version")) != source.version) return null
    val issuer = canonicalWireIssuer(receipt.get("issuer")) ?: return null
    val validation = receipt.deepCopy()
    validation.addProperty("version", source.version)
    validation.add("issuer", issuer)
    validation.addProperty("replacement", replacement)
    return replacement to validation
}
