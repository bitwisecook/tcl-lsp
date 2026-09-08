# Configuration file reference

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

`tcl_lsp_core::tcl_install::user_config_path` resolves the path; its pure core
`config_path_for` takes the environment values and platform flags as
arguments, so the precedence is testable without mutating the process
environment. The order is:

1. `$XDG_CONFIG_HOME` when set and non-empty — on **every** platform.
2. Native Windows (`cfg!(target_os = "windows")` **and** `MSYSTEM` unset) →
   `%APPDATA%`.
3. macOS → `~/Library/Application Support/`.
4. Otherwise → `~/.config/`.

An MSYS2 or Cygwin shell is identified by `MSYSTEM` being set and is treated as
a POSIX environment, so it takes the XDG branch rather than `%APPDATA%`. WSL2
is an ordinary Linux target and needs no special case.

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

1. **Built-in defaults** — the `Default` impls on the server's feature config
   and `FormatterConfig`.
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

## File format

Standard INI. Comment lines start with `#` or `;`; keys before any section
header are ignored; indented continuation lines are joined with a newline, so
a multi-line value (such as `generic_variable_patterns`) works as it does under
`configparser`. Boolean values accept `true`/`false`, `yes`/`no`, `1`/`0`, and
`on`/`off`.

Both files are parsed by `settings_from_ini` into the **same JSON shape** the
editor delivers a `tclLsp` section as, so a file layer is applied by exactly
the same code that applies the editor layer — there is no second settings
interpreter to drift.

## Sections

### Top-level keys — `[global]` (user file) / `[project]` (project file)

| Key | Type | Description |
|-----|------|-------------|
| `dialect` | string | The dialect to analyse the folder as ([dialect-detection.md](dialect-detection.md)) |
| `libraryPaths` | multi-line or comma-separated paths | Extra directories added to the package search path ([package-loading.md](package-loading.md)) |
| `entryPoints` | multi-line or comma-separated paths | The project's "main" files; setting them disables the automatic `source`-graph W120 inheritance for the folder ([package-loading.md](package-loading.md)) |
| `extraCommands` | comma-separated command names | Command names to treat as defined |

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

Toggle individual LSP features.  The key list is `FeatureToggles::KEYS` in
`rust/tcl-lsp-server/src/lib.rs` plus `willSaveWaitUntil`; a key that is not
one of these is stored and never read.  Most default on; the opt-in ones are
`FeatureToggles::DEFAULT_OFF`, marked below.  In VS Code several of the
default-on keys inherit from the matching `editor.*` setting when unset.

| Key | Default | Description |
|-----|---------|-------------|
| `hover` | on | Hover information |
| `completion` | on | Code completion |
| `diagnostics` | on | Inline diagnostics |
| `semanticTokens` | on | Semantic token highlighting |
| `codeActions` | on | Quick fixes and refactorings |
| `definition` | on | Go to definition |
| `declaration` | on | Go to declaration |
| `implementation` | on | Go to implementation (TclOO method overrides) |
| `typeDefinition` | on | Go to type definition |
| `references` | on | Find references |
| `documentSymbols` | on | Document symbol outline |
| `workspaceSymbols` | on | Workspace symbol search |
| `documentHighlight` | on | Occurrence highlighting |
| `documentLinks` | on | Document links |
| `linkedEditingRange` | on | Linked editing range |
| `codeLens` | on | Code lens (inline reference counts, test runners) |
| `folding` | on | Code folding |
| `rename` | on | Rename symbol |
| `signatureHelp` | on | Function signature help |
| `selectionRange` | on | Smart selection |
| `callHierarchy` | on | Call hierarchy |
| `workspaceFileOps` | on | Workspace file operations (rewrite source paths on rename) |
| `inlayTypeHints` | **off** | Inferred-type inlay hints (variables, format specifiers).  The retired `inlayHints` key is accepted as an alias for it |
| `inlayParameterHints` | **off** | Parameter-name inlay hints at proc/method call sites |
| `willSaveWaitUntil` | **off** | Format on save via `textDocument/willSaveWaitUntil` |
| `xcDiagnostics` | **off** | F5 XC translatability diagnostics; the same flag the `[xcDiagnostics]` section sets |
| `crossFileResolution` | **off** | Broader, bare-name workspace W123 inference (independent of `xcDiagnostics`, which is F5 XC Migration-specific). Exact C Tcl command candidates, including their cross-file E002/E003 arity checks, resolve without it; the opt-in setting only adds a deliberately lossier fallback. A workspace-injected `::tcl::mathfunc` override also resolves without it: that namespace is one table per interpreter, so the suppression is a language fact, not a cross-file inference |

