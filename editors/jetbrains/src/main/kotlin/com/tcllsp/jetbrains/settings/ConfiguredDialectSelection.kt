// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains.settings

/** Actual configured identity stays distinct from the generated display catalogue. */
internal class ConfiguredDialectSelection private constructor(
    private val identifiers: List<String>,
    val labels: List<String>,
    val selectedIndex: Int,
    private val storedIdentifier: String,
) {
    /** No selection is not an instruction to replace the configured identity. */
    fun identifierAt(index: Int): String = identifiers.getOrNull(index) ?: storedIdentifier

    companion object {
        fun forStored(options: List<Pair<String, String>>, stored: String): ConfiguredDialectSelection {
            val identifiers = options.map { it.first }.toMutableList()
            val labels = options.map { it.second }.toMutableList()
            var selected = identifiers.indexOf(stored)
            if (selected < 0) {
                selected = identifiers.size
                identifiers.add(stored)
                labels.add("Unavailable configured dialect: $stored")
            }
            return ConfiguredDialectSelection(identifiers, labels, selected, stored)
        }
    }
}
