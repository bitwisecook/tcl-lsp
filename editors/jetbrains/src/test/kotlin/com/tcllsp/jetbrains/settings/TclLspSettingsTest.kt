package com.tcllsp.jetbrains.settings

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse

class TclLspSettingsTest {
    @Test
    fun unconfiguredSignatureSuppressionInheritsIni() {
        val settings = TclLspSettings().toServerSettings()

        assertFalse(settings.containsKey("signatureHelp"))
    }

    @Test
    fun sourceEncodingReachesTheServerAsTheDeclaredName() {
        val settings = TclLspSettings()
        assertEquals("auto", settings.toServerSettings()["sourceEncoding"])
        settings.sourceEncoding = "iso8859-1"
        assertEquals("iso8859-1", settings.toServerSettings()["sourceEncoding"])
    }

    @Test
    fun configuredSignatureSuppressionPreservesExplicitEmptyAndCommands() {
        val settings = TclLspSettings()
        settings.signatureHelpDisabledCommands = " set, ::incr "
        assertEquals(
            mapOf("disabledCommands" to listOf("set", "::incr")),
            settings.toServerSettings()["signatureHelp"],
        )

        settings.signatureHelpDisabledCommands = ""
        assertEquals(
            mapOf("disabledCommands" to emptyList<String>()),
            settings.toServerSettings()["signatureHelp"],
        )
    }

    @Test
    fun signatureSuppressionOverrideCanReturnToIniInheritance() {
        val settings = TclLspSettings()
        settings.signatureHelpDisabledCommands = signatureHelpDisabledCommandsOverride(
            inheritFromIni = false,
            editorValue = "set,incr",
        )
        assertEquals(
            mapOf("disabledCommands" to listOf("set", "incr")),
            settings.toServerSettings()["signatureHelp"],
        )

        settings.signatureHelpDisabledCommands = signatureHelpDisabledCommandsOverride(
            inheritFromIni = true,
            editorValue = "",
        )
        assertFalse(settings.toServerSettings().containsKey("signatureHelp"))
        assertEquals(
            "",
            signatureHelpDisabledCommandsOverride(inheritFromIni = false, editorValue = ""),
            "an enabled empty override remains distinct from inherited null",
        )
    }
}
