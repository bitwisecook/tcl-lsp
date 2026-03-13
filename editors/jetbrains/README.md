# Tcl Language Support — JetBrains Plugin

IntelliJ Platform plugin providing Tcl language support via the [tcl-lsp](../../README.md) language server.

## Requirements

- IntelliJ IDEA Ultimate 2024.1+ (or other paid JetBrains IDE)
- Python 3.10+ (for the bundled language server)

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
- **Compiler Explorer** tool window (IR, CFG, SSA, optimiser, shimmer)
- **Dialect support**: Tcl 8.4–9.0, F5 iRules, F5 iApps, EDA Tools

## Installation

### From Release

1. Download `tcl-lsp-jetbrains-VERSION.zip` from the [GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases)
2. In your JetBrains IDE: **Settings → Plugins → ⚙️ → Install Plugin from Disk...**
3. Select the downloaded `.zip` file
4. Restart the IDE

### From Source

```bash
# Build the plugin
make jetbrains

# The .zip is at build/tcl-lsp-jetbrains-VERSION.zip
```

## Configuration

**Settings → Tools → Tcl Language Server**

- **Python path**: Path to Python 3.10+ (`auto` discovers the best available)
- **Server path**: Path to tcl-lsp project root (dev mode; leave empty for bundled server)
- **Dialect**: Tcl 8.4–9.0, F5 iRules, F5 iApps, EDA Tools
- **Feature toggles**: Enable/disable individual LSP features
- **Formatting**: 20+ style settings (indent, braces, line length, etc.)
- **Diagnostics**: Toggle individual diagnostic codes (E001–W309)
- **Optimiser**: Toggle optimisation suggestions (O100–O125)

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
make jetbrains
```
