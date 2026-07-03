# Error recovery: design for the Rust port

> **Status:** Design, validated by prototypes. (Python is now fully
> retired on this branch; the recovery design is realised in the Rust
> green tree — see the current implementation, not the historical Python
> `tokenise_recovering`/`detect_recovery` described in
> [`error-recovery.md`](error-recovery.md).) This document records the
> design the prototypes validated: it pays off in the incremental green
> tree the Rust port builds, and was the de-risked blueprint for it.
>
> Validating prototypes: these were intentionally **not kept in-tree** (they
> instrument the hot lexer and add no production value), so they live only in
> git history. The complete validation record — `tests/proto_level_index.py`,
> `tests/proto_expr_level_index.py`, `tests/proto_role_recovery.py`, and the
> flag-guarded `track_levels` capture in `compiler/parsing/lexer.py` — is present
> at commit **`4ebd320`** (added across `82d0d6a`, `017eb34`, `4ebd320`; removed
> in `ab3acde`). To re-run or inspect them:
>
> ```bash
> git show 4ebd320:tests/proto_level_index.py        # inspect a prototype
> git worktree add /tmp/recovery-protos 4ebd320       # full tree to re-run
> cd /tmp/recovery-protos && uv run python tests/proto_level_index.py
> ```

## TL;DR

Recovery reduces to **one idea applied per sublanguage** plus a router:

1. A **structural-state index** captured during the *single* first lex — the
   lexer's own entry-state recorded per offset/token. From it, the effect of an
   inserted closer (does it balance, where does it close) is computed with a
   prefix lookup + a sparse forward walk — **no re-lex**.
2. The **command registry's `ArgRole`** classifies an unterminated `{` as a
   script (`BODY`), an expression (`EXPR`), or data, to *route* recovery to the
   right sublanguage.

