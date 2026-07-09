# KCS: A stress-test suite run reported a failure

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Question

One of the [stress-test suites](kcs-howto-run-the-stress-test-suites.md)
failed — how do I find out what went wrong without re-running the
(inherently timing-dependent) harness?

## Symptoms

- A stress suite run exits with a non-zero status.
- The output contains a line starting `STRESS_FAILURE:`.
- Re-running the same suite does not reliably reproduce the failure —
  the suites exist precisely to catch timing-dependent races.

## Answer

1. Grep the combined output for `STRESS_FAILURE:`. Each line names a
   self-contained reproduction bundle directory.
2. Open the bundle. It holds the exact document text as a
   directly-loadable `.tcl` file, plus a JSON-RPC replay transcript and
   recent server stderr for the LSP suite, or a ready-to-adapt static
   unit-test skeleton for the Rust suite.
3. Find the bundle on disk. The Rust suite writes bundles under
   `$TMPDIR/tcl-lsp-stress-failure-*`. The Python suite writes them
   under `$TMPDIR/tcl-lsp-stress-artifacts/` by default; override with
   `--artifacts-dir` or `TCL_LSP_STRESS_ARTIFACTS`. Both suites use the
   same marker, so one `grep STRESS_FAILURE:` over a combined
   `run_all.sh` run finds every bundle from either half.
4. Turn the bundle into a permanent regression test near the query or
   handler it exercised, matching the suite's own test-fixture
   conventions.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [How do I run the stress-test suites?](kcs-howto-run-the-stress-test-suites.md)
- [`scripts/stress/README.md`](../../scripts/stress/README.md) —
  reproduction-bundle contents in full detail.
