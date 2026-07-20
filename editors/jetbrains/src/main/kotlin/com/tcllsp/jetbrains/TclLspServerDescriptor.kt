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

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.SystemInfo
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
import com.intellij.util.system.CpuArch
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.eclipse.lsp4j.ConfigurationItem
import java.io.File
import java.net.JarURLConnection
import java.nio.file.Paths

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.TclLspServerDescriptor")

class TclLspServerDescriptor(project: Project) :
    ProjectWideLspServerDescriptor(project, "Tcl Language Server") {

    override fun isSupportedFile(file: VirtualFile): Boolean =
        TclFileType.isSupported(file)

    override fun createCommandLine(): GeneralCommandLine {
        val settings = TclLspSettings.getInstance()

        // Dev mode: explicit serverPath pointing to a tcl-lsp checkout root
        // (or directly at a built native binary).  Mirrors the VS Code
        // extension, which locates a ``cargo build``-produced
        // ``tcl-lsp-server`` under ``target/{release,debug}/``.
        val serverPath = settings.serverPath.trim()
        if (serverPath.isNotEmpty()) {
            val devBinary = findDevServer(serverPath)
            if (devBinary == null) {
                notifyError(
                    "Tcl LSP: no native ${bundledServerName()} found under server path " +
                    "'$serverPath'. Build it with `cargo build -p tcl-lsp-server` " +
                    "(or `make rust-server`), or point the server path directly at the binary."
                )
                throw IllegalStateException("No dev tcl-lsp-server binary under: $serverPath")
            }

            if (!SystemInfo.isWindows) {
                val binary = File(devBinary)
                if (!binary.canExecute() && !binary.setExecutable(true)) {
                    LOG.warn("Failed to mark dev server executable: $devBinary")
                }
            }

            LOG.info("Dev mode: native server $devBinary")
            return GeneralCommandLine(devBinary)
                .withWorkDirectory(devBinary.substringBeforeLast(File.separator))
                .withCharset(Charsets.UTF_8)
        }

        // Production mode: launch the bundled native LSP server binary
        // directly over stdio (no interpreter, no arguments).
        val serverBinary = findBundledServer()
        if (serverBinary == null) {
            notifyError(
                "Tcl LSP: bundled server (${bundledServerName()}) not found for " +
                "platform ${bundledPlatformDir()}. Set the server path in " +
                "Settings > Tools > Tcl Language Server."
            )
            throw IllegalStateException("Bundled ${bundledServerName()} not found for ${bundledPlatformDir()}")
        }

        // JetBrains extracts plugin files without preserving the +x bit, so
        // ensure the native binary is executable before we spawn it on unix.
        if (!SystemInfo.isWindows) {
            val binary = File(serverBinary)
            if (!binary.canExecute() && !binary.setExecutable(true)) {
                LOG.warn("Failed to mark bundled server executable: $serverBinary")
            }
        }

        LOG.info("Production mode: native server $serverBinary")
        return GeneralCommandLine(serverBinary)
            .withWorkDirectory(serverBinary.substringBeforeLast(File.separator))
            .withCharset(Charsets.UTF_8)
    }

    override fun getWorkspaceConfiguration(item: ConfigurationItem): Any? {
        if (item.section == "tclLsp") {
            return TclLspSettings.getInstance().toServerSettings()
        }
        return super.getWorkspaceConfiguration(item)
    }

    // Name of the bundled native server binary for the current platform.
    private fun bundledServerName(): String =
        if (SystemInfo.isWindows) "tcl-lsp-server.exe" else "tcl-lsp-server"

    // Plugin bundle directory for the current platform, e.g. `darwin-arm64`,
    // `linux-x64`, `win32-arm64` — matches SERVER_TARGET_MAP's bundle-dir
    // naming (and the VS Code extension's `bundlePlatformDir()`) so both
    // editors describe platforms the same way. The plugin is universal
    // across every platform except riscv64 Linux (no official JetBrains IDE
    // build targets it), so CpuArch's x86/ARM distinction is sufficient.
    private fun bundledPlatformDir(): String {
        val os = when {
            SystemInfo.isWindows -> "win32"
            SystemInfo.isMac -> "darwin"
            else -> "linux"
        }
        val arch = if (CpuArch.isArm64()) "arm64" else "x64"
        return "$os-$arch"
    }

    // Locate a dev-checkout native server binary from the configured
    // ``serverPath``.  The path may point directly at the binary, or at a
    // checkout root under which we probe ``target/{release,debug}/`` for a
    // ``cargo build``-produced ``tcl-lsp-server`` (mirrors the VS Code
    // extension's ``resolveRustServer`` logic).
    private fun findDevServer(serverPath: String): String? {
        val name = bundledServerName()
        val direct = File(serverPath)
        // Direct path to the binary itself.
        if (direct.isFile) return direct.absolutePath
        if (direct.isDirectory) {
            // A `target/{release,debug}/` layout under the checkout root.
            for (profile in listOf("release", "debug")) {
                val candidate = File(direct, "target/$profile/$name")
                if (candidate.isFile) return candidate.absolutePath
            }
            // Or the binary sitting directly in the given directory.
            val here = File(direct, name)
            if (here.isFile) return here.absolutePath
        }
        return null
    }

    private fun findBundledServer(): String? {
        // ``build.gradle.kts``'s ``prepareSandbox`` task copies one bundled
        // native LSP server binary per platform to
        // ``server/<platform>-<arch>/tcl-lsp-server`` under the plugin
        // install root (next to ``lib/``), so we can execute the one
        // matching this machine directly from the install directory.  We
        // deliberately avoid putting it inside the plugin jar
        // (``src/main/resources/``) because an executable can't be spawned
        // from a ``jar:file:...!/...`` URL and we'd have to extract on first
        // use, then re-extract on every plugin upgrade (the bug fixed in PR
        // #448).  Pattern matches JetBrains' own Prisma ORM plugin which
        // ships its native ``prisma-language-server`` binaries the same way.
        val pluginDir = findPluginInstallDir() ?: return null
        val name = bundledServerName()
        val platformServer = File(pluginDir, "server/${bundledPlatformDir()}/$name")
        if (platformServer.exists()) return platformServer.absolutePath
        // Defensive fallbacks: tolerate a flat server/<name> (pre-universal
        // single-platform layout) or an install that drops the binary at the
        // plugin root or inside ``lib/``.  Shouldn't happen with the current
        // build but keeps a user's working install working if anyone changes
        // ``prepareSandbox``.
        val flatServer = File(pluginDir, "server/$name")
        if (flatServer.exists()) return flatServer.absolutePath
        val rootServer = File(pluginDir, name)
        if (rootServer.exists()) return rootServer.absolutePath
        val libServer = File(pluginDir, "lib/$name")
        if (libServer.exists()) return libServer.absolutePath
        return null
    }

    private fun findPluginInstallDir(): File? {
        // Locate the jar containing this class, then walk up to the plugin
        // root (parent of ``lib/``).  Go through ``JarURLConnection`` →
        // ``URI`` → ``Path`` rather than parsing ``classResource.path``
        // directly — URLs are percent-encoded, so on macOS the raw path
        // string contains ``Application%20Support`` and ``Tcl%20Language%20Support``
        // and ``File(path)`` resolves to a non-existent directory, leaving
        // the user with a "bundled server not found" error.  ``Paths.get(URI)``
        // handles the decoding correctly (Codex review on PR #448).
        val classResource = this::class.java.getResource("/${this::class.java.name.replace('.', '/')}.class")
            ?: return null
        return try {
            val conn = classResource.openConnection() as? JarURLConnection ?: return null
            val jarFile = Paths.get(conn.jarFileURL.toURI()).toFile()
            jarFile.parentFile?.parentFile
        } catch (e: Exception) {
            LOG.warn("Failed to locate plugin install directory", e)
            null
        }
    }

    private fun notifyError(message: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup("Tcl LSP")
            .createNotification(message, NotificationType.ERROR)
            .notify(project)
    }
}
