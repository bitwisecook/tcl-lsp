# Tcl Language Support — JetBrains Plugin

IntelliJ Platform plugin providing Tcl language support via the [tcl-lsp](../../README.md) language server.

## Requirements

- IntelliJ IDEA Ultimate 2024.1+ (or other paid JetBrains IDE)

Nothing else — the `.zip` plugin bundles the self-contained native
`tcl-lsp-server` binary for every supported platform and launches the one
matching your machine. No Python, runtime, or interpreter is needed.

See the [Installation Guide](../../INSTALL-editors.md) for
full details on Python setup across platforms.

> Starting with IntelliJ IDEA 2025.3, the LSP API will be available to all users,
> including those without a paid subscription.

## Features

All features from the tcl-lsp server are supported:

- **Syntax highlighting** via TextMate grammar
- **Diagnostics** with 30+ configurable rules
- **Auto-completion** for commands, subcommands, variables, and switches
- **Hover** with command help and proc signatures
- **Go-to-definition** and **find references**
- **Rename symbol** (F2)
- **Document formatting** with 20+ style options
- **Document symbols** and **workspace symbols**
- **Call hierarchy** (incoming/outgoing)
- **Code folding**, **inlay hints**, **signature help**
- **Code actions** (quick fixes)
- **Compiler Explorer** tool window (IR, CFG, SSA, optimiser, shimmer) — also
  available by right-clicking a Tcl/iRule file → **Open In Tcl Compiler Explorer**
- **Dialect support**: Tcl 8.4–9.0, F5 iRules, F5 iApps, EDA Tools

## Installation

### From the JetBrains Marketplace (recommended)

Install via **Settings → Plugins → Marketplace → search "Tcl Language
Support"**, or via the plugin page:
<https://plugins.jetbrains.com/plugin/31801-tcl-language-support>.

After install, restart the IDE.

### From Release

1. Download `tcl-lsp-jetbrains-VERSION.zip` from the [GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases)
2. In your JetBrains IDE: **Settings → Plugins → ⚙️ → Install Plugin from Disk...**
3. Select the downloaded `.zip` file
4. Restart the IDE

### From Source

```bash
# Build the plugin
make build-editor-jetbrains

# The .zip is at build/tcl-lsp-jetbrains-VERSION.zip
```

## Configuration

**Settings → Tools → Tcl Language Server**

- **Server path**: Path to a `tcl-lsp-server` binary (dev mode; leave empty for the bundled server)
- **Dialect**: Tcl 8.4–9.0, F5 iRules, F5 iApps, EDA Tools
- **Feature toggles**: Enable/disable individual LSP features
- **Formatting**: 20+ style settings (indent, braces, line length, etc.)
- **Diagnostics**: Toggle individual diagnostic codes (E001–W309)
- **Optimiser**: Toggle optimisation suggestions (O100–O130)

## Development

The plugin source lives in `editors/jetbrains/`. It uses:

- **IntelliJ Platform Gradle Plugin 2.x** for building
- **IntelliJ Platform LSP API** (`ProjectWideLspServerDescriptor`)
- **TextMate** grammar (shared with VS Code)
- **JCEF** browser for the Compiler Explorer webview

```bash
# Build independently with Gradle
cd editors/jetbrains
./gradlew buildPlugin

# Or via the top-level Makefile
make build-editor-jetbrains
```

## Configuration File

tcl-lsp reads a platform-native configuration file for editor-agnostic
defaults (diagnostics, optimiser, shimmer, features, formatting):

| Platform | Default path |
|----------|-------------|
| Linux / BSD / WSL2 | `~/.config/tcl-lsp/config.ini` |
| macOS | `~/Library/Application Support/tcl-lsp/config.ini` |
| Windows | `%APPDATA%\tcl-lsp\config.ini` |
| MSYS2 / Cygwin | `~/.config/tcl-lsp/config.ini` |

`$XDG_CONFIG_HOME` overrides the default on every platform.

Settings from the config file are applied as baseline defaults.  JetBrains
IDE settings (Settings → Tools → Tcl Language Server) override the config
file — so you can set shared defaults in the config file and per-project
overrides in the IDE.

See [docs/design/contracts/xdg-config.md](../../docs/design/contracts/xdg-config.md) for
the full reference.
