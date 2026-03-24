# KCS: XDG Configuration File Reference

## Summary

tcl-lsp reads user-level settings from an INI-format file at
`~/.config/tcl-lsp/config.ini` (respecting `$XDG_CONFIG_HOME`).  These
settings provide baseline defaults that editor settings override.

## Precedence

1. **Built-in defaults** — hardcoded in `FeatureConfig` and `FormatterConfig`
2. **XDG config.ini** — loaded on server initialisation
3. **Editor settings** — received via `workspace/didChangeConfiguration` or
   `workspace/configuration`; always win over XDG config

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

## Exporting Settings

Use the **"Tcl: Export Settings to XDG Config"** command in VS Code
(or `tcl-lsp.exportConfig` via LSP `workspace/executeCommand`) to write
the current editor settings to `config.ini`.  Only non-default values
are written, keeping the file minimal.

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
- Server integration: `lsp/server.py` → `on_initialized()`
- Export command: `lsp/server.py` → `_export_config()`
