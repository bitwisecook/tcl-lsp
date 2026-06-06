# Error recovery: design for the Rust port

> **Status:** Design, validated by prototypes. The current Python recovery
> (single-pass `tokenise_recovering` + unified `detect_recovery` feeding tokens
> *and* diagnostics) is shipped and described in
> [`error-recovery.md`](error-recovery.md). This document records the design that
> the prototypes validated but which is **deliberately not productionised in
> Python** — it only pays off in the incremental/rowan green tree the Rust port
> will build. It is the de-risked blueprint to implement there.
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

## Rust prototype results (`tcl-lexer/src/structural_index.rs`)

A Rust prototype of the **script bracket dimension** validates the design and
de-risks the productionised engine. Like the Python prototypes it is *not wired
into production*; it lives as a documented module with a `#[cfg(test)]`
differential/fuzz harness. Results:

- **Faithfulness to the lexer:** the index's unterminated-`[` verdict matches the
  production `Lexer`'s `missing close-bracket` warning on **8000/8000** fuzz
  cases — confirming the two-context scanner (top-level *word-based* braces vs
  command-sub *count-based* braces, plus escape / `${…}` inert leaves) mirrors
  the lexer, as the doc requires ("store the lexer's entry-state per token; an
  approximation diverges").
- **C Tcl 9.0.3 reference iff:** on a corpus that isolates the bracket dimension
  (balanced, word-separated braces/quotes), the index agrees with `tclsh9.0`'s
  `info complete` **both ways**. The harness shells out to the reference
  interpreter and skips gracefully when it is unavailable.
- **Realistic recovery:** a single forgotten `]` in real code — the index
  predicts the correct close site and the repair is reference-complete.
- **The two corrections** (extra-closer clamp, opaque-to-EOF) and **scalar
  insufficiency** reproduce the doc's findings as pinned tests.

### Brace dimension (`{` / `}`) — added, same methodology

`BraceIndex` extends the prototype to the second script sublanguage. The brace
rules differ from brackets and are mirrored exactly: a `{` opens a brace word
only at a **word boundary** (mid-word `{` is literal — `a{` is *complete* in C
Tcl 9.0.3); inside a brace word nesting is **verbatim** (`\}` does not close);
`#` at command start begins a **comment** whose braces are ignored (`# {` is
complete); quotes make braces literal; `${…}` is a substitution (balanced =
inert, unterminated = a missing close-brace); and the **extra-characters-after-
close** rule (finding 1 below) is *terminal* for braces too (`{b}{`, `"x"{`,
`{a}}` are all complete). Validated against C Tcl 9.0.3:

- **Reference iff** on a bracket/quote-isolated corpus (both ways, 8000 cases).
- **Necessary condition** on an adversarial *bracketless* corpus (`info
  complete` ⇒ braces balanced).
- **Realistic recovery** — a single forgotten `}` in real code is predicted and
  the repair is reference-complete.
- 12 canonical-semantics pins (word-position, comments, quotes, escapes, `${}`,
  extra-`}`).

**Brace boundary (pinned).** `info complete` parses a `[…]` interior
*recursively as a script* (word-based braces + terminal extra-chars), so
`[set x {b}{` is **complete** even though the outer `[` is unterminated. The
prototype's `scan_cmd_sub` uses the lexer's count-based brace rule and so
over-reports unterminated braces *inside* command substitutions. Faithful
command-sub interiors need the full recursive `Tcl_CommandComplete` parse —
the productionised engine must parse `[…]` interiors as nested scripts, not
count braces.

### Two findings to carry into the productionised engine

1. **`info complete` treats "extra characters after a close-brace/quote" as
   complete.** C Tcl 9.0.3 reports `{b}[` as *complete* (the trailing `[` is a
   terminal "extra characters after close-brace" parse error, **not** a command
   substitution), while the lexer/index see an unterminated `[`. For recovery
   this is a benign over-offer on an already-erroring line, but the productionised
   completeness check must special-case post-word-close extra characters if it
   wants byte-exact `info complete` parity. (`a[` — a bareword command
   substitution — is correctly incomplete, so the rule is specifically about a
   delimiter word immediately followed by a non-separator.)
2. **The naive "replay the prebuilt index with one inserted closer" is not sound
   for adversarial multi-bracket input** (~88% self-consistency vs a full
   re-scan, **exact** for the realistic single-`]` case). Inserting a `]` can
   close a bracket early and re-contextualise the tail (a count-based command-sub
   interior becomes top-level word-based, or vice-versa), split a `\\` pair and
   re-align escapes, or be swallowed by a following quote. This is precisely the
   doc's "command-sub interiors need care": step 2's forward walk must **re-derive
   tail structural context after a hypothetical close**, not replay the original
   deltas verbatim.
