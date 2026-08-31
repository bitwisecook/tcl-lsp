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
}
