// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.intellij.openapi.editor.DefaultLanguageHighlighterColors as Colors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

/**
 * The colour map is only reachable through [TclSemanticTokens], and the
 * modifier half of it decides whether a built-in reads differently from a
 * user proc — the distinction the server goes to the trouble of publishing.
 */
class TclSemanticTokensTest {

    private val tokens = TclSemanticTokens()

    @Test
    fun aTokenTypeGetsItsMappedColour() {
        assertEquals(Colors.KEYWORD, tokens.getTextAttributesKey("keyword", emptyList()))
        assertEquals(Colors.MARKUP_TAG, tokens.getTextAttributesKey("event", emptyList()))
        assertEquals(Colors.NUMBER, tokens.getTextAttributesKey("port", emptyList()))
    }

    @Test
    fun aModifierTheServerSetsOverridesTheTypeColour() {
        val call = tokens.getTextAttributesKey("function", emptyList())
        val builtin = tokens.getTextAttributesKey("function", listOf("defaultLibrary"))
        val definition = tokens.getTextAttributesKey("function", listOf("definition"))

        assertEquals(Colors.FUNCTION_CALL, call)
        assertEquals(Colors.PREDEFINED_SYMBOL, builtin)
        assertEquals(Colors.FUNCTION_DECLARATION, definition)
        assertNotEquals(call, builtin)
        assertNotEquals(call, definition)
    }

    @Test
    fun aModifierWithNoRuleOfItsOwnKeepsTheTypeColour() {
        assertEquals(
            Colors.LOCAL_VARIABLE,
            tokens.getTextAttributesKey("variable", listOf("declaration")),
        )
    }

    @Test
    fun everyMappedColourIsDistinctEnoughToBeWorthPublishing() {
        // Not every type needs its own key — the platform's palette is
        // smaller than the legend — but a map that collapsed to one key would
        // paint the whole file in a single colour and still pass the coverage
        // gate in xtask.
        assertEquals(
            true,
            SEMANTIC_TOKEN_COLORS.values.distinct().size >= 10,
            "the legend must map onto a range of colours, not a single key",
        )
    }
}
