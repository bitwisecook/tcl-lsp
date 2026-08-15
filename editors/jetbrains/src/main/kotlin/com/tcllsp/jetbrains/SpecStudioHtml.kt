// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.google.gson.Gson

internal fun getSpecStudioHtml(): String {
    val loader = SpecStudioToolWindowFactory::class.java.classLoader
    val html = loader.getResourceAsStream("spec-studio/index.html")
        ?.bufferedReader()?.use { it.readText() }
        ?: return "<html><body>Spec Studio assets are missing from this plugin build.</body></html>"
    val nativeModule = loader.getResourceAsStream("spec-studio/assets/native-editor-host.js")
        ?.bufferedReader()?.use { it.readText() }
        ?: return "<html><body>Spec Studio native editor bridge is missing.</body></html>"
    val source = Gson().toJson(nativeModule).replace("<", "\\u003c")
    val bridge = """
        <script>
        window.__tclSpecStudioQueue=[];
        window.__tclSpecStudioHost={postMessage:function(message){
          var json=JSON.stringify(message);
          if(window.__tclSpecStudioBridge){window.__tclSpecStudioBridge(json);}
          else{window.__tclSpecStudioQueue.push(json);}
        }};
        window.__tclSpecStudioNativeModuleUrl=URL.createObjectURL(new Blob([$source],{type:'text/javascript'}));
        </script>
    """.trimIndent()
    return html
        .replace("script-src 'self'", "script-src 'self' blob:")
        .replace("<head>", "<head>$bridge")
}
