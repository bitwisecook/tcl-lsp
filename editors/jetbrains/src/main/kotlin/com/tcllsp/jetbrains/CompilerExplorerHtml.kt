package com.tcllsp.jetbrains

/**
 * Generates HTML for the Compiler Explorer, adapted for JCEF from the VS Code webview.
 *
 * The HTML is loaded from the bundled resource (shared with the VS Code extension build).
 * The VS Code API bridge (`acquireVsCodeApi()` / `vscode.postMessage()`) is replaced with
 * a JCEF bridge (`window.__tcllspBridge()`).
 */
fun getCompilerExplorerHtml(): String {
    // Try loading from bundled resource first (build copies compilerExplorer.html)
    val resource = CompilerExplorerToolWindowFactory::class.java.classLoader
        .getResourceAsStream("compilerExplorer.html")

    if (resource != null) {
        val html = resource.bufferedReader().readText()
        return adaptHtmlForJcef(html)
    }

    // Fallback: minimal placeholder
    return """
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="utf-8">
            <style>
                body { font-family: sans-serif; padding: 20px; color: #cdd6f4; background: #1e1e2e; }
                a { color: #89b4fa; }
            </style>
        </head>
        <body>
            <h2>Tcl Compiler Explorer</h2>
            <p>The compiler explorer HTML resource was not found in the plugin bundle.</p>
            <p>This feature requires the plugin to be built with <code>make jetbrains</code>
            which bundles the compiler explorer from the VS Code extension.</p>
        </body>
        </html>
    """.trimIndent()
}

/**
 * Replace VS Code webview API calls with JCEF bridge equivalents.
 */
private fun adaptHtmlForJcef(html: String): String {
    var result = html

    // Replace acquireVsCodeApi with our bridge object
    result = result.replace(
        "const vscode = acquireVsCodeApi();",
        """
        // JCEF bridge — __tcllspBridge is injected by the Kotlin host
        const vscode = {
            postMessage: function(msg) {
                if (typeof window.__tcllspBridge === 'function') {
                    if (msg.type === 'compile' && msg.source) {
                        window.__tcllspBridge('compile:' + msg.source + '\u0000' + (msg.dialect || ''));
                    } else if (msg.type === 'highlightSource') {
                        window.__tcllspBridge('highlightSource:' + msg.start + ',' + msg.end);
                    } else if (msg.type === 'clearHighlight') {
                        window.__tcllspBridge('clearHighlight');
                    } else if (msg.type === 'dialectChange') {
                        // Dialect changes are handled locally
                    }
                }
            }
        };
        """.trimIndent()
    )

    // Remove Content-Security-Policy that blocks inline scripts in JCEF
    result = result.replace(
        Regex("""<meta\s+http-equiv="Content-Security-Policy"[^>]*>"""),
        "<!-- CSP removed for JCEF -->"
    )

    return result
}
