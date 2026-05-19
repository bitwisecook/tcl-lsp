package com.tcllsp.jetbrains

import com.intellij.openapi.diagnostic.Logger
import java.io.File
import java.util.concurrent.TimeUnit

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.PythonDiscovery")

data class PythonInfo(
    val path: String,
    val version: String,
    val major: Int,
    val minor: Int,
    val patch: Int,
    val source: String,
)

private const val MIN_PYTHON_MAJOR = 3
private const val MIN_PYTHON_MINOR = 10
private const val MAX_PYTHON_MINOR_SCAN = 15

/**
 * Discover a Python 3.10+ interpreter, matching the VS Code extension's discovery logic.
 * Returns the best (highest version) interpreter found, or null if none available.
 */
fun discoverPython(configured: String = "auto"): PythonInfo? {
    // User specified an explicit path
    if (configured.isNotBlank() && configured != "auto") {
        val info = probePython(configured, source = "configured")
        if (info != null) {
            LOG.info(
                "Python: using configured interpreter: ${describePath(info.path)} (${info.version})"
            )
            return info
        }
        LOG.warn("Python: configured interpreter '$configured' not found or below 3.10")
        return null
    }

    val seen = mutableSetOf<String>()
    val results = mutableListOf<PythonInfo>()

    fun tryCandidate(command: String, source: String) {
        val info = probePython(command, source = source) ?: return
        val key = info.path
        if (key in seen) return
        seen.add(key)
        results.add(info)
    }

    // 1. Versioned PATH binaries (highest first)
    for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
        tryCandidate("python3.$m", "PATH")
    }
    tryCandidate("python3", "PATH")

    val isWindows = System.getProperty("os.name").lowercase().contains("win")
    if (isWindows) {
        tryCandidate("python", "PATH")
    }

    // 2. Well-known locations
    val os = System.getProperty("os.name").lowercase()
    val home = System.getProperty("user.home")

    when {
        os.contains("mac") || os.contains("darwin") -> {
            // Homebrew — Apple Silicon
            for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                tryCandidate("/opt/homebrew/bin/python3.$m", "Homebrew")
            }
            tryCandidate("/opt/homebrew/bin/python3", "Homebrew")
            // Homebrew — Intel
            for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                tryCandidate("/usr/local/bin/python3.$m", "Homebrew/python.org")
            }
            tryCandidate("/usr/local/bin/python3", "Homebrew/python.org")
            // python.org framework
            for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                tryCandidate(
                    "/Library/Frameworks/Python.framework/Versions/3.$m/bin/python3",
                    "python.org"
                )
            }
        }
        os.contains("win") -> {
            // python.org per-user install
            val localAppData = System.getenv("LOCALAPPDATA") ?: ""
            if (localAppData.isNotBlank()) {
                for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                    tryCandidate(
                        "$localAppData\\Programs\\Python\\Python3$m\\python.exe",
                        "python.org"
                    )
                }
            }
            // python.org system-wide
            val progFiles = System.getenv("ProgramFiles") ?: "C:\\Program Files"
            for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                tryCandidate("$progFiles\\Python3$m\\python.exe", "python.org")
            }
        }
        else -> {
            // Linux
            for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                tryCandidate("/usr/bin/python3.$m", "system")
            }
            for (m in MAX_PYTHON_MINOR_SCAN downTo MIN_PYTHON_MINOR) {
                tryCandidate("/usr/local/bin/python3.$m", "local")
            }
        }
    }

    // Anaconda / Miniconda — all platforms
    if (isWindows) {
        for (dir in listOf("Miniconda3", "Anaconda3")) {
            tryCandidate("$home\\$dir\\python.exe", "Anaconda")
        }
    } else {
        for (dir in listOf("miniconda3", "anaconda3")) {
            tryCandidate("$home/$dir/bin/python3", "Anaconda")
        }
    }

    // Sort by version descending
    results.sortWith(compareByDescending<PythonInfo> { it.major }
        .thenByDescending { it.minor }
        .thenByDescending { it.patch })

    if (results.isEmpty()) {
        LOG.warn("Python: no Python 3.10+ interpreter found")
        return null
    }

    val best = results.first()
    LOG.info(
        "Python: discovered ${results.size} interpreters, using " +
        "${describePath(best.path)} (${best.version}, ${best.source})"
    )
    return best
}

