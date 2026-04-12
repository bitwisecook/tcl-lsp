package com.tcllsp.jetbrains

import com.intellij.lang.Language

object TclLanguage : Language("Tcl") {
    private fun readResolve(): Any = TclLanguage
}
