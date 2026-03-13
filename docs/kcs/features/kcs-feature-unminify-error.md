# KCS: feature — Unminify Error

## Summary

Translate Tcl or iRule error messages produced by minified code back to the original variable, proc, and command names using a saved symbol map.  Optionally remaps line-number references from minified single-line output to approximate original source lines.

## Surface

lsp, vscode-command, sublime-command, cli, mcp, all-editors

## Availability

| Context | How |
|---------|-----|
| CLI | `tcl unminify-error --symbol-map map.txt --error 'can'\''t read "a"'` |
| CLI (file) | `tcl unminify-error --symbol-map map.txt --error-file /var/log/ltm --minified min.tcl --original src.tcl` |
| VS Code | `Tcl: Unminify Error Message` from the command palette or right-click context menu |
| Sublime Text | `Tcl: Unminify Error` from the command palette |
| LSP command | `tcl-lsp.unminifyError(error_message, symbol_map, minified_source?, original_source?)` |
| MCP tool | `unminify_error` — accepts `error_message`, `symbol_map`, optional `minified_source` and `original_source` |

## How to use

### Recommended workflow: keep artefacts together in VCS

When deploying minified iRules or Tcl scripts, **always check the following three files into version control together**:

```
irules/
  my_irule.tcl            # original source
  my_irule.min.tcl        # minified output
  my_irule.map.txt        # symbol map
```

This ensures that anyone debugging a production error can translate it back to the original source without needing to re-run the minifier or guess which version of the code was deployed.  Treat the minified output and symbol map as build artefacts — regenerate them from the original source, but keep all three in sync.

**Why version-control all three?**

- The minifier is deterministic: the same input always produces the same output and symbol map.  But if the original source changes and you only kept the minified version, the old symbol map won't match.
- iRule errors in `/var/log/ltm` reference the deployed (minified) code.  Without the matching symbol map you cannot translate `"a"` back to `"request_uri"`.
- Keeping all three together means any team member can run `tcl unminify-error` without needing the build toolchain.

**Git workflow example:**

```bash
# Minify with symbol map
tcl minify --aggressive src/my_irule.tcl -o deploy/my_irule.min.tcl --symbol-map deploy/my_irule.map.txt

# Commit all three together
git add src/my_irule.tcl deploy/my_irule.min.tcl deploy/my_irule.map.txt
git commit -m "Update my_irule: add rate limiting"
```

### Translating an error

**CLI — inline error:**

```bash
tcl unminify-error \
  --symbol-map deploy/my_irule.map.txt \
  --error 'can'\''t read "a": no such variable'
```

Output:

```
can't read "request_uri": no such variable
```

**CLI — error log file with line remapping:**

```bash
tcl unminify-error \
  --symbol-map deploy/my_irule.map.txt \
  --minified deploy/my_irule.min.tcl \
  --original src/my_irule.tcl \
  --error-file /var/log/ltm
```

**VS Code:**

1. Open any Tcl file (or use the command palette directly).
2. Run `Tcl: Unminify Error Message`.
3. Paste the error message when prompted.
4. Select the symbol map file.
5. Optionally enable line-number remapping by providing the minified and original source files.
6. The translated error opens in a side-by-side panel.

**MCP (AI tool use):**

```json
{
  "tool": "unminify_error",
  "arguments": {
    "error_message": "can't read \"a\": no such variable",
    "symbol_map": "# Procs\n  a <- calculate\n# Variables in ::calculate\n  a <- alpha\n  b <- beta"
  }
}
```

## What it does

1. **Parses the symbol map** — either a `SymbolMap` object or the human-readable text format produced by `--symbol-map`.
2. **Builds a reverse lookup** — maps each compacted identifier back to its original name (procs, variables, command aliases, argument aliases, string aliases).
3. **Replaces identifiers in the error message:**
   - Quoted identifiers: `"a"` → `"request_uri"`
   - Dollar-prefixed variables: `$a` → `$request_uri`
4. **Remaps line numbers** (when minified + original sources provided):
   - Minified code is typically one long line, so Tcl errors report "line 1".
   - The function maps proportionally back to the original source line.
   - Output: `(procedure "test" line 42, minified line 1)`

### Error message patterns handled

| Pattern | Example |
|---------|---------|
| Tcl variable error | `can't read "a": no such variable` |
| Tcl command error | `invalid command name "a"` |
| Tcl wrong args | `wrong # args: should be "a x y"` |
| iRule log entry | `/Common/my_irule:1: can't read "a"` |
| Tcl errorInfo | `(procedure "a" line 1)` |
| Dollar references | `error near $a` |

## Examples

### Basic: variable name translation

**Symbol map** (`map.txt`):

```text
# Variables in ::
  a <- request_uri
  b <- response_code
```

**Error:**

```
can't read "a": no such variable
```

**Translated:**

```
can't read "request_uri": no such variable
```

### Aggressive mode: proc + variable + command alias

**Symbol map:**

```text
# Procs
  a <- handle_request
# Variables in ::handle_request
  a <- uri_path
  b <- pool_name
# Command aliases
  $c <- HTTP::uri
```

**Error:**

```
/Common/my_irule:1: invalid command name "a", can't read "c"
```

**Translated:**

```
/Common/my_irule:1: invalid command name "handle_request", can't read "HTTP::uri"
```

### With line remapping

**Original** (`src.tcl`):

```tcl
# Request handler
proc handle_request {uri} {
    set pool [lookup $uri]
    pool $pool
}
```

**Minified** (`min.tcl`):

```tcl
proc a {a} {set b [lookup $a];pool $b}
```

**Error:**

```
(procedure "a" line 2)
```

**Translated:**

```
(procedure "handle_request" line 4, minified line 2)
```

## Operational context

The `unminify_error` function is a pure function that takes an error string and a symbol map, applies regex-based substitution of compacted identifiers, and optionally remaps line references using proportional mapping between minified command counts and original source lines.

The `SymbolMap.parse()` method is the inverse of `SymbolMap.format()` — it reads the human-readable text format back into a `SymbolMap` object.  The `SymbolMap.reverse()` method builds the compacted→original lookup dictionary used for translation.

## File-path anchors

- `core/minifier/minifier.py` (`unminify_error`, `SymbolMap.parse`, `SymbolMap.reverse`)
- `core/minifier/__init__.py` (exports `unminify_error`)
- `explorer/tcl_cli.py` (`unminify-error` verb with `--symbol-map`, `--error`, `--error-file`, `--minified`, `--original`)
- `lsp/server.py` (`tcl-lsp.unminifyError` command)
- `ai/mcp/tcl_mcp_server.py` (`unminify_error` MCP tool)
- `editors/vscode/src/extension.ts` (`unminifyError` handler)
- `editors/vscode/package.json` (`tclLsp.unminifyError` command registration)
- `editors/sublime-text/plugin.py` (`TclUnminifyErrorCommand`)

## Failure modes

- Symbol map file doesn't match the deployed minified code (different version).
- Short identifier collides with a real word in the error text (false positive replacement).
- Line remapping is approximate — proportional mapping may be off for code with large comment blocks or uneven command density.

## Test anchors

- `tests/test_minifier.py` — `TestSymbolMapParse` (round-trip format/parse) and `TestUnminifyError` (error translation, variable/proc/alias substitution, line remapping).

## Discoverability

- [KCS feature index](README.md)
- [KCS: Minifier](kcs-feature-minifier.md)
