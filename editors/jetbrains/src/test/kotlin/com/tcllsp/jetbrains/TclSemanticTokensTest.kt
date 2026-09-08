// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.intellij.openapi.editor.DefaultLanguageHighlighterColors as Colors
import com.intellij.openapi.editor.colors.TextAttributesKey
import org.w3c.dom.Element
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * The colour map is only reachable through [TclSemanticTokens], and the key it
 * picks decides whether a token is visible at all: a key no stock scheme
 * paints leaves the token in the plain text colour, which is the "only numbers
 * are coloured" bug.
 */
class TclSemanticTokensTest {

    private val tokens = TclSemanticTokens()

    @Test
    fun aTokenTypeGetsItsMappedColour() {
        assertEquals(Colors.KEYWORD, tokens.getTextAttributesKey("keyword", emptyList()))
        assertEquals(Colors.FUNCTION_DECLARATION, tokens.getTextAttributesKey("event", emptyList()))
        assertEquals(Colors.NUMBER, tokens.getTextAttributesKey("port", emptyList()))
    }

    /**
     * A built-in, a call and a definition all read as a command head, so they
     * share the one key stock schemes actually colour.
     */
    @Test
    fun everyCommandHeadReadsAsADeclaration() {
        val call = tokens.getTextAttributesKey("function", emptyList())
        val builtin = tokens.getTextAttributesKey("function", listOf("defaultLibrary"))
        val definition = tokens.getTextAttributesKey("function", listOf("definition"))

        assertEquals(Colors.FUNCTION_DECLARATION, call)
        assertEquals(Colors.FUNCTION_DECLARATION, builtin)
        assertEquals(Colors.FUNCTION_DECLARATION, definition)
    }

    @Test
    fun aModifierTheServerSetsOverridesTheTypeColour() {
        val assigned = tokens.getTextAttributesKey("variable", listOf("declaration"))
        val readonly = tokens.getTextAttributesKey("variable", listOf("readonly"))

        assertEquals(Colors.INSTANCE_FIELD, assigned)
        assertEquals(Colors.CONSTANT, readonly)
        assertNotEquals(assigned, readonly)
    }

    @Test
    fun aModifierWithNoRuleOfItsOwnKeepsTheTypeColour() {
        assertEquals(
            Colors.INSTANCE_FIELD,
            tokens.getTextAttributesKey("parameter", listOf("declaration")),
        )
    }

    @Test
    fun everyMappedColourIsDistinctEnoughToBeWorthPublishing() {
        // Not every type needs its own key — the platform's palette is
        // smaller than the legend — but a map that collapsed to one key would
        // paint the whole file in a single colour and still pass the coverage
        // gate in xtask. The map yields 13 keys today.
        assertEquals(
            true,
            SEMANTIC_TOKEN_COLORS.values.distinct().size >= 10,
            "the legend must map onto a range of colours, not a single key",
        )
    }

    /**
     * The regression: a key that no stock scheme gives a foreground to paints
     * the token in the plain text colour, so the file looks unhighlighted even
     * though the server is publishing tokens. Both stock schemes are checked
     * because several keys are coloured in only one of them.
     */
    @Test
    fun everyCoreTokenIsColouredByTheStockSchemes() {
        val core = listOf(
            "keyword", "function", "variable", "parameter",
            "string", "number", "comment", "escape", "event",
        )
        for (scheme in listOf(darculaScheme(), intellijLightScheme())) {
            for (type in core) {
                val key = assertNotNull(SEMANTIC_TOKEN_COLORS[type], "$type has no colour")
                assertTrue(
                    scheme.paints(key),
                    "the ${scheme.name} scheme leaves $type (${key.externalName}) " +
                        "in the plain text colour",
                )
            }
        }
    }

    /** A stock scheme, resolving an attribute through its parent scheme. */
    private class Scheme(
        val name: String,
        private val foregrounds: Map<String, String>,
        private val parent: Scheme?,
    ) {
        /**
         * Whether some key in [key]'s fallback chain has a foreground. The
         * generic keys are skipped: `DEFAULT_IDENTIFIER` and `TEXT` *are* the
         * plain text colour, so a key reaching a foreground only through them
         * is exactly the token that looks unhighlighted.
         */
        fun paints(key: TextAttributesKey): Boolean {
            var current: TextAttributesKey? = key
            while (current != null) {
                if (current.externalName !in GENERIC_KEYS && foreground(current.externalName) != null) {
                    return true
                }
                current = current.fallbackAttributeKey
            }
            return false
        }

        private fun foreground(name: String): String? =
            foregrounds[name] ?: parent?.foreground(name)
    }

    private companion object {
        val GENERIC_KEYS = setOf("DEFAULT_IDENTIFIER", "TEXT")

        /** `<scheme>` elements of a colour-scheme resource in the IDE's `app.jar`. */
        fun schemeElements(resource: String): List<Element> {
            val stream = assertNotNull(
                TclSemanticTokensTest::class.java.classLoader.getResourceAsStream(resource),
                "$resource is not on the test classpath",
            )
            val root = stream.use {
                DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(it).documentElement
            }
            if (root.tagName == "scheme") {
                return listOf(root)
            }
            val children = root.getElementsByTagName("scheme")
            return (0 until children.length).map { children.item(it) as Element }
        }

        /** Every `<option name=…><value><option name="FOREGROUND" value=…>` in a scheme. */
        fun foregrounds(scheme: Element): Map<String, String> {
            val attributes = scheme.getElementsByTagName("attributes")
            if (attributes.length == 0) {
                return emptyMap()
            }
            val options = (attributes.item(0) as Element).getElementsByTagName("option")
            return (0 until options.length)
                .map { options.item(it) as Element }
                .filter { it.parentNode === attributes.item(0) }
                .mapNotNull { option ->
                    val value = option.getElementsByTagName("option")
                    (0 until value.length)
                        .map { value.item(it) as Element }
                        .firstOrNull { it.getAttribute("name") == "FOREGROUND" }
                        ?.let { option.getAttribute("name") to it.getAttribute("value") }
                }
                .toMap()
        }

        fun defaultScheme(named: String): Scheme {
            val schemes = schemeElements("DefaultColorSchemesManager.xml")
                .associateBy { it.getAttribute("name") }
            val scheme = assertNotNull(schemes[named], "no stock $named scheme")
            val parent = schemes[scheme.getAttribute("parent_scheme")]
            return Scheme(
                named,
                foregrounds(scheme),
                parent?.let { Scheme(it.getAttribute("name"), foregrounds(it), null) },
            )
        }

        fun darculaScheme(): Scheme = defaultScheme("Darcula")

        fun intellijLightScheme(): Scheme {
            val scheme = schemeElements("themes/Light.xml").single()
            return Scheme(
                scheme.getAttribute("name"),
                foregrounds(scheme),
                defaultScheme("Default"),
            )
        }
    }
}
