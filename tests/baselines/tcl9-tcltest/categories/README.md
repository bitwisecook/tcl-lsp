# Tcl 9 tcltest categorisation

Per-stem TOML files placing each known-failing tcltest test ID into one
of four buckets.  The Tcl 9 tcltest harness
(`tests/external/run_tcl9_tests.py`) reads these to decide whether a
failure is a release-blocking bug, a tracked feature gap, a purely-
cosmetic divergence from C Tcl, or a bytecode-shape probe that we
explicitly don't run.

## Buckets

* **must_pass** — observable Tcl 9 user-level semantics: variable
  scoping, control flow, proc dispatch, expr / arithmetic, error
  semantics, list / dict / string ops, catch / return, namespace
  resolution, command introspection.  Failure means a real bug.
  **Default for any test ID not listed below**, so a brand-new
  upstream test fails closed (forces explicit triage rather than
  silently slipping into "tolerated").

* **good_to_have** — real semantic features we haven't implemented yet
  but intend to.  Less-common subcommands, edge cases, features
  blocked behind another stream's work.  Counts are tracked against
  a recorded `baseline.good_to_have_failing` cap — drops are welcome
  but **growth past the baseline fires the gate**, so a previously-
  passing good_to_have test that regresses surfaces even though its
  ID is already listed in the bucket.

* **just_to_match_ctcl** — divergences from C Tcl that we do not claim
  to match but where the test still runs (and fails) on our side:
  * exact error wording where ours is functionally equivalent
    (different word order, more / fewer surrounding quotes, etc.);
  * bignum precision beyond ``tcl_arith``'s parity claim;
  * dict insertion-order shimmer, refcount-probe outputs, internal-
    rep identity tests;
  * `tcl::test` / `memory` / internal-C-extension commands when they
    happen to slip past the upstream `-constraints` guard;
  * tests of behaviour we explicitly diverge on (segment-driver vs.
    asyncify-only paths).

  Counted for visibility, never gating.

* **skip** — tests we explicitly do NOT run.  Reserved for probes
  whose pass criterion is bytecode-internal: `info frame` source-line
  tables emitted by the bcc compiler, `info cmdcount` per-instruction
  dispatch counter, TIP 280 source-line propagation through compiled
  `[subst]`, peephole / shimmer shape, `::tcl::dict::*` /
  `::tcl::test` private namespaces, proc-table refcount probes,
  `info args` of `[info commands]`-shape tests.  The WASM target
  emits its own instruction sequence and explicitly does not match
  bcc shape; running these costs cycles and the failures are noise.
  The harness injects `tcltest::configure -skip` for every entry so
  they land in the **Skipped** summary count alongside upstream
  constraint-driven skips.

The ``skip`` / ``just_to_match_ctcl`` distinction is deliberate:
``skip`` is the strong claim "fundamentally inapplicable to a
non-bcc target", while ``just_to_match_ctcl`` is the weaker "passes
for the wrong reasons in C tcl and we choose not to mirror those
reasons".  Reviewers reading a triage decision can tell at a glance
whether the failing test is asking us to mimic a bytecode VM
internal vs. asking us to mimic a textual diagnostic.

## Gate

```
PASS  if  must_pass failures == 0
      AND good_to_have failures <= baseline.good_to_have_failing
FAIL  otherwise
```

`just_to_match_ctcl` and `skip` counts are recorded in the per-file
JSON telemetry (``--tcl9-report=…``) but never gating.

## File format

One file per stem, located at
``tests/baselines/tcl9-tcltest/categories/<stem>.toml``.  Test IDs
are stored as the *suffix* after the stem prefix — i.e. ``"3.5"`` for
``dict-3.5`` — so the seed file isn't redundant with its filename.
The harness accepts either form when it bucket-of-queries.

```toml
# Stem-level escape hatches (optional).
trap_allowed = false        # set true when a stem currently traps
                            # mid-run pre-existingly and reaching
                            # the summary requires upstream work.

good_to_have = [
  "3.5",                    # dict-3.5: dict get with nested path
                            #   traversal — real feature, not yet
                            #   implemented.
]

just_to_match_ctcl = [
  "24.24",                  # dict-24.24: bignum precision beyond
                            #   what tcl_arith claims to match.
]

skip = [
  "23.1",                   # dict-23.1: bytecode compile-crash
                            #   regression — uses ``::tcl::dict::for``
                            #   and ``info frame -1``, probes C bcc
                            #   shape.  Routed through tcltest's
                            #   own ``-skip`` so it lands in the
                            #   Skipped column.
]

[baseline]
# Snapshot at triage time.  ``good_to_have_failing`` MAY drop
# (better) but MUST NOT grow — a previously-passing test slipping
# into the good_to_have bucket fires the gate even though the
# test name is already listed.  The other two are recorded for
# visibility only — neither direction is gating.
good_to_have_failing = 1
just_to_match_ctcl_failing = 1
skip_failing = 1
```

Top-level keys ``good_to_have`` / ``just_to_match_ctcl`` / ``skip``
carry arrays of test-ID suffixes.  Anything not listed is implicitly
``must_pass``.  Inline ``# comments`` after each entry are
encouraged so the next reader doesn't have to re-derive the
reasoning.

The loader (`tests/external/_tcl9_categories.py`) rejects:

* **Unknown top-level keys** — typo guard (``goodto_have = […]``
  would otherwise silently disable the list).
* **Same ID in two buckets** — every test lives in exactly one
  bucket so the harness can pick a winner.
* **`baseline.<bucket>_failing` exceeding the listed-test count** —
  bookkeeping must reference *named* tests so we can detect a
  shape change.
* **Non-string list entries / non-table `[baseline]`** — TOML
  structural errors.

The loader is cached per-stem with a thread-safe lock so a
200-test sweep parses each file exactly once per pytest worker.

## Workflow

1. **New failure**: defaults to ``must_pass`` — the gate fails
   until you triage it.
2. **Real semantic gap**: add to ``good_to_have`` and bump
   ``baseline.good_to_have_failing``.
3. **Functionally-equivalent wording / display**: add to
   ``just_to_match_ctcl``.
4. **Bytecode-internal probe**: add to ``skip`` (the harness will
   inject ``tcltest::configure -skip`` so the test never runs).
5. **Fix lands**: drop the entry — the must_pass-by-default rule
   keeps regressions out.
