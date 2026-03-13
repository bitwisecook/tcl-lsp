# KCS: feature — Minifier

## Summary

Code minification: strips comments, collapses whitespace, joins commands with semicolons, recursively minifies body arguments.  Optional name compaction shortens variable and proc names with a symbol map for debugging.

## Surface

lsp, vscode-command, sublime-command, cli, all-editors

## How to use

- **VS Code command**: `Tcl: Minify Document` (Ctrl+Alt+M / Cmd+Alt+M) — prompts for basic or compact mode.
- **Sublime Text command**: `Tcl: Minify Document` via the command palette.
- **CLI**: `tcl minify script.tcl` (basic) or `tcl minify --compact script.tcl --symbol-map map.txt` (with name compaction).
- **LSP command**: `tcl-lsp.minifyDocument` — accepts `(uri, compact?, aggressive?)`, returns `{ source, originalLength, minifiedLength, symbolMap?, optimisationsApplied? }`.

## What it does

### Basic mode

1. Strips all comments (`#` to end of line).
2. Collapses inter-command whitespace (blank lines, indentation) to semicolons.
3. Collapses intra-command whitespace to single spaces.
4. Recursively minifies braced body arguments (proc bodies, if/while/for/foreach/switch blocks).
5. Preserves string literals, expressions, and command substitutions verbatim.

### Compact mode (`--compact`)

In addition to basic minification:

6. Renames local variables and parameters to short identifiers (a, b, c, ..., aa, ab, ...).
7. Renames internal proc names to short identifiers.
8. Emits a symbol map for debugging (original name ↔ compacted name).

**Safety rules for name compaction:**

- Variables in scopes containing `global`, `upvar`, `uplevel`, `eval`, `variable`, `trace`, `vwait`, or `tkwait` are **never** renamed (these commands reference variables by name at runtime).
- Namespace-qualified variables (`::var`, `ns::var`) are never renamed.
- Proc names that collide with Tcl builtins are skipped.
- Single-character names are already minimal and skipped.
- Package names (`package require`) are never renamed — they are external identifiers.

The output is semantically equivalent to the input.

### Aggressive mode (`--aggressive`)

In addition to compact mode:

9. Runs optimiser rewrites first (constant folding, dead-code cleanup, etc.) before compaction/minification.
10. Applies command/argument/string aliasing passes for repeated literals.
11. Applies dialect-aware ensemble subcommand abbreviation during rendering.

## Examples

### Basic minification: proc with comments (209 → 107 chars, 49% reduction)

**Input:**

```tcl
# Calculate the sum of a list of numbers
proc sum_list {numbers} {
    set total 0
    foreach num $numbers {
        # Accumulate each number
        set total [expr {$total + $num}]
    }
    return $total
}
```

**Output:**

```tcl
proc sum_list {numbers} {set total 0;foreach num $numbers {set total [expr {$total + $num}]};return $total}
```

### Basic minification: iRule routing (293 → 193 chars, 34% reduction)

**Input:**

```tcl
when HTTP_REQUEST {
    # Log the request
    log local0. "Request: [HTTP::uri]"

    # Route based on URI
    if {[HTTP::uri] starts_with "/api"} {
        pool api_pool
    } elseif {[HTTP::uri] starts_with "/static"} {
        pool static_pool
    } else {
        pool default_pool
    }
}
```

**Output:**

```tcl
when HTTP_REQUEST {log local0. "Request: [HTTP::uri]";if {[HTTP::uri] starts_with "/api"} {pool api_pool} elseif {[HTTP::uri] starts_with "/static"} {pool static_pool} else {pool default_pool}}
```

### Compact mode: proc + variable + param renaming

**Input:**

```tcl
proc calculate {alpha beta} {
    set result [expr {$alpha + $beta}]
    set doubled [expr {$result * 2}]
    return $doubled
}
set answer [calculate 10 20]
puts $answer
```

**Output:**

```tcl
proc a {a b} {set d [expr {$a + $b}];set c [expr {$d * 2}];return $c};set answer [a 10 20];puts $answer
```

**Symbol map:**

```text
# Procs
  a <- calculate
# Variables in ::calculate
  a <- alpha
  b <- beta
  c <- doubled
  d <- result
```

### Compact mode: barrier safety (global/upvar/eval)

**Input:**

```tcl
proc unsafe {} {
    global myvar
    set myvar 42
}
```

**Output (variables NOT renamed — scope has `global`):**

```tcl
proc a {} {global myvar;set myvar 42}
```

The proc name is still compacted, but the variables in the scope are left untouched because `global` creates a dynamic scope barrier.

### String content is preserved exactly

**Input:**

