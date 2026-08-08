# Sublime Text — Tcl Language Support

Full Tcl and iRules language support for Sublime Text, powered by
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

**Replaces** [sublime-iRules](https://github.com/billchurch/sublime-iRules)
with a superset of its features across all Tcl dialects.

## Requirements

- **Sublime Text** 4 (build 4107+)
- Nothing else — the `.sublime-package` bundles the self-contained native
  `tcl-lsp-server` binary. No Python, runtime, or interpreter is needed for
  the server.
- The **[LSP](https://packagecontrol.io/packages/LSP)** package — install
  it **first**, before this package, for full language-server features
  (diagnostics, completions, hover, formatting, code actions, and more).
  Syntax highlighting, snippets, and settings work without it.

Install LSP from Package Control:

> Command Palette → **Package Control: Install Package** → **LSP**

## Installation

### Via Package Control (recommended, once tcl-lsp is on the channel)

1. Open the Command Palette → **Package Control: Install Package**
2. Search for **Tcl-LSP** and install it
3. Restart Sublime Text and install the **LSP** package for full
   language-server features

Maintainer note: Package Control fetches the package source archive from
the `bitwisecook/tcl-lsp-sublime-text` mirror repo (a flat-layout sibling
of this monorepo that exists because Package Control's `tags: true`
discovery downloads the git tarball at each tag and expects the package
contents at the root). The mirror is repopulated and tagged from
`build/sublime-stage/` by `make publish-sublime` on every release; no
edit to that repo is ever needed by hand.

### Manual install

1. Download `Tcl.sublime-package` from the
   [latest release](https://github.com/bitwisecook/tcl-lsp/releases/latest)
2. Copy it to your Sublime Text **Installed Packages** directory:

   ```bash
   # macOS
   cp Tcl.sublime-package ~/Library/Application\ Support/Sublime\ Text/Installed\ Packages/

   # Linux
   cp Tcl.sublime-package ~/.config/sublime-text/Installed\ Packages/

   # Windows (PowerShell)
   Copy-Item Tcl.sublime-package "$env:APPDATA\Sublime Text\Installed Packages\"
   ```

   > **Important:** The file **must** be named `Tcl.sublime-package`.
   > If you downloaded the versioned filename (`tcl-lsp-sublime-*.sublime-package`),
   > rename it to `Tcl.sublime-package` before copying.

3. Restart Sublime Text
4. Install the [LSP](https://packagecontrol.io/packages/LSP) package from
   Package Control for full language-server features

The `.sublime-package` bundles the native `tcl-lsp-server` binary for each
platform, so there is nothing further to install for LSP features. The
bundled server is extracted to the Sublime Text cache on first load and
marked executable there.

To point the package at a different build, set `server_path` in
**Preferences > Package Settings > LSP-Tcl > Settings**.

See the [Installation Guide](../../INSTALL-editors.md) for the full
per-editor matrix.

### Development install (from source)

1. Clone the repository
2. Symlink `editors/sublime-text` into your Sublime Text `Packages`
   directory as `Tcl`:

   ```bash
   # macOS
   ln -s /path/to/tcl-lsp/editors/sublime-text \
       ~/Library/Application\ Support/Sublime\ Text/Packages/Tcl

   # Linux
   ln -s /path/to/tcl-lsp/editors/sublime-text \
       ~/.config/sublime-text/Packages/Tcl
   ```

3. Install the **LSP** package from Package Control

The server source in `server/`, `compiler/`, `analyser/`, and `tooling/explorer/`
is used directly from the repository — no build step needed.

## What the plugin provides

### Standalone (no LSP package required)

These features work out of the box with no additional dependencies:

- **Syntax highlighting** — Tcl (`.tcl`, `.tk`, `.itcl`, `.tm`), iRules
  (`.irul`, `.irule`), iApps (`.iapp`, `.iappimpl`, `.impl`), and EDA
  Tools, with version-specific grammars for Tcl 8.4, 8.5, and 9.0
- **16 snippets** — code templates for common Tcl and iRules patterns
  (proc, namespace, class, foreach, switch, catch, try, iRules event
  handlers, collect/release, data-group lookup, and more)
- **Static completions** for iRules events and commands
- **Comment toggling** (`Ctrl+/` / `Cmd+/`)
- **Symbol indexing** — `Goto Symbol` (`Ctrl+R`) for proc definitions
  and iRules event handlers
- **Editor defaults** — tab size 4, spaces, 120-character ruler
- **Built-in TCL package disabled** automatically to avoid duplicate
  syntax entries in the language menu

### With LSP (full language server)

When the [LSP](https://packagecontrol.io/packages/LSP) package is
installed, the plugin automatically registers the bundled tcl-lsp server
and enables:

- **Diagnostics** — errors, warnings, security, taint-tracking, and
  style hints with squiggly underlines
- **Context-aware completions** — Tcl/iRules commands, variables,
  procedures, and namespaces
- **Hover documentation** — inline docs for commands, variables, and
  events
- **Go to Definition / References** — navigate your codebase
- **Document formatting** via the command palette
- **Code actions** — quick fixes for common issues
- **Semantic tokens** — token-level highlighting from the language server
- **Document symbols** — `Goto Symbol in Project` (`Ctrl+Shift+R`)
- **Signature help** — parameter hints while typing
- **Rename** — project-wide symbol renaming
- **Code folding** for blocks
- **Inlay hints** — inline type/value annotations
- **Call hierarchy** — incoming/outgoing call trees
- **Optimisation suggestions** — iRules performance recommendations
- **Minify / unminify** — minify documents and translate minified error
  messages back to original names using symbol maps

### Automatic dialect syncing

Selecting a dialect-specific syntax from **View > Syntax** (e.g.
"iRule", "Tcl 8.4", "Tcl 9.0") automatically updates the LSP server's
dialect setting so diagnostics and completions match the chosen dialect.

## Supported dialects

| ID | Description |
|----|-------------|
| `tcl8.4` | Tcl 8.4 |
| `tcl8.5` | Tcl 8.5 |
| `tcl8.6` | Tcl 8.6 (default) |
| `tcl9.0` | Tcl 9.0 |
| `f5-irules` | F5 iRules |
| `f5-iapps` | F5 iApps |
| `synopsys-eda-tcl` | Synopsys EDA |
| `cadence-eda-tcl` | Cadence EDA |
| `xilinx-eda-tcl` | Xilinx EDA |
| `intel-quartus-eda-tcl` | Intel Quartus |
| `mentor-eda-tcl` | Mentor EDA |

Select a dialect from the Command Palette: **Tcl: Select Dialect**.

## Configuration

### LSP settings

**Preferences > Package Settings > Tcl > LSP Settings**

```json
{
    "settings": {
        "tclLsp": {
            "dialect": "f5-irules",
            "formatting": {
                "indentSize": 4,
                "maxLineLength": 120
            }
        }
    }
}
```

### Custom server path

If the bundled server is not found or you want to use a different version:

```json
{
    "server_path": "/path/to/tcl-lsp-server"
}
```

### Key bindings

This package ships **no** key bindings by default, to avoid clashing with
your own. Example bindings for every command live in
**Preferences > Package Settings > Tcl > Key Bindings**, which opens the
commented-out example keymap on the left and your own user keymap on the
right. Copy any binding you want into the right-hand pane and uncomment it.

For a step-by-step walkthrough and the full list of bindable commands,
see [How do I bind keys to the Tcl commands in Sublime Text?](https://github.com/bitwisecook/tcl-lsp/blob/main/docs/kcs/kcs-howto-bind-sublime-tcl-commands.md).

## Disabling the context menu

The package adds a few entries (Format Document, Minify Document, Unminify
Error, Apply Safe Quick Fixes) to the editor right-click menu. They only
appear in Tcl and iRules files — each command's `is_visible` check hides
it in other file types.

If you'd rather not have them at all, you can override the menu: create the
file `Packages/Tcl/Context.sublime-menu` (use **Browse Packages…** from the
menu to find your `Packages` directory) and put an empty list in it to
remove every entry:

```json
[]
```

Anything you put in that file replaces the bundled context menu, so you can
also keep only the entries you want.

## Command Palette

| Command | Description |
|---------|-------------|
| **Tcl: Select Dialect** | Choose the active Tcl dialect |
| **Tcl: Restart Language Server** | Restart the LSP server |
| **Tcl: Format Document** | Format the current document |
| **Tcl: Minify Document** | Minify the current document |
| **Tcl: Unminify Error** | Translate minified error messages using a symbol map |
| **Tcl: Apply Safe Quick Fixes** | Apply all safe automatic fixes |
| **Tcl: Apply All Optimisations** | Apply optimisation suggestions |
| **Preferences: Tcl LSP Settings** | Open LSP settings |
| **Preferences: Tcl Editor Settings** | Open editor settings |

## Snippets

Type the trigger and press `Tab` to expand:

### Tcl

| Trigger | Description |
|---------|-------------|
| `tcl-proc` | Procedure definition |
| `tcl-namespace` | Namespace eval block |
| `tcl-package` | Package boilerplate |
| `tcl-class` | TclOO class |
| `tcl-if` | If/else block |
| `tcl-foreach` | Foreach loop |
| `tcl-for` | For loop |
| `tcl-switch` | Switch block |
| `tcl-catch` | Catch with result/options |
| `tcl-try` | Try/trap block |
| `tcl-dict-for` | Dict for loop |

### iRules (`.irul` / `.irule` files only)

| Trigger | Description |
|---------|-------------|
| `irule-rule-init` | RULE_INIT handler |
| `irule-http-request` | HTTP_REQUEST handler |
| `irule-redirect-https` | HTTP to HTTPS redirect |
| `irule-collect-release` | Collect/release pair |
| `irule-class-lookup` | Data-group lookup |

## Migrating from sublime-iRules

This package is a drop-in replacement for
[billchurch/sublime-iRules](https://github.com/billchurch/sublime-iRules).

1. Remove the sublime-iRules package from Package Control
2. Install this package (see Installation above)
3. All `.irul` and `.irule` files automatically use the new syntax
4. The built-in formatter is now powered by the LSP server and supports
   all Tcl dialects, not just iRules

## Licence

AGPL-3.0-or-later — see [LICENSE](../../LICENSE) for details.

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

Settings from the config file are applied as baseline defaults.  Sublime
Text LSP settings (`Preferences > Package Settings > Tcl > LSP Settings`)
override the config file — so you can set shared defaults in the config
file and per-project overrides in Sublime Text.

Use the `tcl-lsp.exportConfig` command via `workspace/executeCommand` to
write current settings to the config file.

See [docs/design/contracts/xdg-config.md](../../docs/design/contracts/xdg-config.md) for
the full reference.
