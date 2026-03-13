plugins {
    id("org.jetbrains.intellij.platform") version "2.12.0"
    kotlin("jvm") version "2.1.20"
}

group = "com.tcllsp"
version = providers.gradleProperty("pluginVersion").get()

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
            Tcl language support with semantic highlighting, diagnostics, completions, and more — powered by tcl-lsp.

            Supports Tcl 8.4–9.0, F5 BIG-IP iRules, F5 iApps, and EDA tooling dialects.
        """.trimIndent()

        ideaVersion {
            sinceBuild = "241"
        }

        vendor {
            name = "tcl-lsp"
            url = "https://github.com/bitwisecook/tcl-lsp"
        }
    }

    buildSearchableOptions = false
}

tasks {
    buildPlugin {
        archiveBaseName.set("tcl-lsp-jetbrains")
    }
}
