import org.jetbrains.kotlin.gradle.dsl.JvmDefaultMode

plugins {
    id("org.jetbrains.intellij.platform") version "2.19.0"
    kotlin("jvm") version "2.4.21"
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
    compilerOptions {
        // The supported IDEs provide JVM default methods; inherit them directly.
        jvmDefault.set(JvmDefaultMode.NO_COMPATIBILITY)
    }
}

dependencies {
    testImplementation(kotlin("test"))
    // IntelliJ's test runner needs JUnit 4's TestRule even for JUnit 5 suites.
    testRuntimeOnly("junit:junit:4.13.2")
    intellijPlatform {
        // Compile against the 2025.3 support floor. The running IDE supplies
        // its LSP capabilities; this plugin supplies the customisers.
        // JCEF is part of this SDK's platform. plugin.xml declares the optional
        // JCEF dependency used by IDEs that package it as a bundled plugin.
        intellijIdeaUltimate("2025.3")
        bundledPlugin("org.jetbrains.plugins.textmate")

        pluginVerifier()
        zipSigner()
    }
}

tasks.test {
    useJUnitPlatform()
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
              <li>Supports Tcl 8.4&ndash;9.1, F5 BIG-IP iRules, F5 iApps, and EDA tooling dialects</li>
            </ul>
        """.trimIndent()

        ideaVersion {
            sinceBuild = "253"
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
            // Cover the support floor, current stable and EAP APIs, and CLion's
            // bundled Web Browser dependency. Keep a >=2026.1 target for the
            // LspClient API and a 263 target for LSP4J 1.0 diagnostic messages.
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2025.3.6.1")
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2026.2.3")
            // LSP4J 1.0 changes Diagnostic.getMessage's return type in 2026.3.
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "263.7031.23")
            // This exact product/version is where JCEF is isolated behind
            // the bundled Web Browser plugin.
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.CLion, "2026.2.3.1")
        }
    }

    publishing {
        token = providers.environmentVariable("JETBRAINS_TOKEN")
        // Release channel — same odd/even-minor convention as the VS Code
        // pre-release track (scripts/release/prerelease.sh is the single
        // source of truth).  The Makefile `publish-jetbrains` target exports
        // JETBRAINS_CHANNEL from that script: "eap" for a pre-release
        // (odd-minor 2.x) build, empty for a stable one.  The Gradle plugin
        // treats "default" as the public Stable channel; a named channel
        // (e.g. "eap") is only visible to users who add the custom repository
        // URL https://plugins.jetbrains.com/plugins/<channel>/list.
        val channel = System.getenv("JETBRAINS_CHANNEL").orEmpty()
        channels = if (channel.isNotBlank()) listOf(channel) else listOf("default")
    }
}

tasks {
    buildPlugin {
        archiveBaseName.set("tcl-lsp-jetbrains")
    }

    // Drop one bundled native LSP server binary per platform under
    // ``server/<platform>-<arch>/`` at the plugin root (next to ``lib/``)
    // in the distribution — one universal plugin covering every platform
    // except riscv64 Linux (``SERVER_TARGETS_JETBRAINS`` in the Makefile),
    // the same ``<platform>-<arch>`` naming the VS Code extension uses.
    // Putting it inside ``src/main/resources/`` would bundle it into the
    // plugin jar, where it can't be spawned via a ``jar:file:...!/...`` URL
    // and we'd be forced to extract to ``${tmpdir}`` at runtime (with a
    // cache-invalidation dance on plugin upgrades — see
    // ``TclLspServerDescriptor.findBundledServer``).  Same pattern
    // JetBrains' own Prisma ORM plugin uses to ship its native
    // ``prisma-language-server`` binaries.  ``make build-editor-jetbrains``
    // stages the whole ``server/`` tree here.
    prepareSandbox {
        from(layout.projectDirectory.dir("server")) {
            into(pluginName.map { "$it/server" })
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