Document formatting is not a feature toggle — there is no `formatting` key.

### `[diagnosticSeverity]`

One `CODE = severity` entry per diagnostic whose severity you want to
override:

```ini
[diagnosticSeverity]
W111 = hint
E002 = warning
```

Accepted values are `error`, `warning`, `information`, `info` and `hint`,
case-insensitively.  An unrecognised value is skipped and leaves the code's
emitted severity untouched.

### `[signatureHelp]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `disabled_commands` | comma- or whitespace-separated command names | (none) | Suppress automatic signatures for selected built-in commands (for example `set, incr`) while retaining other built-ins and user-defined proc signatures. Names are case-sensitive and canonicalised by `tcl-syntax::naming`, the shared Tcl qualified-name owner. |

### `[formatting]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `indent_size` | int | `4` | Spaces per indent level |
| `indent_style` | `spaces`/`tabs` | `spaces` | Indent character |
| `brace_style` | string | `k_and_r` | Brace placement style |
| `max_line_length` | int | `120` | Hard line length limit |
| `goal_line_length` | int | `100` | Soft wrapping target |
| `continuation_indent` | int | `4` | Indent for a continued line |
| `blank_lines_between_procs` | int | `1` | |
| `blank_lines_between_blocks` | int | `1` | |
| `max_consecutive_blank_lines` | int | `2` | |
| `line_ending` | string | `auto` | |
| `space_between_braces` | bool | `true` | |
| `space_after_comment_hash` | bool | `true` | |
| `trim_trailing_whitespace` | bool | `true` | |
| `enforce_braced_variables` | bool | `false` | |
| `ensure_final_newline` | bool | `true` | |
| `expand_single_line_bodies` | bool | `false` | |

These sixteen are the whole INI surface (`insert_formatting` in
`config_ini.rs`); `FormatterConfig` carries further fields an editor can set
through `tclLsp.formatting.*` but the INI parser does not read.

### `[style]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `line_length` | int | `120` | W111 line-length threshold |
| `nonAscii` | `off`/`strict`/`confusables`/`common` | per-dialect auto | W108 non-ASCII detection mode; an unknown value falls back to the auto behaviour |

### `[packages]` / `[packages.provides]`

How the modelled interpreter loads packages: `preferLatest` sets the starting
`package prefer` mode, and `[packages.provides]` maps a package name onto the
packages requiring it also loads.  Owned by
[package-loading.md](package-loading.md).

### `[iruleslx.plugins]` / `[iruleslx.rules]`

The iRulesLX plugin ↔ workspace association, keyed by plugin name.  Owned by
[iruleslx-remote-methods.md](../iruleslx-remote-methods.md).

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

| Concern | Where |
|---|---|
| INI parsing, layer sections, deep merge | `rust/tcl-lsp-server/src/config_ini.rs` — `settings_from_ini`, `Layer`, `merge_settings` |
| Config-path resolution | `rust/tcl-lsp-core/src/tcl_install.rs` — `user_config_path`, `project_config_path`, `config_path_for`, `library_paths_from_ini` |
| Layer application | `rust/tcl-lsp-server/src/lib.rs` — `Backend::apply_global_config`, and the folder-scoped overlay `Backend::resolved_feature_toggles` |
| Effective-config query | `rust/tcl-lsp-server/src/lib.rs` — `get_effective_config_command` |
| Inline / file-level suppression | `rust/tcl-compiler/src/analyser/utils.rs` — `parse_noqa_line_suppressions`, `apply_preceding_noqa` |
| Formatter settings schema | `rust/tcl-lsp-core/src/formatting/config.rs` |
| Editor-settings generation | `cargo xtask gen-editor-settings` / `gen-vscode-package` |

## Discoverability

- [Config precedence contract](config-precedence.md) — *why* the layers are
  ordered this way.
- [Dialect detection](dialect-detection.md)
- [How do I turn a diagnostic off?](../../kcs/kcs-howto-suppress-diagnostics.md)
