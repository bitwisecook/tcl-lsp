# KCS: What sections and keys are valid in tcl-lsp config files?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Question

What sections can I put in `config.ini` or `.tcl-lsp.ini`, what keys
does each section accept, and what values are valid?

## Answer

Both the global `config.ini` and the project `.tcl-lsp.ini` use the
same INI schema. Eleven sections are recognised in total — nine of
them work in either file, plus one location-specific section for each
file (`[global]` and `[project]`). Any section or key the parser does
not know is silently ignored, so a typo turns into a no-op rather
than an error — check your config takes effect after editing.

Keys inside INI sections are **snake_case** (`indent_size`,
`line_length`). The same settings exposed through editor settings
use **camelCase** (`indentSize`, `lineLength`); the server converts
between the two automatically.

### `[global]` (only in `config.ini`)

Top-level settings that have no other natural section. Only honoured
when this section appears in the **global** XDG `config.ini`; a
`[global]` section in `.tcl-lsp.ini` is logged and ignored.

- `dialect` — default dialect for files that have no per-file hint.
  One of `tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `tcl9.1`, `f5-irules`, `expect`.
- `extraCommands` — comma- or newline-separated list of extra Tcl
  command names the analyser should recognise.
- `libraryPaths` — one path per line, or comma-separated for one-line
  values. Extra directories for the package and source resolver.
- `entryPoints` — one path per line, or comma-separated for one-line
  values, relative to the folder root (or absolute). The project's
  "main" files that run the `package require`s and `source` the rest.
  When set, every file inherits these entries' `package require`s for
  the missing-`package require` check
  ([W120](codes/kcs-diagnostic-w120-missing-package-require.md)), and
  the automatic `source`-graph inheritance is turned off. Most useful
  in `.tcl-lsp.ini` (`[project]`).

### `[project]` (only in `.tcl-lsp.ini`)

Exactly the same keys as `[global]`, but only honoured when the
section appears in the **project** `.tcl-lsp.ini`. A `[project]`
section in `config.ini` is logged and ignored.

The two section names are deliberately different so that copying a
file between locations does not silently change which precedence
layer the values occupy — see
[kcs-qa-how-tcl-lsp-loads-configuration.md](kcs-qa-how-tcl-lsp-loads-configuration.md)
for the location-based safeguard this mirrors.

### `[diagnostics]`

Controls which diagnostic codes the analyser reports.

- `disabled` — comma- or whitespace-separated list of codes to turn
  off. Codes come from the **E**, **W**, **S**, **T**, **IRULE**,
  **IAPP**, **BIGIP**, **TK**, and **XC** families. The full
  per-code catalogue lives under
  [`docs/kcs/codes/`](codes/README.md).
- `generic_variable_patterns` — one regex per line, matched
  case-insensitively against bare `static::` variable names. Use
  this to recognise project-specific globals such as `dbg_level`.
- `exclude` — one glob pattern per line (not comma-separated, because
  `{a,b}` alternation already uses commas). Suppresses every
  diagnostic for a matching file; navigation, hover, and formatting
  are unaffected. A pattern with a `/` matches relative to the
  workspace folder root; a pattern with no `/` matches the file name
  at any depth. See
  [how do I turn off all diagnostics for certain files?](kcs-howto-exclude-files-from-diagnostics.md).

### `[optimiser]`

Controls the optimiser pipeline.

- `enabled` — boolean. Turn the optimiser off entirely.
- `profile` — one of `off`, `readability`, `standard`, `full`,
  `aggressive`. Picks the default set of optimisations.
- `disabled` — comma- or whitespace-separated list of O-codes
  (`O100`–`O127`) to turn off, on top of the profile.

### `[shimmer]`

Controls the [shimmer](../GLOSSARY.md#shimmer) analyser.

- `enabled` — boolean.

### `[xcDiagnostics]`

Controls iRule-to-F5-XC (cross-compilation) diagnostics. Only
relevant if you target F5 XC.

- `enabled` — boolean.

### `[features]`

One boolean toggle per LSP feature. Setting a key to `false` makes
the server stop advertising or running that feature. Valid keys:
`hover`, `completion`, `diagnostics`, `semanticTokens`,
`codeActions`, `definition`, `references`, `documentSymbols`,
`folding`, `rename`, `signatureHelp`, `workspaceSymbols`,
`inlayHints`, `callHierarchy`, `documentLinks`, `selectionRange`,
`documentHighlight`, `codeLens`, `workspaceFileOps`,
`willSaveWaitUntil`, `progress`,
`implementation`, `typeDefinition`, `declaration`,
`linkedEditingRange`, `crossFileResolution`.

Delivery-model changes are not configuration toggles: diagnostics use push
publication, while pull requests are supported only when a client requests
them directly.

`crossFileResolution` (default off) enables the broader, bare-name
workspace inference for W123. Exact cross-file command candidates are already
resolved by default, using the same C Tcl lookup order as navigation, and can
therefore also report cross-file E002/E003 arity errors. Enable this setting
only when your workspace is intentionally one program and you want a bare
name to match a definition in another namespace. It is independent of
`[xcDiagnostics]` above — that section is F5 XC Migration-specific and has no
effect on plain Tcl projects.

### `[formatting]`

Controls the formatter. The most commonly tuned keys are:

- `indent_size` — integer, 1–16.
- `indent_style` — `spaces` or `tabs`.
- `continuation_indent` — integer, 1–16.
- `brace_style` — currently `k_and_r`.
- `max_line_length`, `goal_line_length` — integers, both ≥ 40.
- `line_ending` — `lf`, `crlf`, or `cr`.
- `ensure_final_newline`, `trim_trailing_whitespace`,
  `space_after_comment_hash`, `space_between_braces`,
  `align_comments_to_code`, `enforce_braced_variables`,
  `enforce_braced_expr`, `expand_single_line_bodies`,
  `docstring_decoration` — booleans.
- `docstring_style` — `preceding`, `body`, or `none`.
- `docstring_tag_style` — `doxygen`, `plain`, or `none`.

The complete list, with defaults and ranges, is in the
[configuration contract](../design/contracts/xdg-config.md).

### `[style]`

Style settings that affect linting but not formatting.

- `line_length` — integer.

### `[iruleslx.plugins]` and `[iruleslx.rules]`

iRulesLX only. They tell the server which directory holds a plugin's
sources, so `ILX::call`/`ILX::notify` can navigate to the
`ILXServer.addMethod` that implements the method.

Every key is a **plugin name** — the `PLUGIN` word of
`ILX::init PLUGIN EXTENSION` — so there is no fixed key list:

```ini
[iruleslx.plugins]
prod_plugin = workspaces/ws_alpha

