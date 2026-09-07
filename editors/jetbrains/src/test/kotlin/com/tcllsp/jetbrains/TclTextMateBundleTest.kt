// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.JsonParser
import org.jetbrains.plugins.textmate.bundles.BundleType
import org.jetbrains.plugins.textmate.bundles.TextMateFileNameMatcher
import org.jetbrains.plugins.textmate.bundles.readVSCBundle
import java.io.ByteArrayInputStream
import java.nio.file.Files
import java.nio.file.Path
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.io.path.createTempDirectory
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * The grammar only highlights anything if the TextMate service can read it as
 * a bundle, so these drive the real platform bundle reader over exactly what
 * [TclTextMateBundleProvider] unpacks — a manifest that parses but does not
 * resolve a grammar for `.tcl` is the failure that shipped.
 */
class TclTextMateBundleTest {

    @Test
    fun theUnpackedBundleIsOneTheTextMateServiceCanRead() {
        val dir = unpackIntoTempDirectory()

        assertEquals(BundleType.VSCODE, BundleType.detectBundleType(dir))

        val reader = assertNotNull(
            readVSCBundle { relative -> Files.newInputStream(dir.resolve(relative)) },
        )
        val grammars = reader.readGrammars().toList()
        assertEquals(1, grammars.size, "the bundle contributes exactly one grammar")

        val grammar = grammars.single()
        assertEquals("source.tcl", grammar.plist.value.getPlistValue("scopeName")?.string)

        val extensions = grammar.fileNameMatchers
            .filterIsInstance<TextMateFileNameMatcher.Extension>()
            .map { it.extension }
            .toSet()
        for (expected in listOf("tcl", "tm", "irule", "iapp", "tclspec")) {
            assertContains(extensions, expected)
        }
    }

    /**
     * The manifest and the plugin's own file types must claim the same
     * extensions: an extension the manifest omits opens as a Tcl file with no
     * grammar, and one only the manifest claims registers a TextMate mapping
     * for a file type the plugin never handles.
     */
    @Test
    fun theManifestClaimsExactlyTheFileTypeExtensions() {
        val manifest = JsonParser.parseString(String(resource("textmate/package.json")))
            .asJsonObject["contributes"].asJsonObject["languages"].asJsonArray
            .flatMap { it.asJsonObject["extensions"].asJsonArray }
            .map { it.asString.removePrefix(".") }
            .toSortedSet()

        assertEquals(fileTypeExtensions(), manifest)
    }

    @Test
    fun unpackingIsIdempotentAndRepairsATamperedBundle() {
        val dir = unpackIntoTempDirectory()
        val grammar = dir.resolve("syntaxes/tcl.tmLanguage.json")
        val original = Files.readAllBytes(grammar)

        assertNotNull(unpackBundle(dir, ::resourceOrNull))
        assertTrue(Files.readAllBytes(grammar).contentEquals(original))

        Files.write(grammar, "{}".toByteArray())
        assertNotNull(unpackBundle(dir, ::resourceOrNull))
        assertTrue(Files.readAllBytes(grammar).contentEquals(original))
    }

    @Test
    fun aMissingResourceIsReportedRatherThanHandedOverAsAnEmptyBundle() {
        val dir = createTempDirectory("tcl-textmate-missing")
        assertEquals(null, unpackBundle(dir) { null })
    }

    private fun unpackIntoTempDirectory(): Path =
        assertNotNull(unpackBundle(createTempDirectory("tcl-textmate"), ::resourceOrNull))

    /** The `extensions="a;b;c"` attributes of the plugin's two file types. */
    private fun fileTypeExtensions(): Set<String> {
        val document = DocumentBuilderFactory.newInstance().newDocumentBuilder()
            .parse(ByteArrayInputStream(resource("META-INF/plugin.xml")))
        val fileTypes = document.getElementsByTagName("fileType")
        return (0 until fileTypes.length)
            .map { fileTypes.item(it) }
            .flatMap {
                assertNotNull(it.attributes.getNamedItem("extensions")).nodeValue.split(";")
            }
            .toSortedSet()
    }

    private fun resourceOrNull(path: String): ByteArray? =
        javaClass.classLoader.getResourceAsStream(path)?.use { it.readBytes() }

    private fun resource(path: String): ByteArray = assertNotNull(resourceOrNull(path))
}
