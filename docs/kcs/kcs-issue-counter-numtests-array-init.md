# KCS: tcltest `numTests(Failed)` reads as empty string in compiled bundle

> **Audience:** Developer
> **Type:** Issue

## Applies to

all-editors

## Question

Why does the counter-bundle summary line print
`Total 9 Passed 9 Skipped 0 Failed` (no digit) instead of
`Total 9 Passed 9 Skipped 0 Failed 0` when every test passes?

## Symptoms

- `tests/external/run_tcllib_test.py::TestCounterBundle::test_counter_bundle_runs_and_passes`
  has a regex workaround matching `Failed\s*(\d*)` (digit is optional)
  rather than the tclsh-canonical `Failed\s+(\d+)`.
- `::tcltest::numTests(Failed)` reads back as an empty string rather
  than `0` when the counter was never incremented.
- Other `numTests(*)` slots (Total, Passed, Skipped) print as expected
  because they were incremented at least once during the run.

## Answer

Two divergent code paths write to the same logical variable:

- `ArrayDefault` (invoked via the interpreter during
  `::tcltest::Option` bootstrapping) stores the initial value under
  the bare name `numTests` — it calls `array set $varName pairs`
  where `$varName` resolves dynamically to `numTests`.
- Compiled `::tcltest::*` procs have a static `variable numTests`
  alias, so their reads and writes go through
  `tcl_array_get("::tcltest::numTests", ...)` — a fully-qualified
  name.

When `tcltest::test` passes, the compiled proc calls
`incr numTests(Passed)` which reads the null value at the qualified
name, adds 1, and writes it back — that's why `Passed` prints `9`.
When `Failed` is never incremented, the qualified slot stays as the
null TclObj the array was initialised with, whose string rep is `""`.

## Workaround

The test regex accepts the missing digit and treats it as zero:

```python
m = re.search(
    r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s*(\d*)",
    stdout,
)
failed = int(m.group(4)) if m.group(4) else 0
```

## Proper fix (follow-up)

The compiler should resolve the `variable numTests` alias to the same
fully-qualified name (`::tcltest::numTests`) that `ArrayDefault` uses
when it stores the initial value, or `ArrayDefault` should itself use
the qualified name. Either direction works; what matters is that the
read and the initial write agree on the namespace.

Remove the `Failed\s*(\d*)` workaround from
`tests/external/run_tcllib_test.py` when the namespace-array-init
bug is fixed.
