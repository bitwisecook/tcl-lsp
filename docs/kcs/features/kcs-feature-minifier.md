# KCS: feature — Minifier

> **Audience:** User
> **Type:** Functionality

## Summary

Code minification: strips comments, collapses whitespace, joins commands with semicolons, and recursively minifies body arguments.  Optional name compaction shortens variable names (and, in isolated mode, procedure names) with a symbol map for debugging.  Each tier has an explicit, documented contract about what it may and may not change.

## Applies to

all-editors, tcl-lsp CLI, transform

## How to use

- **VS Code command**: `Tcl: Minify Document` (Ctrl+Alt+M / Cmd+Alt+M) — prompts for basic or compact mode.
- **Sublime Text command**: `Tcl: Minify Document` via the command palette.
- **CLI**: `tcl minify script.tcl` (basic) or `tcl minify --compact script.tcl --symbol-map map.txt` (with name compaction).
- **LSP command**: `tcl-lsp.minifyDocument` — accepts `(uri, compact?, aggressive?, isolated?)`, returns `{ source, originalLength, minifiedLength, symbolMap?, optimisationsApplied? }`.

## The tier contract — what each tier may change

| Tier | May rename symbols? | May add variables? | Semantics |
|---|---|---|---|
| Basic (default) | No | No | Equivalent, including reflection |
| Compact | Locals and parameters proven non-observable; procs only with `--isolated` | No | Equivalent, including reflection, under the fences below |
| Aggressive | As compact | **Yes** (alias preambles) | Equivalent output for programs that do not observe frames; `info vars`, variable traces, and host-interpreter variables can see the alias variables |

## What it does

### Basic mode

1. Strips all comments (`#` to end of line — in **script** position only).
2. Collapses inter-command whitespace (blank lines, indentation) to semicolons.
3. Collapses intra-command whitespace to single spaces.
4. Recursively minifies braced body arguments (proc bodies, if/while/for/foreach blocks).
5. Minifies the braced clause list of a `switch` (or Expect `expect`) with the Tcl **list** grammar: a braced case list is a list, not a script, so a `#` there is an ordinary pattern, never a comment.  Patterns, clause flags, and fall-through `-` markers are re-emitted exactly as written; only braced bodies are recursively minified.  A malformed (for example odd-length) case list is preserved verbatim so the runtime error is unchanged.
6. Preserves string literals, expressions, and command substitutions verbatim.
7. Never introduces variables, writes, or any other observable behaviour.

### Compact mode (`--compact`)

In addition to basic minification:

8. Renames proc-local variables and parameters to short identifiers (a, b, c, ..., aa, ab, ...).
9. With `--isolated` only: renames procedure names and global variables too.
10. Emits a symbol map for debugging (original name ↔ compacted name).

**Safety fences (all registry-declared, none keyed on command spellings):**

- **Procedure names are public command identities.**  `info procs`/`info commands`, `rename`, `namespace export`, `interp alias`, `unknown`, execution traces, and callers outside the file can all observe or invoke them, so procs are renamed only under `--isolated` (you assert the script is self-contained), and even then not when any command-name-reflecting command or a computed command name (`$cmd args`) appears anywhere in the script.
- **Array member names are data, never symbols.**  `arr(member)` keys are observable through `array get`/`array names`, traces, and serialization, so no tier ever renames them.
- **Scopes containing a dynamic-barrier command** (`global`, `upvar`, `uplevel`, `eval`, `variable`, `trace`, `vwait`, `tkwait`, …) or a variable-name introspection (`info locals`, `info vars`, `info exists`, …) are never renamed.
- **`upvar` / `uplevel` block every scope**: they reach a caller frame chosen at runtime, so any proc's locals may be observed by name while one exists.
- A variable whose name is the bare target of a read-modify-write (`incr`/`append`/`lappend`/`lset`) or destroy (`unset`) command keeps its name everywhere.
- Re-definition sites (`set x 2` after `set x 1`) and other bare-name reference sites are renamed in lock-step with the declaration and `$x` reads.
- Namespace-qualified variables (`::var`, `ns::var`) and locals aliasing a global/namespace cell (`global v`, `variable v`) are never renamed.
- Procs that override a registry-known command (`proc unknown …`) keep their name.

The output is semantically equivalent to the input, **including Tcl reflection results** (`info locals`, `info procs`, `array get`).  Where the analysis cannot prove a rename is unobservable, it does not rename — less compression, never different behaviour.

### Aggressive mode (`--aggressive`)

In addition to compact mode:

11. Runs optimiser rewrites first (constant folding, dead-code cleanup, etc.) before compaction/minification.
12. Applies command/argument/string aliasing passes for repeated literals.
13. Applies dialect-aware ensemble subcommand abbreviation during rendering.

