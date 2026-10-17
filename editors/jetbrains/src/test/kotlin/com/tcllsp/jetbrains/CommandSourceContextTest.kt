// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.JsonParser
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class CommandSourceContextTest {
    private val uri = "file:///actual-source.tcl"
    private val raw = "puts café\rputs next\u0000\n"

    private fun receipt() = JsonParser.parseString("""{
        "schema":1,"purpose":"current-document-source-context","id":"7",
        "issuer":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
        "uri":"file:///actual-source.tcl","version":17,"source":"puts café\rputs next\u0000\n",
        "analysisSource":"puts café\nputs next\u0000\n","dialect":"tcl8.6","inputsEpoch":4
    }""").asJsonObject

    @Test
    fun actualServerVersionAndRawImageStaySeparateFromAnalysisImage() {
        val source = assertNotNull(PublishedCommandSource.parse(receipt(), uri, raw))
        assertEquals(17, source.version)
        assertEquals(raw, source.source)
        assertEquals("puts café\nputs next\u0000\n", source.analysisSource)
        assertEquals(mapOf("uri" to uri, "version" to 17, "source" to raw), source.transformArgument())
        assertNull(commandTransformProposal(receipt(), source), "metadata receipt alone is not an edit result")
    }

    @Test
    fun foreignSourceMissingVersionMalformedIssuerAndUnrepresentableTextRefuse() {
        assertNull(PublishedCommandSource.parse(receipt(), "file:///foreign.tcl", raw))
        assertNull(PublishedCommandSource.parse(receipt(), uri, "different source"))
        val missingVersion = receipt().apply { remove("version") }
        assertNull(PublishedCommandSource.parse(missingVersion, uri, raw))
        val badIssuer = receipt().apply { getAsJsonArray("issuer").set(0, com.google.gson.JsonPrimitive(256)) }
        assertNull(PublishedCommandSource.parse(badIssuer, uri, raw))
        val surrogate = "\uD800"
        val invalid = receipt().apply { addProperty("source", surrogate) }
        assertNull(PublishedCommandSource.parse(invalid, uri, surrogate))
    }

    @Test
    fun integralTransportNumbersCanonicaliseWithoutInventingVersions() {
        val integral = receipt().apply { addProperty("version", 17.0); addProperty("schema", 1.0) }
        val source = assertNotNull(PublishedCommandSource.parse(integral, uri, raw))
        assertEquals("17", source.wire.get("version").asString)
        val fractional = receipt().apply { addProperty("version", 17.5) }
        assertNull(PublishedCommandSource.parse(fractional, uri, raw))
        val overflowing = receipt().apply { addProperty("version", 2147483648L) }
        assertNull(PublishedCommandSource.parse(overflowing, uri, raw))
    }
}
