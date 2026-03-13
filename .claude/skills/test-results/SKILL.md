---
name: test-results
description: >
  Query reference test results captured from real tclsh (8.4–9.0).
  Look up individual tests, search groups, view suite summaries,
  and compare results across Tcl versions.
allowed-tools: Bash, Read
---

# Test Results Query

Reads reference test results from `tests/test_reference/<version>/` (captured
by `scripts/capture_reference_test_results.sh`) and provides quick lookups
at any granularity: individual test, group within a suite, or full suite.

## Usage

```bash
python3 .claude/skills/test-results/test_results.py <command> [args...]
```

## Commands

| Command | Arguments | What it does |
|---|---|---|
| `status` | | Show which versions and suites have reference data |
| `summary` | `[VERSION]` | Summary table: suite, total/pass/skip/fail, pass rate |
| `suite` | `SUITE [VERSION]` | Full results for one suite — lists every PASS/SKIP/FAIL |
| `group` | `PATTERN [VERSION]` | All tests matching a glob pattern across suites |
| `lookup` | `TEST_NAME [VERSION]` | Look up one test by name, show status in all versions |
| `compare` | `SUITE` | Cross-version comparison for a suite (8.4 vs 8.5 vs 8.6 vs 9.0) |
| `source` | `TEST_NAME [VERSION]` | Extract test source code, parsed options, and file-level setup |

## Arguments

- **VERSION** — `8.4`, `8.5`, `8.6`, or `9.0`.  Defaults to best available (9.0 > 8.6 > 8.5 > 8.4).
- **SUITE** — Test file name: `compExpr-old` or `compExpr-old.test` (both work).
- **PATTERN** — Glob pattern: `compExpr-old-1.*`, `expr-old-2?.*`, `parse-*`.
  If no wildcards, `*` is appended automatically.
- **TEST_NAME** — Exact test name: `expr-old-29.1`, `compExpr-old-1.1`.

## Examples

### Quick summary
```
$ python3 .claude/skills/test-results/test_results.py summary 9.0

Reference test results — Tcl 9.0
Suite                            Total    Pass    Skip    Fail    Rate
──────────────────────────────── ─────── ─────── ─────── ─────── ──────
compExpr-old.test                    184     184       0       0 100.0%
compExpr.test                         82      82       0       0 100.0%
expr-old.test                        461     426      35       0 100.0%
```

### Look up a single test
```
$ python3 .claude/skills/test-results/test_results.py lookup expr-old-29.1

  expr-old-29.1  SKIP (testexprlong)  in expr-old.test

Across all versions:
  Tcl 8.5: SKIP (testexprlong)
  Tcl 8.6: SKIP (testexprlong)
  Tcl 9.0: SKIP (testexprlong)
```

### Search a group
```
$ python3 .claude/skills/test-results/test_results.py group "compExpr-old-1.*" 9.0

Tests matching 'compExpr-old-1.*' — Tcl 9.0
  12 matches

PASSED (12):
  compExpr-old-1.1  (compExpr-old.test)
  compExpr-old-1.2  (compExpr-old.test)
  ...
```

### Cross-version comparison
```
$ python3 .claude/skills/test-results/test_results.py compare expr-old

Cross-version comparison: expr-old
Version      Total    Pass    Skip    Fail    Rate
────────── ─────── ─────── ─────── ─────── ──────
Tcl 8.4        385     350      35       0 100.0%
Tcl 8.5        461     426      35       0 100.0%
Tcl 9.0        461     426      35       0 100.0%

Tests that differ between versions (76):
  ...
```

### View test source code
```
$ python3 .claude/skills/test-results/test_results.py source incr-1.28 8.6

=== incr-1.28 (incr.test:219 — Tcl 8.6) ===

Reference result: PASS

--- File-level setup ---
proc readonly varName {
    upvar 1 $varName var
    trace add variable var write \
	{apply {{args} {error "variable is read-only"}}}
}

--- Test ---
test incr-1.28 {TclCompileIncrCmd: runtime error, readonly variable} -body {
    set x 123
    readonly x
    list [catch {incr x 1} msg] $msg $::errorInfo
} -match glob -cleanup {
    unset -nocomplain x
} -result {1 {can't set "x": variable is read-only} {*variable is read-only ...}}

--- Parsed ---
Name:        incr-1.28
Description: TclCompileIncrCmd: runtime error, readonly variable
Body:        set x 123 \n readonly x \n list [catch {incr x 1} msg] ...
Result:      1 {can't set "x": variable is read-only} ...
Match:       glob
Cleanup:     unset -nocomplain x
```

## When to use

- To check what real tclsh reports for a test our VM fails
- To determine if a test failure is unique to our VM or also fails in real tclsh
- To see which tests are skipped due to constraints (testexprlong, etc.)
- To compare pass rates across Tcl versions
- To guide which known failures to prioritise fixing

## Prerequisites

Reference data must be captured first:
```bash
./scripts/capture_reference_test_results.sh
```

$ARGUMENTS