```tcl
set greeting "Hello,   World!"
set pattern {multiple   spaces   here}
puts $greeting
```

**Output:**

```tcl
set greeting "Hello,   World!";set pattern {multiple   spaces   here};puts $greeting
```

Note: spaces inside `"..."` and `{...}` string arguments are never touched.

### What cannot be renamed

- **Package names**: `package require http` — `http` is an external identifier, not renameable.
- **Namespace-qualified names**: `::ns::var`, `ns::proc` — kept as-is to preserve cross-namespace references.
- **Variables in barrier scopes**: any scope containing `global`, `upvar`, `uplevel`, `eval`, `variable`, `trace`, `vwait`, or `tkwait`.

## Sample files

Sample files are available in `samples/for_screenshots/`.

### Aggressive mode (`--aggressive`)

In addition to compact mode:

9. Runs all compiler optimisation passes (constant propagation, dead code elimination, expression folding, string chain folding, incr conversion, code sinking, unused proc removal).
10. **Static substring folding** — uses IR/CFG/SSA/SCCP to find quoted strings where every `$var` substitution resolves to a compile-time constant.  Also folds **pure command substitutions** like `[string length $x]`, `[expr {$a + $b}]`, `[format ...]` when all arguments are provably constant.  Taint analysis rejects folding unless the tainted value carries safe colours (IP_ADDRESS, PORT, FQDN, LIST_CANONICAL, REGEX_LITERAL).
11. **Dead-set elimination** — removes `set var value` commands whose variable was fully consumed by static folding and is no longer referenced.
12. Command aliasing — replaces repeated long command names with short variable aliases.
13. Argument aliasing — replaces repeated literal arguments with aliases.
14. String-literal substring aliasing — replaces repeated substrings in quoted strings with variable aliases.
15. Template deduplication — replaces repeated quoted strings containing dynamic content with `[subst $alias]`.

**Static substring folding examples:**

```tcl
# Variable folding (SCCP proves $level = "INFO"):
set level INFO
puts "$level: request started"    ;# → puts {INFO: request started}
puts "$level: request complete"   ;# → puts {INFO: request complete}
# After folding, `set level INFO` is dead and removed.

# Pure command folding:
set name hello
puts "length: [string length $name]"  ;# → puts {length: 5}
puts "upper: [string toupper $name]"  ;# → puts {upper: HELLO}

# Expression folding:
set x 10
set y 20
puts "sum: [expr {$x + $y}]"  ;# → puts {sum: 30}
```

The fold is rejected when:
- Any `$var` in the string is overdefined (assigned different values on different control-flow paths).
- A `[cmd]` substitution is impure or non-deterministic (e.g. `[clock seconds]`).
- Any input variable is tainted without safe sanitisation colours.

## Operational context

The minifier entrypoint is `minify_tcl(...)`, which in basic/compact modes returns a minified `str` (or `(str, SymbolMap)` when a symbol map is requested), and in aggressive mode returns a `MinifyResult`.  It uses the lexer's token stream to identify command boundaries and body arguments via the command registry.  Basic minification is idempotent.  Compact mode uses `analyse(source)` from the semantic model to safely identify renameable symbols, respecting scope barriers and builtin name collisions.  Aggressive mode additionally runs the full compiler pipeline (IR → CFG → SSA → SCCP + taint) to prove string arguments are static.

## File-path anchors

- `core/minifier/minifier.py`
- `core/minifier/static_substr.py` — static substring folding (IR/CFG/SSA/SCCP + taint)
- `core/common/text_edits.py` — shared text editing and name generation utilities
- `core/common/suffix_array.py` — shared suffix array and LCP construction
- `lsp/server.py` (`tcl-lsp.minifyDocument` command)
- `editors/vscode/src/extension.ts` (`minifyDocument` handler with basic/compact/aggressive modes)
- `editors/sublime-text/plugin.py` (`TclMinifyDocumentCommand`)
- `explorer/tcl_cli.py` (`minify` verb with `--compact` and `--symbol-map` flags)

## Failure modes

- Non-idempotent minification (re-minify changes output).
- Semantic changes (altered string content, lost commands).
- False rename in compact mode (variable name used dynamically in an undetected way).
- False static fold (SCCP proves constant but runtime behaviour differs due to trace/upvar — guarded by barrier detection).

## Test anchors

- `tests/test_minifier.py` — 130 tests covering basic minification, name compaction, barrier safety, symbol map, static substring folding (SCCP + pure command + dead-set elimination), string aliasing, and template deduplication.

## See also

- [KCS: Unminify Error](kcs-feature-unminify-error.md) — translate error messages from minified code back to original names using the symbol map.

## Discoverability

- [KCS feature index](README.md)
