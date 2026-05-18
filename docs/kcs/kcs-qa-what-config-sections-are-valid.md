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
same INI schema. Seven sections are recognised. Any section or key
the parser does not know is silently ignored, so a typo turns into a
no-op rather than an error — check your config takes effect after
editing.

Keys inside INI sections are **snake_case** (`indent_size`,
`line_length`). The same settings exposed through editor settings
use **camelCase** (`indentSize`, `lineLength`); the server converts
between the two automatically.

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
`pullDiagnostics`, `willSaveWaitUntil`, `progress`,
`implementation`, `typeDefinition`, `declaration`,
`linkedEditingRange`.

A few of these (notably `pullDiagnostics`) only take effect after a
server restart because they change which LSP handlers are
registered.

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

The complete list, with defaults and ranges, lives in
[`core/formatting/config.py`](../../core/formatting/config.py).

### `[style]`

Style settings that affect linting but not formatting.

- `line_length` — integer.

### What you cannot put in an INI file

A handful of settings are only honoured when they come from editor
settings via `workspace/configuration`; the INI parser ignores them.
Today this includes:

- `tclLsp.dialect`, `tclLsp.extraCommands`, `tclLsp.libraryPaths`
  (top-level keys, not in any INI section).
- The `runtimeValidation`, `ai`, and `packageManager` sections.

If you set one of these in `config.ini` or `.tcl-lsp.ini` it has no
effect — use your editor's settings instead. See
[kcs-qa-how-tcl-lsp-loads-configuration.md](kcs-qa-how-tcl-lsp-loads-configuration.md)
for the full list of layers and where to put each kind of setting.

## Related

- [KCS index](README.md)
- [How does tcl-lsp load configuration, and what overrides what?](kcs-qa-how-tcl-lsp-loads-configuration.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [Per-code catalogue](codes/README.md)
- [Glossary](../GLOSSARY.md)
