// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import java.io.ByteArrayInputStream
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull

class JcefSupportTest {
    @Test
    fun descriptorBridgesThePostExtractionJcefClassloader() {
        val descriptor = parseResource("META-INF/plugin.xml")
        val dependencies = descriptor.getElementsByTagName("depends")
        val jcefDependency = (0 until dependencies.length)
            .map { dependencies.item(it) }
            .firstOrNull { it.textContent.trim() == "com.intellij.modules.jcef" }

        assertNotNull(jcefDependency)
        assertEquals("true", jcefDependency.attributes.getNamedItem("optional")?.nodeValue)
        val configFile = assertNotNull(
            jcefDependency.attributes.getNamedItem("config-file")?.nodeValue,
        )
        assertEquals("com.tcllsp.jetbrains-jcef.xml", configFile)
        assertEquals("idea-plugin", parseResource("META-INF/$configFile").documentElement.tagName)
    }

    @Test
    fun unavailableJcefDoesNotEscapeAsNoClassDefFoundError() {
        val emptyLoader = object : ClassLoader(null) {}

        assertFalse(isJcefSupported(emptyLoader))
    }

    private fun parseResource(path: String) =
        DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(
            ByteArrayInputStream(
                assertNotNull(javaClass.classLoader.getResourceAsStream(path)).readAllBytes(),
            ),
        )
}
