package com.tcllsp.jetbrains.settings

import com.intellij.openapi.options.Configurable
import javax.swing.JComponent

class TclLspSettingsConfigurable : Configurable {
    private var panel: TclLspSettingsPanel? = null

    override fun getDisplayName(): String = "Tcl Language Server"

    override fun createComponent(): JComponent {
        val p = TclLspSettingsPanel()
        panel = p
        return p.root
    }

    override fun isModified(): Boolean = panel?.isModified() == true

    override fun apply() {
        panel?.apply()
    }

    override fun reset() {
        panel?.reset()
    }

    override fun disposeUIResources() {
        panel = null
    }
}
