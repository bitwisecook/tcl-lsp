// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.JsonObject
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.vfs.VirtualFile

/** The actual editor input, retained separately from standalone compiler availability. */
internal data class CompilerExplorerSourceSnapshot(
    val token: String,
    val file: VirtualFile,
    val document: Document,
    val source: String,
    val modificationStamp: Long,
    val dialect: String,
) {
    /** Called on the EDT, with no await between this check and navigation/publication. */
    fun isCurrent(): Boolean = file.isValid &&
        FileDocumentManager.getInstance().getDocument(file) === document &&
        document.modificationStamp == modificationStamp && document.text == source
}

internal fun JsonObject.explorerString(key: String): String? =
    get(key)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString

private fun JsonObject.explorerInt(key: String): Int? =
    get(key)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isNumber }?.asString?.toIntOrNull()

/** Complete standalone input; a source token can additionally retain an actual editor owner. */
internal data class ExplorerCompileRequest(
    val source: String,
    val dialect: String,
    val compileToken: String,
    val sourceToken: String?,
) {
    companion object {
        fun parse(message: JsonObject): ExplorerCompileRequest? {
            if (message.explorerString("purpose") != "standalone-source") return null
            val source = message.explorerString("source") ?: return null
            val dialect = message.explorerString("dialect")?.takeIf { it.isNotEmpty() } ?: return null
            val compileToken = message.explorerString("compileToken")?.takeIf { it.isNotEmpty() } ?: return null
            val sourceToken = message.explorerString("sourceToken")
            if (message.has("sourceToken") && sourceToken.isNullOrEmpty()) return null
            val encoder = Charsets.UTF_8.newEncoder()
            if (!encoder.canEncode(source) || !encoder.canEncode(dialect) ||
                !encoder.canEncode(compileToken) || (sourceToken != null && !encoder.canEncode(sourceToken))) return null
            return ExplorerCompileRequest(source, dialect, compileToken, sourceToken)
        }
    }
}

/** UTF-16 renderer positions only. Byte offsets and incomplete legacy tuples decline. */
internal data class ExplorerHighlightRequest(
    val compileToken: String,
    val sourceToken: String,
    val range: ExplorerRange,
) {
    companion object {
        fun parse(message: JsonObject): ExplorerHighlightRequest? {
            if (message.explorerString("purpose") != "standalone-source") return null
            return ExplorerHighlightRequest(
                message.explorerString("compileToken") ?: return null,
                message.explorerString("sourceToken") ?: return null,
                ExplorerRange(
                    message.explorerInt("startLine") ?: return null,
                    message.explorerInt("startCol") ?: return null,
                    message.explorerInt("endLine") ?: return null,
                    message.explorerInt("endCol") ?: return null,
                ),
            )
        }
    }
}

/** Exact IntelliJ line geometry, with no clamping or byte-offset conversion. */
internal fun strictExplorerSourceOffsets(document: Document, range: ExplorerRange): Pair<Int, Int>? {
    fun offset(line: Int, column: Int): Int? {
        if (line < 0 || line >= document.lineCount || column < 0) return null
        val start = document.getLineStartOffset(line)
        val end = document.getLineEndOffset(line)
        if (column > end - start) return null
        val at = start + column
        val text = document.charsSequence
        if (at > start && at < end && text[at - 1].isHighSurrogate() && text[at].isLowSurrogate()) return null
        return at
    }
    val start = offset(range.startLine, range.startColUtf16) ?: return null
    val end = offset(range.endLine, range.endColUtf16) ?: return null
    return if (start <= end) start to end else null
}
