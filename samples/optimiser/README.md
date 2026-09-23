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
  rewritten here, which it was **not** until the `factorial` stanza stopped
  switching the module-wide `expr`-trust gate off — see
  [O115 and the `factorial` stanza](#o115-and-the-factorial-stanza)
- Unbraced `expr` bodies flagged (O111, paired with W100)
- `end`-relative index rewrites (O128)

These suggestions improve clarity without changing the structure of the code.
They never delete lines or introduce new variables. **4 rewrites** on the
sample input.

### `standard`

**Enabled codes:** readability + O100-O105, O110, O113, O116, O118, O119, O129,
O130 (19 codes)

Adds constant folding and pattern recognition on top of readability:

- String build chains folded into a single `set` (O104)
- Consecutive `set`s packed into `lassign` (O119)
- Expression canonicalisation (O110 InstCombine) and strength reduction (O113).
  The sample's `# O113` stanza writes its expression as `return [expr {$r ** 2}]`.
  A `return` body is now visited by the same rewriters as a `set` body
  (issue #1962, fixed), and that proc **on its own** is rewritten to
  `return [expr {$r * $r}]` under this profile.
  It is rewritten *here* too. It used not to be: the `factorial` stanza's
  recursive `return [factorial …]` was read as an unresolvable command head,
  which switched the module-wide `expr`-trust gate off for the whole file, and
  only `aggressive` got past it — its first pass rewrote that call to
  `tailcall`, removing the substitution, and its second pass found `expr`
  trusted again. A self-call is now resolved like any other, so the gate stays
  on and a single pass suffices. See
  [O115 and the `factorial` stanza](#o115-and-the-factorial-stanza).
  Note also that the rewrite is reported as **O110**, not O113: instcombine runs
  before strength reduction and claims `$r ** 2` → `$r * $r`. That is true of
  the `set` form too, and always has been; the stanza's `# O113` label names the
  family, not the code that fires.

Shows "this could be simpler" without deleting any code. Dead stores from
constant propagation remain in the output — the code is simplified but not
shortened. **28 rewrites** on the sample input.

One of those is the `passthrough` stanza's `O100 Fold return of constant
variable`: `set route [passthrough 42]` is the proc's only call, so the
specialiser proves `x` is `42` there and `return $x` becomes `return 42`. The
call site is a *nested* substitution, and until #2134 the call-site evidence
walk only saw a call written as a whole statement, so this rewrite is newer
than the rest of the stanza's prose.

This profile used to leave `set half [expr {$timeout / 2}]` reading `$timeout`,
on the reasoning that folding it to `set half 15` needs the constant propagated
first and the arithmetic folded after — two passes. That was never the real
obstacle: the `expr`-trust gate was off (see the `factorial` note above), so
the arithmetic could not be folded at all. With the gate on, one pass reaches
`set half 15`, `set threshold 40`, `set second beta`,
`set colours {red green blue}` and `return [expr {$r * $r}]`. Every one of
those was checked against `tclsh9.0.4`: the optimised file prints exactly what
the original prints.

The summary footer lists a propagation as O102 because it was *found*; a
hint-only entry is advice, not an applied edit.

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

The aggressive profile finds **45 rewrites** on the sample input against 35 in
single-pass `full`. Both counts moved when the `expr`-trust gate stopped being
switched off by the `factorial` stanza: `full` gained the folds it could not
previously prove, and `aggressive` lost two, because work its second pass used
to discover is now done in the first. (This figure read 42 until #1962; the committed golden
already said 45 before that fix, so it had drifted by three independently —
the readability, standard and full counts above were and remain correct.) It is the only profile that folds the arithmetic through:
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
**every** profile, `readability` included — including in this sample. It did
not always, and the reason it now does is worth keeping.

O115 (like O101 and O129) is gated on `expr` being provably untouched across
the whole module: a command head the analysis cannot resolve could `rename`
`expr`, so an unresolvable head anywhere turns the gate off for the entire
file. `factorial`'s `return [factorial …]` used to count as such a head — not
because a self-call is unresolvable, but because the enclosing `proc`
statement's body was scanned as if it were that statement's own substitution
surface, and the recursive call was judged there rather than inside the
procedure it belongs to. A body is lowered into its own unit; it is not the
`proc` statement's surface. Once that stopped, the self-call resolved like any
other and the gate stayed on.

Measured on this input, `readability`, O115 on `double_expr`:

| file | before | now |
| --- | --- | --- |
| `double_expr` alone | fires | fires |
| `double_expr` + a call to a defined proc or a builtin | fires | fires |
| `double_expr` + the `factorial` stanza | **not reported** | fires |
| `double_expr` + `proc g {cmd} { $cmd }` | fires | fires |
| `double_expr` + `rename ::expr` in a proc body | — | **not reported** |
| `double_expr` + `proc ::expr` defined in a proc body | — | **not reported** |

The last two rows are the gate doing its job, and they are what makes the
third row safe rather than merely more permissive: a genuine threat to `expr`
inside a procedure body still turns the gate off, reached through that
procedure's own unit.

The fourth row is **not** what the gate's description claims, and was already
so before this change: a dynamic command head inside a proc body does not
switch the gate off, though by the stated rule it should. That is a real
remaining gap in the same family, recorded separately rather than papered over
here.

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
