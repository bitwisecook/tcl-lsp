package com.tcllsp.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
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

        // Dev mode: explicit serverPath pointing to the tcl-lsp project root
        val serverPath = settings.serverPath.trim()
        if (serverPath.isNotEmpty()) {
            val serverDir = File(serverPath)
            if (!serverDir.isDirectory) {
                notifyError("Tcl LSP: server path '$serverPath' is not a valid directory.")
                throw IllegalStateException("Invalid server path: $serverPath")
            }

            LOG.info("Dev mode: using uv in $serverPath")
            return GeneralCommandLine("uv", "run", "--directory", serverPath, "--no-dev", "python", "-m", "server")
                .withWorkDirectory(serverPath)
                .withCharset(Charsets.UTF_8)
        }

        // Production mode: use bundled .pyz with discovered Python
        val pyzPath = findBundledPyz()
        if (pyzPath == null) {
            notifyError(
                "Tcl LSP: bundled server (tcl-lsp-server.pyz) not found. " +
                "Set the server path in Settings > Tools > Tcl Language Server."
            )
            throw IllegalStateException("Bundled tcl-lsp-server.pyz not found")
        }

        val python = discoverPython(settings.pythonPath)
        if (python == null) {
            val msg = if (settings.pythonPath.isNotBlank() && settings.pythonPath != "auto") {
                "Tcl LSP: configured Python '${settings.pythonPath}' not found or below 3.10."
            } else {
                "Tcl LSP: Python 3.10+ is required but was not found. " +
                "The plugin bundles all Python dependencies, but a Python interpreter must be installed on your system. " +
                "Install from https://www.python.org/downloads/ or via Homebrew (brew install python@3.14), " +
                "then set the path in Settings > Tools > Tcl Language Server. " +
                "See https://github.com/bitwisecook/tcl-lsp/blob/main/INSTALL.md#python-prerequisite"
            }
            notifyError(msg)
            throw IllegalStateException(msg)
        }

        LOG.info("Production mode: ${describeInterpreter(python.path)} $pyzPath")
        return GeneralCommandLine(python.path, pyzPath)
            .withWorkDirectory(pyzPath.substringBeforeLast(File.separator))
            .withCharset(Charsets.UTF_8)
    }

    override fun getWorkspaceConfiguration(item: ConfigurationItem): Any? {
        if (item.section == "tclLsp") {
            return TclLspSettings.getInstance().toServerSettings()
        }
        return super.getWorkspaceConfiguration(item)
    }

    private fun findBundledPyz(): String? {
        // ``build.gradle.kts``'s ``prepareSandbox`` task copies the bundled
        // LSP server to ``<plugin>/tcl-lsp-server.pyz`` — at the plugin
        // root, next to ``lib/`` — so Python can execute it directly from
        // the install directory.  We deliberately avoid putting it inside
        // the plugin jar (``src/main/resources/``) because Python can't
        // run a zipapp from a ``jar:file:...!/...`` URL and we'd have to
        // extract on first use, then re-extract on every plugin upgrade
        // (the bug fixed 2026-05).  Pattern matches JetBrains' own Prisma
        // ORM plugin which ships ``prisma-language-server.js`` the same way.
        val pluginDir = findPluginInstallDir() ?: return null
        val pyz = File(pluginDir, "tcl-lsp-server.pyz")
        if (pyz.exists()) return pyz.absolutePath
        // Defensive: tolerate an install layout that drops the pyz inside
        // ``lib/``.  Shouldn't happen with the current build but keeps a
        // user's working install working if anyone changes ``prepareSandbox``.
        val libPyz = File(pluginDir, "lib/tcl-lsp-server.pyz")
        if (libPyz.exists()) return libPyz.absolutePath
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
