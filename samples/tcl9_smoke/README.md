# Tcl 9 Smoke Samples

Minimal, self-contained `.tcl` programs that exercise individual
primitives **without** using `tcltest`.  Each sample sits next to
a `.expected` file containing the stdout we expect the compiled
WASM to emit when the sample runs.  The pair serves two purposes:

1. **Regression smoke** — `tests/external/run_tcl9_samples.py`
   compiles every `.tcl` under this tree, runs it under wasmtime,
   and asserts the captured stdout matches its sibling `.expected`
   byte-for-byte.  A failing sample pinpoints the primitive the
   commit broke without requiring tcltest's harness to be healthy.
2. **Architectural signal** — the corpus is organised by
   primitive (one directory per concern), so which primitives
   drift after a compiler change is immediately visible from the
   test report.  Complements the `tcl9-triage` table, which
   groups by tcltest file rather than by primitive.

## Layout

```
samples/tcl9_smoke/
  <primitive>/
    NN_<aspect>.tcl          ; the program
    NN_<aspect>.expected     ; exact stdout including trailing \n
```

The numeric prefix orders related samples and keeps `ls` output
stable.  File bodies cite the tcltest test name they mirror in
their header comment.

## Running locally

```
make test-tcl9-samples
```

## Adding a sample

1. Write a self-contained Tcl program that prints observable
   output via `puts`.  Avoid `tcltest`, `eval $dyn`, or anything
   that requires I/O beyond stdout.
2. Figure out the expected stdout.  Two options:
   * Run `tclsh9.0 path/to/sample.tcl > path/to/sample.expected`
     if you have Tcl 9 installed locally.
   * Hand-author the `.expected` based on the Tcl 9 man pages.
3. Rerun `make test-tcl9-samples`; a green result confirms our
   compiler produces the same stdout.

## When a sample fails

The harness prints a unified diff of expected vs actual.  Treat
the failure as a regression of the compiler, NOT of the sample —
the `.expected` is the source of truth (pinned to Tcl 9 reference
behaviour).  If Tcl 9 semantics change, regenerate the `.expected`
and note the version bump in the commit.
