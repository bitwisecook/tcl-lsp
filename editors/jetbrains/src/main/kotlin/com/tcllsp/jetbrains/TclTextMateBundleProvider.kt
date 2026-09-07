// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.intellij.openapi.application.PathManager
import com.intellij.openapi.diagnostic.Logger
import org.jetbrains.plugins.textmate.api.TextMateBundleProvider
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.TclTextMateBundleProvider")

/** The bundle's name in Settings | Editor | TextMate Bundles. */
private const val BUNDLE_NAME = "Tcl"

/**
 * The bundle's files, as classpath resource → path within the bundle
 * directory. `package.json` is what makes `BundleType.detectBundleType` call
 * this a VS Code-style bundle, and its `contributes.grammars[].path` is what
 * ties the manifest to the grammar.
 */
private val BUNDLE_FILES = mapOf(
    "textmate/package.json" to "package.json",
    "syntaxes/tcl.tmLanguage.json" to "syntaxes/tcl.tmLanguage.json",
)

/**
 * Hands the bundled `source.tcl` TextMate grammar to the TextMate plugin.
 *
 * Without this nothing ever registers the grammar, so
 * `TextMateService.getLanguageDescriptorByFileName` finds no descriptor for a
 * `.tcl` file, `TextMateSyntaxHighlighterFactory` falls back to the plain
 * highlighter, and every Tcl file opens as undifferentiated black text — the
 * plugin's `editorHighlighterProvider` registrations on their own only decide
 * *how* a grammar is lexed, never *which* grammar applies.
 *
 * The TextMate service reads bundles from the filesystem, so the two resources
 * are unpacked out of the plugin jar into the IDE's system directory. Contents
 * are compared before writing, so an unchanged bundle costs two reads and the
 * directory survives plugin upgrades without a cache-invalidation dance.
 */
class TclTextMateBundleProvider : TextMateBundleProvider {
    override fun getBundles(): List<TextMateBundleProvider.PluginBundle> {
        val directory = unpackBundle(defaultBundleDirectory()) ?: return emptyList()
        return listOf(TextMateBundleProvider.PluginBundle(BUNDLE_NAME, directory))
    }
}

/** Where the unpacked bundle lives: `<system>/tcl-lsp/textmate/`. */
internal fun defaultBundleDirectory(): Path =
    Path.of(PathManager.getSystemPath(), "tcl-lsp", "textmate")

/**
 * Unpack [BUNDLE_FILES] into [directory], returning it, or `null` when a
 * resource is missing or the directory cannot be written.
 *
 * [readResource] is injectable so a test can drive the same unpack the
 * provider performs.
 */
internal fun unpackBundle(
    directory: Path,
    readResource: (String) -> ByteArray? = ::bundleResource,
): Path? =
    try {
        for ((resource, relative) in BUNDLE_FILES) {
            val bytes = readResource(resource)
            if (bytes == null) {
                // A mis-packaged plugin. `make verify-jetbrains-resources`
                // fails the build on exactly this, so reaching it means the
                // jar was assembled some other way; warn rather than
                // `LOG.error`, which would abort TextMate service startup
                // for every other bundle too.
                LOG.warn(
                    "Tcl TextMate bundle resource '$resource' is missing from the plugin " +
                        "jar — Tcl files will open without syntax highlighting."
                )
                return null
            }
            val file = directory.resolve(relative)
            Files.createDirectories(file.parent)
            if (!Files.exists(file) || !Files.readAllBytes(file).contentEquals(bytes)) {
                // Two IDEs sharing this system directory can unpack at once
                // after an upgrade. Write beside the target and move it into
                // place, so a concurrent reader never sees a truncated
                // grammar (which parses as no grammar at all).
                val staged = Files.createTempFile(file.parent, ".${file.fileName}", ".tmp")
                try {
                    Files.write(staged, bytes)
                    Files.move(staged, file, StandardCopyOption.REPLACE_EXISTING)
                } finally {
                    Files.deleteIfExists(staged)
                }
            }
        }
        directory
    } catch (e: Exception) {
        LOG.warn("Failed to unpack the Tcl TextMate bundle into $directory", e)
        null
    }

private fun bundleResource(path: String): ByteArray? =
    TclTextMateBundleProvider::class.java.classLoader
        .getResourceAsStream(path)
        ?.use { it.readBytes() }
