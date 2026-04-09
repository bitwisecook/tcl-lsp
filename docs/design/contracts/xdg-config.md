# KCS: Configuration File Reference

## Summary

tcl-lsp reads user-level settings from an INI-format configuration file.
The file location follows platform-native conventions so it sits where
users expect application config to live on each OS.  These settings
provide baseline defaults that editor settings override.

## File Location

| Platform | Default path | Override |
|----------|-------------|----------|
| **Linux / BSD / WSL2** | `~/.config/tcl-lsp/config.ini` | `$XDG_CONFIG_HOME/tcl-lsp/config.ini` |
| **macOS** | `~/Library/Application Support/tcl-lsp/config.ini` | `$XDG_CONFIG_HOME/tcl-lsp/config.ini` |
| **Windows** (native) | `%APPDATA%\tcl-lsp\config.ini` | `$XDG_CONFIG_HOME/tcl-lsp/config.ini` |
| **MSYS2 / Cygwin** | `~/.config/tcl-lsp/config.ini` | `$XDG_CONFIG_HOME/tcl-lsp/config.ini` |

Setting `$XDG_CONFIG_HOME` always takes precedence on every platform.

### How platform is detected

- **MSYS2**: `sys.platform == "msys"`, or `sys.platform == "win32"` with the
  `MSYSTEM` environment variable set (e.g. `MSYSTEM=UCRT64`).  Treated as a
  POSIX environment — uses XDG conventions.
- **Cygwin**: `sys.platform == "cygwin"`.  Uses XDG conventions.
- **WSL2**: Reports `sys.platform == "linux"`.  Uses XDG conventions.
- **Native Windows**: `sys.platform == "win32"` without `MSYSTEM`.  Uses
  `%APPDATA%`.

## Precedence

Settings are applied in layers — later sources override earlier ones:

1. **Built-in defaults** — hardcoded in `FeatureConfig` and `FormatterConfig`
2. **Config file** — loaded on server initialisation
3. **Editor settings** — received via `workspace/didChangeConfiguration` or
   `workspace/configuration`; always win over the config file

This means you can set sensible defaults in the config file and then
fine-tune per-project in your editor.

### Interaction with editor settings

| Editor | How editor settings are sent | Overrides config file? |
|--------|------------------------------|----------------------|
| **VS Code** | `settings.json` → `tclLsp.*` namespace | Yes |
| **Neovim** | `lspconfig.setup({ settings = { tclLsp = { ... } } })` | Yes |
| **Zed** | `settings.json` → `lsp.tcl-lsp.settings.tclLsp` | Yes |
| **Helix** | `languages.toml` → `[language-server.tcl-lsp.config.tclLsp]` | Yes |
| **Emacs** | `eglot-workspace-configuration` / `lsp-mode` | Yes |
| **Sublime Text** | LSP Settings → `settings.tclLsp` | Yes |
| **JetBrains** | Settings → Tools → Tcl Language Server | Yes |

The config file is ideal for settings you want everywhere (e.g. disabling
a noisy diagnostic), while editor settings are best for workspace-specific
overrides (e.g. a different indent size for one project).

### Syncing settings across editors

Run the **"Tcl: Export Settings to Config File"** command in VS Code
(or send `tcl-lsp.exportConfig` via `workspace/executeCommand` from any
editor) to write the current editor settings to the config file.  Only
non-default values are written, keeping the file minimal.

This lets you configure in one editor and have the same defaults apply
in all others.  Editor-specific overrides still take precedence.

## File Format

Standard INI format (Python `configparser`).  Boolean values accept
`true`/`false`/`yes`/`no`/`1`/`0`/`on`/`off`.

## Sections

### `[diagnostics]`

| Key | Type | Description |
|-----|------|-------------|
| `disabled` | comma-separated codes | Diagnostic codes to suppress (e.g. `W111, T100, IRULE1005`) |
| `generic_variable_patterns` | multi-line regexes | Patterns for IRULE4002 generic variable detection |

### `[optimiser]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Master switch for all optimiser suggestions |
| `disabled` | comma-separated codes | (none) | Individual rules to suppress (e.g. `O109, O126`) |

### `[shimmer]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable shimmer type-instability detection |

### `[xcDiagnostics]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `false` | Enable F5 XC translatability diagnostics |

### `[features]`

Toggle individual LSP features.  All default to `true`.

| Key | Description |
|-----|-------------|
| `hover` | Hover information |
| `completion` | Code completion |
| `diagnostics` | Inline diagnostics |
| `formatting` | Document formatting |
| `semanticTokens` | Semantic token highlighting |
| `codeActions` | Quick fixes and refactorings |
| `definition` | Go to definition |
| `references` | Find references |
| `documentSymbols` | Document symbol outline |
| `folding` | Code folding |
| `rename` | Rename symbol |
| `signatureHelp` | Function signature help |
| `workspaceSymbols` | Workspace symbol search |
| `inlayHints` | Inlay hints |
| `callHierarchy` | Call hierarchy |
| `documentLinks` | Document links |
| `selectionRange` | Smart selection |

### `[formatting]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `indent_size` | int | `4` | Spaces per indent level |
| `indent_style` | `spaces`/`tabs` | `spaces` | Indent character |
| `brace_style` | string | `k_and_r` | Brace placement style |
| `max_line_length` | int | `120` | Hard line length limit |
| `goal_line_length` | int | `100` | Soft wrapping target |
| ... | | | See `FormatterConfig` for all keys |

### `[style]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `line_length` | int | `120` | W111 line-length threshold |

## Example

```ini
[diagnostics]
disabled = W111, T100

[optimiser]
disabled = O109

[shimmer]
enabled = true

[features]
inlayHints = false

[formatting]
indent_size = 2
indent_style = tabs

[style]
line_length = 100
```

## Implementation

- Config loading: `core/common/user_config.py`
- Platform detection: `_config_dir()` and `_is_posix_compat_windows()`
- Server integration: `lsp/server.py` → `on_initialized()`
- Export command: `lsp/server.py` → `_export_config()`
- CLI config: `explorer/tcl_cli.py` → `_config_file_paths()`
