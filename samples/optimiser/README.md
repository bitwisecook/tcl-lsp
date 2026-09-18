# Optimisation Profiles

This directory demonstrates the five optimisation profiles and the output each
produces from a single input file.

## Files

| File | Description |
|------|-------------|
| `input.tcl` | Source file exercising the optimisation passes (O100-O130) |
| `profile_readability.tcl` | Output at **readability** — idiomatic rewrites only |
| `profile_standard.tcl` | Output at **standard** — readability + constant folding |
| `profile_full.tcl` | Output at **full** — all passes, single pass |
| `profile_aggressive.tcl` | Output at **aggressive** — all passes, multi-pass to fixpoint |
| `deep_pipeline.tcl` | Deep multi-pass stress sample — layered so each optimisation exposes the next (5+ passes, interprocedural folding). Runs on C Tcl 9; the optimised / formatted / minified forms all print the same line (see its header comment). |

## Profiles

### `off`

All optimisations disabled. Output is identical to `input.tcl`.

### `readability` (editor default)

**Enabled codes:** O111, O114, O115, O117, O120, O128 (6 codes)

Idiomatic Tcl rewrites only — no code removal or restructuring:

- `set x [expr {$x + N}]` &rarr; `incr x N` (O114) — the target must be provably
  `TclType::Int`, since `expr` promotes a float operand where `incr` errors