[iruleslx.rules]
prod_plugin =
    irules/http
    irules/tcp
```

- `[iruleslx.plugins]` — `PLUGIN = <workspace directory>`. Needed only
  when the plugin's name differs from the directory that holds its
  `extensions/`; when the two match, navigation already works with no
  configuration. A declared plugin ignores the directory-name rule
  entirely, so this can also correct a directory that happens to
  collide with a plugin name.
- `[iruleslx.rules]` — `PLUGIN = <directory>[, <directory>…]`, one per
  line or comma-separated. Extra directories to search for iRules that
  call this plugin, on top of the workspace's own `rules/`. Use it when
  your repository keeps its iRules outside the workspace it builds.
  These directories *are* searched recursively (to 8 levels); `rules/`
  itself is not. An entry for a plugin with no `[iruleslx.plugins]`
  entry is ignored.

Paths are relative to the folder the config file is in; absolute paths
are taken as given. In the global `config.ini` prefer absolute paths —
a relative one resolves against whichever workspace folder is reading
it. The same settings are available to editors as `tclLsp.iruleslx`
(`{ "plugins": {…}, "rules": {…} }`).

### What you cannot put in an INI file

A handful of settings are only honoured when they come from editor
settings via `workspace/configuration`; the INI parser ignores them.
Today this includes the `runtimeValidation`, `ai`, and
`packageManager` sections.

If you set one of these in `config.ini` or `.tcl-lsp.ini` it has no
effect — use your editor's settings instead. See
[kcs-qa-how-tcl-lsp-loads-configuration.md](kcs-qa-how-tcl-lsp-loads-configuration.md)
for the full list of layers and where to put each kind of setting.

## Related

- [KCS index](README.md)
- [How does tcl-lsp load configuration, and what overrides what?](kcs-qa-how-tcl-lsp-loads-configuration.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [How do I turn off all diagnostics for certain files?](kcs-howto-exclude-files-from-diagnostics.md)
- [Per-code catalogue](codes/README.md)
- [iRulesLX remote methods](../design/iruleslx-remote-methods.md)
- [Glossary](../GLOSSARY.md)
