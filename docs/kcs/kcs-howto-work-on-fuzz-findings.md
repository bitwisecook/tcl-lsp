# KCS: How do I triage and fix a differential-fuzzer finding?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

The differential fuzzer recorded a divergence between two engines. How do
I decide whether it is a real bug, find the root cause, fix it in the
right layer, and confirm it is closed?

## Before you start

- The fuzzer lives in `rust/tcl-fuzz`. Build it once with
  `cargo build -p tcl-fuzz`.
- A reference `tclsh` on your `PATH`, at a version that matches the
  engine you are testing.
- The `fuzz-findings` skill drives the tool for you and carries the full
  command reference, including how to build the `runtime-rust` engine.

## Answer

### 1. Check the versions before you believe the finding

A divergence is evidence of a bug **only when both engines speak the same
version of Tcl**. `lt`/`gt`/`le`/`ge`, `isfinite()`/`isinf()`/`isnan()`,
and the namespace-scope global fallback for relative variable names all
differ deliberately between 8.6 and 9.0. Every campaign prints both
engines' releases before it starts and warns when they differ, and every
finding records the reference version, the subject version, and whether
they were skewed. Read those first.

### 2. Classify by category

A finding's category is the verdict that produced it:

| Category | Meaning | What it usually means |
|---|---|---|
| `stdout_mismatch` | The engines produced different output | A real semantic divergence — trace it. |
| `status_mismatch` | The engines disagreed on error versus success | The subject is too permissive or too strict. |
| `error_text_mismatch` | Both errored, but the stderr text differed | Only recorded on request; often just wording. |
| `timeout` | The subject hung | Expensive input, or a genuine non-termination. |

### 3. Replay the seed to get a minimal reproducer

Findings are keyed by their generating seed, so they replay exactly.
Replay prints both engines' output side by side including stderr. Cut the
script down to the smallest expression or command that still diverges.

### 4. Verify what C Tcl actually does

Use the `test-results` skill to look up the reference tests for the
command family, and search the upstream test sources under `tmp/` for the
exact error message text you are trying to match.

### 5. Fix as early in the pipeline as possible

A bug caught while analysing is better than one that only surfaces at run
time, because the user gets the feedback in the editor before running
anything. In order of preference:

1. **Lowering.** Reject a malformed structure so nothing downstream sees
   it.
2. **Diagnostics.** Report the same structure as an error, so the editor
   shows a squiggle.
3. **The runtime.** Validate at run time so the engine rejects it too,
   with the error message C Tcl uses.

Not every fix needs all three layers. But if a structural error is
statically detectable, it **must** also produce a diagnostic.

### 6. Pin it with a regression test

Add a test next to the code you changed, naming the seed it came from, and
assert the error message text — not just that an error occurred. Matching
C Tcl's wording is the point.

## How to tell it worked

Replay the seed. The two engines agree, and the finding no longer
reproduces. There is no separate "fixed" flag to set: the registry
de-duplicates by seed, and a finding is closed when its seed stops
diverging.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [How do I run the C tcltest suite through the bytecode VM?](kcs-howto-run-tcltest-bundles.md)
  — the systematic parity sweep the fuzzer complements.
