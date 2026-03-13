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
                "Tcl LSP: no Python 3.10+ interpreter found. " +
                "Install Python from python.org or set the Python path in Settings > Tools > Tcl Language Server."
            }
            notifyError(msg)
            throw IllegalStateException(msg)
        }

        LOG.info("Production mode: ${python.path} $pyzPath")
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
            // For jar:// protocol, extract to temp
            try {
                val tempDir = System.getProperty("java.io.tmpdir")
                val tempPyz = File(tempDir, "tcl-lsp-server.pyz")
                if (!tempPyz.exists() || tempPyz.length() == 0L) {
                    pluginClassLoader.getResourceAsStream("tcl-lsp-server.pyz")?.use { input ->
                        tempPyz.outputStream().use { output ->
                            input.copyTo(output)
                        }
                    }
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