**Aggressive mode is not frame-transparent.**  The aliasing passes inject `set alias value` preambles: these create real Tcl variables that are visible to `info vars`, fire variable traces, and would collide with a same-named variable that exists only in the hosting interpreter (one set before the minified script is sourced).  The alias generators avoid every name visible in the script itself — compacted shorts, every analysed or SSA-known variable name, and every `$name` reference — so a collision with a name that appears anywhere in the script cannot happen.  Use basic or compact mode when the script must not add variables.

## Examples

### Basic minification: proc with comments

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

### Basic minification: `#` switch patterns survive

A braced `switch` case list is a Tcl list, so `#` is a pattern there — real code uses it to dispatch on comment characters:

**Input:**

```tcl
switch $char {
    # {handleComment}
    default {handleOther}
}
```

**Output (the `#` arm is preserved — tclsh prints `handleComment` for `$char` = `#` before and after):**

```tcl
switch $char {# {handleComment} default {handleOther}}
```

### Compact mode: locals compact, public names survive

**Input:**

```tcl
proc calculate {alpha beta} {
    set result [expr {$alpha + $beta}]
    return $result
}
puts [info procs calculate]
```

**Output (locals renamed; `calculate` untouched — it is reflected by `info procs`):**

```tcl
proc calculate {a b} {set c [expr {$a+$b}];return $c};puts [info procs calculate]
```

**Symbol map:**

```text
# Variables in ::calculate
  a <- alpha
  b <- beta
  c <- result
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
proc unsafe {} {global myvar;set myvar 42}
```

### What is never renamed

- **Array member keys**: `set config(timeout) 30` — `timeout` is data `array get config` returns.
- **Package names**: `package require http` — `http` is an external identifier.
- **Namespace-qualified names**: `::ns::var` — kept to preserve cross-namespace references.
- **Variables in barrier scopes** and **all variables when `upvar`/`uplevel` is present** (see the fences above).
- **Procedure names**, except under `--isolated` with no reflection present.

## Limits

- The compact tier's observability analysis is static and conservative: any command the registry marks as reflecting names, evaluating dynamic code, or crossing frames disables renaming for the affected scopes.  Scripts that use such commands heavily compress less; they never break.
- Aggressive mode's alias variables cannot be proven absent from the *hosting* interpreter's frames — only from the script itself.  That is why the tier is documented as behaviour-changing.
- `isolated` is a user assertion, not something the tool verifies: only pass it for scripts with no external callers (e.g. a self-contained iRule event body or a standalone script).

## Operational context

The minifier entrypoints are `minify_tcl` (basic), `minify_tcl_compact` (compact, returns `(source, SymbolMap)`), and `minify_tcl_aggressive` (returns a `MinifyResult`), all in `rust/tcl-lsp-core/src/minify.rs`.  Basic minification is idempotent.  Compact mode uses the analyser's scope model to identify renameable symbols; every observability fact it consults is a `tcl-registry` trait (`CREATES_DYNAMIC_BARRIER`, `INTROSPECTS_BY_NAME`, `REFLECTS_COMMAND_NAMES`, `ALIASES_CALLER_FRAME`, `EVALUATES_IN_SHIFTED_FRAME`, `DESTROYS_VARIABLE`), never a command-name list in the minifier.  Case-list minification is driven by the registry's `CommandSpec::case_list` descriptor and the central list parser in `rust/tcl-syntax/src/list.rs`.

## File-path anchors

- `rust/tcl-lsp-core/src/minify.rs` — all three tiers
- `rust/tcl-registry/src/traits.rs` — the observability traits the fences consume
- `rust/tcl-syntax/src/list.rs`, `rust/tcl-syntax/src/case_list.rs` — the list grammar used for case lists
- `rust/tcl-lsp-server/src/lib.rs` (`tcl-lsp.minifyDocument` command)
- `editors/vscode/src/extension.ts` (`minifyDocument` handler)

## Failure modes

- Non-idempotent minification (re-minify changes output).
- Semantic changes (altered string content, lost commands, changed reflection results) — covered by C Tcl differential tests on the regression corpus.
- Aggressive-tier alias collisions with host-interpreter variables (documented limit, not a defect of the semantics-preserving tiers).

## Test anchors

- `rust/tcl-lsp-core/src/minify.rs` (`mod tests`) — tier contracts, switch `#` patterns, reflection fences, lock-step renames.
- `rust/tcl-lsp-core/tests/minify_residual.rs` — symbol map serialization, switch edges, aliasing edges.
- `rust/tcl-lsp-server/tests/e2e/commands.rs` — LSP `minifyDocument` end-to-end.
- `editors/vscode/src/test/commandExecution.test.ts` — editor-visible behaviour.

## See also

- [KCS: Unminify Error](kcs-feature-unminify-error.md) — translate error messages from minified code back to original names using the symbol map.

## Discoverability

- [KCS feature index](README.md)
