// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * The JCEF adapter never ran against a real page: the compiler explorer HTML
 * was missing from every published plugin, so the tool window always fell back
 * to its placeholder. These pin the two rewrites it performs against the shape
 * the VS Code build actually emits — a bare `<head>` and a
 * `Content-Security-Policy` meta whose attributes span two lines.
 */
class CompilerExplorerHtmlTest {

    private val shippedShape = """
        <!DOCTYPE html>
        <html lang="en">
        <head>
        <meta charset="utf-8">
        <meta http-equiv="Content-Security-Policy"
          content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-123' data:;">
        <title>Tcl Compiler Explorer</title>
        </head>
        <body>
        <script>var __vscodeApi = acquireVsCodeApi();</script>
        </body>
        </html>
    """.trimIndent()

    @Test
    fun theBridgeShimIsInjectedBeforeAnyScriptThatCapturesIt() {
        val adapted = adaptHtmlForJcef(shippedShape)

        val shim = adapted.indexOf("window.acquireVsCodeApi = function()")
        val capture = adapted.indexOf("var __vscodeApi = acquireVsCodeApi()")
        assertTrue(shim > 0, "the shim must be injected")
        assertTrue(
            shim < capture,
            "the shim must run before the page captures the host bridge",
        )
        assertContains(adapted, "window.__tcllspFlushQueue")
    }

    @Test
    fun theContentSecurityPolicyThatWouldBlockTheShimIsStripped() {
        val adapted = adaptHtmlForJcef(shippedShape)

        assertFalse(adapted.contains("Content-Security-Policy", ignoreCase = true))
        assertContains(adapted, "<!-- CSP removed for JCEF -->")
    }

    @Test
    fun aHeadTagCarryingAttributesIsStillFound() {
        val adapted = adaptHtmlForJcef("<html><HEAD data-x='1'><title>t</title></head><body></body></html>")

        assertContains(adapted, "window.acquireVsCodeApi = function()")
    }

    /**
     * A template with no `<head>` is a broken upstream build, and the adapter
     * reports it through `LOG.error` — which the platform test logger turns
     * into a throw. That the failure is loud rather than a silent no-op is the
     * behaviour worth having; asserting on it here would pin the test logger,
     * not the adapter.
     */
}
