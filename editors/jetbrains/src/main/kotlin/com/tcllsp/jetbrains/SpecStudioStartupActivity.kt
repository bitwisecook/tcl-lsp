// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.StartupActivity
import java.nio.file.Files
import java.nio.file.Path
import java.util.Comparator

/** Cleans crash-abandoned Studio packs before the project LSP is used. */
class SpecStudioStartupActivity : StartupActivity.DumbAware {
    override fun runActivity(project: Project) {
        val base = project.basePath ?: return
        val root = Path.of(base, ".tcl-lsp", ".spec-studio")
        val abandoned = root.resolve("jetbrains")
        try {
            if (Files.exists(abandoned)) {
                Files.walk(abandoned).use { paths ->
                    paths.sorted(Comparator.reverseOrder()).forEach(Files::deleteIfExists)
                }
            }
            Files.createDirectories(root)
            // The ignore file intentionally ignores itself as well. Git still
            // reads it, but neither it nor any live projection enters status.
            Files.writeString(root.resolve(".gitignore"), "*\n")
        } catch (error: Exception) {
            LOG.warn("Could not clean abandoned Spec Studio session", error)
        }
    }

    private companion object {
        val LOG = Logger.getInstance(SpecStudioStartupActivity::class.java)
    }
}
