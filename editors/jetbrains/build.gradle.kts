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

plugins {
    id("org.jetbrains.intellij.platform") version "2.18.1"
    kotlin("jvm") version "2.4.10"
}

group = "com.tcllsp"
// Version resolution order:
//   1. RELEASE_VERSION env var (set by the Makefile from ``git describe``)
//   2. -PpluginVersion=... command-line override
//   3. pluginVersion in gradle.properties (always ``0.0.0-dev`` in source)
// The source file is therefore never mutated by the release flow.
version = (System.getenv("RELEASE_VERSION")
    ?: providers.gradleProperty("pluginVersion").orNull
    ?: "0.0.0-dev")

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

kotlin {
    jvmToolchain(17)
}

dependencies {
    intellijPlatform {
        intellijIdeaUltimate("2024.1")
        bundledPlugin("org.jetbrains.plugins.textmate")

        pluginVerifier()
        zipSigner()
    }
}

intellijPlatform {
    pluginConfiguration {
        id = "com.tcllsp.jetbrains"
        name = "Tcl Language Support"
        version = project.version.toString()
        description = """
            <p>Tcl language support powered by the tcl-lsp language server.</p>
            <ul>
              <li>Semantic highlighting, diagnostics, and code actions</li>
              <li>Auto-completion for commands, subcommands, variables, and switches</li>
              <li>Hover information with command help and proc signatures</li>
              <li>Go-to-definition, find references, and rename symbol</li>
              <li>Document formatting with configurable style</li>
              <li>Document symbols, workspace symbols, and call hierarchy</li>
              <li>Code folding, inlay hints, and signature help</li>
              <li>Compiler Explorer tool window (IR, CFG, SSA, optimiser, shimmer analysis)</li>
              <li>iRule actions for Mermaid diagrams, Tk previews, F5 XC translation, and BIG-IP cleanup script generation</li>
              <li>Catalogue lookups: list iRule events, known packages, ensemble subcommands; describe a command or event by name</li>
              <li>Supports Tcl 8.4&ndash;9.0, F5 BIG-IP iRules, F5 iApps, and EDA tooling dialects</li>
            </ul>
        """.trimIndent()

        ideaVersion {
            sinceBuild = "241"
            untilBuild = provider { null }
        }

        vendor {
            name = "tcl-lsp"
            url = "https://github.com/bitwisecook/tcl-lsp"
        }

        changeNotes = provider {
            val notes = rootProject.projectDir.resolve("../../RELEASE_NOTES.md")
            if (notes.isFile) markdownToHtml(notes.readText()) else ""
        }
    }

    buildSearchableOptions = false

    pluginVerification {
        freeArgs = listOf("-offline")
        ides {
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2024.1")
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2025.1.7.1")
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2025.2.6.2")
        }
    }

    publishing {
        token = providers.environmentVariable("JETBRAINS_TOKEN")
    }
}

tasks {
    buildPlugin {
        archiveBaseName.set("tcl-lsp-jetbrains")
    }

    // Drop the bundled LSP server pyz at the plugin root (next to ``lib/``)
    // in the distribution so Python can execute it directly from the install
    // directory.  Putting it inside ``src/main/resources/`` would bundle it
    // into the plugin jar, where Python can't reach it via a
    // ``jar:file:...!/...`` URL and we'd be forced to extract to
    // ``${tmpdir}`` at runtime (with a cache-invalidation dance on plugin
    // upgrades — see ``TclLspServerDescriptor.findBundledPyz``).  Same
    // pattern JetBrains' own Prisma ORM plugin uses to ship its
    // ``prisma-language-server.js``.
    prepareSandbox {
        from(layout.projectDirectory.file("server/tcl-lsp-server.pyz")) {
            into(pluginName)
        }
    }
}

// Minimal markdown → HTML for the change-notes block. JetBrains
// Marketplace renders HTML in <change-notes>, and we want to surface
// RELEASE_NOTES.md without pulling in a full markdown library. Covers
// the subset our release notes actually use: H1/H2 headings, bulleted
// lists, bold/italic, and inline code.
fun markdownToHtml(md: String): String {
    val lines = md.lines()
    val out = StringBuilder()
    var inList = false
    fun closeList() { if (inList) { out.append("</ul>\n"); inList = false } }
    for (raw in lines) {
        val line = raw.trimEnd()
        val bullet = Regex("^\\s*[-*]\\s+(.*)").matchEntire(line)
        val h1 = Regex("^# (.*)").matchEntire(line)
        val h2 = Regex("^## (.*)").matchEntire(line)
        val h3 = Regex("^### (.*)").matchEntire(line)
        when {
            h1 != null -> { closeList(); out.append("<h2>").append(inline(h1.groupValues[1])).append("</h2>\n") }
            h2 != null -> { closeList(); out.append("<h3>").append(inline(h2.groupValues[1])).append("</h3>\n") }
            h3 != null -> { closeList(); out.append("<h4>").append(inline(h3.groupValues[1])).append("</h4>\n") }
            bullet != null -> {
                if (!inList) { out.append("<ul>\n"); inList = true }
                out.append("  <li>").append(inline(bullet.groupValues[1])).append("</li>\n")
            }
            line.isBlank() -> closeList()
            else -> { closeList(); out.append("<p>").append(inline(line)).append("</p>\n") }
        }
    }
    closeList()
    return out.toString()
}

fun inline(text: String): String =
    text
        .replace(Regex("`([^`]+)`"), "<code>$1</code>")
        .replace(Regex("\\*\\*([^*]+)\\*\\*"), "<b>$1</b>")
        .replace(Regex("(?<![*])\\*([^*]+)\\*(?![*])"), "<i>$1</i>")