- `[string length $s] == 0` &rarr; `$s eq ""` (O117)
- `==`/`!=` on strings &rarr; `eq`/`ne` (O120)
- Redundant nested `[expr {...}]` removed (O115) — in a branch condition **and**
  in a `return` body (`propagation::try_fold_return_terminator`); a `set` value
  position is the known gap. The sample's `return [expr {[expr {$x * 2}]}]` is
  nevertheless left alone until `aggressive`, and the reason is elsewhere in
  the file — see [O115 and the `factorial` stanza](#o115-and-the-factorial-stanza)
- Unbraced `expr` bodies flagged (O111, paired with W100)
- `end`-relative index rewrites (O128)

These suggestions improve clarity without changing the structure of the code.
They never delete lines or introduce new variables. **3 rewrites** on the
sample input.

### `standard`

**Enabled codes:** readability + O100-O105, O110, O113, O116, O118, O119, O129,
O130 (19 codes)

Adds constant folding and pattern recognition on top of readability:

- String build chains folded into a single `set` (O104)
- Consecutive `set`s packed into `lassign` (O119)
- Expression canonicalisation (O110 InstCombine) and strength reduction (O113).
  The sample's `# O113` stanza writes its expression as `return [expr {$r ** 2}]`
  and is **not** rewritten: no expression rewriter visits a `return` body
  (issue #1962). The same expression in a `set` becomes `$r * $r`.

Shows "this could be simpler" without deleting any code. Dead stores from
constant propagation remain in the output — the code is simplified but not
shortened. **18 rewrites** on the sample input.

Note what this profile does *not* do to the sample, because it is a **single**
pass: `set half [expr {$timeout / 2}]` still reads `$timeout`. Folding it to
`set half 15` needs the constant propagated first and the arithmetic folded
after, which is two passes — so those folds appear only under `aggressive`
below. The summary footer lists the propagations as O102 because they were
*found*; a hint-only entry is advice, not an applied edit.

### `full`

**Enabled codes:** all 31 codes, single pass

Adds dead-code elimination, code motion, and recursion transforms:

- Dead stores removed (`set stale 1` before `set stale 2`)
- Unreachable `if {0} { ... }` blocks removed
- Unused variable assignments removed
- Fully tail-recursive procs rewritten to an iterative `while {1}` loop
  (O122), the recursive call becoming a `lassign` that reassigns the
  parameters. *Overlap selection* prefers this whole-proc rewrite over the
  per-site O121 `tailcall` covering the same range, which is why the sample
  shows a loop and not `tailcall`; `tail_call_loop_conversion_o122` pins this
  body
- Loop-invariant code hoisted
- Single-use variables inlined

This profile changes the shape and length of the code.

### `aggressive`

**Enabled codes:** all 31 codes, multi-pass (up to 5 iterations)

Same passes as `full`, but after applying rewrites, the source is recompiled
and re-analysed to find opportunities exposed by earlier passes. For example:

1. Pass 1: Constant propagation replaces `$timeout` with `30` in expressions
2. Pass 1: Expression folding simplifies `30 / 2` to `15`
3. Pass 1: Dead store elimination removes the now-unused `set timeout 30`
4. Pass 2: `set second beta` and `set colours {red green blue}` fold, now that
   their arguments are literals

The three folded assignments (`half`, `threshold`, `route`) are **not** packed
into a `lassign`: O119 packs consecutive `set`s of *literals as written*, and
these become literals only after folding. They are not consecutive either —
the committed output reads `set half 15`, `set threshold 40`,
`set candidate [expr {$request_count + 3}]`, `set route 42`, so the
non-literal `candidate` assignment separates `route` from the other two. The
`lassign` in the committed output is the O119 stanza's own
`set a 1; set b 2; set c 3`.

The aggressive profile finds **42 rewrites** on the sample input against 24 in
single-pass `full`. It is the only profile that folds the arithmetic through:
`set half 15`, `set threshold 40`, `set colours {red green blue}`,
`set second beta`, and the whole `set count 0; incr count; puts $count` stanza
down to `puts 1`. Convergence is typically 3-4 iterations even for large
codebases; the multi-pass engine stops early when the source text reaches a
fixpoint (no further changes).

## Defaults per surface

| Surface | Default Profile | Rationale |
|---------|----------------|-----------|
| Editor diagnostics (squiggles) | `readability` | Non-intrusive while editing |
| `/optimise` chat command | `full` | Explicit user action |
| CLI `optimize` | `full` | Explicit user action |
| MCP `optimize` tool | `full` | AI-driven, explicit |
| AI skills | `full` | Explicit |

## O115 and the `factorial` stanza

O115 fires on `proc double_expr {x} { return [expr {[expr {$x * 2}]}] }` under
**every** profile when that proc is the whole file — `readability` included.
In this sample it only survives to `aggressive`, and nothing about the proc or
the profile explains it: the cause is 90 lines further down.

O115 (like O101 and O129) is gated on `expr` being provably untouched across
the whole module. A command head the analysis cannot resolve could `rename`
`expr`, so an unresolvable head anywhere turns the gate off for the entire
file. `factorial`'s `return [factorial …]` is such a head, and it suppresses
O115 in `double_expr`. Measured on this input:

| file | O115 under `readability` |
| --- | --- |
| `double_expr` alone | fires |
| `double_expr` + a call to a defined proc or a builtin | fires |
| `double_expr` + a call to an unresolvable head | **not reported** |
| `input.tcl` with the `factorial` stanza removed | fires |
| the committed `profile_full.tcl` output, re-optimised | fires |

The last row is why `aggressive` gets it: `full` rewrites the recursion to
`tailcall factorial …` in pass 1, which removes the command substitution; pass
2 re-analyses, `expr` is trusted again, and O115 is reported. A single-pass
profile never gets a second look.

This is conservative rather than wrong — the gate exists so a renamed `expr`
is never folded as if it were the builtin — but the blast radius is the whole
module for a single unresolved head. Recorded as part of #1962.

## Design decisions

1. **O111 is in readability** — bracing expressions is an idiomatic Tcl best
   practice and pairs with the W100 diagnostic warning.

2. **Editor default is `readability`** — 6 non-intrusive hints with no code
   deletion. Users who want more can change to `standard` or `full` in settings.

3. **`aggressive` is a separate profile**, not a `--multi-pass` flag — one
   dropdown, and no multi-pass readability (which would be pointless).

4. **Profiles resolve to `disabled_optimisations` sets**, so every consumer
   keeps using per-code filtering. A profile is a configuration-layer concept.

5. **Individual O1xx toggles override profiles** — three-state logic
   (`null`/`true`/`false`). `null` means "inherit from profile".

## Regenerating samples

```bash
for p in readability standard full aggressive; do
    tcl opt --profile "$p" samples/optimiser/input.tcl \
        > "samples/optimiser/profile_$p.tcl"
done
```

The committed outputs are exactly what that loop produces;
`samples_optimiser_profiles_are_regenerated` in `rust/tcl-cli/tests/cli.rs`
fails if they drift.
