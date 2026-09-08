# Tcl Language Support — JetBrains Plugin

IntelliJ Platform plugin providing Tcl language support via the [tcl-lsp](../../README.md) language server.

## Requirements

- IntelliJ IDEA Ultimate 2025.3+ (or another paid JetBrains IDE)

Nothing else — the `.zip` plugin bundles the self-contained native
`tcl-lsp-server` binary for every supported platform and launches the one
matching your machine. No Python, runtime, or interpreter is needed.

See the [Installation Guide](../../INSTALL-editors.md) for full details.

> The platform's LSP API reaches the free editions in 2025.3. The plugin still
> declares `com.intellij.modules.ultimate`, so dropping that dependency — and
> with it the paid-IDE requirement — is a separate change from raising the
> build floor to 2025.3.

## Features

Everything the IDE's LSP client asks for is supported:

- **Syntax highlighting** via the bundled TextMate grammar, which the
  plugin registers with the IDE's TextMate service at startup
- **Semantic highlighting** from the language server's own tokens, layered
  over the grammar (toggle it under Features → Semantic tokens)
- **Diagnostics** with 30+ configurable rules
- **Auto-completion** for commands, subcommands, variables, and switches
- **Hover** with command help and proc signatures
- **Go-to-definition**, **go-to-type-definition**, and **find references**
- **Document formatting** with 20+ style options
- **Document symbols** and **workspace symbols**
- **Call hierarchy** (incoming/outgoing) and **type hierarchy**
- **Code folding**, **inlay hints**, **signature help**
- **Rename symbol** (F2) — needs IntelliJ 2026.2 or newer
- **Code actions** (quick fixes)
- **Compiler Explorer** tool window (IR, CFG, SSA, optimiser, shimmer) — also
  available by right-clicking a Tcl/iRule file → **Open In Tcl Compiler Explorer**.
  **Open in editor** puts the current view in an ordinary read-only editor tab,
  where find, folding and the colour scheme all work, and moving the caret in it
  reveals the matching source. The CFG, WASM and optimiser-diff panes stay in
  the tool window, since their edges and connectors are drawn rather than
  written.
- **Dialect support**: Tcl 8.4–9.1, F5 iRules, F5 iApps, EDA Tools
- **Pack-declared file extensions** registered as the packs that claim them
  load and unload (see below)

### What your IDE version changes

The LSP capability set is assembled by the platform's `lsp` module inside the
IDE you are running, not by the SDK this plugin was built against, so a newer
IDE asks the server for more without a plugin update. Relative to the 2025.3
floor:

| Feature | Needs |
|---|---|
| Code lens | 2026.1 |
| Rename symbol, on-type formatting | 2026.2 |

These the server implements and the VS Code extension uses, but no JetBrains
IDE requests at any version from 2025.3 through 2026.3, so they do nothing
here: **go-to-implementation**, **go-to-declaration**, **linked editing range**,
and **auto-rewriting source paths on rename**. Their toggles stay in
**Settings → Features**, marked with a dagger, because the same server settings
are shared with editors that do support them.

Both lists come from scanning every class in the published `lsp` platform module
for `TextDocumentClientCapabilities.set*` and `WorkspaceClientCapabilities.set*`
across builds 253, 261, 262 and 263.

## Pack-declared file extensions

A SpecTcl pack can claim a file extension of its own with a
`file_extension` row, or through the `file_extensions` of an `environment`
block. The server routes those extensions as soon as it discovers the pack,
but the IDE learns its file types from the plugin's static manifest, so a
`.irulex` file would otherwise open as plain text with no server attached.

The plugin registers the claimed extensions with `FileTypeManager` while their
packs are loaded, and retires them again when the packs go. `tcl-irule` rows
land on the **iRule** file type; everything else lands on **Tcl**. Two things
are worth knowing, because JetBrains file-type associations are IDE-wide
rather than per project:

- **A mapping you made by hand always wins.** If the extension is already
  associated with any file type — including the plugin's own — the plugin
  leaves it alone and never removes it. It only ever retires an association it
  installed itself and that still points where it left it. The ledger of what
  it installed is kept in `TclLspPackAssociations.xml` beside your other IDE
  settings.
- **Several open projects share one set of associations.** The plugin
  registers the union of what every open project's server reports, and drops
  an extension only when no open project still claims it. Closing a project
  retires what it alone claimed straight away, as long as another project is
  still open. A project whose server has not started yet claims nothing, so
  shortly after IDE startup an extension can disappear and come back once that
  project's server reports.

Nothing is torn down when the IDE exits, so the associations survive a
restart. If a pack has been deleted in the meantime, the first report of the
next session is what removes its extension.

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
- **Dialect**: Tcl 8.4–9.1, F5 iRules, F5 iApps, EDA Tools
- **Feature toggles**: Enable/disable individual LSP features
- **Formatting**: 20+ style settings (indent, braces, line length, etc.)
- **Diagnostics**: Toggle any individual diagnostic code
- **Optimiser**: Toggle any individual optimisation suggestion

## Development

The plugin source lives in `editors/jetbrains/`. It uses:

- **IntelliJ Platform Gradle Plugin 2.x** for building
- **IntelliJ Platform LSP API** (`ProjectWideLspServerDescriptor`)
- **TextMate** grammar (shared with VS Code), contributed through
  `com.intellij.textmate.bundleProvider`
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
