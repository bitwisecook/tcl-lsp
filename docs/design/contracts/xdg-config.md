# KCS: Configuration File Reference

## Summary

tcl-lsp reads settings from two INI-format configuration files: a
**global** file per user and an optional **project** file at the
workspace root.  Both use the same schema.  The global file sits at
the platform-native config location on each OS; the project file is
named `.tcl-lsp.ini` and is checked in with the source tree so every
contributor on the project picks up the same rules.

For the user-facing answer to "how do I silence a specific code?",
see [`../../kcs/kcs-howto-suppress-diagnostics.md`](../../kcs/kcs-howto-suppress-diagnostics.md).

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

## Project-level config file

In addition to the global file above, tcl-lsp looks for a
`.tcl-lsp.ini` in the workspace root when the server initialises.
The schema is identical to the global file — every section documented
below works in either place.  The project file is intended to be
committed to source control so team conventions follow the code.

| Location | Role |
|----------|------|
| `<workspace-root>/.tcl-lsp.ini` | Per-project overrides checked in with the source |
| Global user config (above) | Per-user defaults across all projects |

The server does not walk upward from the workspace root looking for
ancestor config files.  Each workspace gets exactly one project file,
directly at its root.

## Precedence

Settings are applied in layers — later sources override earlier ones.
The full chain, from lowest priority to highest:

1. **Built-in defaults** — hardcoded in `FeatureConfig` and `FormatterConfig`.
2. **Global config file** — `~/.config/tcl-lsp/config.ini` (or the
   platform equivalent).  Loaded once on server initialisation.
3. **Editor settings** — received via `workspace/didChangeConfiguration`
   or `workspace/configuration`.  Overwrite the global file layer.
4. **Project config file** — `<workspace-root>/.tcl-lsp.ini`.  The
   most specific server-level source, so a project config enabling or
   disabling a code always wins over whatever the editor sends.

Two further document-local suppression scopes are applied after
server-level filtering and only to the document they appear in.  Both
scopes only *add* suppressions — neither can re-enable a code
suppressed by the other.  They are not ordered relative to each other;
inline is more specific (one command), while file-level is broader
(the whole file):

- **Inline** — a ``# noqa`` or ``# noqa: CODE`` comment on the line
  before a command suppresses codes for that command only (most
  specific scope).
- **File-level** — a top-of-file ``# tcl-lsp: disable=CODE,CODE``
  comment suppresses the listed codes for the whole file.

Both scopes override all four server-level layers (1–4) above.

See [`../../kcs/kcs-howto-suppress-diagnostics.md`](../../kcs/kcs-howto-suppress-diagnostics.md)
for the user-facing walkthrough of each scope.

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
| `inlayTypeHints` | Inferred-type inlay hints (variables, format specifiers) |
| `inlayParameterHints` | Parameter-name inlay hints at proc/method call sites |
| `callHierarchy` | Call hierarchy |
| `documentLinks` | Document links |
| `selectionRange` | Smart selection |
| `crossFileResolution` | Cross-file W120/W123 suppression and cross-file E002/E003 arity — off by default (independent of `[xcDiagnostics]`, which is F5 XC Migration-specific) |

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
inlayTypeHints = false
inlayParameterHints = false

[formatting]
indent_size = 2
indent_style = tabs

[style]
line_length = 100
```

## Implementation

- Global + project config loading: `shared/user_config.py`
  (`load_user_config`, `load_project_config`, `merge_settings_layers`)
- Platform detection: `_config_dir()` and `_is_posix_compat_windows()`
- Layer storage: `server/state.py`
  (`global_config_settings`, `editor_config_settings`, `project_config_settings`)
- Layer merge + apply: `server/settings.py` (`_merged_settings`,
  `_apply_merged_settings_now`)
- Server integration: `server/workspace_init.py` → `on_initialized()`
  loads global and project layers before any analysis runs
- File-level directive parser: `analyser/_analyser/_utils.py`
  (`parse_file_suppression`)
- Per-document suppression filter: `server/features/diagnostics.py`
  (`_is_suppressed`)
- Export command: `server/server.py` → `_export_config()`
- CLI config: `tooling/tcl/main.py` → `_config_file_paths()`
