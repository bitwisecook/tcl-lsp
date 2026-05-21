package com.tcllsp.jetbrains

import com.intellij.openapi.components.Service
import com.intellij.openapi.vfs.VirtualFile

/**
 * Project-level handle to the live Compiler Explorer tool-window panel.
 *
 * The panel registers itself on creation so actions (e.g. the editor
 * "Open In Tcl Compiler Explorer" entry) can push a specific file into the
 * already-loaded webview without reaching into the tool-window internals.
 */
@Service(Service.Level.PROJECT)
class CompilerExplorerService {

    @Volatile
    private var panel: CompilerExplorerPanel? = null

    internal fun register(panel: CompilerExplorerPanel) {
        this.panel = panel
    }

    /** Push the given file's source into the explorer, if the panel is live. */
    fun pushFile(file: VirtualFile) {
        panel?.pushFile(file)
    }
}
