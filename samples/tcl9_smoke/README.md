# Tcl 9 Smoke Samples

Minimal, self-contained `.tcl` programs that exercise individual
primitives **without** using `tcltest`.  Each sample sits next to
a `.expected` file containing the stdout we expect the compiled
WASM to emit when the sample runs.  The pair serves two purposes:

1. **Regression smoke** — compile a sample to WASM, run it under
   wasmtime, and compare the captured stdout against its sibling
   `.expected` byte-for-byte.  A failing sample pinpoints the
   primitive the commit broke without requiring tcltest's harness to
   be healthy.  The Python runner that did this for the whole tree
   went with the rest of Python; there is currently no automated
   harness, so run samples by hand (below) until one is rebuilt.
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

Evaluate a sample through the Rust runtime and diff it against its
expectation (`--quiet` matches `tclsh script.tcl`: only `puts` reaches
stdout):

```sh
cd runtime/rust
sample=../../samples/tcl9_smoke/<primitive>/NN_<aspect>
cargo run --quiet --example run_script -- --quiet "$sample.tcl" \
    | diff - "$sample.expected"
```

No output from `diff` means the sample matches its expectation.  To check
the compiled path instead, `tcl compwasm` emits the module — note it
imports the runtime's host functions, so a bare `wasmtime` cannot
instantiate it.

## Adding a sample

1. Write a self-contained Tcl program that prints observable
   output via `puts`.  Avoid `tcltest`, `eval $dyn`, or anything
   that requires I/O beyond stdout.
2. Figure out the expected stdout.  Two options:
   * Run `tclsh9.0 path/to/sample.tcl > path/to/sample.expected`
     if you have Tcl 9 installed locally.
   * Hand-author the `.expected` based on the Tcl 9 man pages.
3. Rerun the run-and-diff above; a silent `diff` confirms our
   implementation produces the same stdout.

## When a sample fails

`diff` shows expected vs actual.  Treat the failure as a
regression of the compiler or runtime, NOT of the sample —
the `.expected` is the source of truth (pinned to Tcl 9 reference
behaviour).  If Tcl 9 semantics change, regenerate the `.expected`
and note the version bump in the commit.
