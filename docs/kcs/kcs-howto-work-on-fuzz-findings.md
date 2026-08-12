# KCS: How do I work on a differential-fuzzer finding?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

Claude skill

## Question

The differential fuzzer has recorded a divergence. How do I triage it,
fix it, and confirm it is gone?

## Before you start

The fuzzer is the `tcl-fuzz` crate, driven with
`cargo run -q -p tcl-fuzz -- <command>`. It runs each generated Tcl
program through a **pair** of engines and records every divergence. The
engines are `tclvm` (our bytecode virtual machine), `tclsh` (a reference C
Tcl interpreter on your `PATH`), and `runtime-rust` (the tree-walking
interpreter under `runtime/rust`). The default pair is `tclvm` as subject
against `tclsh` as reference.

Findings live under `fuzz-findings/` by default (`--findings DIR` moves
it). Each is one JSON record plus the generated script, keyed by the seed
that produced it, so any finding replays exactly. A non-default pair is
namespaced in its own sub-directory, so two pairs never collide on a seed.

The `fuzz-findings` Claude skill wraps all of this and is the quickest way
to drive it.

## Answer

### 1. See what has diverged

```bash
cargo run -q -p tcl-fuzz -- summary
```

The summary groups the registry by category: `stdout_mismatch` (the
engines printed different output), `status_mismatch` (they disagreed on
whether the script failed), `error_text_mismatch` (both failed but worded
it differently — only recorded when a campaign passes
`--compare-error-text`), and `timeout` (the subject hung).

### 2. Replay the seed

```bash
cargo run -q -p tcl-fuzz -- replay 1772893252
```

This regenerates the script and prints both engines' stdout and stderr
side by side. Add the same `--subject` and `--reference` the finding was
recorded under if it did not come from the default pair.

### 3. Rule out a version difference before anything else

A divergence is evidence of a bug **only when both engines speak the same
release of Tcl**. Tcl 9.0 added string-comparison operators and several
maths functions that 8.6 rejects outright, and it removed a namespace
variable fallback that 8.6 had. Each finding records both engines'
versions and a skew flag, and every campaign prints the two releases
before it starts. Treat every finding from a skewed run as suspect until
the version difference is ruled out.

To run version-matched against an older reference, pin the subject with
`--subject-tcl-version`. Only the engines that accept a version flag can
be pinned; pinning any other subject is refused rather than silently
ignored.

### 4. Fix it as early in the pipeline as you can

A fault caught while compiling is better than one that only appears when
the script runs, because the user sees it in the editor immediately. Work
outwards from the earliest stage that could have caught it: structural
rejection during [lowering](../GLOSSARY.md), then a diagnostic so the
language server reports it, then the runtime check in the command
implementation. Not every fix needs all three, but a
statically detectable structural error should always also produce a
diagnostic.

### 5. Confirm the fix, then pin it

There is no "fixed" flag in the registry. A finding is fixed when its seed
stops reproducing, so rebuild and replay it — the engines should now
agree:

```bash
cargo build -p tcl-vm
cargo run -q -p tcl-fuzz -- replay 1772893252
```

Then add a regression test in the crate that owns the fix, using the
minimal reproducer rather than the whole generated script, and name the
seed in a comment so the next reader can replay it.

## How to tell it worked

`replay` prints no divergence, and a fresh campaign over the same pair
does not record the seed again:

```bash
cargo run -q -p tcl-fuzz -- run --iterations 5000 --verbose
cargo run -q -p tcl-fuzz -- summary
```

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
