// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.Gson
import com.google.gson.JsonParser
import com.intellij.openapi.editor.impl.DocumentImpl
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class CompilerExplorerSourceTest {
    @Test
    fun originalNulAndDialectRemainIndependentJsonFields() {
        val source = "set value \u0000tcl8.4\nputs \uD834\uDD1E"
        val wire = Gson().toJson(mapOf("type" to "compile", "purpose" to "standalone-source",
            "source" to source, "dialect" to "tcl9.0", "compileToken" to "request:3", "sourceToken" to "source:2"))
        val request = ExplorerCompileRequest.parse(JsonParser.parseString(wire).asJsonObject)!!
        assertEquals(source, request.source)
        assertEquals("tcl9.0", request.dialect)
        assertEquals("request:3", request.compileToken)
        assertEquals("source:2", request.sourceToken)
    }

    @Test
    fun unknownSourceOrDialectFieldsCannotBecomeAStandaloneFallback() {
        val message = JsonParser.parseString("""{"purpose":"standalone-source","source":"puts source","dialect":"tcl9.0","compileToken":"request"}""").asJsonObject
        assertEquals(null, ExplorerCompileRequest.parse(message)!!.sourceToken)
        message.add("sourceToken", com.google.gson.JsonNull.INSTANCE)
        assertNull(ExplorerCompileRequest.parse(message))
        message.remove("sourceToken")
        message.remove("dialect")
        assertNull(ExplorerCompileRequest.parse(message))
        message.addProperty("dialect", 9)
        assertNull(ExplorerCompileRequest.parse(message))
    }

    @Test
    fun utf8TransportRefusesUnpairedCodeUnitsWithoutChangingValidSource() {
        val message = JsonParser.parseString("""{"purpose":"standalone-source","source":"puts source","dialect":"tcl9.0","compileToken":"request"}""").asJsonObject
        message.addProperty("source", "set value \uD800")
        assertNull(ExplorerCompileRequest.parse(message))
        message.addProperty("source", "set value \uD834\uDD1E\u0000")
        assertEquals("set value \uD834\uDD1E\u0000", ExplorerCompileRequest.parse(message)!!.source)
        message.addProperty("dialect", "tcl9.0\uDC00")
        assertNull(ExplorerCompileRequest.parse(message))
    }

    @Test
    fun rendererUtf16RangesUseActualDocumentBoundsWithoutByteFallback() {
        val document = DocumentImpl("a\uD834\uDD1Eβ\nnext")
        assertEquals(1 to 3, strictExplorerSourceOffsets(document, ExplorerRange(0, 1, 0, 3)))
        assertEquals(4 to 9, strictExplorerSourceOffsets(document, ExplorerRange(0, 4, 1, 4)))
        assertNull(strictExplorerSourceOffsets(document, ExplorerRange(0, 2, 0, 3)))
        assertNull(strictExplorerSourceOffsets(document, ExplorerRange(0, 0, 0, 5)))
        assertNull(strictExplorerSourceOffsets(document, ExplorerRange(2, 0, 2, 0)))
        assertNull(strictExplorerSourceOffsets(document, ExplorerRange(1, 2, 0, 3)))
        assertNull(strictExplorerSourceOffsets(document, ExplorerRange(0, -1, 0, 1)))
        val bytesOnly = JsonParser.parseString("""{"purpose":"standalone-source","sourceToken":"source","compileToken":"request","start":1,"end":4}""").asJsonObject
        assertNull(ExplorerHighlightRequest.parse(bytesOnly))
        bytesOnly.addProperty("startLine", 0)
        bytesOnly.addProperty("startCol", 1.5)
        bytesOnly.addProperty("endLine", 0)
        bytesOnly.addProperty("endCol", 3)
        assertNull(ExplorerHighlightRequest.parse(bytesOnly))
    }
}
