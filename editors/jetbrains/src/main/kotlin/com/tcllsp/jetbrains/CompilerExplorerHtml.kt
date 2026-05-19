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
 *
 * The VS Code HTML calls `acquireVsCodeApi()` early during body-script
 * evaluation to capture the host bridge, then keeps that reference for the
 * lifetime of the page (`var __vscodeApi = ...` near the top of the first
 * script block, plus a second `vscodeApi` capture later on). In JCEF there
 * is no native `acquireVsCodeApi` — the host's own bridge
 * (`window.__tcllspBridge`) is injected from Kotlin in `onLoadEnd`, which
 * fires *after* those captures have already run.
 *
 * To plug that gap we inject a shim into `<head>` so it runs before any
 * body script. The shim defines `acquireVsCodeApi` to hand back an object
 * whose `postMessage` either forwards to `__tcllspBridge` (if already set)
 * or queues the message; `window.__tcllspFlushQueue()` drains the queue
 * once the host calls it on load-end. This keeps the captured `vscodeApi`
 * reference live, makes the status line read "explorer UI ready" instead
 * of "webview API unavailable", and ensures the `compile` request reaches
 * Kotlin so the IR pane stops showing "Waiting for source from editor...".
 */
private fun adaptHtmlForJcef(html: String): String {
    var result = html

    val shim = """
        <script>
        (function() {
            var queue = [];
            function dispatch(msg) {
                if (typeof window.__tcllspBridge !== 'function') {
                    queue.push(msg);
                    return;
                }
                try {
                    if (!msg || typeof msg.type !== 'string') return;
                    if (msg.type === 'compile' && typeof msg.source === 'string') {
                        window.__tcllspBridge('compile:' + msg.source + '\u0000' + (msg.dialect || ''));
                    } else if (msg.type === 'highlightSource') {
                        window.__tcllspBridge('highlightSource:' + msg.start + ',' + msg.end);
                    } else if (msg.type === 'clearHighlight') {
                        window.__tcllspBridge('clearHighlight');
                    }
                    // Other types (ready, dialectChange, scriptError, ...)
                    // have no host action in JCEF and are intentionally dropped.
                } catch (e) {
                    // Bridge errors must not break the webview UI.
                }
            }
            window.__tcllspFlushQueue = function() {
                while (queue.length) dispatch(queue.shift());
            };
            window.acquireVsCodeApi = function() {
                return { postMessage: dispatch };
            };
        })();
        </script>
    """.trimIndent()

    result = result.replace("<head>", "<head>\n$shim")

    // Remove Content-Security-Policy that blocks inline scripts in JCEF
    result = result.replace(
        Regex("""<meta\s+http-equiv="Content-Security-Policy"[^>]*>"""),
        "<!-- CSP removed for JCEF -->"
    )

    return result
}