/**
 * Format an interpreter path for log messages. For bare PATH names
 * (e.g. ``python3.14``) the actual executable is resolved by
 * ProcessBuilder at spawn time, but for diagnostics it's useful to
 * show which file on disk the name would resolve to. Returns
 * ``"<command> (-> <resolved>)"`` when resolution succeeds, otherwise
 * just the command itself.
 */
internal fun describeInterpreter(command: String): String = describePath(command)

private fun describePath(command: String): String {
    // Anything that looks like a filesystem path (absolute, or contains
    // either platform's separator) is already self-describing.  We treat
    // ``\\`` and ``/`` interchangeably so Windows paths spelled
    // ``C:/Python/python.exe`` aren't misclassified as bare PATH names
    // and re-resolved against PATH for logging (Copilot review, PR #438).
    if (command.contains('/') || command.contains('\\') || File(command).isAbsolute) {
        return command
    }
    val resolved = resolveOnPath(command) ?: return command
    return "$command (-> $resolved)"
}

private fun resolveOnPath(command: String): String? {
    val pathEnv = System.getenv("PATH") ?: return null
    val pathSep = System.getProperty("path.separator") ?: ":"
    // ``os.name.contains("win")`` matches ``"Darwin"`` (macOS), which
    // would prevent PATH resolution on macOS and apply Windows-only
    // ``PATHEXT`` logic to other Unixes.  Match the official prefix
    // instead (Copilot review, PR #438).
    val isWindows = System.getProperty("os.name").lowercase().startsWith("windows")
    val pathExts = if (isWindows) {
        (System.getenv("PATHEXT") ?: ".EXE;.BAT;.CMD;.COM").split(";")
    } else {
        listOf("")
    }
    for (dir in pathEnv.split(pathSep)) {
        if (dir.isBlank()) continue
        for (ext in pathExts) {
            val candidate = File(dir, command + ext)
            if (candidate.isFile && candidate.canExecute()) {
                return try {
                    candidate.canonicalPath
                } catch (_: Exception) {
                    candidate.absolutePath
                }
            }
        }
    }
    return null
}

private val VERSION_REGEX = Regex("""Python\s+(\d+)\.(\d+)\.(\d+)""")

private fun probePython(command: String, source: String): PythonInfo? {
    return try {
        val file = File(command)
        val hasPathSep = command.contains(File.separator)
        // For absolute / relative paths, check existence first.
        if (hasPathSep && !file.exists()) return null

        val process = ProcessBuilder(command, "--version")
            .redirectErrorStream(true)
            .start()

        val completed = process.waitFor(3, TimeUnit.SECONDS)
        if (!completed) {
            process.destroyForcibly()
            return null
        }

        val output = process.inputStream.bufferedReader().readText().trim()
        val match = VERSION_REGEX.find(output) ?: return null
        val major = match.groupValues[1].toInt()
        val minor = match.groupValues[2].toInt()
        val patch = match.groupValues[3].toInt()

        if (major < MIN_PYTHON_MAJOR) return null
        if (major == MIN_PYTHON_MAJOR && minor < MIN_PYTHON_MINOR) return null

        // Only canonicalise when the caller already gave us a real
        // filesystem path. For a bare PATH name like ``python3.14``,
        // ``File(command).canonicalPath`` resolves against ``user.dir``
        // and produces a fake path (e.g. ``/home/jim/python3.14``) that
        // doesn't exist — which the LSP launcher then tries to spawn,
        // breaking the plugin entirely. Pass bare names through and let
        // ProcessBuilder resolve them via PATH at spawn time.
        val resolvedPath = if (hasPathSep) {
            try { file.canonicalPath } catch (_: Exception) { command }
        } else {
            command
        }

        PythonInfo(
            path = resolvedPath,
            version = "$major.$minor.$patch",
            major = major,
            minor = minor,
            patch = patch,
            source = source,
        )
    } catch (_: Exception) {
        null
    }
}
