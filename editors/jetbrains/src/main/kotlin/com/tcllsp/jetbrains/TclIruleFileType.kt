package com.tcllsp.jetbrains

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

class TclIruleFileType private constructor() : LanguageFileType(TclLanguage) {
    override fun getName(): String = "iRule"
    override fun getDescription(): String = "F5 BIG-IP iRule file"
    override fun getDefaultExtension(): String = "irul"
    override fun getIcon(): Icon = TclIcons.Tcl

    companion object {
        @JvmField
        val INSTANCE = TclIruleFileType()
    }
}
