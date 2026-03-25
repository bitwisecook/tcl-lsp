# KCS: Proc docstring handling

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

All parsing and rendering lives in `core/formatting/docstring.py`:

- `parse_docstring(text) -> DocstringInfo` -- parse raw comment text
- `render_markdown(info) -> str` -- render for LSP hover display
- `render_comment_block(info, ...) -> str` -- render as Tcl `#` comment
- `generate_stub(proc_name, params, ...) -> str` -- generate a template
- `extract_body_docstring(body) -> str` -- extract from proc body

The `DocstringInfo` dataclass provides structured access:
`brief`, `description`, `params: list[ParamDoc]`, `returns`.

## Formatter configuration

Five settings control docstring formatting (under `tclLsp.formatting.*`):

| Setting | Values | Default | Purpose |
|---------|--------|---------|---------|
| `docstringStyle` | `preceding`, `body`, `none` | `none` | Docstring placement |
| `docstringTagStyle` | `doxygen`, `plain`, `none` | `doxygen` | Tag format |
| `docstringDecoration` | boolean | `false` | Add border lines |
| `docstringDecorationChar` | `.`, `-`, `=`, `*`, `~` | `.` | Border character |
| `docstringDecorationWidth` | 20-120 | 70 | Border width |

Settings are defined in `FORMATTER_SETTINGS_CATALOGUE` in
`core/formatting/config.py` and code-generated into editor extensions.

## Code action

A "Generate docstring for 'name'" source action is offered when the cursor
is on a `proc` definition that has no docstring.  It inserts a doxygen-style
stub with `@param` tags for each parameter.

## MCP AI tools

Three tools expose docstring operations:

- **`generate_docstring`** -- generate a stub for a named proc
- **`read_proc_docs`** -- extract structured docs from all procs
- **`update_docstrings`** -- add stubs to all undocumented procs

The `_proc_to_dict` serialiser includes a `doc_structured` field with
the parsed `DocstringInfo` when a proc has documentation.

## CLI commands

`ai/claude/tcl_ai.py` adds:

- `proc-docs <file>` -- JSON output of all proc documentation
- `generate-docstring <file> --proc name` -- print docstring stub
- `update-docstrings <file>` -- print source with stubs added

## Comment bleed prevention

`_last_comment` is saved before and restored after proc body analysis in
the analyser, preventing comments inside one proc's body from leaking to
the next proc.
