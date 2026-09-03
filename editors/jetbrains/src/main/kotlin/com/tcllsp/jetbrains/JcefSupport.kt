// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

private const val JBCEF_APP_CLASS = "com.intellij.ui.jcef.JBCefApp"

/**
 * Whether this IDE can construct a JCEF browser.
 *
 * JCEF was platform-owned through 2025.3.0, then exposed through the
 * `com.intellij.modules.jcef` dependency alias before moving behind that
 * plugin's classloader in 2026.2.  A source-level JBCefApp reference would
 * itself throw NoClassDefFoundError when that optional plugin is unavailable,
 * so probe reflectively before either tool window touches a JCEF type.
 */
internal fun isJcefSupported(
    classLoader: ClassLoader = JcefSupportMarker::class.java.classLoader,
): Boolean = try {
    val appClass = Class.forName(JBCEF_APP_CLASS, false, classLoader)
    appClass.getMethod("isSupported").invoke(null) == true
} catch (_: ReflectiveOperationException) {
    false
} catch (_: LinkageError) {
    false
} catch (_: SecurityException) {
    false
}

private object JcefSupportMarker
