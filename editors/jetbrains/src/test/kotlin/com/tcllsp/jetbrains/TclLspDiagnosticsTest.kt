// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.JsonObject
import org.eclipse.lsp4j.Diagnostic
import org.eclipse.lsp4j.jsonrpc.messages.Either
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * IntelliJ's problems view renders only a diagnostic's message, so anything the
 * server puts in the `code` field or the quick-fix payload is invisible there
 * unless this customizer puts it back.
 */
class TclLspDiagnosticsTest {

    private val support = TclLspDiagnostics()

    private fun diagnostic(code: String?, message: String, data: Any? = null): Diagnostic {
        val d = Diagnostic()
        d.message = message
        code?.let { d.setCode(Either.forLeft<String, Int>(it)) }
        d.data = data
        return d
    }

    @Test
    fun `the message carries the code the problems view would otherwise drop`() {
        val d = diagnostic("O102", "Forward literal load of 'a'")
        assertEquals("[O102] Forward literal load of 'a'", support.getMessage(d))
    }

    @Test
    fun `a diagnostic with no code is left exactly as the server sent it`() {
        val d = diagnostic(null, "something happened")
        assertEquals("something happened", support.getMessage(d))
    }

    @Test
    fun `an optimiser code is named as a rewrite rather than a defect`() {
        val tip = support.getTooltip(diagnostic("O102", "Forward literal load of 'a'"))
        assertTrue(tip.contains("optimiser rewrite"), tip)
    }

    @Test
    fun `an analyser code is named by its catalogue section`() {
        val tip = support.getTooltip(diagnostic("E001", "Missing dispatch word"))
        assertTrue(tip.contains("diagnostic"), tip)
        assertTrue(tip.contains("E001"), tip)
    }

    @Test
    fun `a hint-only optimisation says plainly that nothing can be applied`() {
        // The server withholds `data` when the rewrite has no precise sub-span,
        // which is exactly when a user most needs telling that the suggestion
        // is informational — otherwise it reads as a fix that failed to appear.
        val tip = support.getTooltip(diagnostic("O102", "Forward literal load of 'a'"))
        assertTrue(tip.contains("No automatic fix."), tip)
    }

    @Test
    fun `an applicable rewrite previews what accepting it would write`() {
        val data = JsonObject().apply { addProperty("replacement", "7") }
        val tip = support.getTooltip(diagnostic("O102", "Forward literal load of 'n'", data))
        assertTrue(tip.contains("replaces the highlighted code with"), tip)
        assertTrue(tip.contains("7"), tip)
    }

    @Test
    fun `a replacement is escaped rather than injected into the tooltip markup`() {
        val data = JsonObject().apply { addProperty("replacement", "<b>&x</b>") }
        val tip = support.getTooltip(diagnostic("O102", "m", data))
        assertTrue(tip.contains("&lt;b&gt;"), tip)
        assertTrue(!tip.contains("<b>&x"), tip)
    }
}
