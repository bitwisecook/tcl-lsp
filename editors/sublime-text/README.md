# TclLsp — Tcl Language Support for Sublime Text

Full Tcl and iRules language support for Sublime Text, powered by
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

**Replaces** [sublime-iRules](https://github.com/billchurch/sublime-iRules)
with a superset of its features across all Tcl dialects.

## Requirements

- **Sublime Text** 4 (build 4107+)
- The **[LSP](https://packagecontrol.io/packages/LSP)** package, for
  language-server features (diagnostics, completions, hover, formatting,
  code actions, and more). Syntax highlighting, snippets, symbol
  indexing and editor defaults work without it.
- Nothing else. With LSP installed, the `tcl-lsp-server` build for your
  platform is downloaded on first use — no Python, runtime, or
  interpreter is needed for the server.

Install LSP from Package Control:

> Command Palette → **Package Control: Install Package** → **LSP**

## Installation

### Via Package Control (recommended)

1. Open the Command Palette → **Package Control: Install Package**
2. Search for **TclLsp** and install it
3. Install the **LSP** package the same way, for full language-server
   features

### Manual install

1. Download `TclLsp.sublime-package` from the
   [latest release](https://github.com/bitwisecook/tcl-lsp/releases/latest)
2. Copy it, filename unchanged, into your Sublime Text
   **Installed Packages** directory:

   ```bash
   # macOS
   cp TclLsp.sublime-package ~/Library/Application\ Support/Sublime\ Text/Installed\ Packages/

   # Linux
   cp TclLsp.sublime-package ~/.config/sublime-text/Installed\ Packages/

   # Windows (PowerShell)
   Copy-Item TclLsp.sublime-package "$env:APPDATA\Sublime Text\Installed Packages\"
   ```

3. Restart Sublime Text
4. Install the [LSP](https://packagecontrol.io/packages/LSP) package from
   Package Control for language-server features

See the
[Installation Guide](https://github.com/bitwisecook/tcl-lsp/blob/rust/INSTALL-editors.md)
for the full per-editor matrix.

### Development install (from source)

1. Clone the repository
2. Symlink `editors/sublime-text` into your Sublime Text `Packages`
   directory as `TclLsp`:

   ```bash
   # macOS
   ln -s /path/to/tcl-lsp/editors/sublime-text \
       ~/Library/Application\ Support/Sublime\ Text/Packages/TclLsp

   # Linux
   ln -s /path/to/tcl-lsp/editors/sublime-text \
       ~/.config/sublime-text/Packages/TclLsp
   ```

3. Install the **LSP** package from Package Control

The plugin itself needs no build step. A checkout has no release version
stamped into it, so the plugin resolves the newest published release
instead; a `tcl-lsp-server` binary staged at `server/tcl-lsp-server`
inside the package directory takes precedence over any download, which is
how you point Sublime Text at a local `cargo build`.

## The language server

The package ships no binary — one `.sublime-package` serves every
platform, so a bundled server would be right for one platform and wrong
for the rest. Instead, the first time you open a Tcl file with the LSP
package installed, the plugin downloads
`tcl-lsp-server-<target-triple>` for your platform from the tcl-lsp
release matching this package, and stores it under LSP's package storage.
Older versions are pruned on upgrade.

The download is accepted only if it matches the SHA-256 **pinned inside
this package**, which CI computed from the very binaries it attached to
that release. That digest reaches you through Package Control rather than
from the release the binary comes from, so a swapped release asset is
rejected even though an attacker could publish a matching `SHA256SUMS`
beside it. A package built from a source checkout has no pins and falls
back to the release's `SHA256SUMS` — an integrity check on the transfer,
not proof of origin — and says so in the console when it does.

The `.tclspec` packs for the EDA dialects ship *inside* the package (they
are plain data, identical everywhere) and are staged beside the server.

To use a server you built or installed yourself, set `server_path` — the
download is then skipped entirely:

```json
{
    "server_path": "/path/to/tcl-lsp-server"
}
```

Supported download platforms are macOS, Linux and Windows on x64 and
arm64. On anything else (Linux riscv64, say), install the server yourself
and set `server_path`.

## What the plugin provides

### Standalone (no LSP package required)

These features work out of the box with no additional dependencies:

- **Syntax highlighting** — Tcl (`.tcl`, `.tk`, `.itcl`, `.tm`,
  `.tclspec`), iRules (`.irul`, `.irule`), iApps (`.iapp`, `.iappimpl`,
  `.impl`), and EDA Tools, with version-specific grammars for Tcl 8.4,
  8.5, and 9.0
- **16 snippets** — code templates for common Tcl and iRules patterns
  (proc, namespace, class, foreach, switch, catch, try, iRules event
  handlers, collect/release, data-group lookup, and more)
- **Static completions** for iRules events and commands
- **Comment toggling** (`Ctrl+/` / `Cmd+/`)
- **Symbol indexing** — `Goto Symbol` (`Ctrl+R`) for proc definitions
  and iRules event handlers
- **Editor defaults** — tab size 4, spaces, 120-character ruler

### With LSP (full language server)

When the [LSP](https://packagecontrol.io/packages/LSP) package is
installed, the plugin automatically registers the tcl-lsp server and
enables:

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

## Recommended setup

Two settings live outside this package, so it asks before touching them —
once after installation, and on demand from the Command Palette via
**Tcl: Recommended Setup**:

- **Disable Sublime Text's built-in `TCL` package.** It ships its own Tcl
  syntax, so without this each Tcl syntax appears twice in the language
  menu. Disabling it adds `TCL` to `ignored_packages` in your
  preferences.
- **Turn on LSP's `semantic_highlighting`.** It is off by default in the
  LSP package, and without it tcl-lsp's semantic tokens never reach the
  buffer.

Decline and everything else still works; both are ordinary preferences
you can change at any time.

## Supported dialects

| ID | Description |
|----|-------------|
<!-- @generated:dialects:begin -->
| `bpf` | BPF |
| `cadence-eda-tcl` | Cadence EDA Tcl |
| `expect` | Expect |
| `f5-bigip` | F5 BIG-IP |
| `f5-iapps` | F5 iApps |
| `f5-irules` | F5 iRules |
| `f5-tmsh` | F5 tmsh Scripts |
| `intel-quartus-eda-tcl` | Intel Quartus EDA Tcl |
| `mentor-eda-tcl` | Mentor EDA Tcl |
| `microchip-libero-eda-tcl` | Microchip Libero EDA Tcl |
| `spectcl` | SpecTcl |
| `synopsys-eda-tcl` | Synopsys EDA Tcl |
| `tcl8.4` | Tcl 8.4 |
| `tcl8.5` | Tcl 8.5 |
| `tcl8.6` | Tcl 8.6 (default) |
| `tcl9.0` | Tcl 9.0 |
| `tcl9.1` | Tcl 9.1 |
| `xilinx-eda-tcl` | Xilinx EDA Tcl |
<!-- @generated:dialects:end -->

Select a dialect from the Command Palette: **Tcl: Select Dialect**.

## Configuration

### LSP settings

**Preferences > Package Settings > TclLsp > LSP Settings**

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

### Editor settings

**Preferences > Package Settings > TclLsp > Editor Settings** — tab size,
rulers, and the other per-syntax editor defaults.

## Key bindings

This package ships **no** key bindings, so a fresh install can never take
a chord you already use. Bind any command yourself in
**Preferences > Key Bindings**:

```json
[
	{"keys": ["ctrl+alt+f"], "command": "lsp_format_document"},
	{"keys": ["ctrl+alt+d"], "command": "tcl_select_dialect"}
]
```

The bindable command names are the `command` column of the
[Command Palette](#command-palette) table below, plus LSP's own commands
(`lsp_format_document` and friends). For a step-by-step walkthrough,
including scoping a binding to Tcl buffers only, see
[How do I bind keys to the Tcl commands in Sublime Text?](https://github.com/bitwisecook/tcl-lsp/blob/rust/docs/kcs/kcs-howto-bind-sublime-tcl-commands.md).

## Command Palette

| Command | `command` | Description |
|---------|-----------|-------------|
| **Tcl: Select Dialect** | `tcl_select_dialect` | Choose the active Tcl dialect |
| **Tcl: Restart Language Server** | `tcl_restart_server` | Restart the LSP server |
| **Tcl: Format Document** | `tcl_format_document` | Format the current document |
| **Tcl: Minify Document** | `tcl_minify_document` | Minify the current document |
| **Tcl: Unminify Error** | `tcl_unminify_error` | Translate minified error messages using a symbol map |
| **Tcl: Apply Safe Quick Fixes** | `tcl_fix_all_safe_issues` | Apply all safe automatic fixes |
| **Tcl: Apply All Optimisations** | `tcl_optimise_document` | Apply optimisation suggestions |
| **Tcl: Recommended Setup** | `tcl_recommended_setup` | Re-offer the two recommended settings |
| **Preferences: Tcl LSP Settings** | `edit_settings` | Open LSP settings |
| **Preferences: Tcl Editor Settings** | `edit_settings` | Open editor settings |

## Context menu

The package adds a few entries (Format Document, Minify Document, Unminify
Error, Apply Safe Quick Fixes) to the editor right-click menu. They only
appear in Tcl and iRules files — each command's `is_visible` check hides
it in other file types.

To remove them, override the menu: create the file
`Packages/TclLsp/Context.sublime-menu` (use **Browse Packages…** from the
menu to find your `Packages` directory) containing an empty list:

```json
[]
```

Anything in that file replaces the bundled context menu, so you can also
keep only the entries you want.

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

## Migrating from a hand-installed `Tcl.sublime-package`

Before this package reached Package Control it was installed by hand as
`Tcl.sublime-package`. Delete that file from your **Installed Packages**
directory after installing **TclLsp** — left in place, every syntax and
the language server are registered twice. The plugin says so once if it
finds one. Settings you saved in `Packages/User/LSP-Tcl.sublime-settings`
carry over unchanged.

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

Settings from the config file are applied as baseline defaults. Sublime
Text LSP settings (`Preferences > Package Settings > TclLsp > LSP
Settings`) override the config file — so you can set shared defaults in
the config file and per-project overrides in Sublime Text.

Use the `tcl-lsp.exportConfig` command via `workspace/executeCommand` to
write current settings to the config file.

See
[docs/design/contracts/xdg-config.md](https://github.com/bitwisecook/tcl-lsp/blob/rust/docs/design/contracts/xdg-config.md)
for the full reference.

## Licence

AGPL-3.0-or-later — see
[LICENSE](https://github.com/bitwisecook/tcl-lsp/blob/rust/LICENSE) for
details.
