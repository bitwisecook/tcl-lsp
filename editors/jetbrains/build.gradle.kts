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
    testImplementation(kotlin("test"))
    intellijPlatform {
        // Compile at the 2024.1 support floor. JCEF is part of the platform at
        // this version, so the 2025.3.1+ `intellij.platform.ui.jcef` bundled
        // plugin cannot be added to this compile classpath. plugin.xml carries
        // the optional compatibility dependency used by newer IDEs instead.
        intellijIdeaUltimate("2024.1")
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
            // Verify against the sinceBuild floor and a spread of newer
            // majors. The 2026.1 entry is load-bearing: 2026.1 is where the
            // deprecated LspServer/LspServerManager/LspServerDescriptor API was
            // superseded by LspClient*, and where `sendRequestSync` moved up to
            // the LspClient super-interface. Without a >=2026.1 target the
            // verifier cannot catch the `LspServer.sendRequestSync$default`
            // class of binary incompatibility (see the jetbrains-plugin-compat
            // skill). Keep the newest verified stable major here as JetBrains
            // ships it.
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2024.1")
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2025.1.7.1")
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2025.2.6.2")
            // First release whose core plugin advertises the JCEF dependency
            // alias used to bridge the pre- and post-extraction layouts.
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2025.3.1")
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.IntellijIdeaUltimate, "2026.2.1")
            // #1780 was reported against this exact product/version, where
            // JCEF is isolated behind the bundled Web Browser plugin.
            create(org.jetbrains.intellij.platform.gradle.IntelliJPlatformType.CLion, "2026.2.2")
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
