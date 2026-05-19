package com.tcllsp.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.extensions.PluginId
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
import com.tcllsp.jetbrains.settings.TclLspSettings
import org.eclipse.lsp4j.ConfigurationItem
import java.io.File
import java.nio.file.Path

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
        // Look for the .pyz in the plugin's resources / lib directory
        val pluginClassLoader = this::class.java.classLoader
        val resourceUrl = pluginClassLoader.getResource("tcl-lsp-server.pyz")
        if (resourceUrl != null) {
            // If running from a jar, the resource is inside the jar.
            // We need to extract it to a temp location.
            val protocol = resourceUrl.protocol
            if (protocol == "file") {
                val path = Path.of(resourceUrl.toURI()).toString()
                if (File(path).exists()) return path
            }
            // For jar:// protocol, extract to temp.  Include the plugin
            // version in the filename so plugin upgrades pick up the new
            // bundled pyz instead of reusing the previous version's cached
            // extract — the size-only existence check ran the previous
            // version's server even after the user upgraded (reported
            // 2026-05-19: W105 quick-fix still produced ``{script}`` from
            // ``$script`` in v1.10.6 because the v1.10.5 pyz was reused).
            try {
                val tempDir = System.getProperty("java.io.tmpdir")
                val pluginVersion = currentPluginVersion()
                val tempPyz = File(tempDir, "tcl-lsp-server-$pluginVersion.pyz")
                if (!tempPyz.exists() || tempPyz.length() == 0L) {
                    pluginClassLoader.getResourceAsStream("tcl-lsp-server.pyz")?.use { input ->
                        tempPyz.outputStream().use { output ->
                            input.copyTo(output)
                        }
                    }
                    cleanStaleExtractedPyzs(File(tempDir), tempPyz)
                }
                if (tempPyz.exists() && tempPyz.length() > 0) {
                    return tempPyz.absolutePath
                }
            } catch (e: Exception) {
                LOG.warn("Failed to extract bundled pyz", e)
            }
        }

        // Also check in the plugin's installation directory
        val pluginDir = findPluginInstallDir()
        if (pluginDir != null) {
            val pyz = File(pluginDir, "tcl-lsp-server.pyz")
            if (pyz.exists()) return pyz.absolutePath

            // Also check lib/ subdirectory
            val libPyz = File(pluginDir, "lib/tcl-lsp-server.pyz")
            if (libPyz.exists()) return libPyz.absolutePath
        }

        return null
    }

    private fun currentPluginVersion(): String {
        val descriptor = PluginManagerCore.getPlugin(PluginId.getId("com.tcllsp.jetbrains"))
        val raw = descriptor?.version
        // Sanitise so the version can safely live in a filename on any OS.
        // Replace anything that isn't a letter, digit, ``.``, ``-``, or ``_``.
        return raw?.replace(Regex("[^A-Za-z0-9._-]"), "_")?.takeIf { it.isNotEmpty() } ?: "unknown"
    }

    private fun cleanStaleExtractedPyzs(tempDir: File, keep: File) {
        // Remove ``tcl-lsp-server-*.pyz`` files left over from previous plugin
        // versions so /tmp doesn't accumulate stale extracts.  Best-effort:
        // failures are silently ignored (locked file, permission denied).
        try {
            val matches = tempDir.listFiles { f ->
                val name = f.name
                f.isFile &&
                    name.startsWith("tcl-lsp-server-") &&
                    name.endsWith(".pyz") &&
                    name != keep.name
            } ?: return
            for (f in matches) {
                runCatching { f.delete() }
            }
        } catch (e: Exception) {
            LOG.debug("Failed to clean stale pyz extracts", e)
        }
    }

    private fun findPluginInstallDir(): File? {
        // Try to determine the plugin installation directory from the classloader
        val classResource = this::class.java.getResource("/${this::class.java.name.replace('.', '/')}.class")
            ?: return null
        val path = classResource.path
        // jar:file:/path/to/plugin/lib/plugin.jar!/com/...
        val jarPrefix = "file:"
        val jarSuffix = "!"
        val jarIdx = path.indexOf(jarSuffix)
        if (jarIdx > 0) {
            val jarPath = path.substring(
                if (path.startsWith(jarPrefix)) jarPrefix.length else 0,
                jarIdx
            )
            val jarFile = File(jarPath)
            // Plugin dir is typically parent of lib/
            return jarFile.parentFile?.parentFile
        }
        return null
    }

    private fun notifyError(message: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup("Tcl LSP")
            .createNotification(message, NotificationType.ERROR)
            .notify(project)
    }
}
