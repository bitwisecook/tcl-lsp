package com.tcllsp.jetbrains

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.vfs.VirtualFile
import javax.swing.Icon

class TclFileType private constructor() : LanguageFileType(TclLanguage) {
    override fun getName(): String = "Tcl"
    override fun getDescription(): String = "Tcl script file"
    override fun getDefaultExtension(): String = "tcl"
    override fun getIcon(): Icon = TclIcons.Tcl

    companion object {
        @JvmField
        val INSTANCE = TclFileType()

        private val SUPPORTED_EXTENSIONS = setOf(
            "tcl", "tk", "itcl", "tm",
            "iapp", "iappimpl", "impl",
            "irul", "irule"
        )

        @JvmStatic
        fun isSupported(file: VirtualFile): Boolean {
            val ext = file.extension?.lowercase() ?: return false
            return ext in SUPPORTED_EXTENSIONS
        }
    }
}