In a **batch** tokeniser (today's Python) the index buys nothing — token
production re-lexes the region anyway. In an **incremental/rowan** tokeniser the
same index *is* the per-node entry-state that enables re-lexing only the changed
region, so recovery and incrementality share one structure. That is why this is
a Rust-port design, not a Python change.

## The structural-state index

During the first scan the lexer already tracks delimiter state and throws it
away. Record it instead, as a sparse snapshot at every effective transition.

**Script sublanguage** — snapshot `(offset, bracket_level, brace_level,
in_quote, bracket_delta)` at each effective `[ ] " { }`, plus **inert spans**
for backslash escapes and `${…}` var-name braces. Absolute level at an offset is
the prefix sum; `bracket_delta ∈ {+1,0,-1}` distinguishes a closer from an
opener and from a quote/brace toggle.

**Expr sublanguage** — the same, with **`paren_level` added** (parens nest in
expr where they don't at script level), and `[…]` / `"…"` / `${…}` / `$arr(idx)`
treated as opaque whole tokens (their interior delimiters never count).

The decision procedure — "does inserting closer `C` at offset `O` close the
outermost open delimiter?" — is identical in both:

```
if O in an inert span:                      return False   # closer is literal
state = last snapshot before O
if O inside a quote/brace where C is inert: return False
L = level(O)                                # prefix sum
if L - 1 == 0:                              return True    # closes at O
# else the insert shifts the running level by -1; a *later* real closer can
# now balance the outer one — walk the sparse deltas forward to the first
# closer whose (shifted) level reaches 0 outside any inert/quote/brace span.
```

### Two corrections the prototypes forced (must carry to Rust)

1. **An extra closer closes nothing.** `)` / `]` / `}` with nothing open must
   *not* drive the running level negative — clamp at 0, matching stack
   semantics. (Expr prototype: 86% → 99.98% on this alone.)
2. **An unterminated opaque token reaching EOF makes the tail inert.** A
   `$arr(idx`, `[…`, or `"…` with no closer swallows to EOF; a closer inserted
   after it is absorbed by *it*, not by the outer delimiter. Extend the inert
   span of such a token to EOF. (Expr prototype: 99.98% → 99.9995%.)

A scalar level is **not** enough — the index must mirror *every* lexer rule
(quotes, braces, escapes, `${}`, opaque/unterminated spans). Concretely:
**store the lexer's entry-state per token**; an approximation diverges.

### Validation (re-run these as Rust tests)

Predict the recovery outcome from the index alone, ground-truthed against a real
re-lex:

- Script `[`/`}`: **28298/28298 = 100%** over ~140k fuzz cases. Convergence by
  dimension: scalar 80.0% → +brace+quote 97.6% → +forward-walk 99.0% →
  +escape/`${}` 100.0%.
- Expr `(`: **191988/191989 = 99.9995%** over ~192k cases (lone residual a deep
  `$arr(idx)` re-lex edge, not a model gap).

## The `ArgRole` router

`registry::resolve_arg_roles(cmd, args)` tags the unterminated `{`’s argument:
`BODY` (script), `EXPR` (expression), or neither (data). Use it to **route**,
not to gate:

- **`BODY`** → recover as a script (close before a de-indented known command).
- **`EXPR`** → recover as an expression, consulting the expr index. Because expr
  content is structured, a bare known-command word at the start of a following
  line is a strong "forgotten close" signal even **without** a de-indent — so
  EXPR braces can use the aggressive bracket-style command-break, unlike data.
  (This is the one additive win also being shipped in Python; see below.)
- **data** → keep the conservative de-indent heuristic.

### Caveat that cost a reverted change — do not repeat it

Do **not** *suppress* recovery for data braces on the basis of the role. The
prototype hypothesis "DATA + recovered = false positive" was **falsified**: a
de-indented known command is a strong forgotten-close signal *regardless* of
role —

```
array set b32 {
  A 0 B 1
set totp_secret foo     ← user forgot the }; recovering is correct
```

— so gating data braces off regresses real recoveries (it broke 5 tests and was
reverted). The role's value is **routing `EXPR`**, not silencing data.

## How it composes

Semantic **proposes** (role + heuristic candidate offset); syntactic
**validates** (does the index say it balances; veto if the offset is inert);
`EXPR` **bridges** to the parallel expr index. "Inert" means the same thing in
both sublanguages (quotes, braces, escapes, `${}`, unterminated-opaque-at-EOF),
just with `paren` added for expr.

## Cross-implementation correctness contract

The recovery behaviour is pinned by **implementation-agnostic** black-box tests
that must pass against the Rust server unchanged:

- `tests/lsp_e2e/test_recovery_e2e.py` and
  `editors/vscode/src/test/errorRecovery.test.ts` assert the same six contracts
  (C1–C6): unterminated delimiters are flagged; recovery is non-fatal so a proc
  after the break is still a document symbol; the recovered token stream is
  well-formed and re-lexes the tail (incl. pathological/deeply-nested input
  without hanging); no duplicate diagnostics; edits toggle recovery; well-formed
  code is clean.

Language-internal soundness is pinned by the differential oracle
(`tests/test_recovering_lexer_differential.py`) and the standalone fuzz campaign
(`tests/fuzz_recovery_campaign.py`); reimplement their *checks* in Rust against
the new engine.

## Implementation checklist for Rust

1. Green-tree node carries its **entry structural state** (the index), interned
   — it is a deterministic property of the lex, so it caches with the tokens.
2. Recovery's balance/close decision reads the index (prefix + forward walk),
   not a re-lex; incremental reparse reads the same index to bound dirty regions.
3. A parallel expr index over the expr lexer (`paren` added).
4. `ArgRole` routes `BODY`/`EXPR`/data; never suppress data on role alone.
5. Diagnostics are assembled once and **de-duplicated** (first parse + recovered
   re-parse can emit the same warning); the token path and analyser path share
   one detection so they surface byte-identical diagnostics.
6. Port the differential + contract tests as the acceptance gate.
