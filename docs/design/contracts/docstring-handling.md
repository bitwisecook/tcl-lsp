# Proc docstring handling

## Summary

The LSP extracts, parses, and displays documentation comments (docstrings)
for Tcl `proc` definitions.  Docstrings appear in hover, completion, and
signature help.  The formatter can generate docstring stubs, and AI tools
provide structured access to parsed documentation.

## Docstring sources

Docstrings are extracted from two locations (in priority order):

1. **Preceding comment** -- a contiguous block of `#` lines directly above
   the `proc` statement (no blank line between comment and proc).
2. **Body-internal comment** -- the first comment block inside the proc body.
   Decoration-only lines (e.g. `# ....`, `########`, `# ----`) are stripped.

Multi-line comments are accumulated by the segmenter (`preceding_comment`
field on `SegmentedCommand`).  A blank line between comment blocks resets
accumulation.

## Supported tag formats

The `@param`, `@return`/`@returns`, and `@brief` doxygen-style tags are
recognised and rendered as structured markdown:

```tcl
# @brief Calculate the sum
# @param a - First number
# @param b - Second number
# @return The sum
proc add {a b} { expr {$a + $b} }
```

Plain-prose docstrings (no tags) are also supported and displayed verbatim.

## Shared docstring module

All parsing and rendering lives in one consumer-agnostic module,
`rust/tcl-lsp-core/src/formatting/docstring.rs`:

- `parse_docstring(text) -> DocstringInfo` — parse raw comment text
- `render_comment_block(info, …)` — render back to Tcl `#` comment lines
  (Doxygen or plain style, with optional decoration)
- `generate_stub_for_proc(…)` — generate a stub from a `ProcDef`
- `resolve_tag_style(name)` — map the setting string to `DocstringTagStyle`

`DocstringInfo` carries `brief`, `description`, `params: Vec<ParamDoc>`, and
`returns`. Decoration-only rule lines (`# ......`, `# ----`) are recognised by
their character set (`DECORATION_CHARS`) and skipped when parsing description
text, so they never leak into the rendered hover.

## Configuration

Five settings live under `tclLsp.formatting.*`. They split by consumer:
`docstringTagStyle` and the `docstringDecoration*` fields drive the stub
generator's *content*; `docstringStyle` drives the code action's *placement*.
The formatter itself never rewrites an existing docstring, so none of these
affect a plain format pass — only the explicit generate-docstring action.

| Setting | Values | Default | Purpose |
|---------|--------|---------|---------|
| `docstringStyle` | `preceding`, `body`, `none` | `none` | Docstring placement |
| `docstringTagStyle` | `doxygen`, `plain`, `none` | `doxygen` | Tag format |
| `docstringDecoration` | boolean | `false` | Add border lines |
| `docstringDecorationChar` | `.`, `-`, `=`, `*`, `~` | `.` | Border character |
| `docstringDecorationWidth` | 20-120 | 70 | Border width |

They are declared once on `FormatterConfig`
(`rust/tcl-lsp-core/src/formatting/config.rs`, with `DocstringStyle` and
`DocstringTagStyle`) and code-generated into the editor extensions by
`cargo xtask gen-editor-settings`.

## Code action

A "Generate docstring for 'name'" source action is offered when the cursor
is on a `proc` definition that has no docstring.  It inserts a doxygen-style
stub with `@param` tags for each parameter.

The resolved `docstringStyle` setting gates both *whether* and
*where* this action fires, in the LSP server
(`rust/tcl-lsp-server`, `Backend::resolved_docstring_style` ->
`tcl_lsp_core::code_actions::code_actions_in_program`):

- `none` (the default) — the action is not offered at all.
- `preceding` — the stub is inserted directly above the `proc` line, as a
  standalone comment block (the only placement the plain `code_actions()`
  entry point used by the MCP `code_actions` tool and by tests offers).
- `body` — the stub is inserted as the first line inside the `proc` body,
  indented to match the body's existing content (four spaces when the body
  has none to match, e.g. an empty or single-line proc).

Only the LSP server's `codeAction` request resolves this setting; the MCP
`generate_docstring` / `update_docstrings` tools have no client config to
read and always use `preceding` placement.

## MCP AI tools

Three tools expose docstring operations:

- **`generate_docstring`** -- generate a stub for a named proc
- **`read_proc_docs`** -- extract structured docs from all procs
- **`update_docstrings`** -- add stubs to all undocumented procs

All three are registered in `rust/tcl-mcp/src/tools.rs` and serialise a proc's
parsed `DocstringInfo` alongside its signature.

## Comment bleed prevention

The analyser saves and restores its pending-comment state around a proc body
walk, so a comment inside one proc's body cannot become the docstring of the
next proc.

## Key files

| File | Role |
|---|---|
| `rust/tcl-lsp-core/src/formatting/docstring.rs` | parsing, rendering, stub generation |
| `rust/tcl-lsp-core/src/formatting/config.rs` | `DocstringStyle`, `DocstringTagStyle`, the five settings |
| `rust/tcl-lsp-server/src/lib.rs` | `Backend::resolved_docstring_style`, the code-action wiring |
| `rust/tcl-mcp/src/tools.rs` | the three MCP docstring tools |
| `rust/tcl-lsp-server/tests/e2e/code_actions.rs`, `e2e/hover.rs` | over-the-wire coverage |
