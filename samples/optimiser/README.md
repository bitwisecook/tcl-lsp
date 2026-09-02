# Optimisation Profiles

This directory demonstrates the five optimisation profiles and the output each
produces from a single input file.

## Files

| File | Description |
|------|-------------|
| `input.tcl` | Source file exercising all 28 optimisation passes (O100-O127) |
| `profile_readability.tcl` | Output at **readability** — idiomatic rewrites only |
| `profile_standard.tcl` | Output at **standard** — readability + constant folding |
| `profile_full.tcl` | Output at **full** — all passes, single pass |
| `profile_aggressive.tcl` | Output at **aggressive** — all passes, multi-pass to fixpoint |
| `deep_pipeline.tcl` | Deep multi-pass stress sample — layered so each optimisation exposes the next (5+ passes, interprocedural folding). Runs on C Tcl 9; the optimised / formatted / minified forms all print the same line (see its header comment). |

## Profiles

### `off`

All optimisations disabled. Output is identical to `input.tcl`.

### `readability` (editor default)

**Enabled codes:** O111, O114, O115, O117, O120 (5 codes)

Idiomatic Tcl rewrites only — no code removal or restructuring:

- `set x [expr {$x + N}]` &rarr; `incr x N`
- `[string length $s] == 0` &rarr; `$s eq ""`
- `==`/`!=` on strings &rarr; `eq`/`ne`
- Redundant nested `[expr {...}]` removed
- Unbraced `expr` bodies flagged (paired with W100)

These suggestions improve clarity without changing the structure of the code.
They never delete lines or introduce new variables.

### `standard`

**Enabled codes:** readability + O100-O105, O110, O113, O116, O118, O119 (16 codes)

Adds constant folding and pattern recognition on top of readability:

- `set timeout 30; set half [expr {$timeout / 2}]` &rarr; `set half 15`
- `[list red green blue]` &rarr; `{red green blue}`
- `[lindex {a b c} 1]` &rarr; `b`
- `expr {$r ** 2}` &rarr; `expr {$r * $r}` (strength reduction)
- String build chains folded into single `set`
- Expression canonicalisation (InstCombine)

Shows "this could be simpler" without deleting any code. Dead stores from
constant propagation remain in the output — the code is simplified but not
shortened.

### `full`

**Enabled codes:** all 28 codes, single pass

Adds dead-code elimination, code motion, and recursion transforms:

- Dead stores removed (`set stale 1` before `set stale 2`)
- Unreachable `if {0} { ... }` blocks removed
- Unused variable assignments removed
- Tail-recursive procs converted to `while` loops
- Loop-invariant code hoisted
- Single-use variables inlined

This profile changes the shape and length of the code. It was the previous
default for all surfaces.

### `aggressive`

**Enabled codes:** all 28 codes, multi-pass (up to 5 iterations)

Same passes as `full`, but after applying rewrites, the source is recompiled
and re-analysed to find opportunities exposed by earlier passes. For example:

1. Pass 1: Constant propagation replaces `$timeout` with `30` in expressions
2. Pass 1: Expression folding simplifies `30 / 2` to `15`
3. Pass 1: Dead store elimination removes the now-unused `set timeout 30`
4. Pass 2: The three consecutive `set half 15; set threshold 40; set route 42`
   are packed into `lassign {15 40 42} half threshold route` (O119)

The aggressive profile found **38 optimisations across 3 iterations** vs 31 in
single-pass `full` for the sample input. Convergence is typically 3-4
iterations even for large codebases; the multi-pass engine stops early when the
source text reaches a fixpoint (no further changes).

## Defaults per surface

| Surface | Default Profile | Rationale |
|---------|----------------|-----------|
| Editor diagnostics (squiggles) | `readability` | Non-intrusive while editing |
| `/optimise` chat command | `full` | Explicit user action |
| CLI `optimize` | `full` | Explicit user action |
| MCP `optimize` tool | `full` | AI-driven, explicit |
| AI skills | `full` | Explicit |

## Design decisions

1. **O111 is in readability** — bracing expressions is an idiomatic Tcl best
   practice and pairs with the W100 diagnostic warning.

2. **Editor default is `readability`** — 5 non-intrusive hints with no code
   deletion. Users who want more can change to `standard` or `full` in settings.

3. **`aggressive` is a separate profile** — not a `--multi-pass` flag. Keeps
   the UI simple (single dropdown) and avoids edge cases like multi-pass
   readability (which would be pointless).

4. **Descriptive profile names** — `off`/`readability`/`standard`/`full`/
   `aggressive`. Self-documenting; no compiler background needed.

5. **Profiles resolve to `disabled_optimisations` sets** — all internal
   plumbing continues using the existing per-code filtering mechanism.
   Profiles are a configuration-layer concept.

6. **Individual O1xx toggles override profiles** — three-state logic
   (`null`/`true`/`false`). `null` means "inherit from profile".

## Regenerating samples

```bash
for p in readability standard full aggressive; do
    tcl opt --profile "$p" samples/optimiser/input.tcl \
        > "samples/optimiser/profile_$p.tcl"
done
```

The committed outputs predate the Rust optimiser and have not been
regenerated: it is deliberately more conservative on some passes (O114's
`incr` rewrite, for one, now needs the target proven `TclType::Int`), so
re-running the loop above will not reproduce them byte-for-byte.
