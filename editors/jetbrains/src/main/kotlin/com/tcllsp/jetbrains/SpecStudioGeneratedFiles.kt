// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

/**
 * Persisted receipts let startup/disposal remove only files whose bytes still
 * equal the last Studio-authored version. A pathname alone never establishes
 * ownership of an IDE document.
 */
internal object SpecStudioGeneratedFiles {
    private const val RECEIPTS = ".generated-receipts.json"

    data class Snapshot(
        val file: VirtualFile,
        val document: Document?,
        val modificationStamp: Long?,
        val text: String,
    )

    fun receiptPath(sessionDir: Path): Path = sessionDir.resolve(RECEIPTS)

    fun readReceipts(sessionDir: Path): MutableMap<String, String> {
        val path = receiptPath(sessionDir)
        if (!Files.isRegularFile(path)) return mutableMapOf()
        return try {
            val root = JsonParser.parseString(Files.readString(path, StandardCharsets.UTF_8)).asJsonObject
            root.entrySet().mapNotNull { (name, value) ->
                if (name in StudioSurfaces && value.isJsonPrimitive && value.asJsonPrimitive.isString) {
                    name to value.asString
                } else null
            }.toMap().toMutableMap()
        } catch (_: Exception) {
            // An unreadable receipt cannot authorize deletion or replacement.
            mutableMapOf()
        }
    }

    fun writeReceipts(sessionDir: Path, receipts: Map<String, String>) {
        val root = JsonObject()
        receipts.forEach { (name, digest) -> if (name in StudioSurfaces) root.addProperty(name, digest) }
        val target = receiptPath(sessionDir)
        val temporary = sessionDir.resolve("$RECEIPTS.tmp")
        Files.createDirectories(sessionDir)
        Files.writeString(temporary, root.toString(), StandardCharsets.UTF_8)
        try {
            Files.move(temporary, target, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
        } catch (_: java.nio.file.AtomicMoveNotSupportedException) {
            Files.move(temporary, target, StandardCopyOption.REPLACE_EXISTING)
        }
    }

    fun utf8(text: String): ByteArray? = try {
        val encoder = StandardCharsets.UTF_8.newEncoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        val buffer = encoder.encode(java.nio.CharBuffer.wrap(text))
        ByteArray(buffer.remaining()).also { buffer.get(it) }
    } catch (_: java.nio.charset.CharacterCodingException) {
        null
    }

    fun digest(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
        .digest(bytes)
        .joinToString("") { "%02x".format(it) }

    fun currentText(file: VirtualFile, document: Document?): String? {
        if (!file.isValid) return null
        if (document != null) return document.text
        val decoder = StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        return try {
            decoder.decode(java.nio.ByteBuffer.wrap(file.contentsToByteArray())).toString()
        } catch (_: Exception) {
            null
        }
    }

    /** Delete only receipt-matching, unopened generated files; never recurse. */
    fun cleanAbandoned(project: Project, sessionDir: Path) {
        if (SpecStudioPanelGenerations.isActive(sessionDir) || !Files.isDirectory(sessionDir)) return
        val receipts = readReceipts(sessionDir)
        val fs = LocalFileSystem.getInstance()
        val documents = FileDocumentManager.getInstance()
        val editors = FileEditorManager.getInstance(project)
        val surfaces = mapOf(
            "dsl" to "active.tclspec",
            "sample" to "test.tcl",
            "rust" to "generated.rs",
            "stub" to "generated-stub.tcl",
        )
        for ((surface, filename) in surfaces) {
            val expected = receipts[surface] ?: continue
            val path = sessionDir.resolve(filename)
            val file = fs.refreshAndFindFileByNioFile(path) ?: continue
            if (!file.isValid || editors.isFileOpen(file)) continue
            val document = documents.getCachedDocument(file)
            if (document != null && documents.isDocumentUnsaved(document)) continue
            val bytes = try { file.contentsToByteArray() } catch (_: Exception) { continue }
            if (digest(bytes) != expected) continue
            try {
                file.delete(this)
                receipts.remove(surface)
            } catch (_: Exception) {
                // Preserve a file if the platform refuses the guarded cleanup.
            }
        }
        try {
            if (receipts.isEmpty()) Files.deleteIfExists(receiptPath(sessionDir))
            else writeReceipts(sessionDir, receipts)
            if (Files.isDirectory(sessionDir) && Files.list(sessionDir).use { !it.findAny().isPresent }) {
                Files.deleteIfExists(sessionDir)
            }
        } catch (_: Exception) {
            // Cleanup is best-effort and may not remove unowned/user data.
        }
    }

}

internal val StudioSurfaces = setOf("dsl", "sample", "rust", "stub")
