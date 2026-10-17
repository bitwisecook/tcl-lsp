// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains.settings

import kotlin.test.Test
import kotlin.test.assertEquals

class ConfiguredDialectSelectionTest {
    private val options = listOf("bpf" to "BPF", "tcl8.6" to "Tcl 8.6")

    @Test
    fun unknownConfiguredIdentitySurvivesResetAndUnrelatedApply() {
        val selection = ConfiguredDialectSelection.forStored(options, "custom-dialect")
        assertEquals("custom-dialect", selection.identifierAt(selection.selectedIndex))
        assertEquals("custom-dialect", selection.identifierAt(-1))
        assertEquals("custom-dialect", selection.identifierAt(100))
        assertEquals("Unavailable configured dialect: custom-dialect", selection.labels.last())
    }

    @Test
    fun onlyActualSelectedCatalogueEntryReplacesUnknownIdentity() {
        val selection = ConfiguredDialectSelection.forStored(options, "custom-dialect")
        assertEquals("tcl8.6", selection.identifierAt(1))
        assertEquals("bpf", selection.identifierAt(0))
        val known = ConfiguredDialectSelection.forStored(options, "tcl8.6")
        assertEquals(1, known.selectedIndex)
        assertEquals(listOf("BPF", "Tcl 8.6"), known.labels)
    }

    @Test
    fun configuredIdentifierIsNotTrimmedOrCaseFoldedIntoAnotherDialect() {
        for (stored in listOf("TCL8.6", " tcl8.6 ", "")) {
            val selection = ConfiguredDialectSelection.forStored(options, stored)
            assertEquals(stored, selection.identifierAt(selection.selectedIndex))
        }
    }
}
